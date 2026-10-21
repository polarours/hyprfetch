use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

use super::config::clean_config_error_text;
use super::count_label;
use super::layers::summarize_namespaces;
use super::rules::first_object_value_string;

pub struct HyprLayoutsModule;
impl Module for HyprLayoutsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprLayouts
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.layouts() {
            Ok(raw) => {
                let layouts = collect_layouts(&raw);
                let compact = config.option_bool("compact", true);
                let rows = if compact {
                    vec![OutputRow::new("Layouts", summarize_layouts(&layouts))]
                } else {
                    layout_rows(&layouts)
                };

                ModuleOutput::ok(self.id(), rows, json!({ "raw": raw, "layouts": layouts }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct LayoutSummary {
    name: String,
    source: Option<String>,
}

fn summarize_layouts(layouts: &[LayoutSummary]) -> String {
    let mut parts = vec![count_label(layouts.len(), "layout", "layouts")];
    let names = layout_names(layouts);
    let plugin_count = layouts
        .iter()
        .filter(|layout| layout.source.is_some())
        .count();

    if !names.is_empty() {
        parts.push(summarize_namespaces(&names, 5, "/"));
    }
    if plugin_count > 0 {
        parts.push(format!("{plugin_count} plugin"));
    }

    parts.join(", ")
}

fn layout_rows(layouts: &[LayoutSummary]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Layouts",
        count_label(layouts.len(), "layout", "layouts"),
    )];
    let names = layout_names(layouts);
    let sources = layout_sources(layouts);

    if !names.is_empty() {
        rows.push(OutputRow::new(
            "Layout Names",
            summarize_namespaces(&names, 8, ", "),
        ));
    }
    if !sources.is_empty() {
        rows.push(OutputRow::new(
            "Layout Sources",
            summarize_namespaces(&sources, 6, ", "),
        ));
    }

    rows
}

fn collect_layouts(value: &Value) -> Vec<LayoutSummary> {
    let mut layouts = Vec::new();
    collect_layouts_into(value, &mut layouts);
    layouts.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.source.cmp(&right.source))
    });
    layouts.dedup();
    layouts
}

fn collect_layouts_into(value: &Value, layouts: &mut Vec<LayoutSummary>) {
    match value {
        Value::String(value) => push_layout_name(layouts, value, None),
        Value::Array(items) => {
            for item in items {
                collect_layouts_into(item, layouts);
            }
        }
        Value::Object(object) => {
            for key in [
                "layouts",
                "available",
                "availableLayouts",
                "available_layouts",
            ] {
                if let Some(nested) = object.get(key) {
                    collect_layouts_into(nested, layouts);
                    return;
                }
            }

            if let Some(layout) = layout_from_object(object) {
                layouts.push(layout);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn layout_from_object(object: &Map<String, Value>) -> Option<LayoutSummary> {
    let name = first_object_value_string(object, &["name", "layout", "id", "type"])?;
    let source = first_object_value_string(object, &["plugin", "source", "owner", "namespace"]);
    Some(LayoutSummary { name, source })
}

fn push_layout_name(layouts: &mut Vec<LayoutSummary>, name: &str, source: Option<String>) {
    let name = clean_config_error_text(name);
    if name.is_empty() {
        return;
    }

    layouts.push(LayoutSummary { name, source });
}

fn layout_names(layouts: &[LayoutSummary]) -> Vec<String> {
    layouts
        .iter()
        .map(|layout| layout.name.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn layout_sources(layouts: &[LayoutSummary]) -> Vec<String> {
    layouts
        .iter()
        .filter_map(|layout| layout.source.as_deref())
        .map(str::trim)
        .filter(|source| !source.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_hypr_layouts() {
        let raw = serde_json::json!({
            "layouts": [
                "dwindle",
                "master",
                {
                    "name": "hy3",
                    "plugin": "hyprland-hy3"
                }
            ]
        });

        let layouts = collect_layouts(&raw);
        assert_eq!(
            layouts,
            vec![
                LayoutSummary {
                    name: "dwindle".to_string(),
                    source: None,
                },
                LayoutSummary {
                    name: "hy3".to_string(),
                    source: Some("hyprland-hy3".to_string()),
                },
                LayoutSummary {
                    name: "master".to_string(),
                    source: None,
                },
            ]
        );
        assert_eq!(
            summarize_layouts(&layouts),
            "3 layouts, dwindle/hy3/master, 1 plugin"
        );
        assert_eq!(
            layout_rows(&layouts),
            vec![
                OutputRow::new("Layouts", "3 layouts"),
                OutputRow::new("Layout Names", "dwindle, hy3, master"),
                OutputRow::new("Layout Sources", "hyprland-hy3"),
            ]
        );
    }

    #[test]
    fn handles_empty_hypr_layouts() {
        let layouts = collect_layouts(&serde_json::json!([]));
        assert!(layouts.is_empty());
        assert_eq!(summarize_layouts(&layouts), "0 layouts");
        assert_eq!(
            layout_rows(&layouts),
            vec![OutputRow::new("Layouts", "0 layouts")]
        );
    }
}
