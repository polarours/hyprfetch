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
                let mut rows = if compact {
                    vec![OutputRow::new(
                        "Shortcuts",
                        summarize_global_shortcuts(&shortcuts),
                    )]
                } else {
                    global_shortcut_rows(&shortcuts)
                };

                // Every shortcut on its own row, so a rice screenshot shows
                // which application claimed which shortcut.
                if config.option_bool("list", false) {
                    for shortcut in listed_global_shortcuts(&shortcuts) {
                        rows.push(shortcut);
                    }
                }

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
    id: Option<String>,
    description: Option<String>,
    trigger: Option<String>,
}

fn summarize_global_shortcuts(shortcuts: &[GlobalShortcutSummary]) -> String {
    let mut parts = vec![count_label(shortcuts.len(), "shortcut", "shortcuts")];
    let apps = global_shortcut_apps(shortcuts);
    let ids = global_shortcut_ids(shortcuts);

    if !apps.is_empty() {
        parts.push(format!("apps:{}", summarize_namespaces(&apps, 4, "/")));
    }
    if !ids.is_empty() {
        parts.push(format!("ids:{}", summarize_namespaces(&ids, 4, "/")));
    }

    parts.join(", ")
}

fn global_shortcut_rows(shortcuts: &[GlobalShortcutSummary]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Global Shortcuts",
        count_label(shortcuts.len(), "shortcut", "shortcuts"),
    )];
    let apps = global_shortcut_apps(shortcuts);
    let ids = global_shortcut_ids(shortcuts);
    let descriptions = global_shortcut_descriptions(shortcuts);
    let triggers = global_shortcut_triggers(shortcuts);

    if !apps.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Apps",
            summarize_namespaces(&apps, 8, ", "),
        ));
    }
    if !ids.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Ids",
            summarize_namespaces(&ids, 8, ", "),
        ));
    }
    if !descriptions.is_empty() {
        rows.push(OutputRow::new(
            "Shortcut Descriptions",
            summarize_namespaces(&descriptions, 8, ", "),
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

/// One row per shortcut, named the way Hyprland names it.
fn listed_global_shortcuts(shortcuts: &[GlobalShortcutSummary]) -> Vec<OutputRow> {
    shortcuts
        .iter()
        .map(|shortcut| {
            let key = match (shortcut.app.as_deref(), shortcut.id.as_deref()) {
                (Some(app), Some(id)) => format!("{app}:{id}"),
                (Some(app), None) => app.to_string(),
                (None, Some(id)) => id.to_string(),
                (None, None) => "?".to_string(),
            };

            let description = shortcut.description.as_deref().map(str::trim);
            let trigger = shortcut.trigger.as_deref().map(str::trim);
            let value = match (description, trigger) {
                (Some(description), Some(trigger)) if !description.is_empty() => {
                    format!("{description} ({trigger})")
                }
                (Some(description), _) if !description.is_empty() => description.to_string(),
                (_, Some(trigger)) => trigger.to_string(),
                _ => "?".to_string(),
            };

            OutputRow::new(key, value)
        })
        .collect()
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
    let identifier = first_object_value_string(object, &["name", "id", "identifier"]);
    let (app_from_name, id) = split_shortcut_identifier(identifier.as_deref());
    let app = first_object_value_string(
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
    )
    .or(app_from_name);

    GlobalShortcutSummary {
        app,
        id,
        description: first_object_value_string(object, &["description", "summary", "label"]),
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

/// Hyprland packs the application and the shortcut id into one name.
fn split_shortcut_identifier(identifier: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(identifier) = identifier.map(str::trim).filter(|value| !value.is_empty()) else {
        return (None, None);
    };

    match identifier.split_once(':') {
        Some((app, id)) => {
            let app = (!app.is_empty()).then(|| app.to_string());
            let id = if id.is_empty() {
                Some(identifier.to_string())
            } else {
                Some(id.to_string())
            };
            (app, id)
        }
        None => (None, Some(identifier.to_string())),
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
        "summary",
        "label",
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

fn global_shortcut_ids(shortcuts: &[GlobalShortcutSummary]) -> Vec<String> {
    unique_sorted_shortcut_field(
        shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.id.as_deref()),
    )
}

fn global_shortcut_descriptions(shortcuts: &[GlobalShortcutSummary]) -> Vec<String> {
    unique_sorted_shortcut_field(
        shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.description.as_deref()),
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
                    id: None,
                    description: Some("Mute microphone".to_string()),
                    trigger: Some("CTRL+ALT+M".to_string()),
                },
                GlobalShortcutSummary {
                    app: Some("OBS Studio".to_string()),
                    id: None,
                    description: Some("Toggle recording".to_string()),
                    trigger: Some("CTRL+SHIFT+R".to_string()),
                },
                GlobalShortcutSummary {
                    app: Some("hyprland".to_string()),
                    id: Some("launcher".to_string()),
                    description: None,
                    trigger: Some("SUPER+Space".to_string()),
                },
            ]
        );
        assert_eq!(
            summarize_global_shortcuts(&shortcuts),
            "3 shortcuts, apps:OBS Studio/com.discordapp.Discord/hyprland, ids:launcher"
        );
        assert_eq!(
            global_shortcut_rows(&shortcuts),
            vec![
                OutputRow::new("Global Shortcuts", "3 shortcuts"),
                OutputRow::new(
                    "Shortcut Apps",
                    "OBS Studio, com.discordapp.Discord, hyprland"
                ),
                OutputRow::new("Shortcut Ids", "launcher"),
                OutputRow::new("Shortcut Descriptions", "Mute microphone, Toggle recording"),
                OutputRow::new("Shortcut Keys", "CTRL+ALT+M, CTRL+SHIFT+R, SUPER+Space"),
            ]
        );
    }

    #[test]
    fn splits_the_application_out_of_the_hypr_shortcut_name() {
        // This is what hyprctl reports: the application and the id of the
        // shortcut are packed into the name.
        let raw = serde_json::json!([
            {
                "name": "quickshell:mediaPrev",
                "description": "Skip to the previous track"
            }
        ]);

        let shortcuts = collect_global_shortcuts(&raw);
        assert_eq!(
            shortcuts,
            vec![GlobalShortcutSummary {
                app: Some("quickshell".to_string()),
                id: Some("mediaPrev".to_string()),
                description: Some("Skip to the previous track".to_string()),
                trigger: None,
            }]
        );
        assert_eq!(
            summarize_global_shortcuts(&shortcuts),
            "1 shortcut, apps:quickshell, ids:mediaPrev"
        );
    }

    #[test]
    fn lists_every_global_shortcut() {
        let raw = serde_json::json!([
            {
                "name": "quickshell:mediaPrev",
                "description": "Skip to the previous track"
            },
            {
                "name": "quickshell:mediaToggle",
                "description": "Play or pause the active media player"
            }
        ]);

        let shortcuts = collect_global_shortcuts(&raw);
        let rows = listed_global_shortcuts(&shortcuts);
        assert_eq!(
            rows,
            vec![
                OutputRow::new("quickshell:mediaPrev", "Skip to the previous track"),
                OutputRow::new(
                    "quickshell:mediaToggle",
                    "Play or pause the active media player"
                ),
            ]
        );
    }

    #[test]
    fn lists_a_global_shortcut_with_its_key() {
        let shortcuts = vec![GlobalShortcutSummary {
            app: Some("com.discordapp.Discord".to_string()),
            id: Some("mute".to_string()),
            description: Some("Mute microphone".to_string()),
            trigger: Some("CTRL+ALT+M".to_string()),
        }];

        let rows = listed_global_shortcuts(&shortcuts);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "com.discordapp.Discord:mute");
        assert_eq!(rows[0].value, "Mute microphone (CTRL+ALT+M)");
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
        assert!(listed_global_shortcuts(&shortcuts).is_empty());
    }
}
