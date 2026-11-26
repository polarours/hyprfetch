//! Scrubbing the output before it becomes public.
//!
//! A fetch tool prints exactly the things a screenshot should not: the account
//! name, the hostname, private addresses, the Wi-Fi SSID, hardware serials.
//! `--redact` replaces those values while keeping every row, label and column
//! in place, so a screenshot still shows the layout and nothing identifying.
//!
//! This is deliberately value-level, not row-level: dropping the battery row
//! would also hide that the module works, and a demo should look like the real
//! output.

use std::env;
use std::fs;

/// Stand-in values. Fixed on purpose: two screenshots taken on different days
/// should not look like two different machines.
const USER_PLACEHOLDER: &str = "user";
const HOST_PLACEHOLDER: &str = "archlinux";
const IP_PLACEHOLDER: &str = "192.168.1.100";
const MAC_PLACEHOLDER: &str = "00:00:00:00:00:00";
const SSID_PLACEHOLDER: &str = "Home";
const SERIAL_PLACEHOLDER: &str = "XXXXXXXXX";

/// Replace every identifying value in `text`.
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();

    // Hostname first: it usually contains the account name, and replacing the
    // longer string first keeps `user-archlinux` from turning into `user-arch`.
    if let Some(host) = hostname() {
        out = replace_token(&out, &host, HOST_PLACEHOLDER);
    }
    if let Some(user) = env::var("USER").ok().filter(|u| u.len() > 2) {
        out = replace_token(&out, &user, USER_PLACEHOLDER);
    }

    out = redact_ipv4(&out);
    out = redact_mac(&out);
    out = redact_ssid(&out);
    out = redact_serials(&out);
    out
}

fn hostname() -> Option<String> {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|name| !name.is_empty())
}

/// Replace a whole token, not a substring: `user` inside `username` must stay.
fn replace_token(text: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if text[index..].starts_with(needle) {
            let before_ok =
                index == 0 || !bytes[index - 1].is_ascii_alphanumeric() && bytes[index - 1] != b'_';
            let after = index + needle.len();
            let after_ok = after >= bytes.len()
                || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
            if before_ok && after_ok {
                out.push_str(replacement);
                index = after;
                continue;
            }
        }
        // advance one char, not one byte
        let ch = text[index..].chars().next().unwrap_or('\0');
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// `172.24.172.43` → the placeholder, keeping any `/prefix` untouched.
fn redact_ipv4(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            if let Some((end, octets)) = ipv4_at(text, index)
                && octets == 4
            {
                out.push_str(IP_PLACEHOLDER);
                index = end;
                continue;
            }
            // No address here. Skip the whole digit run, or `999.1.1.1` would
            // retry from its second digit and match `99.1.1.1`.
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                out.push(bytes[index] as char);
                index += 1;
            }
            continue;
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// Parse an IPv4 at `start`; returns the end offset and the octet count.
fn ipv4_at(text: &str, start: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut index = start;
    let mut octets = 1;
    loop {
        let digits_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let digits = index - digits_start;
        if digits == 0 || digits > 3 {
            return None;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            octets += 1;
            continue;
        }
        break;
    }
    // A bare number is not an address; require every octet to be <= 255.
    if octets != 4 {
        return None;
    }
    for part in text[start..index].split('.') {
        if part.parse::<u16>().ok()? > 255 {
            return None;
        }
    }
    Some((index, octets))
}

/// `aa:bb:cc:dd:ee:ff` → the placeholder.
fn redact_mac(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_hexdigit() {
            let mut cursor = index;
            let mut groups = 0;
            loop {
                let mut digits = 0;
                while cursor < bytes.len() && bytes[cursor].is_ascii_hexdigit() && digits < 2 {
                    cursor += 1;
                    digits += 1;
                }
                if digits == 0 || digits > 2 {
                    break;
                }
                groups += 1;
                if cursor < bytes.len() && bytes[cursor] == b':' && groups < 6 {
                    cursor += 1;
                    continue;
                }
                break;
            }
            if groups == 6 {
                out.push_str(MAC_PLACEHOLDER);
                index = cursor;
                continue;
            }
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// `SSID: Something` → `SSID: Home`, up to the next comma or line end.
fn redact_ssid(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find("SSID: ") {
        out.push_str(&rest[..position]);
        out.push_str("SSID: ");
        out.push_str(SSID_PLACEHOLDER);
        rest = &rest[position + 6..];
        // eat the original value
        let cut = rest.find([',', '\n']).unwrap_or(rest.len());
        rest = &rest[cut..];
    }
    out.push_str(rest);
    out
}

/// `[BATTERY (L23M4PK5)]` and friends: keep the label, hide the serial.
fn redact_serials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let Some(open) = line.find("BATTERY (")
            && let Some(close) = line[open..].find(')')
        {
            let start = open + "BATTERY (".len();
            let end = open + close;
            out.push_str(&line[..start]);
            out.push_str(SERIAL_PLACEHOLDER);
            out.push_str(&line[end..]);
            continue;
        }
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{redact_ipv4, redact_mac, redact_serials, redact_ssid, replace_token};

    #[test]
    fn ipv4_addresses_are_swapped() {
        let text = "[NETWORK] 172.24.172.43 (wlan0), SSID: UPC";
        let redacted = redact_ipv4(text);
        assert!(redacted.contains("192.168.1.100"));
        assert!(!redacted.contains("172.24"));
    }

    #[test]
    fn version_numbers_survive() {
        // `7.3.5-arch1-1` has three octets; a kernel version must not be IP-scrubbed.
        let text = "[KERNEL] 7.3.5-arch1-1  [CPU] 9.7%";
        assert_eq!(redact_ipv4(text), text);
    }

    #[test]
    fn out_of_range_octets_are_not_addresses() {
        assert_eq!(redact_ipv4("999.1.1.1"), "999.1.1.1");
    }

    #[test]
    fn mac_addresses_are_swapped() {
        let redacted = redact_mac("[LINK] aa:bb:cc:dd:ee:ff up");
        assert!(redacted.contains("00:00:00:00:00:00"));
        assert!(!redacted.contains("aa:bb"));
    }

    #[test]
    fn ssid_values_are_replaced_but_the_label_stays() {
        let redacted = redact_ssid("172.24.172.43 (wlan0), SSID: UPC, Signal: 88%");
        assert!(redacted.contains("SSID: Home"));
        assert!(redacted.contains("Signal: 88%"));
        assert!(!redacted.contains("UPC"));
    }

    #[test]
    fn battery_serials_are_replaced() {
        let redacted = redact_serials("[BATTERY (L23M4PK5)] 99% [Not charging]\n");
        assert!(redacted.contains("[BATTERY (XXXXXXXXX)]"));
        assert!(redacted.contains("99%"));
        assert!(!redacted.contains("L23M4PK5"));
    }

    #[test]
    fn tokens_are_replaced_whole() {
        // Same word, so the text must come back untouched — and `username`
        // must not be treated as a match for `user`.
        assert_eq!(
            replace_token("a user's username", "user", "user"),
            "a user's username"
        );
        assert_eq!(replace_token("user@host", "user", "u"), "u@host");
        // the real account name is longer, so it is replaced
        assert_eq!(
            replace_token("polarours@polarours-archlinux", "polarours", "user"),
            "user@user-archlinux"
        );
    }
}
