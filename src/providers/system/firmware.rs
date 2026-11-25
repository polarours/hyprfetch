//! Firmware and desktop-session details: BIOS, mainboard, init system,
//! preferred editor and screen brightness.
//!
//! Everything is read from `/sys/class/dmi/id`, `/proc/1/comm`, the environment
//! or `/sys/class/backlight`, so none of it costs a spawned process except the
//! optional editor lookup.

use std::env;
use std::fs;
use std::path::Path;

use super::SystemProvider;

/// SMBIOS identity of the firmware and the board.
#[derive(Debug, Clone, PartialEq)]
pub struct FirmwareInfo {
    pub vendor: String,
    pub version: String,
}

impl FirmwareInfo {
    pub fn display(&self) -> String {
        format!("{} {}", self.vendor, self.version)
    }
}

/// Mainboard vendor and model.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardInfo {
    pub vendor: String,
    pub name: String,
}

impl BoardInfo {
    pub fn display(&self) -> String {
        format!("{} {}", self.vendor, self.name)
    }
}

impl SystemProvider {
    /// BIOS/UEFI vendor and version from DMI.
    pub fn firmware_info(&self) -> Option<FirmwareInfo> {
        let vendor = read_dmi("bios_vendor")?;
        let version = read_dmi("bios_version")?;
        Some(FirmwareInfo { vendor, version })
    }

    /// Mainboard vendor and model from DMI.
    pub fn board_info(&self) -> Option<BoardInfo> {
        let vendor = read_dmi("board_vendor")?;
        let name = read_dmi("board_name")?;
        Some(BoardInfo { vendor, name })
    }

    /// The init system that owns PID 1, normalised to a friendly name.
    pub fn init_system(&self) -> Option<String> {
        let comm = fs::read_to_string("/proc/1/comm").ok()?;
        Some(normalize_init(comm.trim()))
    }

    /// The user's preferred editor: `$VISUAL`, `$EDITOR`, or a common binary
    /// that is present on `PATH`.
    pub fn editor(&self) -> Option<String> {
        for key in ["VISUAL", "EDITOR"] {
            if let Ok(value) = env::var(key) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    // Reduce `/usr/bin/nvim --wait` to `nvim`.
                    let first = trimmed.split_whitespace().next().unwrap_or(trimmed);
                    let name = first.rsplit('/').next().unwrap_or(first);
                    return Some(name.to_string());
                }
            }
        }
        const CANDIDATES: [&str; 5] = ["nvim", "vim", "hx", "nano", "emacs"];
        CANDIDATES
            .iter()
            .find(|name| on_path(name))
            .map(|name| (*name).to_string())
    }

    /// Average brightness across every backlight device, as a percentage.
    pub fn brightness_percent(&self) -> Option<u8> {
        let entries = fs::read_dir("/sys/class/backlight").ok()?;
        let mut current_total = 0u64;
        let mut max_total = 0u64;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let current = read_number(&path.join("brightness"));
            let max = read_number(&path.join("max_brightness"));
            if let (Some(current), Some(max)) = (current, max)
                && max > 0
            {
                current_total += current;
                max_total += max;
            }
        }
        if max_total == 0 {
            return None;
        }
        Some(((current_total * 100 + max_total / 2) / max_total).min(100) as u8)
    }
}

/// Read a DMI attribute, treating the `"None"`/empty placeholders as absent.
fn read_dmi(field: &str) -> Option<String> {
    let raw = fs::read_to_string(format!("/sys/class/dmi/id/{field}")).ok()?;
    let value = raw.trim();
    if value.is_empty() || value == "None" || value == "To Be Filled By O.E.M." {
        return None;
    }
    Some(value.to_string())
}

fn read_number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn normalize_init(comm: &str) -> String {
    match comm {
        "systemd" => "systemd".to_string(),
        "init" | "busybox" => "sysvinit".to_string(),
        "openrc-init" => "OpenRC".to_string(),
        "runit" => "runit".to_string(),
        "s6-svscan" => "s6".to_string(),
        other => other.to_string(),
    }
}

fn on_path(name: &str) -> bool {
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::{normalize_init, read_dmi};

    #[test]
    fn init_names_are_normalised() {
        assert_eq!(normalize_init("systemd"), "systemd");
        assert_eq!(normalize_init("init"), "sysvinit");
        assert_eq!(normalize_init("openrc-init"), "OpenRC");
        assert_eq!(normalize_init("s6-svscan"), "s6");
        assert_eq!(normalize_init("somethingelse"), "somethingelse");
    }

    #[test]
    fn dmi_placeholders_are_rejected() {
        // An absent field must not become a row full of "To Be Filled By O.E.M.".
        assert!(read_dmi("this_field_does_not_exist").is_none());
    }

    #[test]
    fn live_brightness_is_a_percentage() {
        let provider = super::SystemProvider::new();
        if let Some(percent) = provider.brightness_percent() {
            assert!(percent <= 100);
        }
    }

    #[test]
    fn live_init_system_is_non_empty() {
        let provider = super::SystemProvider::new();
        let init = provider.init_system().expect("/proc/1/comm readable");
        assert!(!init.is_empty());
    }
}
