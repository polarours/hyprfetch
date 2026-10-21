use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

use super::count_label;
use super::layers::summarize_namespaces;
use super::rules::first_object_value_string;

pub struct HyprShortcutsModule;
impl Module for HyprShortcutsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprShortcuts
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.global_shortcuts() {
            Ok(raw) => {
                let shortcuts = collect_global_shortcuts(&raw);
                let compact = config.option_bool("compact", true);
                let rows = if compact {
                    vec![OutputRow::new(
                        "Shortcuts",
                        summarize_global_shortcuts(&shortcuts),
                    )]
                } else {
                    global_shortcut_rows(&shortcuts)
                };

                ModuleOutput::ok(
                    self.id(),
                    rows,
                    json!({ "raw": raw, "shortcuts": shortcuts }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct GlobalShortcutSummary {
    app: Option<String>,
    name: Option<String>,
    trigger: Option<String>,
}

fn summarize_global_shortcuts(shortcuts: &[GlobalShortcutSummary]) -> String {
    let mut parts = vec![count_label(shortcuts.len(), "shortcut", "shortcuts")];
    let apps = global_shortcut_apps(shortcuts);
    let names = global_shortcut_names(shortcuts);

    if !apps.is_empty() {
        parts.push(format!("apps:{}", summarize_namespaces(&apps, 4, "/")));
    }
    if !names.is_empty() {
        parts.push(format!("names:{}", summarize_namespaces(&names, 4, "/")));
    }

    parts.join(", ")
}

fn global_shortcut_rows(shortcuts: &[GlobalShortcutSummary]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Global Shortcuts",
        count_label(shortcuts.len(), "shortcut", "shortcuts"),
    )];
    let apps = global_shortcut_apps(shortcuts);
    let names = global_shortcut_names(shortcuts);
    let triggers = global_shortcut_triggers(shortcuts);

    if !apps.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Apps",
            summarize_namespaces(&apps, 8, ", "),
        ));
    }
    if !names.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Names",
            summarize_namespaces(&names, 8, ", "),
        ));
    }
    if !triggers.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Keys",
            summarize_namespaces(&triggers, 6, ", "),
        ));
    }

    rows
}

fn collect_global_shortcuts(value: &Value) -> Vec<GlobalShortcutSummary> {
    let mut shortcuts = Vec::new();
    collect_global_shortcuts_into(value, &mut shortcuts);
    shortcuts
}

fn collect_global_shortcuts_into(value: &Value, shortcuts: &mut Vec<GlobalShortcutSummary>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_global_shortcuts_into(item, shortcuts);
            }
        }
        Value::Object(object) => {
            for key in [
                "shortcuts",
                "globalShortcuts",
                "global_shortcuts",
                "global-shortcuts",
            ] {
                if let Some(nested) = object.get(key) {
                    collect_global_shortcuts_into(nested, shortcuts);
                    return;
                }
            }

            if object_looks_like_global_shortcut(object) {
                shortcuts.push(global_shortcut_from_object(object));
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn global_shortcut_from_object(object: &Map<String, Value>) -> GlobalShortcutSummary {
    GlobalShortcutSummary {
        app: first_object_value_string(
            object,
            &[
                "app_id",
                "appid",
                "application_id",
                "applicationId",
                "application",
                "app",
                "owner",
                "client",
                "namespace",
            ],
        ),
        name: first_object_value_string(
            object,
            &["description", "name", "title", "id", "identifier"],
        ),
        trigger: first_object_value_string(
            object,
            &[
                "shortcut",
                "trigger",
                "keybind",
                "binding",
                "accelerator",
                "key",
            ],
        ),
    }
}

fn object_looks_like_global_shortcut(object: &Map<String, Value>) -> bool {
    [
        "app_id",
        "appid",
        "application_id",
        "applicationId",
        "application",
        "app",
        "owner",
        "client",
        "namespace",
        "description",
        "name",
        "title",
        "shortcut",
        "trigger",
        "keybind",
        "binding",
        "accelerator",
        "key",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

fn global_shortcut_apps(shortcuts: &[GlobalShortcutSummary]) -> Vec<String> {
    unique_sorted_shortcut_field(
        shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.app.as_deref()),
    )
}

fn global_shortcut_names(shortcuts: &[GlobalShortcutSummary]) -> Vec<String> {
    unique_sorted_shortcut_field(
        shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.name.as_deref()),
    )
}

fn global_shortcut_triggers(shortcuts: &[GlobalShortcutSummary]) -> Vec<String> {
    unique_sorted_shortcut_field(
        shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.trigger.as_deref()),
    )
}

fn unique_sorted_shortcut_field<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    values
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_hypr_global_shortcuts() {
        let raw = serde_json::json!({
            "shortcuts": [
                {
                    "app_id": "com.discordapp.Discord",
                    "description": "Mute microphone",
                    "shortcut": "CTRL+ALT+M"
                },
                {
                    "application": "OBS Studio",
                    "description": "Toggle recording",
                    "trigger": "CTRL+SHIFT+R"
                },
                {
                    "owner": "hyprland",
                    "name": "launcher",
                    "key": "SUPER+Space"
                }
            ]
        });

        let shortcuts = collect_global_shortcuts(&raw);
        assert_eq!(
            shortcuts,
            vec![
                GlobalShortcutSummary {
                    app: Some("com.discordapp.Discord".to_string()),
                    name: Some("Mute microphone".to_string()),
                    trigger: Some("CTRL+ALT+M".to_string()),
                },
                GlobalShortcutSummary {
                    app: Some("OBS Studio".to_string()),
                    name: Some("Toggle recording".to_string()),
                    trigger: Some("CTRL+SHIFT+R".to_string()),
                },
                GlobalShortcutSummary {
                    app: Some("hyprland".to_string()),
                    name: Some("launcher".to_string()),
                    trigger: Some("SUPER+Space".to_string()),
                },
            ]
        );
        assert_eq!(
            summarize_global_shortcuts(&shortcuts),
            "3 shortcuts, apps:OBS Studio/com.discordapp.Discord/hyprland, names:Mute microphone/Toggle recording/launcher"
        );
        assert_eq!(
            global_shortcut_rows(&shortcuts),
            vec![
                OutputRow::new("Global Shortcuts", "3 shortcuts"),
                OutputRow::new(
                    "Shortcut Apps",
                    "OBS Studio, com.discordapp.Discord, hyprland"
                ),
                OutputRow::new(
                    "Shortcut Names",
                    "Mute microphone, Toggle recording, launcher"
                ),
                OutputRow::new("Shortcut Keys", "CTRL+ALT+M, CTRL+SHIFT+R, SUPER+Space"),
            ]
        );
    }

    #[test]
    fn handles_empty_hypr_global_shortcuts() {
        let shortcuts = collect_global_shortcuts(&serde_json::json!([]));
        assert!(shortcuts.is_empty());
        assert_eq!(summarize_global_shortcuts(&shortcuts), "0 shortcuts");
        assert_eq!(
            global_shortcut_rows(&shortcuts),
            vec![OutputRow::new("Global Shortcuts", "0 shortcuts")]
        );
    }
}
