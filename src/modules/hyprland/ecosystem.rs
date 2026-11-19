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
                    let show_versions = config.option_bool("versions", false);
                    // A compact summary stays short: a few backends, no versions
                    // unless asked, so one row never dominates the screenshot.
                    let summary = summarize_ecosystem(&ecosystem, show_versions, 4);
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Ecosystem", summary)],
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
                if let Some((label, _version)) = &ecosystem.network {
                    rows.push(OutputRow::new("Network", label));
                }
                if let Some((label, _version)) = &ecosystem.audio {
                    rows.push(OutputRow::new("Audio", label));
                }
                if let Some((label, _version)) = &ecosystem.bluetooth {
                    rows.push(OutputRow::new("Bluetooth", label));
                }
                if let Some((label, _version)) = &ecosystem.greeter {
                    rows.push(OutputRow::new("Greeter", label));
                }
                if let Some((label, _version)) = &ecosystem.ime {
                    rows.push(OutputRow::new("IME", label));
                }
                if let Some((label, _version)) = &ecosystem.nightlight {
                    rows.push(OutputRow::new("Night light", label));
                }
                if let Some((label, _version)) = &ecosystem.power {
                    rows.push(OutputRow::new("Power", label));
                }
                if let Some((label, _version)) = &ecosystem.automount {
                    rows.push(OutputRow::new("Automount", label));
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

/// Maximum display width of one backend name in the compact summary. Common
/// system services such as NetworkManager (14) and power-profiles (14) must
/// stay intact; only genuinely long names are cut.
const ECO_NAME_WIDTH: usize = 16;

pub fn summarize_ecosystem(
    ecosystem: &EcosystemInfo,
    show_versions: bool,
    max_items: usize,
) -> String {
    // (short prefix, name, version) for every detected backend.
    let mut entries: Vec<(&str, &String, Option<&String>)> = Vec::new();
    macro_rules! push {
        ($prefix:literal, $field:expr) => {
            if let Some((name, version)) = &$field {
                entries.push(($prefix, name, version.as_ref()));
            }
        };
    }
    push!("wall", ecosystem.wallpaper);
    push!("bar", ecosystem.bar);
    push!("launch", ecosystem.launcher);
    push!("notify", ecosystem.notifications);
    push!("clip", ecosystem.clipboard);
    push!("shot", ecosystem.screenshot);
    push!("idle", ecosystem.idle);
    push!("lock", ecosystem.lock);
    push!("portal", ecosystem.portal);
    push!("polkit", ecosystem.polkit);
    push!("tray", ecosystem.tray);
    push!("picker", ecosystem.picker);
    push!("greet", ecosystem.greeter);
    push!("net", ecosystem.network);
    push!("snd", ecosystem.audio);
    push!("bt", ecosystem.bluetooth);
    push!("ime", ecosystem.ime);
    push!("night", ecosystem.nightlight);
    push!("power", ecosystem.power);
    push!("mount", ecosystem.automount);

    let mut parts = Vec::new();
    for (prefix, name, version) in entries.iter().take(max_items) {
        let short = truncate_name(name, ECO_NAME_WIDTH);
        let mut part = format!("{prefix}:{short}");
        if show_versions
            && let Some(version) = version
            && !version.is_empty()
        {
            part.push_str(&format!(" ({version})"));
        }
        parts.push(part);
    }
    let mut summary = parts.join(" ");
    if entries.len() > max_items {
        summary.push_str(&format!(" +{}", entries.len() - max_items));
    }
    summary
}

/// Cut a backend name to `width` display columns, marking the cut with `…`.
fn truncate_name(name: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0;
    for character in name.chars() {
        let width_of = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if used + width_of > width.saturating_sub(1) {
            output.push('…');
            break;
        }
        output.push(character);
        used += width_of;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::theme::EcosystemInfo;

    #[test]
    fn compact_summary_drops_versions_and_limits_length() {
        let ecosystem = EcosystemInfo {
            bar: Some(("quickshell".to_string(), Some("0.3.2".to_string()))),
            launcher: Some(("walker".to_string(), Some("2.17.2".to_string()))),
            notifications: Some(("swaync".to_string(), Some("0.12.6".to_string()))),
            clipboard: Some(("wl-clipboard".to_string(), Some("2.3.0".to_string()))),
            portal: Some(("xdg-desktop-portal-hyprland".to_string(), None)),
            polkit: Some(("hyprpolkitagent".to_string(), None)),
            ..EcosystemInfo::default()
        };

        let short = summarize_ecosystem(&ecosystem, false, 4);
        assert!(
            !short.contains("0.3.2"),
            "versions must be omitted by default"
        );
        assert!(short.contains("+2"), "remaining backends must be counted");
        assert!(
            short.chars().count() < 70,
            "summary must stay short: {short}"
        );

        let with_versions = summarize_ecosystem(&ecosystem, true, 4);
        assert!(
            with_versions.contains("0.3.2"),
            "versions=true must restore them"
        );
    }

    #[test]
    fn long_backend_names_are_truncated() {
        let ecosystem = EcosystemInfo {
            portal: Some(("xdg-desktop-portal-hyprland".to_string(), None)),
            ..EcosystemInfo::default()
        };
        let summary = summarize_ecosystem(&ecosystem, false, 4);
        assert!(
            summary.chars().count() <= ECO_NAME_WIDTH + 8,
            "long names must be truncated: {summary}"
        );
        assert!(
            summary.contains('…'),
            "truncation must be marked: {summary}"
        );
    }

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
            greeter: Some(("sddm".to_string(), None)),
            network: Some(("NetworkManager".to_string(), None)),
            audio: Some(("pipewire".to_string(), None)),
            bluetooth: Some(("bluetoothd".to_string(), None)),
            ime: Some(("fcitx5".to_string(), None)),
            nightlight: Some(("wlsunset".to_string(), None)),
            power: Some(("power-profiles".to_string(), None)),
            automount: Some(("udisks".to_string(), None)),
        };

        assert_eq!(
            summarize_ecosystem(&ecosystem, true, 20),
            "wall:hyprpaper bar:waybar launch:fuzzel notify:mako clip:cliphist shot:grim idle:hypridle lock:hyprlock portal:portal polkit:polkit tray:tray picker:hyprpicker greet:sddm net:NetworkManager snd:pipewire bt:bluetoothd ime:fcitx5 night:wlsunset power:power-profiles mount:udisks"
        );
        // A compact summary still caps at a few entries so the row stays short.
        assert_eq!(
            summarize_ecosystem(&ecosystem, false, 4),
            "wall:hyprpaper bar:waybar launch:fuzzel notify:mako +16"
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
        assert_eq!(summarize_ecosystem(&filtered, false, 4), "bar:waybar");
    }
}
