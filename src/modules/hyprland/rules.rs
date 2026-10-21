use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

use super::count_label;

pub struct HyprRulesModule;
impl Module for HyprRulesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprRules
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.workspace_rules() {
            Ok(raw) => {
                let rules = collect_workspace_rules(&raw);
                let compact = config.option_bool("compact", true);
                let rows = if compact {
                    vec![OutputRow::new("Rules", summarize_workspace_rules(&rules))]
                } else {
                    workspace_rule_rows(&rules)
                };

                ModuleOutput::ok(self.id(), rows, json!({ "raw": raw, "rules": rules }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_hypr_workspace_rules() {
        let raw = serde_json::json!([
            {
                "workspaceString": "1",
                "monitor": "eDP-1",
                "default": true,
                "persistent": true
            },
            {
                "workspaceString": "special:music",
                "monitor": "DP-1"
            },
            {
                "workspaceString": "name:web",
                "gapsIn": [5, 5, 5, 5]
            }
        ]);

        let rules = collect_workspace_rules(&raw);
        assert_eq!(
            rules,
            vec![
                WorkspaceRuleSummary {
                    target: Some("1".to_string()),
                    monitor: Some("eDP-1".to_string()),
                    is_default: true,
                    persistent: true,
                    has_gaps: false,
                },
                WorkspaceRuleSummary {
                    target: Some("special:music".to_string()),
                    monitor: Some("DP-1".to_string()),
                    is_default: false,
                    persistent: false,
                    has_gaps: false,
                },
                WorkspaceRuleSummary {
                    target: Some("name:web".to_string()),
                    monitor: None,
                    is_default: false,
                    persistent: false,
                    has_gaps: true,
                },
            ]
        );
        assert_eq!(
            summarize_workspace_rules(&rules),
            "3 rules, targets:1/name:web/special:music, monitors:DP-1/eDP-1, 1 default, 1 persistent, gaps:1"
        );
        assert_eq!(
            workspace_rule_rows(&rules),
            vec![
                OutputRow::new("Workspace Rules", "3 rules"),
                OutputRow::new("Rule Targets", "1, name:web, special:music"),
                OutputRow::new("Rule Monitors", "DP-1, eDP-1"),
                OutputRow::new("Rule Flags", "default:1 persistent:1 gaps:1"),
            ]
        );
    }

    #[test]
    fn handles_empty_hypr_workspace_rules() {
        let rules = collect_workspace_rules(&serde_json::json!([]));
        assert!(rules.is_empty());
        assert_eq!(summarize_workspace_rules(&rules), "0 rules");
        assert_eq!(
            workspace_rule_rows(&rules),
            vec![OutputRow::new("Workspace Rules", "0 rules")]
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WorkspaceRuleSummary {
    pub target: Option<String>,
    pub monitor: Option<String>,
    pub is_default: bool,
    pub persistent: bool,
    pub has_gaps: bool,
}

pub fn summarize_workspace_rules(rules: &[WorkspaceRuleSummary]) -> String {
    let mut parts = vec![count_label(rules.len(), "rule", "rules")];
    let targets = workspace_rule_targets(rules);
    let monitors = workspace_rule_monitors(rules);
    let default_count = rules.iter().filter(|rule| rule.is_default).count();
    let persistent_count = rules.iter().filter(|rule| rule.persistent).count();
    let gap_count = rules.iter().filter(|rule| rule.has_gaps).count();

    if !targets.is_empty() {
        parts.push(format!(
            "targets:{}",
            super::layers::summarize_namespaces(&targets, 4, "/")
        ));
    }
    if !monitors.is_empty() {
        parts.push(format!(
            "monitors:{}",
            super::layers::summarize_namespaces(&monitors, 3, "/")
        ));
    }
    if default_count > 0 {
        parts.push(format!("{default_count} default"));
    }
    if persistent_count > 0 {
        parts.push(format!("{persistent_count} persistent"));
    }
    if gap_count > 0 {
        parts.push(format!("gaps:{gap_count}"));
    }

    parts.join(", ")
}

pub fn workspace_rule_rows(rules: &[WorkspaceRuleSummary]) -> Vec<crate::model::OutputRow> {
    let mut rows = vec![crate::model::OutputRow::new(
        "Workspace Rules",
        count_label(rules.len(), "rule", "rules"),
    )];
    let targets = workspace_rule_targets(rules);
    let monitors = workspace_rule_monitors(rules);
    let flags = workspace_rule_flags(rules);

    if !targets.is_empty() {
        rows.push(crate::model::OutputRow::new(
            "Rule Targets",
            super::layers::summarize_namespaces(&targets, 8, ", "),
        ));
    }
    if !monitors.is_empty() {
        rows.push(crate::model::OutputRow::new(
            "Rule Monitors",
            super::layers::summarize_namespaces(&monitors, 6, ", "),
        ));
    }
    if !flags.is_empty() {
        rows.push(crate::model::OutputRow::new("Rule Flags", flags));
    }

    rows
}

pub fn collect_workspace_rules(value: &serde_json::Value) -> Vec<WorkspaceRuleSummary> {
    let mut rules = Vec::new();
    collect_workspace_rules_into(value, &mut rules);
    rules
}

fn collect_workspace_rules_into(value: &serde_json::Value, rules: &mut Vec<WorkspaceRuleSummary>) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_workspace_rules_into(item, rules);
            }
        }
        serde_json::Value::Object(object) => {
            for key in ["rules", "workspace_rules", "workspaceRules"] {
                if let Some(nested) = object.get(key) {
                    collect_workspace_rules_into(nested, rules);
                    return;
                }
            }

            if object_looks_like_workspace_rule(object) {
                rules.push(workspace_rule_from_object(object));
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

fn workspace_rule_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> WorkspaceRuleSummary {
    WorkspaceRuleSummary {
        target: first_object_value_string(
            object,
            &[
                "workspaceString",
                "workspace_string",
                "workspace",
                "name",
                "workspaceName",
                "workspace_name",
            ],
        ),
        monitor: first_object_value_string(object, &["monitor", "output"]),
        is_default: first_object_bool(object, &["default", "isDefault", "is_default"]),
        persistent: first_object_bool(object, &["persistent", "isPersistent", "is_persistent"]),
        has_gaps: object_has_any_non_empty_key_containing(object, "gap"),
    }
}

fn object_looks_like_workspace_rule(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    [
        "workspaceString",
        "workspace_string",
        "workspace",
        "workspaceName",
        "workspace_name",
        "monitor",
        "output",
        "default",
        "persistent",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
        || object
            .keys()
            .any(|key| key.to_ascii_lowercase().contains("gap"))
}

fn workspace_rule_targets(rules: &[WorkspaceRuleSummary]) -> Vec<String> {
    rules
        .iter()
        .filter_map(|rule| rule.target.as_deref())
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToString::to_string)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn workspace_rule_monitors(rules: &[WorkspaceRuleSummary]) -> Vec<String> {
    rules
        .iter()
        .filter_map(|rule| rule.monitor.as_deref())
        .map(str::trim)
        .filter(|monitor| !monitor.is_empty())
        .map(ToString::to_string)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn workspace_rule_flags(rules: &[WorkspaceRuleSummary]) -> String {
    let mut parts = Vec::new();
    let default_count = rules.iter().filter(|rule| rule.is_default).count();
    let persistent_count = rules.iter().filter(|rule| rule.persistent).count();
    let gap_count = rules.iter().filter(|rule| rule.has_gaps).count();

    if default_count > 0 {
        parts.push(format!("default:{default_count}"));
    }
    if persistent_count > 0 {
        parts.push(format!("persistent:{persistent_count}"));
    }
    if gap_count > 0 {
        parts.push(format!("gaps:{gap_count}"));
    }

    parts.join(" ")
}

pub(super) fn first_object_value_string(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(value_to_rule_string))
        .map(|value| super::config::clean_config_error_text(&value))
        .find(|value| !value.is_empty())
}

fn value_to_rule_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(super::rice::bool_label(*value).to_string()),
        serde_json::Value::Object(object) => first_object_value_string(
            object,
            &["name", "id", "workspaceString", "workspace", "value"],
        ),
        serde_json::Value::Array(_) | serde_json::Value::Null => None,
    }
}

pub(super) fn first_object_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> bool {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(super::rice::json_value_bool))
        .unwrap_or(false)
}

fn object_has_any_non_empty_key_containing(
    object: &serde_json::Map<String, serde_json::Value>,
    needle: &str,
) -> bool {
    object
        .iter()
        .any(|(key, value)| key.to_ascii_lowercase().contains(needle) && value_is_not_empty(value))
}

fn value_is_not_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(values) => !values.is_empty(),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
    }
}
