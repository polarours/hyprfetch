use serde_json::{Map, Value, json};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::truncate_visible;

use super::count_label;

pub struct HyprConfigModule;
impl Module for HyprConfigModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprConfig
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.config_errors() {
            Ok(raw) => {
                let errors = collect_config_error_messages(&raw);
                let data = json!({ "raw": raw, "errors": errors });

                if errors.is_empty() {
                    return ModuleOutput::ok(self.id(), vec![OutputRow::new("Config", "ok")], data);
                }

                let compact = config.option_bool("compact", true);
                let rows = if compact {
                    vec![OutputRow::new("Config", summarize_config_errors(&errors))]
                } else {
                    config_error_rows(&errors)
                };
                let message = count_label(errors.len(), "config error", "config errors");

                ModuleOutput::partial(self.id(), rows, data, message)
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_config_errors(errors: &[String]) -> String {
    let first = errors
        .first()
        .map(|error| truncate_visible(error, 72))
        .unwrap_or_default();
    format!("{}: {first}", count_label(errors.len(), "error", "errors"))
}

fn config_error_rows(errors: &[String]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Config Errors",
        count_label(errors.len(), "error", "errors"),
    )];

    for (index, error) in errors.iter().take(3).enumerate() {
        rows.push(OutputRow::new(
            format!("Error {}", index + 1),
            truncate_visible(error, 96),
        ));
    }

    let hidden_count = errors.len().saturating_sub(3);
    if hidden_count > 0 {
        rows.push(OutputRow::new("More Errors", format!("+{hidden_count}")));
    }

    rows
}

fn collect_config_error_messages(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_config_error_messages_into(value, &mut errors);
    errors
}

fn collect_config_error_messages_into(value: &Value, errors: &mut Vec<String>) {
    match value {
        Value::Null => {}
        Value::String(value) => push_config_error_string(errors, value),
        Value::Array(items) => {
            for item in items {
                collect_config_error_messages_into(item, errors);
            }
        }
        Value::Object(object) => {
            for key in ["errors", "config_errors", "configErrors"] {
                if let Some(nested) = object.get(key) {
                    collect_config_error_messages_into(nested, errors);
                    return;
                }
            }

            if let Some(message) = config_error_object_message(object) {
                errors.push(message);
            }
        }
        Value::Bool(_) | Value::Number(_) => {}
    }
}

fn config_error_object_message(object: &Map<String, Value>) -> Option<String> {
    let message = first_object_string(
        object,
        &["message", "error", "reason", "description", "err"],
    )
    .or_else(|| first_any_object_string(object))?;

    let message = clean_config_error_text(message);
    if message_means_no_config_errors(&message) {
        return None;
    }

    let location = config_error_location(object);
    if location.is_empty() {
        Some(message)
    } else {
        Some(format!("{location}: {message}"))
    }
}

fn config_error_location(object: &Map<String, Value>) -> String {
    let file =
        first_object_string(object, &["file", "filename", "config"]).map(clean_config_error_text);
    let line = object
        .get("line")
        .or_else(|| object.get("line_number"))
        .or_else(|| object.get("lineno"))
        .and_then(value_to_location_part);
    let column = object
        .get("column")
        .or_else(|| object.get("col"))
        .and_then(value_to_location_part);

    let mut location = String::new();
    if let Some(file) = file {
        location.push_str(&file);
    }
    if let Some(line) = line {
        if !location.is_empty() {
            location.push(':');
        }
        location.push_str(&line);
    }
    if let Some(column) = column {
        if !location.is_empty() {
            location.push(':');
        }
        location.push_str(&column);
    }
    location
}

fn value_to_location_part(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn push_config_error_string(errors: &mut Vec<String>, value: &str) {
    for line in value.lines().map(clean_config_error_text) {
        if !message_means_no_config_errors(&line) {
            errors.push(line);
        }
    }
}

pub(super) fn clean_config_error_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn message_means_no_config_errors(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "ok"
        || normalized == "[]"
        || normalized == "null"
        || normalized.contains("no config errors")
        || normalized.contains("no errors")
}

fn first_object_string<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
}

fn first_any_object_string(object: &Map<String, Value>) -> Option<&str> {
    object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "file" | "filename" | "config"))
        .filter_map(|(_, value)| value.as_str())
        .map(str::trim)
        .find(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hypr_config_errors_flexibly() {
        let raw = serde_json::json!([
            {
                "file": "/home/user/.config/hypr/hyprland.conf",
                "line": 42,
                "message": "invalid workspace rule"
            },
            {
                "description": "unknown option decoration:fake_option"
            },
            "No config errors!"
        ]);

        let errors = collect_config_error_messages(&raw);
        assert_eq!(
            errors,
            vec![
                "/home/user/.config/hypr/hyprland.conf:42: invalid workspace rule",
                "unknown option decoration:fake_option",
            ]
        );
        assert_eq!(
            summarize_config_errors(&errors),
            "2 errors: /home/user/.config/hypr/hyprland.conf:42: invalid workspace rule"
        );
        assert_eq!(
            config_error_rows(&errors),
            vec![
                OutputRow::new("Config Errors", "2 errors"),
                OutputRow::new(
                    "Error 1",
                    "/home/user/.config/hypr/hyprland.conf:42: invalid workspace rule"
                ),
                OutputRow::new("Error 2", "unknown option decoration:fake_option"),
            ]
        );
    }

    #[test]
    fn treats_empty_hypr_config_errors_as_ok() {
        assert!(collect_config_error_messages(&serde_json::json!([])).is_empty());
        assert!(collect_config_error_messages(&serde_json::json!("No config errors!")).is_empty());
        assert!(collect_config_error_messages(&serde_json::json!({ "errors": [] })).is_empty());
    }
}
