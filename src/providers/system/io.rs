//! Disk and network throughput, sampled over a short window.
//!
//! `/proc/diskstats` and `/proc/net/dev` are counters, not rates, so both need
//! two reads with a delay in between. That makes them the slowest modules in
//! the set — which is why they are opt-in and never part of the default order.

use std::fs;
use std::thread;
use std::time::Duration;

use super::SystemProvider;

/// The sampling window. Long enough for a steady number, short enough that a
/// person waiting for a fetch does not notice.
const SAMPLE_WINDOW: Duration = Duration::from_millis(150);

const SECTOR_BYTES: f64 = 512.0;

/// Bytes per second in each direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Throughput {
    pub read: f64,
    pub write: f64,
}

impl Throughput {
    /// `12.3 MB/s read, 4.5 MB/s write`, scaled to whatever unit reads best.
    pub fn display(&self) -> String {
        format!(
            "{} read, {} write",
            format_rate(self.read),
            format_rate(self.write)
        )
    }
}

/// Human units, binary so the numbers match what `btop` and friends show.
fn format_rate(bytes: f64) -> String {
    const UNITS: [&str; 5] = ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

impl SystemProvider {
    /// Disk throughput across all physical disks, excluding partitions and
    /// loop/ram devices so the same bytes are not counted twice.
    pub fn disk_throughput(&self) -> Option<Throughput> {
        let first = disk_counters()?;
        thread::sleep(SAMPLE_WINDOW);
        let second = disk_counters()?;
        let seconds = SAMPLE_WINDOW.as_secs_f64();
        Some(Throughput {
            read: (second.0.saturating_sub(first.0)) as f64 * SECTOR_BYTES / seconds,
            write: (second.1.saturating_sub(first.1)) as f64 * SECTOR_BYTES / seconds,
        })
    }

    /// Network throughput across the real interfaces, skipping `lo` and the
    /// container/virtual bridges that mirror traffic already counted.
    pub fn network_throughput(&self) -> Option<Throughput> {
        let first = net_counters()?;
        thread::sleep(SAMPLE_WINDOW);
        let second = net_counters()?;
        let seconds = SAMPLE_WINDOW.as_secs_f64();
        Some(Throughput {
            read: (second.0.saturating_sub(first.0)) as f64 / seconds,
            write: (second.1.saturating_sub(first.1)) as f64 / seconds,
        })
    }
}

/// Total sectors read and written across physical disks.
fn disk_counters() -> Option<(u64, u64)> {
    let raw = fs::read_to_string("/proc/diskstats").ok()?;
    let mut read = 0u64;
    let mut written = 0u64;
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let name = fields[2];
        if !is_physical_disk(name) {
            continue;
        }
        read += fields[5].parse::<u64>().unwrap_or(0);
        written += fields[9].parse::<u64>().unwrap_or(0);
    }
    Some((read, written))
}

/// A disk, not a partition, and not a loop/ram/dm device.
fn is_physical_disk(name: &str) -> bool {
    if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
        return false;
    }
    // nvme0n1, sda, vda, mmcblk0: the last character must not be a digit
    // unless the whole name ends in the namespace form `n<digits>`.
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        // `nvme0n1p1` is a partition; `nvme0n1` is the disk.
        return !name.contains('p')
            || !name.rsplit_once('p').is_some_and(|(_, tail)| {
                !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())
            });
    }
    // sda / sdb / vda: a trailing digit means a partition.
    !name.chars().last().is_some_and(|c| c.is_ascii_digit())
}

/// Total bytes received and transmitted across real interfaces.
fn net_counters() -> Option<(u64, u64)> {
    let raw = fs::read_to_string("/proc/net/dev").ok()?;
    let mut received = 0u64;
    let mut sent = 0u64;
    for line in raw.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !is_real_interface(name) {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 9 {
            continue;
        }
        received += fields[0].parse::<u64>().unwrap_or(0);
        sent += fields[8].parse::<u64>().unwrap_or(0);
    }
    Some((received, sent))
}

/// Skip loopback and the virtual interfaces whose traffic is counted twice.
fn is_real_interface(name: &str) -> bool {
    const VIRTUAL: [&str; 6] = ["lo", "docker", "veth", "br-", "virbr", "tun"];
    !VIRTUAL.iter().any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{format_rate, is_physical_disk, is_real_interface};

    #[test]
    fn rates_scale_like_btop() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(512.0), "512 B/s");
        assert_eq!(format_rate(2048.0), "2.0 KiB/s");
        assert_eq!(format_rate(5.0 * 1024.0 * 1024.0), "5.0 MiB/s");
        assert_eq!(format_rate(3.5 * 1024.0 * 1024.0 * 1024.0), "3.5 GiB/s");
    }

    #[test]
    fn partitions_and_virtual_disks_are_skipped() {
        assert!(is_physical_disk("sda"));
        assert!(is_physical_disk("nvme0n1"));
        assert!(!is_physical_disk("sda1"));
        assert!(!is_physical_disk("nvme0n1p1"));
        assert!(!is_physical_disk("loop0"));
        assert!(!is_physical_disk("dm-0"));
    }

    #[test]
    fn loopback_and_bridges_are_skipped() {
        assert!(is_real_interface("wlan0"));
        assert!(is_real_interface("enp8s0"));
        assert!(!is_real_interface("lo"));
        assert!(!is_real_interface("docker0"));
        assert!(!is_real_interface("veth1234"));
        assert!(!is_real_interface("br-abc"));
    }

    #[test]
    fn a_live_sample_is_non_negative() {
        let provider = super::SystemProvider::new();
        if let Some(disk) = provider.disk_throughput() {
            assert!(disk.read >= 0.0 && disk.write >= 0.0);
            assert!(!disk.display().is_empty());
        }
        if let Some(net) = provider.network_throughput() {
            assert!(net.read >= 0.0 && net.write >= 0.0);
        }
    }
}
