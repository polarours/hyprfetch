use crate::providers::hyprland::HyprWorkspace;
use crate::providers::theme::EcosystemInfo;

use super::count_label;

pub fn summarize_workspaces(workspaces: &[HyprWorkspace], active_id: Option<i32>) -> String {
    workspaces
        .iter()
        .map(|workspace| workspace_compact_token(workspace, active_id))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn workspace_label(workspace: &HyprWorkspace, active_id: Option<i32>) -> String {
    let base = workspace_name(workspace);
    if workspace.id == active_id {
        format!("*{base}")
    } else {
        base
    }
}

pub fn workspace_details(workspace: &HyprWorkspace) -> String {
    let mut parts = Vec::new();

    if let Some(monitor) = workspace
        .monitor
        .as_deref()
        .filter(|monitor| !monitor.is_empty())
    {
        parts.push(monitor.to_string());
    }

    match workspace.windows {
        Some(0) => parts.push("empty".to_string()),
        Some(windows) => {
            let label = if windows == 1 { "window" } else { "windows" };
            parts.push(format!("{windows} {label}"));
        }
        None => {}
    }

    if workspace_is_special(workspace) {
        parts.push("special".to_string());
    }

    if workspace.has_fullscreen.unwrap_or(false) {
        parts.push("fullscreen".to_string());
    }

    if parts.is_empty() {
        return "active".to_string();
    }

    parts.join(", ")
}

pub fn workspace_sort_key(workspace: &HyprWorkspace) -> (i32, String) {
    let order = match workspace.id {
        Some(id) if id >= 0 => id,
        _ => i32::MAX,
    };
    (order, workspace.display_name())
}

pub fn workspace_compact_token(workspace: &HyprWorkspace, active_id: Option<i32>) -> String {
    let mut token = workspace_label(workspace, active_id);

    if workspace.has_fullscreen.unwrap_or(false) {
        token.push('!');
    }
    if let Some(windows) = workspace.windows {
        token.push(':');
        token.push_str(&windows.to_string());
    }

    token
}

pub fn workspace_name(workspace: &HyprWorkspace) -> String {
    let display = workspace.display_name();
    if workspace_is_special(workspace) {
        let name = display.strip_prefix("special:").unwrap_or(display.as_str());
        format!("[{name}]")
    } else {
        display
    }
}

pub fn workspace_is_special(workspace: &HyprWorkspace) -> bool {
    workspace.id.map(|id| id < 0).unwrap_or(false)
        || workspace
            .name
            .as_deref()
            .map(|name| name.starts_with("special:"))
            .unwrap_or(false)
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
    parts.join(" ")
}

fn eco_display(label: &str, version: &Option<String>) -> String {
    match version {
        Some(v) if !v.is_empty() => format!("{label} ({v})"),
        _ => label.to_string(),
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
            super::summarize_namespaces(&targets, 4, "/")
        ));
    }
    if !monitors.is_empty() {
        parts.push(format!(
            "monitors:{}",
            super::summarize_namespaces(&monitors, 3, "/")
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
            super::summarize_namespaces(&targets, 8, ", "),
        ));
    }
    if !monitors.is_empty() {
        rows.push(crate::model::OutputRow::new(
            "Rule Monitors",
            super::summarize_namespaces(&monitors, 6, ", "),
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

pub fn first_object_value_string(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(value_to_rule_string))
        .map(|value| super::clean_config_error_text(&value))
        .find(|value| !value.is_empty())
}

fn value_to_rule_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(super::bool_label(*value).to_string()),
        serde_json::Value::Object(object) => first_object_value_string(
            object,
            &["name", "id", "workspaceString", "workspace", "value"],
        ),
        serde_json::Value::Array(_) | serde_json::Value::Null => None,
    }
}

pub fn first_object_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> bool {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(super::json_value_bool))
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
