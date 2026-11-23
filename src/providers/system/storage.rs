use std::ffi::CString;
use std::fs;

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;
use super::read_trimmed;

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub mount: String,
    pub filesystem: Option<String>,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub name: String,
    pub capacity_percent: Option<u8>,
    pub status: Option<String>,
}

fn read_disk_info(mount: &str) -> Result<DiskInfo, ProviderError> {
    let (total_bytes, available_bytes) = filesystem_usage(mount)?;

    Ok(DiskInfo {
        mount: mount.to_string(),
        filesystem: root_filesystem_type(mount),
        total_bytes,
        available_bytes,
    })
}

/// Total and available bytes of the filesystem that holds `path`.
///
/// macOS has no `statvfs` call, so it reports the same numbers through
/// `statfs`, which counts blocks in `f_bsize` units.
#[cfg(not(target_os = "macos"))]
fn filesystem_usage(path: &str) -> Result<(u64, u64), ProviderError> {
    let path_cstring = CString::new(path)
        .map_err(|error| ProviderError::parse(format!("invalid mount path: {error}")))?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(path_cstring.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(ProviderError::io(std::io::Error::last_os_error()));
    }
    let stat = unsafe { stat.assume_init() };
    let fragment_size = stat.f_frsize;
    Ok((
        stat.f_blocks.saturating_mul(fragment_size),
        stat.f_bavail.saturating_mul(fragment_size),
    ))
}

#[cfg(target_os = "macos")]
fn filesystem_usage(path: &str) -> Result<(u64, u64), ProviderError> {
    let path_cstring = CString::new(path)
        .map_err(|error| ProviderError::parse(format!("invalid mount path: {error}")))?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    let result = unsafe { libc::statfs(path_cstring.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(ProviderError::io(std::io::Error::last_os_error()));
    }
    let stat = unsafe { stat.assume_init() };
    let block_size = u64::from(stat.f_bsize);
    Ok((
        stat.f_blocks.saturating_mul(block_size),
        stat.f_bavail.saturating_mul(block_size),
    ))
}

fn root_filesystem_type(mount: &str) -> Option<String> {
    let raw = fs::read_to_string("/proc/self/mountinfo").ok()?;
    parse_mountinfo_filesystem(&raw, mount)
}

fn parse_mountinfo_filesystem(raw: &str, mount: &str) -> Option<String> {
    for line in raw.lines() {
        let Some((pre, post)) = line.split_once(" - ") else {
            continue;
        };
        let Some(mount_point) = pre.split_whitespace().nth(4) else {
            continue;
        };
        if unescape_mountinfo_field(mount_point) != mount {
            continue;
        }
        return post.split_whitespace().next().map(ToString::to_string);
    }
    None
}

fn unescape_mountinfo_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

fn read_battery_info() -> Result<Vec<BatteryInfo>, ProviderError> {
    let entries = fs::read_dir("/sys/class/power_supply").map_err(ProviderError::io)?;
    let mut batteries = Vec::new();

    for entry in entries {
        let entry = entry.map_err(ProviderError::io)?;
        let path = entry.path();
        let power_type = read_trimmed(path.join("type")).unwrap_or_default();
        if power_type != "Battery" {
            continue;
        }
        if matches!(
            read_trimmed(path.join("scope")).ok().as_deref(),
            Some("Device")
        ) {
            continue;
        }
        let fallback_name = entry.file_name().to_string_lossy().to_string();
        let name = read_trimmed(path.join("model_name"))
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_name);
        let capacity_percent = read_trimmed(path.join("capacity"))
            .ok()
            .and_then(|value| value.parse::<u8>().ok());
        let status = read_trimmed(path.join("status"))
            .ok()
            .filter(|value| !value.is_empty());

        batteries.push(BatteryInfo {
            name,
            capacity_percent,
            status,
        });
    }

    if batteries.is_empty() {
        Err(ProviderError::not_found("no battery found"))
    } else {
        Ok(batteries)
    }
}

impl SystemProvider {
    pub fn disk_info(&self, mount: &str) -> Result<DiskInfo, ProviderError> {
        read_disk_info(mount)
    }

    pub fn battery_info(&self) -> Result<Vec<BatteryInfo>, ProviderError> {
        read_battery_info()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mountinfo_filesystem() {
        let raw = "36 29 0:32 / / rw,relatime - btrfs /dev/nvme0n1p2 rw,ssd\n";
        assert_eq!(
            parse_mountinfo_filesystem(raw, "/").as_deref(),
            Some("btrfs")
        );
    }
}
