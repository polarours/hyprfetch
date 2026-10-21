use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;

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

fn locale_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|key| env::var(key).ok())
        .find(|value| !value.trim().is_empty())
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

impl SystemProvider {
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
}
#[cfg(test)]
mod tests {
    use super::*;

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
    fn parses_ip_addr_returns_none_on_empty() {
        assert_eq!(parse_ip_addr_output(""), None);
        assert_eq!(parse_ip_addr_output("no interfaces found"), None);
    }
}
