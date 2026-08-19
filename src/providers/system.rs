use std::collections::BTreeSet;
use std::env;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::util::{basename, strip_wrapping_quotes};

#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("command error: {0}")]
    Command(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl ProviderError {
    pub fn io(error: impl ToString) -> Self {
        Self::Io(error.to_string())
    }

    pub fn parse(error: impl ToString) -> Self {
        Self::Parse(error.to_string())
    }

    pub fn command(error: impl ToString) -> Self {
        Self::Command(error.to_string())
    }

    pub fn not_found(error: impl ToString) -> Self {
        Self::NotFound(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
    pub name: String,
    pub id: Option<String>,
    pub id_like: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_kib: u64,
    pub available_kib: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapInfo {
    pub total_kib: u64,
    pub free_kib: u64,
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

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: String,
    pub device_id: String,
    pub model: Option<String>,
    pub vram_mib: Option<u64>,
}

impl GpuInfo {
    pub fn display_name(&self) -> String {
        match (&self.model, self.vram_mib) {
            (Some(model), Some(vram)) if starts_with_vendor(model, &self.vendor) => {
                format!("{model} ({vram} MiB)")
            }
            (Some(model), Some(vram)) => format!("{} {} ({vram} MiB)", self.vendor, model),
            (Some(model), _) if starts_with_vendor(model, &self.vendor) => model.clone(),
            (Some(model), _) => format!("{} {}", self.vendor, model),
            (None, Some(vram)) => format!("{} {} ({vram} MiB)", self.vendor, self.device_id),
            (None, _) => format!("{} {}", self.vendor, self.device_id),
        }
    }
}

#[derive(Debug, Clone)]
struct PciDeviceName {
    vendor: String,
    device: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo {
    pub interface: String,
    pub ip: String,
    pub ssid: Option<String>,
    pub signal_percent: Option<u8>,
    pub speed_mbps: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioInfo {
    pub backend: String,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemperatureInfo {
    pub zone: String,
    pub temp_celsius: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatetimeInfo {
    pub date: String,
    pub time: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorschemeInfo {
    pub gtk_theme: Option<String>,
    pub bg_color: Option<String>,
    pub fg_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SystemProvider;

impl SystemProvider {
    pub fn new() -> Self {
        Self
    }

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

    pub fn disk_info(&self, mount: &str) -> Result<DiskInfo, ProviderError> {
        read_disk_info(mount)
    }

    pub fn battery_info(&self) -> Result<Vec<BatteryInfo>, ProviderError> {
        read_battery_info()
    }

    pub fn locale(&self) -> Option<String> {
        locale_from_env()
    }

    pub fn network_info(&self) -> Result<NetworkInfo, ProviderError> {
        read_network_info()
    }

    pub fn audio_info(&self) -> Result<AudioInfo, ProviderError> {
        read_audio_info()
    }

    pub fn temperature_info(&self) -> Result<TemperatureInfo, ProviderError> {
        read_temperature_info()
    }

    pub fn datetime_info(&self) -> Result<DatetimeInfo, ProviderError> {
        read_datetime_info()
    }

    pub fn colorscheme_info(&self) -> Result<ColorschemeInfo, ProviderError> {
        read_colorscheme_info()
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

    pub fn cpu_model(&self) -> Result<String, ProviderError> {
        let raw = fs::read_to_string("/proc/cpuinfo").map_err(ProviderError::io)?;
        parse_cpu_model(&raw)
    }

    pub fn memory_info(&self) -> Result<MemoryInfo, ProviderError> {
        let raw = fs::read_to_string("/proc/meminfo").map_err(ProviderError::io)?;
        parse_meminfo(&raw)
    }

    pub fn swap_info(&self) -> Result<SwapInfo, ProviderError> {
        let raw = fs::read_to_string("/proc/meminfo").map_err(ProviderError::io)?;
        parse_swapinfo(&raw)
    }

    pub fn gpu_info(&self) -> Result<Vec<GpuInfo>, ProviderError> {
        let drm_path = Path::new("/sys/class/drm");
        let entries = fs::read_dir(drm_path).map_err(ProviderError::io)?;
        let mut gpus = BTreeSet::new();

        for entry in entries {
            let entry = entry.map_err(ProviderError::io)?;
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }

            let device_path = entry.path().join("device");
            if !device_path.exists() {
                continue;
            }

            let vendor_id = read_trimmed(device_path.join("vendor")).unwrap_or_default();
            let device_id = read_trimmed(device_path.join("device")).unwrap_or_default();
            if vendor_id.is_empty() || device_id.is_empty() {
                continue;
            }

            let product_name = read_trimmed(device_path.join("product_name"))
                .ok()
                .filter(|value| !value.is_empty());
            let pci_device = lookup_pci_device_name(&vendor_id, &device_id);
            let vendor = vendor_name(
                &vendor_id,
                pci_device.as_ref().map(|device| device.vendor.as_str()),
            );
            let model = pci_device.map(|device| device.device).or(product_name);
            let vram_mib = read_gpu_vram(&entry.path().join("device"));
            gpus.insert((vendor, device_id, model, vram_mib));
        }

        if gpus.is_empty() {
            return Err(ProviderError::not_found("no DRM GPU devices found"));
        }

        Ok(gpus
            .into_iter()
            .map(|(vendor, device_id, model, vram_mib)| GpuInfo {
                vendor,
                device_id,
                model,
                vram_mib,
            })
            .collect())
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

fn parse_os_release(raw: &str) -> Result<OsInfo, ProviderError> {
    let mut pretty_name = None;
    let mut name = None;
    let mut version = None;
    let mut id = None;
    let mut id_like = Vec::new();

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
            _ => {}
        }
    }

    if let Some(pretty_name) = pretty_name {
        return Ok(OsInfo {
            name: pretty_name,
            id,
            id_like,
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
        });
    }

    Err(ProviderError::parse("missing os-release name fields"))
}

fn parse_cpu_model(raw: &str) -> Result<String, ProviderError> {
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if matches!(key.trim(), "model name" | "Hardware" | "Processor") {
            return Ok(value.trim().to_string());
        }
    }
    Err(ProviderError::parse("missing cpu model"))
}

fn parse_meminfo(raw: &str) -> Result<MemoryInfo, ProviderError> {
    let mut total_kib = None;
    let mut available_kib = None;
    let mut free_kib = None;

    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let kib = value
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok());
        match key.trim() {
            "MemTotal" => total_kib = kib,
            "MemAvailable" => available_kib = kib,
            "MemFree" => free_kib = kib,
            _ => {}
        }
    }

    let total_kib = total_kib.ok_or_else(|| ProviderError::parse("missing MemTotal"))?;
    let available_kib = available_kib.or(free_kib).ok_or_else(|| {
        ProviderError::parse("missing MemAvailable and MemFree from /proc/meminfo")
    })?;

    Ok(MemoryInfo {
        total_kib,
        available_kib,
    })
}

fn parse_swapinfo(raw: &str) -> Result<SwapInfo, ProviderError> {
    let mut total_kib = None;
    let mut free_kib = None;

    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let kib = value
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok());
        match key.trim() {
            "SwapTotal" => total_kib = kib,
            "SwapFree" => free_kib = kib,
            _ => {}
        }
    }

    Ok(SwapInfo {
        total_kib: total_kib.ok_or_else(|| ProviderError::parse("missing SwapTotal"))?,
        free_kib: free_kib.ok_or_else(|| ProviderError::parse("missing SwapFree"))?,
    })
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

fn read_disk_info(mount: &str) -> Result<DiskInfo, ProviderError> {
    let mount_cstring = CString::new(mount)
        .map_err(|error| ProviderError::parse(format!("invalid mount path: {error}")))?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(mount_cstring.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(ProviderError::io(std::io::Error::last_os_error()));
    }
    let stat = unsafe { stat.assume_init() };
    let fragment_size = stat.f_frsize;
    let total_bytes = stat.f_blocks * fragment_size;
    let available_bytes = stat.f_bavail * fragment_size;

    Ok(DiskInfo {
        mount: mount.to_string(),
        filesystem: root_filesystem_type(mount),
        total_bytes,
        available_bytes,
    })
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

fn locale_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|key| env::var(key).ok())
        .find(|value| !value.trim().is_empty())
}

fn lookup_pci_device_name(vendor_id: &str, device_id: &str) -> Option<PciDeviceName> {
    for path in pci_ids_paths() {
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        if let Some(device) = parse_pci_ids(&raw, vendor_id, device_id) {
            return Some(device);
        }
    }

    None
}

fn pci_ids_paths() -> &'static [&'static str] {
    &[
        "/usr/share/hwdata/pci.ids",
        "/usr/share/misc/pci.ids",
        "/usr/share/pci.ids",
    ]
}

fn parse_pci_ids(raw: &str, vendor_id: &str, device_id: &str) -> Option<PciDeviceName> {
    let vendor_id = normalize_pci_id(vendor_id);
    let device_id = normalize_pci_id(device_id);
    let mut active_vendor = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with('\t') {
            let Some((id, name)) = split_pci_id_line(trimmed) else {
                active_vendor = None;
                continue;
            };
            if id.eq_ignore_ascii_case(&vendor_id) {
                active_vendor = Some(name.to_string());
            } else if active_vendor.is_some() {
                break;
            }
            continue;
        }

        if active_vendor.is_none() || line.starts_with("\t\t") {
            continue;
        }

        let Some((id, name)) = split_pci_id_line(trimmed) else {
            continue;
        };
        if id.eq_ignore_ascii_case(&device_id) {
            return Some(PciDeviceName {
                vendor: active_vendor.unwrap_or_default(),
                device: clean_pci_device_name(name),
            });
        }
    }

    None
}

fn split_pci_id_line(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let id = parts.next()?.trim();
    let name = parts.next()?.trim();
    (!id.is_empty() && !name.is_empty()).then_some((id, name))
}

fn clean_pci_device_name(name: &str) -> String {
    let name = name.trim();
    if let Some(start) = name.find('[')
        && let Some(end) = name.rfind(']')
        && end > start + 1
    {
        return name[start + 1..end].trim().to_string();
    }
    name.to_string()
}

fn normalize_pci_id(value: &str) -> String {
    value
        .trim()
        .strip_prefix("0x")
        .unwrap_or_else(|| value.trim())
        .to_ascii_lowercase()
}

fn vendor_name(vendor_id: &str, fallback: Option<&str>) -> String {
    match normalize_pci_id(vendor_id).as_str() {
        "1002" => "AMD".to_string(),
        "10de" => "NVIDIA".to_string(),
        "8086" => "Intel".to_string(),
        "1af4" => "Virtio".to_string(),
        _ => fallback
            .filter(|value| !value.is_empty())
            .unwrap_or("Unknown")
            .to_string(),
    }
}

fn starts_with_vendor(model: &str, vendor: &str) -> bool {
    model
        .to_ascii_lowercase()
        .starts_with(&vendor.to_ascii_lowercase())
}

fn read_gpu_vram(device_path: &Path) -> Option<u64> {
    let vram_total = device_path.join("mem_info_vram_total");
    if let Ok(raw) = fs::read_to_string(vram_total) {
        if let Ok(bytes) = raw.trim().parse::<u64>() {
            let mib = bytes / (1024 * 1024);
            if mib > 0 {
                return Some(mib);
            }
        }
    }
    let bar_info = device_path.join("mem_info_vram_total");
    if !bar_info.exists() {
        let bar0 = device_path.join("resource0");
        if bar0.exists() {
            if let Ok(metadata) = fs::metadata(&bar0) {
                let size = metadata.len() / (1024 * 1024);
                if size > 0 {
                    return Some(size);
                }
            }
        }
    }
    None
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

fn read_trimmed(path: PathBuf) -> Result<String, ProviderError> {
    fs::read_to_string(path)
        .map(|value| value.trim().to_string())
        .map_err(ProviderError::io)
}

fn read_network_info() -> Result<NetworkInfo, ProviderError> {
    let route_raw = fs::read_to_string("/proc/net/route").map_err(ProviderError::io)?;
    let default_iface = parse_default_interface(&route_raw)
        .ok_or_else(|| ProviderError::not_found("no default network interface found"))?;

    let ip = read_interface_ip(&default_iface).ok_or_else(|| {
        ProviderError::not_found(format!("no IP address found for interface {default_iface}"))
    })?;

    let (ssid, signal_percent, speed_mbps) = read_wifi_info(&default_iface);

    Ok(NetworkInfo {
        interface: default_iface,
        ip,
        ssid,
        signal_percent,
        speed_mbps,
    })
}

fn parse_default_interface(route_raw: &str) -> Option<String> {
    for line in route_raw.lines().skip(1) {
        let mut fields = line.split('\t');
        let iface = fields.next()?.trim();
        let destination = fields.next()?.trim();
        if destination == "00000000" {
            return Some(iface.to_string());
        }
    }
    None
}

fn read_interface_ip(iface: &str) -> Option<String> {
    let output = std::process::Command::new("ip")
        .args(["-4", "addr", "show", "dev", iface])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ip_addr_output(&stdout)
}

fn read_wifi_info(iface: &str) -> (Option<String>, Option<u8>, Option<u64>) {
    let wireless_path = Path::new("/sys/class/net").join(iface).join("wireless");
    if !wireless_path.exists() {
        return (None, None, None);
    }

    let ssid = fs::read_to_string(format!("/sys/class/net/{iface}/wireless/ssid"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let signal_percent = fs::read_to_string(format!("/sys/class/net/{iface}/wireless/link"))
        .ok()
        .and_then(|v| v.trim().parse::<u8>().ok())
        .filter(|&v| v > 0)
        .map(|v| (v as f64 / 70.0 * 100.0).min(100.0) as u8);

    let speed_mbps = fs::read_to_string(format!("/sys/class/net/{iface}/speed"))
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|&v| v > 0)
        .map(|v| v as u64);

    (ssid, signal_percent, speed_mbps)
}

fn parse_ip_addr_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            let addr = rest.split('/').next()?;
            if !addr.is_empty() && addr != "127.0.0.1" {
                return Some(addr.to_string());
            }
        }
    }
    None
}

fn read_colorscheme_info() -> Result<ColorschemeInfo, ProviderError> {
    let theme_name = env::var("GTK_THEME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let config_path = env::var_os("XDG_CONFIG_HOME")
                .map(|v| PathBuf::from(v).join("gtk-3.0/settings.ini"))
                .or_else(|| {
                    env::var_os("HOME")
                        .map(|home| PathBuf::from(home).join(".config/gtk-3.0/settings.ini"))
                });
            config_path.and_then(|path| {
                fs::read_to_string(path).ok().and_then(|raw| {
                    for line in raw.lines() {
                        if let Some((key, value)) = line.split_once('=') {
                            if key.trim() == "gtk-theme-name" {
                                return Some(value.trim().to_string());
                            }
                        }
                    }
                    None
                })
            })
        });

    Ok(ColorschemeInfo {
        gtk_theme: theme_name,
        bg_color: None,
        fg_color: None,
    })
}

fn read_datetime_info() -> Result<DatetimeInfo, ProviderError> {
    let output = std::process::Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        .map_err(ProviderError::io)?;

    if !output.status.success() {
        return Err(ProviderError::command("date command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim().to_string();

    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(ProviderError::parse(format!(
            "unexpected date output: {line}"
        )));
    }

    let date = parts[0].to_string();
    let time = parts[1].to_string();
    let timezone = if parts.len() > 2 {
        parts[2].to_string()
    } else {
        env::var("TZ").unwrap_or_else(|_| "local".to_string())
    };

    Ok(DatetimeInfo {
        date,
        time,
        timezone,
    })
}

fn read_temperature_info() -> Result<TemperatureInfo, ProviderError> {
    let thermal_dir = Path::new("/sys/class/thermal");
    if !thermal_dir.exists() {
        return Err(ProviderError::not_found("/sys/class/thermal not available"));
    }

    let entries = fs::read_dir(thermal_dir).map_err(ProviderError::io)?;
    let mut zones: Vec<(String, f64)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(ProviderError::io)?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }

        let type_path = entry.path().join("type");
        let temp_path = entry.path().join("temp");

        let zone_type = fs::read_to_string(&type_path)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|_| name.clone());

        if let Ok(raw) = fs::read_to_string(&temp_path) {
            if let Ok(millidegrees) = raw.trim().parse::<i64>() {
                zones.push((zone_type, millidegrees as f64 / 1000.0));
            }
        }
    }

    if zones.is_empty() {
        return Err(ProviderError::not_found("no thermal zones found"));
    }

    zones.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (zone, temp) = zones.into_iter().next().unwrap();

    Ok(TemperatureInfo {
        zone,
        temp_celsius: temp,
    })
}

