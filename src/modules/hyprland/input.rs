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
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Input", summarize_input_devices(&devices))],
                        json!({ "devices": devices }),
                    );
                }

                let mut rows = vec![OutputRow::new("Input", devices.total_count().to_string())];
                push_device_row(&mut rows, "Keyboards", devices.keyboards.len());
                push_device_row(&mut rows, "Mice", devices.mice.len());
                push_device_row(&mut rows, "Tablets", devices.tablets.len());
                push_device_row(&mut rows, "Touch", devices.touch.len());
                push_device_row(&mut rows, "Switches", devices.switches.len());
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
    parts.join(", ")
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
}
