use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;
use super::run_with_timeout;

/// How long a wireless tool may take to answer.
const WIFI_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// How long an audio tool may take to answer.
const AUDIO_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

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
    if wireless_path.exists() {
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

        if ssid.is_some() || signal_percent.is_some() {
            return (ssid, signal_percent, speed_mbps);
        }
    }

    // Drivers such as iwlwifi do not publish the wireless files, so ask the
    // tools that talk to the driver directly.
    let (ssid, signal, speed) = read_iw_link(iface);
    if ssid.is_some() || signal.is_some() {
        return (ssid, signal, speed);
    }

    read_iwconfig(iface)
}

/// The SSID, signal and link speed of an interface, read with `iw`.
fn read_iw_link(iface: &str) -> (Option<String>, Option<u8>, Option<u64>) {
    let Some(output) = run_with_timeout("iw", &["dev", iface, "link"], WIFI_PROBE_TIMEOUT) else {
        return (None, None, None);
    };
    parse_iw_link(&output)
}

/// The SSID, signal and link speed in the output of `iw dev <iface> link`.
fn parse_iw_link(output: &str) -> (Option<String>, Option<u8>, Option<u64>) {
    let mut ssid = None;
    let mut signal = None;
    let mut speed = None;
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("SSID: ") {
            let value = rest.trim();
            if !value.is_empty() {
                ssid = Some(value.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("signal: ")
            && let Some(dbm) = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<i32>().ok())
        {
            signal = Some(dbm_to_percent(dbm));
        } else if let Some(rest) = line.strip_prefix("tx bitrate: ") {
            speed = parse_link_speed(rest);
        }
    }

    (ssid, signal, speed)
}

/// The SSID, signal and link speed of an interface, read with `iwconfig`.
fn read_iwconfig(iface: &str) -> (Option<String>, Option<u8>, Option<u64>) {
    let Some(output) = run_with_timeout("iwconfig", &[iface], WIFI_PROBE_TIMEOUT) else {
        return (None, None, None);
    };
    parse_iwconfig(&output)
}

/// The SSID, signal and link speed in the output of `iwconfig <iface>`.
fn parse_iwconfig(output: &str) -> (Option<String>, Option<u8>, Option<u64>) {
    let mut ssid = None;
    let mut signal = None;
    let mut speed = None;
    for line in output.lines() {
        if let Some(rest) = line.split("ESSID:").nth(1) {
            let value = rest.trim().trim_matches('"');
            if !value.is_empty() {
                ssid = Some(value.to_string());
            }
        } else if let Some(rest) = line.split("Link Quality=").nth(1)
            && let Some((quality, max)) = rest.split_once('/')
            && let Some(quality) = quality.trim().parse::<u64>().ok()
            && let Some(max) = max
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok())
            && max > 0
        {
            signal = Some((quality as f64 / max as f64 * 100.0).min(100.0) as u8);
        } else if let Some(rest) = line.split("Bit Rate=").nth(1) {
            speed = parse_link_speed(rest);
        }
    }

    (ssid, signal, speed)
}

/// A signal level in dBm as a percentage.
fn dbm_to_percent(dbm: i32) -> u8 {
    ((dbm + 100) * 2).clamp(0, 100) as u8
}

/// A link speed such as `144.4 Mb/s` in megabits per second.
fn parse_link_speed(text: &str) -> Option<u64> {
    let value = text.split_whitespace().next()?;
    value.parse::<f64>().ok().map(|value| value.round() as u64)
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
    let output = run_with_timeout("wpctl", &["status"], AUDIO_PROBE_TIMEOUT)?;
    parse_wpctl_default_sink(&output)
}

