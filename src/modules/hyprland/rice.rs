use serde_json::{Map, Value, json};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::hyprland::HyprOption;
use crate::providers::system::ProviderError;

use super::{failure_summary, push_failure};

pub struct HyprRiceModule;
impl Module for HyprRiceModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprRice
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let mut failures = Vec::new();
        let mut data_options = Map::new();
        let layout = match read_option(ctx, "general:layout", &mut data_options) {
            Ok(option) => hypr_option_value(&option),
            Err(error) => {
                return ModuleOutput::unavailable(
                    self.id(),
                    format!("hyprctl getoption unavailable: {error}"),
                );
            }
        };

        let snapshot = HyprRiceSnapshot {
            layout,
            gaps_in: read_option_value(ctx, "general:gaps_in", &mut data_options, &mut failures),
            gaps_out: read_option_value(ctx, "general:gaps_out", &mut data_options, &mut failures),
            border_size: read_option_value(
                ctx,
                "general:border_size",
                &mut data_options,
                &mut failures,
            ),
            rounding: read_option_value(
                ctx,
                "decoration:rounding",
                &mut data_options,
                &mut failures,
            ),
            blur: read_option_bool(
                ctx,
                "decoration:blur:enabled",
                &mut data_options,
                &mut failures,
            ),
            animations: read_option_bool(
                ctx,
                "animations:enabled",
                &mut data_options,
                &mut failures,
            ),
        };

        let compact = config.option_bool("compact", true);
        let rows = if compact {
            summarize_hypr_rice(&snapshot)
                .map(|summary| vec![OutputRow::new("Rice", summary)])
                .unwrap_or_default()
        } else {
            hypr_rice_rows(&snapshot)
        };
        let data = json!({ "options": Value::Object(data_options) });

        if rows.is_empty() {
            let message = if failures.is_empty() {
                "hyprctl returned no readable rice options".to_string()
            } else {
                failure_summary(&failures)
            };
            ModuleOutput::unavailable(self.id(), message)
        } else if failures.is_empty() {
            ModuleOutput::ok(self.id(), rows, data)
        } else {
            ModuleOutput::partial(self.id(), rows, data, failure_summary(&failures))
        }
    }
}

#[derive(Debug, Clone, Default)]
struct HyprRiceSnapshot {
    layout: Option<String>,
    gaps_in: Option<String>,
    gaps_out: Option<String>,
    border_size: Option<String>,
    rounding: Option<String>,
    blur: Option<bool>,
    animations: Option<bool>,
}

fn read_option_value(
    ctx: &ModuleContext<'_>,
    name: &str,
    data_options: &mut Map<String, Value>,
    failures: &mut Vec<String>,
) -> Option<String> {
    match read_option(ctx, name, data_options) {
        Ok(option) => hypr_option_value(&option),
        Err(error) => {
            push_failure(failures, error);
            None
        }
    }
}

fn read_option_bool(
    ctx: &ModuleContext<'_>,
    name: &str,
    data_options: &mut Map<String, Value>,
    failures: &mut Vec<String>,
) -> Option<bool> {
    match read_option(ctx, name, data_options) {
        Ok(option) => hypr_option_bool(&option),
        Err(error) => {
            push_failure(failures, error);
            None
        }
    }
}

fn read_option(
    ctx: &ModuleContext<'_>,
    name: &str,
    data_options: &mut Map<String, Value>,
) -> Result<HyprOption, ProviderError> {
    let option = ctx.providers.hyprland.option(name)?;
    data_options.insert(name.to_string(), json!(option));
    Ok(option)
}

fn hypr_rice_rows(snapshot: &HyprRiceSnapshot) -> Vec<OutputRow> {
    let mut rows = Vec::new();
    if let Some(layout) = snapshot.layout.as_deref() {
        rows.push(OutputRow::new("Layout", layout));
    }
    if let Some(gaps) = format_pair(snapshot.gaps_in.as_deref(), snapshot.gaps_out.as_deref()) {
        rows.push(OutputRow::new("Gaps", gaps));
    }
    if let Some(border_size) = snapshot.border_size.as_deref() {
        rows.push(OutputRow::new("Border", format!("{border_size}px")));
    }
    if let Some(rounding) = snapshot.rounding.as_deref() {
        rows.push(OutputRow::new("Rounding", format!("{rounding}px")));
    }
    if let Some(blur) = snapshot.blur {
        rows.push(OutputRow::new("Blur", bool_label(blur)));
    }
    if let Some(animations) = snapshot.animations {
        rows.push(OutputRow::new("Animations", bool_label(animations)));
    }
    rows
}

