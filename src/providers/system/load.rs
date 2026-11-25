//! Load average, process count, logged-in users and the local IP address.
//!
//! Everything here is local and cheap: two small `/proc` reads, a directory
//! count and a UDP socket that is connected (never sent on) to discover which
//! address the kernel would route with.

use std::fs;
use std::net::UdpSocket;
use std::time::Duration;

use super::{SystemProvider, run_with_timeout};

/// The three load averages, in the order the kernel prints them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
    /// Number of currently runnable entities, when the kernel reports it.
    pub running: Option<usize>,
    /// Total number of processes, when the kernel reports it.
    pub total: Option<usize>,
}

impl LoadAverage {
    /// `0.42, 0.58, 0.71` — the way a rice screenshot usually shows it.
    pub fn display(&self) -> String {
        format!("{:.2}, {:.2}, {:.2}", self.one, self.five, self.fifteen)
    }
}

impl SystemProvider {
    /// Parse `/proc/loadavg`.
    pub fn load_average(&self) -> Option<LoadAverage> {
        let raw = fs::read_to_string("/proc/loadavg").ok()?;
        parse_load_average(&raw)
    }

    /// Count the numeric entries under `/proc` — every process has one.
    pub fn process_count(&self) -> Option<usize> {
        let entries = fs::read_dir("/proc").ok()?;
        let count = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_name().to_str().is_some_and(|name| {
                    !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit())
                })
            })
            .count();
        (count > 0).then_some(count)
    }

    /// Logged-in users, from `who`. The command is bounded and optional: a
    /// system without it simply has no users row.
    pub fn logged_in_users(&self) -> Option<Vec<String>> {
        let output = run_with_timeout("who", &[], Duration::from_millis(800))?;
        let mut users: Vec<String> = Vec::new();
        for line in output.lines() {
            if let Some(name) = line.split_whitespace().next()
                && !name.is_empty()
                && !users.iter().any(|seen| seen == name)
            {
                users.push(name.to_string());
            }
        }
        (!users.is_empty()).then_some(users)
    }

    /// The local IPv4 address the kernel would use for outbound traffic.
    ///
    /// The socket is only `connect`ed — a UDP connect sends nothing — so this
    /// works offline as long as a route exists.
    pub fn local_ip(&self) -> Option<String> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        let address = socket.local_addr().ok()?;
        let ip = address.ip();
        if ip.is_loopback() || ip.is_unspecified() {
            return None;
        }
        Some(ip.to_string())
    }
}

/// Parse the five space-separated fields of `/proc/loadavg`.
fn parse_load_average(raw: &str) -> Option<LoadAverage> {
    let mut fields = raw.split_whitespace();
    let one = fields.next()?.parse().ok()?;
    let five = fields.next()?.parse().ok()?;
    let fifteen = fields.next()?.parse().ok()?;
    // The fourth field is "running/total".
    let (running, total) = fields
        .next()
        .and_then(|field| field.split_once('/'))
        .map(|(running, total)| (running.parse().ok(), total.parse().ok()))
        .unwrap_or((None, None));
    Some(LoadAverage {
        one,
        five,
        fifteen,
        running,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_load_average;

    #[test]
    fn parses_the_kernel_layout() {
        let load = parse_load_average("0.42 0.58 0.71 2/512 12345\n").unwrap();
        assert_eq!(load.one, 0.42);
        assert_eq!(load.five, 0.58);
        assert_eq!(load.fifteen, 0.71);
        assert_eq!(load.running, Some(2));
        assert_eq!(load.total, Some(512));
        assert_eq!(load.display(), "0.42, 0.58, 0.71");
    }

    #[test]
    fn missing_progress_field_is_tolerated() {
        let load = parse_load_average("1.00 2.00 3.00").unwrap();
        assert_eq!(load.one, 1.0);
        assert_eq!(load.running, None);
        assert_eq!(load.total, None);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse_load_average("").is_none());
        assert!(parse_load_average("nope 1 2").is_none());
    }

    /// A real `/proc/loadavg` must always parse on Linux.
    #[test]
    fn reads_the_live_load_average() {
        let provider = super::SystemProvider::new();
        let load = provider.load_average().expect("live loadavg parses");
        assert!(load.fifteen >= 0.0);
    }

    #[test]
    fn counts_live_processes() {
        let provider = super::SystemProvider::new();
        let count = provider.process_count().expect("live process count");
        assert!(
            count > 1,
            "at least init and this test are running: {count}"
        );
    }
}
