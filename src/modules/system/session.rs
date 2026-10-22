use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct LocaleModule;

pub struct NetworkModule;

pub struct TemperatureModule;

pub struct AudioModule;

pub struct DatetimeModule;

pub struct ColorschemeModule;

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
                let display = info
                    .gtk_theme
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
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
