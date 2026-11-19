use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::util::{basename, strip_wrapping_quotes};

use super::ProviderError;
use super::SystemProvider;
use super::read_trimmed;

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
    pub name: String,
    pub id: Option<String>,
    pub id_like: Vec<String>,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub vendor: Option<String>,
    pub product: String,
    pub version: Option<String>,
}

impl HostInfo {
    pub fn display_name(&self) -> String {
        if let Some(version) = self.version.as_deref()
            && !self.product.eq_ignore_ascii_case(version)
        {
            return format!("{} ({version})", self.product);
        }

        match self.vendor.as_deref() {
            Some(vendor)
                if !self
                    .product
                    .to_ascii_lowercase()
                    .contains(&vendor.to_ascii_lowercase()) =>
            {
                format!("{vendor} {}", self.product)
            }
            _ => self.product.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub manager: String,
    pub count: usize,
}

fn parse_os_release(raw: &str) -> Result<OsInfo, ProviderError> {
    let mut pretty_name = None;
    let mut name = None;
    let mut version = None;
    let mut id = None;
    let mut id_like = Vec::new();
    let mut variant = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = strip_wrapping_quotes(value);
        match key {
            "PRETTY_NAME" => pretty_name = Some(value),
            "NAME" => name = Some(value),
            "VERSION" => version = Some(value),
            "ID" => id = Some(value),
            "ID_LIKE" => {
                id_like = value.split_whitespace().map(ToString::to_string).collect();
            }
            "VARIANT" | "VARIANT_ID" if !value.is_empty() => {
                variant = Some(value);
            }
            _ => {}
        }
    }

    if let Some(pretty_name) = pretty_name {
        return Ok(OsInfo {
            name: pretty_name,
            id,
            id_like,
            variant,
        });
    }

    if let Some(name) = name {
        let combined = match version {
            Some(version) if !version.is_empty() => format!("{name} {version}"),
            _ => name,
        };
        return Ok(OsInfo {
            name: combined,
            id,
            id_like,
            variant,
        });
    }

    Err(ProviderError::parse("missing os-release name fields"))
}

fn read_host_info() -> Result<HostInfo, ProviderError> {
    let vendor = read_dmi_field("sys_vendor");
    let product = read_dmi_field("product_name")
        .or_else(|| read_dmi_field("board_name"))
        .ok_or_else(|| ProviderError::not_found("no DMI host product name found"))?;
    let version = read_dmi_field("product_version");

    Ok(HostInfo {
        vendor,
        product,
        version,
    })
}

fn read_dmi_field(name: &str) -> Option<String> {
    read_trimmed(PathBuf::from("/sys/devices/virtual/dmi/id").join(name))
        .ok()
        .and_then(clean_dmi_value)
}

fn clean_dmi_value(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let normalized = value.to_ascii_lowercase();
    let placeholders = [
        "none",
        "unknown",
        "undefined",
        "default string",
        "to be filled by o.e.m.",
        "system product name",
        "system version",
    ];
    if placeholders.contains(&normalized.as_str()) {
        None
    } else {
        Some(value.to_string())
    }
}

fn read_package_info() -> Result<Vec<PackageInfo>, ProviderError> {
    let mut packages = Vec::new();

    if let Some(count) = count_pacman_packages() {
        packages.push(PackageInfo {
            manager: "pacman".to_string(),
            count,
        });
    }
    if let Some(count) = count_dpkg_packages() {
        packages.push(PackageInfo {
            manager: "dpkg".to_string(),
            count,
        });
    }
    if let Some(count) = count_rpm_packages() {
        packages.push(PackageInfo {
            manager: "rpm".to_string(),
            count,
        });
    }
    if let Some(count) = count_nix_packages() {
        packages.push(PackageInfo {
            manager: "nix".to_string(),
            count,
        });
    }
    if let Some(count) = count_brew_packages() {
        packages.push(PackageInfo {
            manager: "brew".to_string(),
            count,
        });
    }

    if packages.is_empty() {
        Err(ProviderError::not_found(
            "no supported package database found",
        ))
    } else {
        Ok(packages)
    }
}

fn count_pacman_packages() -> Option<usize> {
    let entries = fs::read_dir("/var/lib/pacman/local").ok()?;
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .count();
    (count > 0).then_some(count)
}

fn count_dpkg_packages() -> Option<usize> {
    let raw = fs::read_to_string("/var/lib/dpkg/status").ok()?;
    let count = raw
        .lines()
        .filter(|line| line.starts_with("Package:"))
        .count();
    (count > 0).then_some(count)
}

fn count_rpm_packages() -> Option<usize> {
    let output = std::process::Command::new("rpm")
        .args(["-qa", "--queryformat", "%{NAME}\n"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|line| !line.is_empty()).count();
    (count > 0).then_some(count)
}

fn count_nix_packages() -> Option<usize> {
    let entries = fs::read_dir("/nix/store").ok()?;
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".drv"))
        })
        .count();
    (count > 0).then_some(count)
}

