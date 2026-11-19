use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct HyprDevicesModule;
impl Module for HyprDevicesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprDevices
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.devices() {
            Ok(devices) => {
                if devices.total_count() == 0 {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "hyprctl reported no input devices",
                    );
                }

                let compact = config.option_bool("compact", true);
                let list_devices = config.option_bool("devices", false);
                if compact {
                    let mut rows = vec![OutputRow::new("Input", summarize_input_devices(&devices))];
                    if list_devices {
                        rows.extend(device_rows(&devices));
                    }
                    return ModuleOutput::ok(self.id(), rows, json!({ "devices": devices }));
                }

                let mut rows = vec![OutputRow::new("Input", devices.total_count().to_string())];

                // Every device on its own row, for a machine with more than one
                // keyboard or an unusual pointer.
                if list_devices {
                    for device in device_rows(&devices) {
                        rows.push(device);
                    }
                } else {
                    push_device_row(&mut rows, "Keyboards", devices.keyboards.len());
                    push_device_row(&mut rows, "Mice", devices.mice.len());
                    push_device_row(&mut rows, "Tablets", devices.tablets.len());
                    push_device_row(&mut rows, "Touch", devices.touch.len());
                    push_device_row(&mut rows, "Switches", devices.switches.len());
                }
                if let Some(keyboard) = primary_keyboard(&devices) {
                    if let Some(name) = keyboard.name.as_deref().filter(|name| !name.is_empty()) {
                        rows.push(OutputRow::new("Main Keyboard", name));
                    }
                    if let Some(keymap) = keyboard
                        .active_keymap
                        .as_deref()
                        .filter(|keymap| !keymap.is_empty())
                    {
                        rows.push(OutputRow::new("Keymap", keymap));
                    }
                    if let Some(layout) = keyboard.layout.as_deref().filter(|l| !l.is_empty()) {
                        rows.push(OutputRow::new("Layout", layout));
                    }
                    if let Some(caps) = keyboard.caps_lock {
                        rows.push(OutputRow::new("Caps Lock", lock_label(caps)));
                    }
                    if let Some(num) = keyboard.num_lock {
                        rows.push(OutputRow::new("Num Lock", lock_label(num)));
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "devices": devices }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_input_devices(devices: &crate::providers::hyprland::HyprDevices) -> String {
    let mut parts = Vec::new();
    push_summary_part(&mut parts, devices.keyboards.len(), "keyboard", "keyboards");
    push_summary_part(&mut parts, devices.mice.len(), "mouse", "mice");
    push_summary_part(&mut parts, devices.tablets.len(), "tablet", "tablets");
    push_summary_part(&mut parts, devices.touch.len(), "touch", "touch");
    push_summary_part(&mut parts, devices.switches.len(), "switch", "switches");
    if let Some(keymap) = primary_keyboard_keymap(devices) {
        parts.push(format!("keymap:{keymap}"));
    }
    // Lock states are only worth a mention while they are engaged.
    for (label, engaged) in lock_states(devices) {
        if engaged {
            parts.push(format!("{label}:on"));
        }
    }
    parts.join(", ")
}

/// The lock states of the main keyboard.
fn lock_states(devices: &crate::providers::hyprland::HyprDevices) -> Vec<(&'static str, bool)> {
    let Some(keyboard) = primary_keyboard(devices) else {
        return Vec::new();
    };

    let mut states = Vec::new();
    if let Some(caps) = keyboard.caps_lock {
        states.push(("caps", caps));
    }
    if let Some(num) = keyboard.num_lock {
        states.push(("num", num));
    }
    states
}

/// One row per input device.
fn device_rows(devices: &crate::providers::hyprland::HyprDevices) -> Vec<OutputRow> {
    let mut rows = Vec::new();

    for (label, group) in [
        ("Keyboard", &devices.keyboards),
        ("Mouse", &devices.mice),
        ("Tablet", &devices.tablets),
        ("Touch", &devices.touch),
        ("Switch", &devices.switches),
    ] {
        for device in group {
            let name = device.name.as_deref().unwrap_or("unknown");
            let mut value = name.to_string();
            if let Some(keymap) = device
                .active_keymap
                .as_deref()
                .filter(|keymap| !keymap.is_empty())
            {
                value.push_str(&format!(" ({keymap})"));
            }
            if device.caps_lock == Some(true) {
                value.push_str(" caps");
            }
            if device.num_lock == Some(true) {
                value.push_str(" num");
            }
            if device.main == Some(true) {
                value.push_str(" main");
            }
            // A pointer only earns its settings a mention when they deviate
            // from the default, so an untouched mouse stays a plain name.
            if let Some(speed) = device.default_speed.filter(|speed| *speed != 0.0) {
                value.push_str(&format!(" speed {speed:+}"));
            }
            if let Some(scroll) = device.scroll_factor.filter(|factor| *factor != 1.0) {
                value.push_str(&format!(" scroll ×{scroll}"));
            }
            rows.push(OutputRow::new(label, value));
        }
    }

    rows
}

fn primary_keyboard(
    devices: &crate::providers::hyprland::HyprDevices,
) -> Option<&crate::providers::hyprland::HyprInputDevice> {
    devices
        .keyboards
        .iter()
        .find(|keyboard| keyboard.main.unwrap_or(false))
        .or_else(|| devices.keyboards.first())
}

fn primary_keyboard_keymap(devices: &crate::providers::hyprland::HyprDevices) -> Option<&str> {
    primary_keyboard(devices).and_then(|keyboard| {
        keyboard
            .active_keymap
            .as_deref()
            .filter(|keymap| !keymap.is_empty())
    })
}

fn push_device_row(rows: &mut Vec<OutputRow>, label: &str, count: usize) {
    if count > 0 {
        rows.push(OutputRow::new(label, count.to_string()));
    }
}

fn lock_label(engaged: bool) -> &'static str {
    if engaged { "on" } else { "off" }
}

fn push_summary_part(parts: &mut Vec<String>, count: usize, singular: &str, plural: &str) {
    if count == 0 {
        return;
    }

    let label = if count == 1 { singular } else { plural };
    parts.push(format!("{count} {label}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::{HyprDevices, HyprInputDevice};

    #[test]
    fn summarizes_primary_keyboard_keymap() {
        let devices = HyprDevices {
            keyboards: vec![
                HyprInputDevice {
                    name: Some("Fallback Keyboard".to_string()),
                    active_keymap: Some("English (US)".to_string()),
                    ..HyprInputDevice::default()
                },
                HyprInputDevice {
                    name: Some("Keychron Q1".to_string()),
                    main: Some(true),
                    active_keymap: Some("Colemak".to_string()),
                    ..HyprInputDevice::default()
                },
            ],
            mice: vec![HyprInputDevice {
                name: Some("Logitech G Pro Wireless".to_string()),
                ..HyprInputDevice::default()
            }],
            ..HyprDevices::default()
        };

        assert_eq!(primary_keyboard_keymap(&devices), Some("Colemak"));
        assert_eq!(
            summarize_input_devices(&devices),
            "2 keyboards, 1 mouse, keymap:Colemak"
        );
    }

    #[test]
    fn mentions_engaged_lock_states_only() {
        let devices = HyprDevices {
            keyboards: vec![HyprInputDevice {
                name: Some("Keychron Q1".to_string()),
                main: Some(true),
                active_keymap: Some("Colemak".to_string()),
                caps_lock: Some(true),
                num_lock: Some(false),
                ..HyprInputDevice::default()
            }],
            ..HyprDevices::default()
        };

        assert_eq!(
            summarize_input_devices(&devices),
            "1 keyboard, keymap:Colemak, caps:on"
        );
    }

    #[test]
    fn keeps_quiet_while_the_locks_are_off() {
        let devices = HyprDevices {
            keyboards: vec![HyprInputDevice {
                name: Some("Keychron Q1".to_string()),
                main: Some(true),
                active_keymap: Some("Colemak".to_string()),
                caps_lock: Some(false),
                num_lock: Some(false),
                ..HyprInputDevice::default()
            }],
            ..HyprDevices::default()
        };

        assert_eq!(
            summarize_input_devices(&devices),
            "1 keyboard, keymap:Colemak"
        );
    }

    #[test]
    fn reports_lock_labels() {
        assert_eq!(lock_label(true), "on");
        assert_eq!(lock_label(false), "off");
    }

    #[test]
    fn lists_every_device_on_its_own_row() {
        let devices = HyprDevices {
            keyboards: vec![HyprInputDevice {
                name: Some("Keychron".to_string()),
                active_keymap: Some("Colemak".to_string()),
                main: Some(true),
                ..HyprInputDevice::default()
            }],
            mice: vec![HyprInputDevice {
                name: Some("G Pro".to_string()),
                ..HyprInputDevice::default()
            }],
            ..HyprDevices::default()
        };

        let rows = device_rows(&devices);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, "Keyboard");
        assert_eq!(rows[0].value, "Keychron (Colemak) main");
        assert_eq!(rows[1].key, "Mouse");
        assert_eq!(rows[1].value, "G Pro");
    }

    #[test]
    fn shows_pointer_settings_only_when_they_deviate() {
        // A mouse with natural scrolling and a non-default speed reports both;
        // an untouched one stays a plain name.
        let devices = HyprDevices {
            mice: vec![
                HyprInputDevice {
                    name: Some("Logitech G304".to_string()),
                    scroll_factor: Some(-1.0),
                    default_speed: Some(0.35),
                    ..HyprInputDevice::default()
                },
                HyprInputDevice {
                    name: Some("trackpoint".to_string()),
                    scroll_factor: Some(1.0),
                    default_speed: Some(0.0),
                    ..HyprInputDevice::default()
                },
            ],
            ..HyprDevices::default()
        };

        let rows = device_rows(&devices);
        assert_eq!(rows[0].value, "Logitech G304 speed +0.35 scroll ×-1");
        assert_eq!(rows[1].value, "trackpoint");
    }

    #[test]
    fn compact_mode_honours_the_devices_option() {
        use std::collections::HashMap;
        use std::sync::Arc;

        use crate::config::{AppearanceConfig, RuntimeConfig};
        use crate::model::{ColorMode, PaletteName, RenderStyle};
        use crate::modules::ModuleContext;
        use crate::providers::Providers;
        use crate::providers::hyprland::MockRunner;

        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/hyprctl/devices.json"
        ))
        .unwrap();
        let providers = Providers::with_hypr_runner(Arc::new(MockRunner {
            payloads: HashMap::from([("devices".to_string(), raw)]),
        }));
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: crate::logo::LogoPreset::None,
            resolved_logo: crate::logo::LogoPreset::None,
            modules: Vec::new(),
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let context = ModuleContext {
            runtime: &runtime,
            providers: &providers,
        };

        let mut config = ModuleConfig::from(ModuleId::HyprDevices);
        config
            .options
            .insert("compact".to_string(), toml::Value::Boolean(true));
        config
            .options
            .insert("devices".to_string(), toml::Value::Boolean(true));

        let output = HyprDevicesModule.run(&context, &config);
        assert!(
            output.rows.len() > 1,
            "devices=true must list devices even in compact mode, got {output:?}"
        );
    }
}
