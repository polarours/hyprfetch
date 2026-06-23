use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::{humanize_kib, humanize_uptime};

pub struct OsModule;
pub struct HostModule;
pub struct KernelModule;
pub struct PackagesModule;
pub struct UptimeModule;
pub struct ShellModule;
pub struct TerminalModule;
pub struct CpuModule;
pub struct GpuModule;
pub struct MemoryModule;
pub struct SwapModule;
pub struct DiskModule;
pub struct BatteryModule;
pub struct LocaleModule;
pub struct NetworkModule;
pub struct TemperatureModule;
pub struct AudioModule;
pub struct DatetimeModule;
pub struct ColorschemeModule;

impl Module for OsModule {
    fn id(&self) -> ModuleId {
        ModuleId::Os
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.os_info() {
            Ok(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("OS", info.name.clone())],
                json!({ "name": info.name }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HostModule {
    fn id(&self) -> ModuleId {
        ModuleId::Host
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.host_info() {
            Ok(host) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Host", host.display_name())],
                json!(host),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for KernelModule {
    fn id(&self) -> ModuleId {
        ModuleId::Kernel
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.kernel_version() {
            Ok(kernel) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Kernel", kernel.clone())],
                json!({ "version": kernel }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for PackagesModule {
    fn id(&self) -> ModuleId {
        ModuleId::Packages
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.package_info() {
            Ok(packages) => {
                let rendered = packages
                    .iter()
                    .map(|info| format!("{} ({})", info.count, info.manager))
                    .collect::<Vec<_>>()
                    .join(", ");
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Packages", rendered)],
                    json!(packages),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for UptimeModule {
    fn id(&self) -> ModuleId {
        ModuleId::Uptime
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.uptime_seconds() {
            Ok(seconds) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Uptime", humanize_uptime(seconds))],
                json!({ "seconds": seconds }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for ShellModule {
    fn id(&self) -> ModuleId {
        ModuleId::Shell
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.shell_name() {
            Some(shell) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Shell", shell.clone())],
                json!({ "name": shell }),
            ),
            None => ModuleOutput::unavailable(self.id(), "shell not detected"),
        }
    }
}

impl Module for TerminalModule {
    fn id(&self) -> ModuleId {
        ModuleId::Terminal
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.terminal_name() {
            Some(terminal) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Terminal", terminal.clone())],
                json!({ "name": terminal }),
            ),
            None => ModuleOutput::unavailable(self.id(), "terminal not detected"),
        }
    }
}

impl Module for CpuModule {
    fn id(&self) -> ModuleId {
        ModuleId::Cpu
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.cpu_model() {
            Ok(cpu) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("CPU", cpu.clone())],
                json!({ "model": cpu }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for GpuModule {
    fn id(&self) -> ModuleId {
        ModuleId::Gpu
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.gpu_info() {
            Ok(gpus) => {
                let joined = gpus
                    .iter()
                    .map(|gpu| gpu.display_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("GPU", joined.clone())],
                    json!({ "gpus": gpus }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for MemoryModule {
    fn id(&self) -> ModuleId {
        ModuleId::Memory
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.memory_info() {
            Ok(memory) => {
                let used_kib = memory.total_kib.saturating_sub(memory.available_kib);
                let rendered = format!(
                    "{} / {} ({:.0}%)",
                    humanize_kib(used_kib),
                    humanize_kib(memory.total_kib),
                    memory_usage_percent(used_kib, memory.total_kib)
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Memory", rendered)],
                    json!({
                        "used_kib": used_kib,
                        "available_kib": memory.available_kib,
                        "total_kib": memory.total_kib
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for SwapModule {
    fn id(&self) -> ModuleId {
        ModuleId::Swap
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.swap_info() {
            Ok(swap) => {
                let used_kib = swap.total_kib.saturating_sub(swap.free_kib);
                let rendered = format!(
                    "{} / {} ({:.0}%)",
                    humanize_kib(used_kib),
                    humanize_kib(swap.total_kib),
                    memory_usage_percent(used_kib, swap.total_kib)
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Swap", rendered)],
                    json!({
                        "used_kib": used_kib,
                        "free_kib": swap.free_kib,
                        "total_kib": swap.total_kib
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for DiskModule {
    fn id(&self) -> ModuleId {
        ModuleId::Disk
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.disk_info("/") {
            Ok(disk) => {
                let used_bytes = disk.total_bytes.saturating_sub(disk.available_bytes);
                let mut rendered = format!(
                    "{} / {} ({:.0}%)",
                    humanize_kib(used_bytes / 1024),
                    humanize_kib(disk.total_bytes / 1024),
                    disk_usage_percent(used_bytes, disk.total_bytes)
                );
                if let Some(filesystem) = disk.filesystem.as_deref() {
                    rendered.push_str(&format!(" - {filesystem}"));
                }

                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Disk (/)", rendered)],
                    json!({
                        "mount": disk.mount,
                        "filesystem": disk.filesystem,
                        "used_bytes": used_bytes,
                        "available_bytes": disk.available_bytes,
                        "total_bytes": disk.total_bytes
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for BatteryModule {
    fn id(&self) -> ModuleId {
        ModuleId::Battery
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.battery_info() {
            Ok(batteries) => {
                let rows = batteries
                    .iter()
                    .map(|battery| {
                        OutputRow::new(
                            format!("Battery ({})", battery.name),
                            render_battery_value(
                                battery.capacity_percent,
                                battery.status.as_deref(),
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                ModuleOutput::ok(self.id(), rows, json!(batteries))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for LocaleModule {
    fn id(&self) -> ModuleId {
        ModuleId::Locale
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.locale() {
            Some(locale) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Locale", locale.clone())],
                json!({ "locale": locale }),
            ),
            None => ModuleOutput::unavailable(self.id(), "locale not detected"),
        }
    }
}

impl Module for NetworkModule {
    fn id(&self) -> ModuleId {
        ModuleId::Network
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.network_info() {
            Ok(info) => {
                let mut parts = vec![format!("{} ({})", info.ip, info.interface)];
                if let Some(ref ssid) = info.ssid {
                    parts.push(format!("SSID: {ssid}"));
                }
                if let Some(signal) = info.signal_percent {
                    parts.push(format!("Signal: {signal}%"));
                }
                if let Some(speed) = info.speed_mbps {
                    parts.push(format!("{speed} Mbps"));
                }
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Network", parts.join(", "))],
                    json!({
                        "interface": info.interface,
                        "ip": info.ip,
                        "ssid": info.ssid,
                        "signal_percent": info.signal_percent,
                        "speed_mbps": info.speed_mbps
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for AudioModule {
    fn id(&self) -> ModuleId {
        ModuleId::Audio
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.audio_info() {
            Ok(info) => {
                let display = match &info.device {
                    Some(device) => format!("{} ({})", info.backend, device),
                    None => info.backend.clone(),
                };
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Audio", display)],
                    json!({ "backend": info.backend, "device": info.device }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for TemperatureModule {
    fn id(&self) -> ModuleId {
        ModuleId::Temperature
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.temperature_info() {
            Ok(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new(
                    "Temp",
                    format!("{}°C ({})", info.temp_celsius.round(), info.zone),
                )],
                json!({ "zone": info.zone, "celsius": info.temp_celsius }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for DatetimeModule {
    fn id(&self) -> ModuleId {
        ModuleId::Datetime
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.datetime_info() {
            Ok(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new(
                    "DateTime",
                    format!("{} {} ({})", info.date, info.time, info.timezone),
                )],
                json!({ "date": info.date, "time": info.time, "timezone": info.timezone }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for ColorschemeModule {
    fn id(&self) -> ModuleId {
        ModuleId::Colorscheme
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.colorscheme_info() {
            Ok(info) => {
                let display = info.gtk_theme.clone().unwrap_or_else(|| "unknown".to_string());
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Colors", display)],
                    json!({ "gtk_theme": info.gtk_theme }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn memory_usage_percent(used_kib: u64, total_kib: u64) -> f64 {
    if total_kib == 0 {
        0.0
    } else {
        used_kib as f64 / total_kib as f64 * 100.0
    }
}

fn disk_usage_percent(used_bytes: u64, total_bytes: u64) -> f64 {
    if total_bytes == 0 {
        0.0
    } else {
        used_bytes as f64 / total_bytes as f64 * 100.0
    }
}

fn render_battery_value(capacity_percent: Option<u8>, status: Option<&str>) -> String {
    match (capacity_percent, status) {
        (Some(capacity), Some(status)) => format!("{capacity}% [{status}]"),
        (Some(capacity), None) => format!("{capacity}%"),
        (None, Some(status)) => status.to_string(),
        (None, None) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{disk_usage_percent, memory_usage_percent, render_battery_value};

    #[test]
    fn calculates_memory_usage_percent() {
        assert_eq!(memory_usage_percent(512, 1024), 50.0);
        assert_eq!(memory_usage_percent(512, 0), 0.0);
    }

    #[test]
    fn calculates_disk_usage_percent() {
        assert_eq!(disk_usage_percent(9, 10), 90.0);
        assert_eq!(disk_usage_percent(9, 0), 0.0);
    }

    #[test]
    fn formats_battery_value() {
        assert_eq!(
            render_battery_value(Some(49), Some("Charging")),
            "49% [Charging]"
        );
        assert_eq!(render_battery_value(Some(49), None), "49%");
    }
}