/// The default sink in the output of `wpctl status`.
///
/// The sinks are listed under a "Sinks:" heading and the default one is
/// marked with a star.
fn parse_wpctl_default_sink(output: &str) -> Option<String> {
    let mut in_sinks = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.ends_with("Sinks:") {
            in_sinks = true;
            continue;
        }
        if !in_sinks {
            continue;
        }
        if trimmed.ends_with(':') {
            // the next section starts
            break;
        }

        let Some(rest) = trimmed.split_once('*').map(|(_, rest)| rest.trim_start()) else {
            continue;
        };
        // the line starts with the index of the sink, for example "52. "
        let name = match rest.split_once(". ") {
            Some((index, name)) if index.chars().all(|character| character.is_ascii_digit()) => {
                name
            }
            _ => rest,
        };
        let name = name.split('[').next().unwrap_or(name).trim();
        if !name.is_empty() {
            return Some(name.to_string());
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
    let output = run_with_timeout("pactl", &["info"], AUDIO_PROBE_TIMEOUT)?;
    for line in output.lines() {
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

    #[test]
    fn reads_the_wifi_details_from_iw() {
        let output = "Connected to 16:74:9c:41:f2:2b (on wlan0)\n\tSSID: UPC\n\tfreq: 5765.00 MHz\n\tsignal: -46 dBm\n\ttx bitrate: 144.4 MBit/s\n";
        let (ssid, signal, speed) = parse_iw_link(output);

        assert_eq!(ssid.as_deref(), Some("UPC"));
        assert_eq!(signal, Some(100));
        assert_eq!(speed, Some(144));
    }

    #[test]
    fn reads_the_wifi_details_from_iwconfig() {
        let output = "wlan0     IEEE 802.11  ESSID:\"UPC\"  \n          Mode:Managed  Frequency:5.765 GHz  \n          Bit Rate=144.4 Mb/s   Tx-Power=20 dBm   \n          Link Quality=64/70  Signal level=-46 dBm  \n";
        let (ssid, signal, speed) = parse_iwconfig(output);

        assert_eq!(ssid.as_deref(), Some("UPC"));
        assert_eq!(signal, Some(91));
        assert_eq!(speed, Some(144));
    }

    #[test]
    fn ignores_a_wireless_tool_that_reports_nothing() {
        assert_eq!(parse_iw_link(""), (None, None, None));
        assert_eq!(parse_iwconfig(""), (None, None, None));
    }

    #[test]
    fn turns_a_signal_level_into_a_percentage() {
        assert_eq!(dbm_to_percent(-46), 100);
        assert_eq!(dbm_to_percent(-70), 60);
        assert_eq!(dbm_to_percent(-100), 0);
    }

    #[test]
    fn reads_a_link_speed() {
        assert_eq!(parse_link_speed("144.4 Mb/s"), Some(144));
        assert_eq!(parse_link_speed("1.0 Mb/s"), Some(1));
        assert_eq!(parse_link_speed("not a speed"), None);
    }

    #[test]
    fn reads_the_default_sink_from_wpctl() {
        let output = "PipeWire 'pipewire-0' [1.6.9]\n \u{251c}\u{2500} Clients:\n        32. WirePlumber   [1.6.9]\n\nAudio\n \u{251c}\u{2500} Devices:\n \u{2502}      44. AD107 High Definition Audio Controller [alsa]\n \u{2502}  \n \u{251c}\u{2500} Sinks:\n \u{2502}  *   52. Built-in Audio Analog Stereo        [vol: 0.35]\n \u{2502}  \n \u{251c}\u{2500} Sources:\n \u{2502}  *   53. Built-in Audio Analog Stereo        [vol: 0.39]\n";
        assert_eq!(
            parse_wpctl_default_sink(output),
            Some("Built-in Audio Analog Stereo".to_string())
        );
    }

    #[test]
    fn keeps_a_device_name_that_contains_a_dot() {
        let output = " \u{251c}\u{2500} Sinks:\n \u{2502}  *   52. HDA Intel PCH. Analog Stereo   [vol: 0.35]\n";
        assert_eq!(
            parse_wpctl_default_sink(output),
            Some("HDA Intel PCH. Analog Stereo".to_string())
        );
    }

    #[test]
    fn has_no_sink_when_wpctl_lists_none() {
        assert_eq!(parse_wpctl_default_sink(""), None);
        assert_eq!(
            parse_wpctl_default_sink(" \u{251c}\u{2500} Sinks:\n \u{2502}  \n"),
            None
        );
    }
}
