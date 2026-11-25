//! Weather, public IP and the primary display.
//!
//! Weather and the public IP are opt-in: both send a request off the machine,
//! which a fetch tool should only do when explicitly asked. The display row is
//! local and prefers Hyprland's answer (it knows the refresh rate) before
//! falling back to DRM.

use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::system::DisplayInfo;

pub struct WeatherModule;

pub struct PublicIpModule;

pub struct DisplayModule;

impl Module for WeatherModule {
    fn id(&self) -> ModuleId {
        ModuleId::Weather
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.weather() {
            Some(weather) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Weather", weather.display())],
                json!({ "condition": weather.condition, "temperature": weather.temperature }),
            ),
            None => ModuleOutput::unavailable(self.id(), "weather lookup failed".to_string()),
        }
    }
}

impl Module for PublicIpModule {
    fn id(&self) -> ModuleId {
        ModuleId::PublicIp
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.public_ip() {
            Some(ip) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Public IP", ip.clone())],
                json!({ "address": ip }),
            ),
            None => ModuleOutput::unavailable(self.id(), "public IP lookup failed".to_string()),
        }
    }
}

impl Module for DisplayModule {
    fn id(&self) -> ModuleId {
        ModuleId::Display
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        // Hyprland reports the refresh rate, so prefer the focused monitor.
        if let Ok(monitors) = ctx.providers.hyprland.monitors() {
            let monitor = monitors
                .iter()
                .find(|monitor| monitor.focused)
                .or_else(|| monitors.first());
            if let Some(monitor) = monitor {
                let info = DisplayInfo {
                    name: monitor.name.clone(),
                    width: monitor.width.max(0) as u32,
                    height: monitor.height.max(0) as u32,
                    refresh_hz: Some(monitor.refresh_rate),
                };
                return ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Display", info.display())],
                    json!({
                        "name": info.name,
                        "width": info.width,
                        "height": info.height,
                        "refresh_hz": info.refresh_hz,
                    }),
                );
            }
        }
        match ctx.providers.system.drm_display() {
            Some(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Display", info.display())],
                json!({
                    "name": info.name,
                    "width": info.width,
                    "height": info.height,
                    "refresh_hz": info.refresh_hz,
                }),
            ),
            None => ModuleOutput::unavailable(self.id(), "no connected display".to_string()),
        }
    }
}
