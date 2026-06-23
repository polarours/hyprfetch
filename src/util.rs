use std::path::Path;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| path.to_string(), ToString::to_string)
}

pub fn strip_wrapping_quotes(value: &str) -> String {
    value.trim_matches('"').trim().to_string()
}

pub fn humanize_uptime(total_seconds: f64) -> String {
    let mut remaining = total_seconds.max(0.0).floor() as u64;
    let days = remaining / 86_400;
    remaining %= 86_400;
    let hours = remaining / 3_600;
    remaining %= 3_600;
    let minutes = remaining / 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 || parts.is_empty() {
        parts.push(format!("{minutes}m"));
    }
    parts.join(" ")
}

pub fn humanize_kib(kib: u64) -> String {
    let gib = kib as f64 / 1024.0 / 1024.0;
    if gib >= 10.0 {
        format!("{gib:.0} GiB")
    } else {
        format!("{gib:.1} GiB")
    }
}

pub fn format_refresh_hz(refresh_hz: f64) -> String {
    let rounded = refresh_hz.round();
    if (refresh_hz - rounded).abs() < 0.05 {
        format!("{rounded:.0}Hz")
    } else {
        format!("{refresh_hz:.1}Hz")
    }
}

pub fn truncate_visible(value: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    if UnicodeWidthStr::width(value) <= max_chars {
        return value.to_string();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    let mut output = String::new();
    let mut width = 0;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width + 1 > max_chars {
            break;
        }
        output.push(character);
        width += character_width;
    }
    output.push('…');
    output
}

#[cfg(test)]
mod tests {
    use super::{format_refresh_hz, humanize_kib, humanize_uptime, truncate_visible};

    #[test]
    fn humanizes_uptime() {
        assert_eq!(humanize_uptime(61.0), "1m");
        assert_eq!(humanize_uptime(9_061.0), "2h 31m");
        assert_eq!(humanize_uptime(90_061.0), "1d 1h 1m");
    }

    #[test]
    fn humanizes_kib() {
        assert_eq!(humanize_kib(8 * 1024 * 1024), "8.0 GiB");
        assert_eq!(humanize_kib(32 * 1024 * 1024), "32 GiB");
    }

    #[test]
    fn formats_refresh_rate() {
        assert_eq!(format_refresh_hz(60.0), "60Hz");
        assert_eq!(format_refresh_hz(59.94), "59.9Hz");
    }

    #[test]
    fn truncates_long_strings() {
        assert_eq!(truncate_visible("hyprfetch", 5), "hypr…");
        assert_eq!(truncate_visible("hypr", 10), "hypr");
    }

    #[test]
    fn truncates_visible_width() {
        assert_eq!(truncate_visible("你好world", 6), "你好w…");
        assert_eq!(truncate_visible("你好", 1), "…");
    }
}
