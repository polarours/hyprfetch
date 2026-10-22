use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::theme::EcosystemInfo;

pub struct HyprEcosystemModule;

impl Module for HyprEcosystemModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprEcosystem
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.theme.ecosystem_info() {
            Ok(ecosystem) => {
                let include_wallpaper = !ctx
                    .runtime
                    .modules
                    .iter()
                    .any(|module| module.name == ModuleId::Wallpaper);
                let ecosystem = prepare_ecosystem_for_display(ecosystem, include_wallpaper);
                if ecosystem.is_empty() {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "no additional Hyprland ecosystem tools detected",
                    );
                }

                let compact = config.option_bool("compact", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Ecosystem", summarize_ecosystem(&ecosystem))],
                        json!(ecosystem),
                    );
                }

                let mut rows = Vec::new();
                if let Some((label, _version)) = &ecosystem.wallpaper {
                    rows.push(OutputRow::new("Wallpaper", label));
                }
                if let Some((label, _version)) = &ecosystem.bar {
                    rows.push(OutputRow::new("Bar", label));
                }
                if let Some((label, _version)) = &ecosystem.launcher {
                    rows.push(OutputRow::new("Launcher", label));
                }
                if let Some((label, _version)) = &ecosystem.notifications {
                    rows.push(OutputRow::new("Notifications", label));
                }
                if let Some((label, _version)) = &ecosystem.clipboard {
                    rows.push(OutputRow::new("Clipboard", label));
                }
                if let Some((label, _version)) = &ecosystem.screenshot {
                    rows.push(OutputRow::new("Screenshot", label));
                }
                if let Some((label, _version)) = &ecosystem.idle {
                    rows.push(OutputRow::new("Idle", label));
                }
                if let Some((label, _version)) = &ecosystem.lock {
                    rows.push(OutputRow::new("Lock", label));
                }
                if let Some((label, _version)) = &ecosystem.portal {
                    rows.push(OutputRow::new("Portal", label));
                }
                if let Some((label, _version)) = &ecosystem.polkit {
                    rows.push(OutputRow::new("Polkit", label));
                }
                if let Some((label, _version)) = &ecosystem.tray {
                    rows.push(OutputRow::new("Tray", label));
                }
                if let Some((label, _version)) = &ecosystem.picker {
                    rows.push(OutputRow::new("Picker", label));
                }

                ModuleOutput::ok(self.id(), rows, json!(ecosystem))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

pub fn prepare_ecosystem_for_display(
    mut ecosystem: EcosystemInfo,
    include_wallpaper: bool,
) -> EcosystemInfo {
    if !include_wallpaper {
        ecosystem.wallpaper = None;
    }
    ecosystem
}

pub fn summarize_ecosystem(ecosystem: &EcosystemInfo) -> String {
    let mut parts = Vec::new();
    if let Some((label, version)) = &ecosystem.wallpaper {
        parts.push(format!("wall:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.bar {
        parts.push(format!("bar:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.launcher {
        parts.push(format!("launch:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.notifications {
        parts.push(format!("notify:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.clipboard {
        parts.push(format!("clip:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.screenshot {
        parts.push(format!("shot:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.idle {
        parts.push(format!("idle:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.lock {
        parts.push(format!("lock:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.portal {
        parts.push(format!("portal:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.polkit {
        parts.push(format!("polkit:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.tray {
        parts.push(format!("tray:{}", eco_display(label, version)));
    }
    if let Some((label, version)) = &ecosystem.picker {
        parts.push(format!("picker:{}", eco_display(label, version)));
    }
    parts.join(" ")
}

fn eco_display(label: &str, version: &Option<String>) -> String {
    match version {
        Some(v) if !v.is_empty() => format!("{label} ({v})"),
        _ => label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::theme::EcosystemInfo;

    #[test]
    fn summarizes_ecosystem_compact_output() {
        let ecosystem = EcosystemInfo {
            wallpaper: Some(("hyprpaper".to_string(), None)),
            bar: Some(("waybar".to_string(), None)),
            launcher: Some(("fuzzel".to_string(), None)),
            notifications: Some(("mako".to_string(), None)),
            clipboard: Some(("cliphist".to_string(), None)),
            screenshot: Some(("grim".to_string(), None)),
            idle: Some(("hypridle".to_string(), None)),
            lock: Some(("hyprlock".to_string(), None)),
            portal: Some(("portal".to_string(), None)),
            polkit: Some(("polkit".to_string(), None)),
            tray: Some(("tray".to_string(), None)),
            picker: Some(("hyprpicker".to_string(), None)),
        };

        assert_eq!(
            summarize_ecosystem(&ecosystem),
            "wall:hyprpaper bar:waybar launch:fuzzel notify:mako clip:cliphist shot:grim idle:hypridle lock:hyprlock portal:portal polkit:polkit tray:tray picker:hyprpicker"
        );
    }

    #[test]
    fn drops_wallpaper_from_ecosystem_when_wallpaper_module_is_enabled() {
        let ecosystem = EcosystemInfo {
            wallpaper: Some(("hyprpaper".to_string(), None)),
            bar: Some(("waybar".to_string(), None)),
            ..EcosystemInfo::default()
        };

        let filtered = prepare_ecosystem_for_display(ecosystem, false);
        assert_eq!(summarize_ecosystem(&filtered), "bar:waybar");
    }
}