fn count_brew_packages() -> Option<usize> {
    let cellar = if Path::new("/opt/homebrew/Cellar").exists() {
        Path::new("/opt/homebrew/Cellar")
    } else if Path::new("/usr/local/Cellar").exists() {
        Path::new("/usr/local/Cellar")
    } else {
        return None;
    };

    let entries = fs::read_dir(cellar).ok()?;
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .count();
    (count > 0).then_some(count)
}

fn terminal_from_env() -> Option<String> {
    let env_candidates = [
        ("TERM_PROGRAM", None),
        ("KITTY_WINDOW_ID", Some("kitty")),
        ("ALACRITTY_LOG", Some("alacritty")),
        ("WEZTERM_EXECUTABLE", Some("wezterm")),
        ("KONSOLE_VERSION", Some("konsole")),
        ("GHOSTTY_RESOURCES_DIR", Some("ghostty")),
        ("WT_SESSION", Some("windows-terminal")),
    ];

    for (key, alias) in env_candidates {
        if let Ok(value) = env::var(key) {
            return Some(alias.map_or_else(|| basename(&value), ToString::to_string));
        }
    }

    env::var("TERM")
        .ok()
        .filter(|value| value != "dumb")
        .map(|value| value.to_string())
}

fn parent_process_chain() -> Vec<String> {
    let mut names = Vec::new();
    let mut pid = std::process::id();

    for _ in 0..12 {
        let Some(ppid) = read_parent_pid(pid) else {
            break;
        };
        if ppid == 0 {
            break;
        }
        let Some(name) = read_process_name(ppid) else {
            break;
        };
        names.push(name.clone());
        pid = ppid;
    }

    names
}

fn read_parent_pid(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let end = stat.rfind(')')?;
    let remainder = stat.get(end + 2..)?;
    remainder
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u32>().ok())
}

fn read_process_name(pid: u32) -> Option<String> {
    read_trimmed(PathBuf::from(format!("/proc/{pid}/comm"))).ok()
}

fn is_shell(name: &str) -> bool {
    matches!(
        name,
        "bash" | "zsh" | "fish" | "sh" | "dash" | "nu" | "xonsh"
    )
}

fn is_ignorable_process(name: &str) -> bool {
    matches!(name, "sudo" | "env" | "systemd" | "codex" | "python")
}

impl SystemProvider {
    pub fn os_info(&self) -> Result<OsInfo, ProviderError> {
        let raw = fs::read_to_string("/etc/os-release").map_err(ProviderError::io)?;
        parse_os_release(&raw)
    }

    pub fn kernel_version(&self) -> Result<String, ProviderError> {
        fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|value| value.trim().to_string())
            .map_err(ProviderError::io)
    }

    pub fn host_info(&self) -> Result<HostInfo, ProviderError> {
        read_host_info()
    }

    pub fn package_info(&self) -> Result<Vec<PackageInfo>, ProviderError> {
        read_package_info()
    }

    pub fn uptime_seconds(&self) -> Result<f64, ProviderError> {
        let raw = fs::read_to_string("/proc/uptime").map_err(ProviderError::io)?;
        let value = raw
            .split_whitespace()
            .next()
            .ok_or_else(|| ProviderError::parse("missing uptime field"))?;
        value
            .parse::<f64>()
            .map_err(|error| ProviderError::parse(format!("invalid uptime: {error}")))
    }

    pub fn shell_name(&self) -> Option<String> {
        env::var("SHELL")
            .ok()
            .map(|value| basename(&value))
            .or_else(|| self.find_shell_in_parents())
    }

    pub fn terminal_name(&self) -> Option<String> {
        terminal_from_env().or_else(|| self.find_terminal_in_parents())
    }

    fn find_shell_in_parents(&self) -> Option<String> {
        parent_process_chain()
            .into_iter()
            .find(|name| is_shell(name))
            .map(|name| name.to_string())
    }

    fn find_terminal_in_parents(&self) -> Option<String> {
        parent_process_chain().into_iter().find_map(|name| {
            if is_shell(&name) || is_ignorable_process(&name) {
                None
            } else {
                Some(name.to_string())
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_os_release() {
        let raw = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
ID_LIKE="archlinux"
"#;
        let info = parse_os_release(raw).unwrap();
        assert_eq!(info.name, "Arch Linux");
        assert_eq!(info.id.as_deref(), Some("arch"));
        assert_eq!(info.id_like, vec!["archlinux".to_string()]);
    }

    #[test]
    fn formats_host_info_without_duplicate_version() {
        let host = HostInfo {
            vendor: Some("Lenovo".to_string()),
            product: "83DG".to_string(),
            version: Some("Legion Y7000P IRX9".to_string()),
        };

        assert_eq!(host.display_name(), "83DG (Legion Y7000P IRX9)");
    }

    #[test]
    fn drops_placeholder_dmi_values() {
        assert_eq!(clean_dmi_value("To Be Filled By O.E.M.".to_string()), None);
        assert_eq!(
            clean_dmi_value("Legion".to_string()).as_deref(),
            Some("Legion")
        );
    }
}