fn detect_pipewire_default_sink() -> Option<String> {
    let output = std::process::Command::new("wpctl")
        .args(["status"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.contains('*') && trimmed.contains("Sinks:") {
            continue;
        }
        if trimmed.contains("Sinks:") {
            continue;
        }
    }
    None
}

fn read_audio_info() -> Result<AudioInfo, ProviderError> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_default();
    let pulse_sock = Path::new(&runtime_dir).join("pulse/native");
    let pipewire_sock = Path::new(&runtime_dir).join("pipewire-0");

    if pipewire_sock.exists() {
        let device = detect_pipewire_default_sink();
        return Ok(AudioInfo {
            backend: "PipeWire".to_string(),
            device,
        });
    }

    if pulse_sock.exists() {
        let device = detect_pulse_default_sink();
        return Ok(AudioInfo {
            backend: "PulseAudio".to_string(),
            device,
        });
    }

    Err(ProviderError::not_found(
        "no audio backend detected (neither PipeWire nor PulseAudio socket found)",
    ))
}

fn detect_pulse_default_sink() -> Option<String> {
    let output = std::process::Command::new("pactl")
        .args(["info"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(sink) = line.strip_prefix("Default Sink: ") {
            let sink = sink.trim();
            if !sink.is_empty() {
                return Some(sink.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        GpuInfo, HostInfo, clean_dmi_value, parse_cpu_model, parse_default_interface,
        parse_ip_addr_output, parse_meminfo, parse_mountinfo_filesystem, parse_os_release,
        parse_pci_ids, parse_swapinfo,
    };

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
    fn parses_cpu_model() {
        let raw = r#"
processor   : 0
model name  : AMD Ryzen 7 7840HS w/ Radeon 780M Graphics
"#;
        let model = parse_cpu_model(raw).unwrap();
        assert!(model.contains("Ryzen 7"));
    }

    #[test]
    fn parses_meminfo() {
        let raw = r#"
MemTotal:       32773524 kB
MemFree:        10926812 kB
MemAvailable:   19534000 kB
"#;
        let memory = parse_meminfo(raw).unwrap();
        assert_eq!(memory.total_kib, 32_773_524);
        assert_eq!(memory.available_kib, 19_534_000);
    }

    #[test]
    fn parses_swapinfo() {
        let raw = r#"
SwapTotal:       8060924 kB
SwapFree:        7000000 kB
"#;
        let swap = parse_swapinfo(raw).unwrap();
        assert_eq!(swap.total_kib, 8_060_924);
        assert_eq!(swap.free_kib, 7_000_000);
    }

    #[test]
    fn parses_mountinfo_filesystem() {
        let raw = "36 29 0:32 / / rw,relatime - btrfs /dev/nvme0n1p2 rw,ssd\n";
        assert_eq!(
            parse_mountinfo_filesystem(raw, "/").as_deref(),
            Some("btrfs")
        );
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

    #[test]
    fn parses_pci_ids_for_gpu_names() {
        let raw = r#"
10de  NVIDIA Corporation
	28e0  AD107M [GeForce RTX 4060 Max-Q / Mobile]
8086  Intel Corporation
	7d55  Meteor Lake-P [Intel Arc Graphics]
"#;

        let nvidia = parse_pci_ids(raw, "0x10de", "0x28e0").unwrap();
        assert_eq!(nvidia.vendor, "NVIDIA Corporation");
        assert_eq!(nvidia.device, "GeForce RTX 4060 Max-Q / Mobile");

        let intel = parse_pci_ids(raw, "8086", "7d55").unwrap();
        assert_eq!(intel.vendor, "Intel Corporation");
        assert_eq!(intel.device, "Intel Arc Graphics");
    }

    #[test]
    fn gpu_display_name_avoids_duplicate_vendor_prefix() {
        let gpu = GpuInfo {
            vendor: "Intel".to_string(),
            device_id: "0x7d55".to_string(),
            model: Some("Intel Arc Graphics".to_string()),
            vram_mib: None,
        };

        assert_eq!(gpu.display_name(), "Intel Arc Graphics");
    }

    #[test]
    fn parses_default_interface_from_route() {
        let raw = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\nwlan0\t00000000\t010A14AC\t0003\t0\t0\t600\t00000000\t0\t0\t0\neth0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0\n";
        assert_eq!(parse_default_interface(raw).as_deref(), Some("wlan0"));
    }

    #[test]
    fn parses_ip_addr_output() {
        let output = "2: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP group default qlen 1000\n    inet 172.20.10.2/28 brd 172.20.10.15 scope global dynamic wlan0\n       valid_lft 84513sec preferred_lft 84513sec\n";
        assert_eq!(parse_ip_addr_output(output).as_deref(), Some("172.20.10.2"));
    }

    #[test]
    fn parses_ip_addr_skips_loopback() {
        let output = "1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN group default\n    inet 127.0.0.1/8 scope host lo\n2: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500\n    inet 192.168.1.100/24 brd 192.168.1.255 scope global wlan0\n";
        assert_eq!(
            parse_ip_addr_output(output).as_deref(),
            Some("192.168.1.100")
        );
    }

    #[test]
    fn parse_ip_addr_returns_none_on_empty() {
        assert_eq!(parse_ip_addr_output(""), None);
        assert_eq!(parse_ip_addr_output("no interfaces found"), None);
    }
}