fn summarize_hypr_rice(snapshot: &HyprRiceSnapshot) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(layout) = snapshot.layout.as_deref() {
        parts.push(layout.to_string());
    }
    if let Some(gaps) = format_pair(snapshot.gaps_in.as_deref(), snapshot.gaps_out.as_deref()) {
        parts.push(format!("gaps:{gaps}"));
    }
    if let Some(border_size) = snapshot.border_size.as_deref() {
        parts.push(format!("border:{border_size}px"));
    }
    if let Some(rounding) = snapshot.rounding.as_deref() {
        parts.push(format!("round:{rounding}px"));
    }
    if let Some(blur) = snapshot.blur {
        parts.push(format!("blur:{}", bool_label(blur)));
    }
    if let Some(animations) = snapshot.animations {
        parts.push(format!("anim:{}", bool_label(animations)));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn hypr_option_value(option: &HyprOption) -> Option<String> {
    option
        .data
        .as_ref()
        .and_then(json_value_string)
        .or_else(|| {
            option
                .str_value
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| option.int.map(|value| value.to_string()))
        .or_else(|| option.float.map(format_float))
        .or_else(|| option.bool_value.map(|value| bool_label(value).to_string()))
        .or_else(|| {
            option
                .css
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(shorten_gaps)
        })
        .or_else(|| {
            option
                .custom
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            option
                .gradient
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            option
                .font_weight
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            option.vec2.as_ref().map(|values| {
                values
                    .iter()
                    .map(|value| format_float(*value))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        })
}

/// The gaps of a gap option, shortened when every side has the same size.
///
/// Hyprland reports the gaps as a CSS string such as "5 5 5 5", which reads
/// better as "5" when all of them are the same.
fn shorten_gaps(value: &str) -> String {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() > 1 && parts.iter().all(|part| *part == parts[0]) {
        return parts[0].to_string();
    }

    value.to_string()
}

fn hypr_option_bool(option: &HyprOption) -> Option<bool> {
    option
        .bool_value
        .or_else(|| option.data.as_ref().and_then(json_value_bool))
        .or_else(|| option.str_value.as_deref().and_then(parse_bool_like_string))
        .or_else(|| option.int.map(|value| value != 0))
}

fn json_value_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(bool_label(*value).to_string()),
        _ => None,
    }
}

pub(super) fn json_value_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_i64().map(|value| value != 0),
        Value::String(value) => parse_bool_like_string(value),
        _ => None,
    }
}

fn parse_bool_like_string(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "enabled" => Some(true),
        "0" | "false" | "no" | "off" | "disabled" => Some(false),
        _ => None,
    }
}

fn format_pair(left: Option<&str>, right: Option<&str>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("{left}/{right}")),
        (Some(value), None) | (None, Some(value)) => Some(value.to_string()),
        (None, None) => None,
    }
}

pub(super) fn bool_label(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn format_float(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::HyprOption;

    #[test]
    fn summarizes_hypr_rice_options() {
        let snapshot = HyprRiceSnapshot {
            layout: Some("dwindle".to_string()),
            gaps_in: Some("5".to_string()),
            gaps_out: Some("20".to_string()),
            border_size: Some("2".to_string()),
            rounding: Some("10".to_string()),
            blur: Some(true),
            animations: Some(false),
        };

        assert_eq!(
            summarize_hypr_rice(&snapshot).as_deref(),
            Some("dwindle gaps:5/20 border:2px round:10px blur:on anim:off")
        );
        assert_eq!(
            hypr_rice_rows(&snapshot),
            vec![
                OutputRow::new("Layout", "dwindle"),
                OutputRow::new("Gaps", "5/20"),
                OutputRow::new("Border", "2px"),
                OutputRow::new("Rounding", "10px"),
                OutputRow::new("Blur", "on"),
                OutputRow::new("Animations", "off"),
            ]
        );
    }

    #[test]
    fn parses_hypr_option_values() {
        let string_option = HyprOption {
            str_value: Some("master".to_string()),
            ..HyprOption::default()
        };
        let bool_option = HyprOption {
            data: Some(serde_json::json!("1")),
            ..HyprOption::default()
        };

        assert_eq!(hypr_option_value(&string_option).as_deref(), Some("master"));
        assert_eq!(hypr_option_bool(&bool_option), Some(true));
    }

    #[test]
    fn parses_the_gap_and_boolean_options() {
        // Hyprland reports the gaps as a CSS string and the booleans in a
        // field of their own.
        let gaps = HyprOption {
            css: Some("5 5 5 5".to_string()),
            ..HyprOption::default()
        };
        let blur = HyprOption {
            bool_value: Some(true),
            ..HyprOption::default()
        };

        assert_eq!(hypr_option_value(&gaps).as_deref(), Some("5"));
        assert_eq!(hypr_option_bool(&blur), Some(true));
    }

    #[test]
    fn keeps_the_gaps_of_every_side() {
        let gaps = HyprOption {
            css: Some("5 0 5 0".to_string()),
            ..HyprOption::default()
        };

        assert_eq!(hypr_option_value(&gaps).as_deref(), Some("5 0 5 0"));
    }
}
