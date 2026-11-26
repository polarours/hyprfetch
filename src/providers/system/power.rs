//! Chassis type, wall power and Bluetooth devices.
//!
//! The first two are single `/sys` reads. Bluetooth asks `bluetoothctl`, which
//! is the only reliable way to tell "adapter present" from "a device is
//! actually connected", so it carries a timeout and stays opt-in.

use std::fs;
use std::time::Duration;

use super::{SystemProvider, run_with_timeout};

/// A connected power supply, as `name → watts` where the wattage may be
/// unknown (many laptops do not expose `power_now`).
#[derive(Debug, Clone, PartialEq)]
pub struct PowerSupply {
    pub name: String,
    pub online: bool,
    /// Watts, when the kernel reports them.
    pub watts: Option<f64>,
}

impl PowerSupply {
    pub fn display(&self) -> String {
        let state = if self.online {
            "connected"
        } else {
            "disconnected"
        };
        match self.watts {
            Some(watts) => format!("{state} ({watts:.0} W)"),
            None => state.to_string(),
        }
    }
}

impl SystemProvider {
    /// SMBIOS chassis type, mapped to the wording the spec uses.
    pub fn chassis(&self) -> Option<&'static str> {
        let raw = fs::read_to_string("/sys/class/dmi/id/chassis_type").ok()?;
        let code: u8 = raw.trim().parse().ok()?;
        chassis_name(code)
    }

    /// Every `Mains` supply, so a dock and a barrel jack both show up.
    pub fn power_supplies(&self) -> Option<Vec<PowerSupply>> {
        let entries = fs::read_dir("/sys/class/power_supply").ok()?;
        let mut supplies = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if read_text(&path.join("type")).as_deref() != Some("Mains") {
                continue;
            }
            let Some(online) = read_number(&path.join("online")) else {
                continue;
            };
            let watts = read_number(&path.join("power_now"))
                .map(|microwatts| microwatts as f64 / 1_000_000.0)
                .filter(|watts| *watts > 0.0);
            supplies.push(PowerSupply {
                name: entry.file_name().to_string_lossy().to_string(),
                online: online > 0,
                watts,
            });
        }
        if supplies.is_empty() {
            return None;
        }
        // Connected supplies first: that is the interesting one.
        supplies.sort_by_key(|supply| !supply.online);
        Some(supplies)
    }

    /// Names of the Bluetooth devices currently connected.
    pub fn bluetooth_devices(&self) -> Option<Vec<String>> {
        let output = run_with_timeout(
            "bluetoothctl",
            &["devices", "Connected"],
            Duration::from_millis(1500),
        )?;
        let mut devices = Vec::new();
        for line in output.lines() {
            // "Device AA:BB:CC:DD:EE:FF Name"
            let mut parts = line.split_whitespace();
            if parts.next() != Some("Device") {
                continue;
            }
            let _address = parts.next();
            let name: String = parts.collect::<Vec<_>>().join(" ");
            if !name.is_empty() {
                devices.push(name);
            }
        }
        Some(devices)
    }
}

/// Map the SMBIOS chassis code to its name. Unknown codes return `None` rather
/// than a number the reader has to look up.
fn chassis_name(code: u8) -> Option<&'static str> {
    Some(match code {
        1 => "Other",
        2 => "Unknown",
        3 => "Desktop",
        4 => "Low-profile desktop",
        5 => "Pizza box",
        6 => "Mini tower",
        7 => "Tower",
        8 => "Portable",
        9 => "Laptop",
        10 => "Notebook",
        11 => "Handheld",
        12 => "Docking station",
        13 => "All-in-one",
        14 => "Sub-notebook",
        15 => "Space-saving",
        16 => "Lunch box",
        17 => "Main server",
        18 => "Expansion chassis",
        19 => "Sub-chassis",
        20 => "Bus expansion",
        21 => "Peripheral",
        22 => "RAID chassis",
        23 => "Rack mount",
        24 => "Sealed-case PC",
        25 => "Multi-system",
        30 => "Tablet",
        31 => "Convertible",
        32 => "Detachable",
        33 => "IoT gateway",
        34 => "Embedded PC",
        35 => "Mini PC",
        36 => "Stick PC",
        _ => return None,
    })
}

fn read_text(path: &std::path::Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let value = raw.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn read_number(path: &std::path::Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{PowerSupply, chassis_name};

    #[test]
    fn chassis_codes_map_to_names() {
        assert_eq!(chassis_name(9), Some("Laptop"));
        assert_eq!(chassis_name(10), Some("Notebook"));
        assert_eq!(chassis_name(3), Some("Desktop"));
        assert_eq!(chassis_name(23), Some("Rack mount"));
        assert_eq!(chassis_name(0), None);
        assert_eq!(chassis_name(99), None);
    }

    #[test]
    fn power_supply_reads_naturally() {
        let connected = PowerSupply {
            name: "ACAD".into(),
            online: true,
            watts: Some(65.0),
        };
        assert_eq!(connected.display(), "connected (65 W)");

        let unknown_watts = PowerSupply {
            watts: None,
            ..connected.clone()
        };
        assert_eq!(unknown_watts.display(), "connected");

        let unplugged = PowerSupply {
            online: false,
            ..connected
        };
        assert_eq!(unplugged.display(), "disconnected (65 W)");
    }

    #[test]
    fn live_chassis_is_known() {
        let provider = super::SystemProvider::new();
        // Not every VM exposes DMI, so absence is fine; a present value must map.
        if let Some(name) = provider.chassis() {
            assert!(!name.is_empty());
        }
    }
}
