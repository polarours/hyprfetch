use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;
use super::read_trimmed;

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: String,
    pub device_id: String,
    pub model: Option<String>,
    pub vram_mib: Option<u64>,
    pub usage_percent: Option<f64>,
}

impl GpuInfo {
    pub fn display_name(&self) -> String {
        let base = match (&self.model, self.vram_mib) {
            (Some(model), Some(vram)) if starts_with_vendor(model, &self.vendor) => {
                format!("{model} ({vram} MiB)")
            }
            (Some(model), Some(vram)) => format!("{} {} ({vram} MiB)", self.vendor, model),
            (Some(model), _) if starts_with_vendor(model, &self.vendor) => model.clone(),
            (Some(model), _) => format!("{} {}", self.vendor, model),
            (None, Some(vram)) => format!("{} {} ({vram} MiB)", self.vendor, self.device_id),
            (None, _) => format!("{} {}", self.vendor, self.device_id),
        };
        if let Some(usage) = self.usage_percent {
            format!("{base} ({usage:.1}%)")
        } else {
            base
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PciDeviceName {
    vendor: String,
    device: String,
}

/// A DRM GPU device: vendor, PCI device id, model, VRAM in MiB, sysfs path.
type GpuDevice = (String, String, Option<String>, Option<u64>, PathBuf);

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

fn read_gpu_usage(
    vendor: &str,
    card_path: &Path,
    nvidia_usages: &HashMap<String, f64>,
) -> Option<f64> {
    if vendor.eq_ignore_ascii_case("nvidia") {
        return nvidia_gpu_usage(card_path, nvidia_usages);
    }
    if vendor.eq_ignore_ascii_case("amd") || vendor.eq_ignore_ascii_case("advanced micro devices") {
        return read_amd_gpu_usage(card_path);
    }
    if vendor.eq_ignore_ascii_case("intel") {
        return read_intel_gpu_usage(card_path);
    }
    None
}

/// Utilisation of a single NVIDIA device, matched by PCI bus id.
fn nvidia_gpu_usage(card_path: &Path, nvidia_usages: &HashMap<String, f64>) -> Option<f64> {
    match_nvidia_usage(drm_pci_bus_id(card_path).as_deref(), nvidia_usages)
}

/// Match a reported NVIDIA utilisation against a device.
fn match_nvidia_usage(bus_id: Option<&str>, nvidia_usages: &HashMap<String, f64>) -> Option<f64> {
    if let Some(bus_id) = bus_id
        && let Some(usage) = nvidia_usages.get(bus_id)
    {
        return Some(*usage);
    }

    // Fall back to the only reported value when the bus ids cannot be matched,
    // for example because nvidia-smi reports a different domain width.
    match nvidia_usages.len() {
        1 => nvidia_usages.values().next().copied(),
        _ => None,
    }
}

/// Utilisation of every NVIDIA device, keyed by PCI bus id.
fn read_nvidia_gpu_usages() -> HashMap<String, f64> {
    let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=pci.bus_id,utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return HashMap::new();
    };
    if !output.status.success() {
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let bus_id = fields.next()?.trim();
            let usage = fields.next()?.trim().parse::<f64>().ok()?;
            Some((normalize_pci_bus_id(bus_id), usage))
        })
        .collect()
}

/// PCI bus id of a DRM device, for example `0000:01:00.0`.
fn drm_pci_bus_id(card_path: &Path) -> Option<String> {
    let resolved = fs::canonicalize(card_path.join("device")).ok()?;
    let name = resolved.file_name()?.to_str()?;
    name.contains(':').then(|| normalize_pci_bus_id(name))
}

/// Normalize a PCI bus id to the 8-digit domain form used by `nvidia-smi`.
///
/// Sysfs reports a 4-digit domain (`0000:01:00.0`) while `nvidia-smi` reports
/// an 8-digit one (`00000000:01:00.0`).
fn normalize_pci_bus_id(bus_id: &str) -> String {
    let Some((domain, rest)) = bus_id.split_once(':') else {
        return bus_id.to_string();
    };

    let mut domain = domain.to_string();
    while domain.len() < 8 {
        domain.insert(0, '0');
    }
    format!("{domain}:{rest}")
}

fn read_amd_gpu_usage(card_path: &Path) -> Option<f64> {
    let raw = fs::read_to_string(card_path.join("device/gpu_busy_percent")).ok()?;
    raw.trim().parse::<f64>().ok()
}

fn read_intel_gpu_usage(card_path: &Path) -> Option<f64> {
    let raw = fs::read_to_string(card_path.join("device/gt_perf_status")).ok()?;
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() >= 2 {
        if let Ok(percent) = fields[1].trim_end_matches('%').parse::<f64>() {
            return Some(percent);
        }
    }
    None
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

impl SystemProvider {
    pub fn gpu_info(&self) -> Result<Vec<GpuInfo>, ProviderError> {
        let drm_path = Path::new("/sys/class/drm");
        let entries = fs::read_dir(drm_path).map_err(ProviderError::io)?;
        let mut gpus: BTreeSet<GpuDevice> = BTreeSet::new();

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
            let vram_mib = read_gpu_vram(&device_path);
            gpus.insert((vendor, device_id, model, vram_mib, entry.path()));
        }

        if gpus.is_empty() {
            return Err(ProviderError::not_found("no DRM GPU devices found"));
        }

        // Query the vendor tool once for the whole run instead of once per GPU.
        let nvidia_usages = read_nvidia_gpu_usages();

        Ok(gpus
            .into_iter()
            .map(|(vendor, device_id, model, vram_mib, card_path)| {
                let usage_percent = read_gpu_usage(&vendor, &card_path, &nvidia_usages);
                GpuInfo {
                    vendor,
                    device_id,
                    model,
                    vram_mib,
                    usage_percent,
                }
            })
            .collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

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
            usage_percent: None,
        };

        assert_eq!(gpu.display_name(), "Intel Arc Graphics");
    }

    #[test]
    fn normalizes_pci_bus_id_domains() {
        assert_eq!(normalize_pci_bus_id("0000:01:00.0"), "00000000:01:00.0");
        assert_eq!(normalize_pci_bus_id("00000000:01:00.0"), "00000000:01:00.0");
        assert_eq!(normalize_pci_bus_id("not-a-bus-id"), "not-a-bus-id");
    }

    #[test]
    fn matches_nvidia_usage_by_bus_id() {
        let mut usages = HashMap::new();
        usages.insert("00000000:01:00.0".to_string(), 12.0);
        usages.insert("00000000:02:00.0".to_string(), 34.0);

        assert_eq!(
            match_nvidia_usage(Some("00000000:02:00.0"), &usages),
            Some(34.0)
        );
        assert_eq!(match_nvidia_usage(Some("00000000:03:00.0"), &usages), None);
    }

    #[test]
    fn falls_back_to_the_only_nvidia_usage() {
        let mut usages = HashMap::new();
        usages.insert("00000000:01:00.0".to_string(), 12.0);

        assert_eq!(
            match_nvidia_usage(Some("00000000:09:00.0"), &usages),
            Some(12.0)
        );
        assert_eq!(match_nvidia_usage(None, &usages), Some(12.0));
    }
}
