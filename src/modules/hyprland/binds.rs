use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

use super::count_label;

pub struct HyprBindsModule;
impl Module for HyprBindsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprBinds
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.binds() {
            Ok(binds) => {
                if binds.is_empty() {
                    return ModuleOutput::unavailable(self.id(), "hyprctl reported no binds");
                }

                let compact = config.option_bool("compact", true);
                let mut rows = if compact {
                    vec![OutputRow::new("Binds", summarize_binds(&binds))]
                } else {
                    bind_rows(&binds)
                };

                // Every binding on its own row, for a rice screenshot.
                if config.option_bool("list", false) {
                    for bind in listed_binds(&binds) {
                        rows.push(bind);
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "binds": binds }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_binds(binds: &[crate::providers::hyprland::HyprBind]) -> String {
    let mut parts = vec![count_label(binds.len(), "bind", "binds")];
    let mouse_count = binds.iter().filter(|bind| bind.mouse).count();
    let repeat_count = binds.iter().filter(|bind| bind.repeat).count();
    let submaps = bind_submaps(binds);

    if mouse_count > 0 {
        parts.push(format!("{mouse_count} mouse"));
    }
    if repeat_count > 0 {
        parts.push(format!("{repeat_count} repeat"));
    }
    if !submaps.is_empty() {
        parts.push(format!("{} submaps:{}", submaps.len(), submaps.join("/")));
    }
    if let Some(dispatchers) = compact_dispatcher_summary(binds) {
        parts.push(format!("top:{dispatchers}"));
    }

    parts.join(", ")
}

fn bind_rows(binds: &[crate::providers::hyprland::HyprBind]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Binds",
        count_label(binds.len(), "bind", "binds"),
    )];
    let mouse_count = binds.iter().filter(|bind| bind.mouse).count();
    let repeat_count = binds.iter().filter(|bind| bind.repeat).count();
    let locked_count = binds.iter().filter(|bind| bind.locked).count();
    let submaps = bind_submaps(binds);

    push_bind_count_row(&mut rows, "Mouse Binds", mouse_count);
    push_bind_count_row(&mut rows, "Repeat Binds", repeat_count);
    push_bind_count_row(&mut rows, "Locked Binds", locked_count);

    if !submaps.is_empty() {
        rows.push(OutputRow::new("Submaps", submaps.join(", ")));
    }
    if let Some(dispatchers) = counted_dispatcher_summary(binds, 6) {
        rows.push(OutputRow::new("Dispatchers", dispatchers));
    }

    rows
}

/// One row per binding, with the modifiers spelled out.
fn listed_binds(binds: &[crate::providers::hyprland::HyprBind]) -> Vec<OutputRow> {
    binds
        .iter()
        .map(|bind| {
            let key = bind.key.as_deref().unwrap_or("?");
            let mut label = match bind.modmask.unwrap_or_default() {
                0 => key.to_string(),
                mask => format!("{} + {key}", modifiers(mask)),
            };
            if bind.mouse {
                label.push_str(" (mouse)");
            }
            if bind.repeat {
                label.push_str(" (repeat)");
            }

            let mut value = match bind.description.as_deref().map(str::trim) {
                Some(description) if !description.is_empty() => description.to_string(),
                _ => match bind.dispatcher.as_deref().map(str::trim) {
                    Some(dispatcher) if !dispatcher.is_empty() => dispatcher.to_string(),
                    _ => "exec".to_string(),
                },
            };
            if let Some(arg) = bind
                .arg
                .as_deref()
                .map(str::trim)
                .filter(|arg| !arg.is_empty())
            {
                value.push_str(&format!(" ({arg})"));
            }

            OutputRow::new(label, value)
        })
        .collect()
}

/// The modifiers of a binding, spelled out.
fn modifiers(mask: i64) -> String {
    const NAMES: &[(i64, &str)] = &[
        (1 << 0, "SHIFT"),
        (1 << 1, "CAPS"),
        (1 << 2, "CTRL"),
        (1 << 3, "ALT"),
        (1 << 4, "MOD2"),
        (1 << 5, "MOD3"),
        (1 << 6, "SUPER"),
        (1 << 7, "MOD5"),
    ];

    let names: Vec<&str> = NAMES
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, name)| *name)
        .collect();

    if names.is_empty() {
        "NONE".to_string()
    } else {
        names.join(" + ")
    }
}

fn bind_submaps(binds: &[crate::providers::hyprland::HyprBind]) -> Vec<String> {
    binds
        .iter()
        .filter_map(|bind| bind.submap.as_deref())
        .map(str::trim)
        .filter(|submap| !submap.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn compact_dispatcher_summary(binds: &[crate::providers::hyprland::HyprBind]) -> Option<String> {
    let dispatchers = top_dispatchers(binds, 3);
    if dispatchers.is_empty() {
        return None;
    }

    Some(
        dispatchers
            .into_iter()
            .map(|(dispatcher, _)| dispatcher)
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn counted_dispatcher_summary(
    binds: &[crate::providers::hyprland::HyprBind],
    limit: usize,
) -> Option<String> {
    let dispatchers = top_dispatchers(binds, limit);
    if dispatchers.is_empty() {
        return None;
    }

    Some(
        dispatchers
            .into_iter()
            .map(|(dispatcher, count)| format!("{dispatcher}:{count}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn top_dispatchers(
    binds: &[crate::providers::hyprland::HyprBind],
    limit: usize,
) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::new();
    for dispatcher in binds
        .iter()
        .filter_map(|bind| bind.dispatcher.as_deref())
        .map(str::trim)
        .filter(|dispatcher| !dispatcher.is_empty())
    {
        *counts.entry(dispatcher.to_string()).or_insert(0) += 1;
    }

    let mut dispatchers = counts.into_iter().collect::<Vec<_>>();
    dispatchers.sort_by(|(left_name, left_count), (right_name, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_name.cmp(right_name))
    });
    dispatchers.truncate(limit);
    dispatchers
}

fn push_bind_count_row(rows: &mut Vec<OutputRow>, label: &str, count: usize) {
    if count > 0 {
        rows.push(OutputRow::new(label, count_label(count, "bind", "binds")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::HyprBind;

    #[test]
    fn summarizes_hypr_binds() {
        let binds = vec![
            HyprBind {
                dispatcher: Some("exec".to_string()),
                ..HyprBind::default()
            },
            HyprBind {
                dispatcher: Some("workspace".to_string()),
                repeat: true,
                ..HyprBind::default()
            },
            HyprBind {
                dispatcher: Some("exec".to_string()),
                mouse: true,
                ..HyprBind::default()
            },
            HyprBind {
                dispatcher: Some("submap".to_string()),
                submap: Some("resize".to_string()),
                locked: true,
                ..HyprBind::default()
            },
        ];

        assert_eq!(
            summarize_binds(&binds),
            "4 binds, 1 mouse, 1 repeat, 1 submaps:resize, top:exec/submap/workspace"
        );
        assert_eq!(
            top_dispatchers(&binds, 2),
            vec![("exec".to_string(), 2), ("submap".to_string(), 1)]
        );
        assert_eq!(
            bind_rows(&binds),
            vec![
                OutputRow::new("Binds", "4 binds"),
                OutputRow::new("Mouse Binds", "1 bind"),
                OutputRow::new("Repeat Binds", "1 bind"),
                OutputRow::new("Locked Binds", "1 bind"),
                OutputRow::new("Submaps", "resize"),
                OutputRow::new("Dispatchers", "exec:2, submap:1, workspace:1"),
            ]
        );
    }

    #[test]
    fn spells_out_the_modifiers_of_a_binding() {
        assert_eq!(modifiers(0), "NONE");
        assert_eq!(modifiers(64), "SUPER");
        assert_eq!(modifiers(64 | 1), "SHIFT + SUPER");
        assert_eq!(modifiers(64 | 4), "CTRL + SUPER");
    }

    #[test]
    fn lists_a_binding_with_its_dispatcher() {
        let binds = vec![HyprBind {
            key: Some("Q".to_string()),
            modmask: Some(64),
            dispatcher: Some("__lua".to_string()),
            arg: Some("6".to_string()),
            ..HyprBind::default()
        }];

        let rows = listed_binds(&binds);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "SUPER + Q");
        assert_eq!(rows[0].value, "__lua (6)");
    }

    #[test]
    fn prefers_the_description_of_a_binding() {
        let binds = vec![HyprBind {
            key: Some("Q".to_string()),
            modmask: Some(0),
            description: Some("close window".to_string()),
            dispatcher: Some("killactive".to_string()),
            ..HyprBind::default()
        }];

        let rows = listed_binds(&binds);
        assert_eq!(rows[0].key, "Q");
        assert_eq!(rows[0].value, "close window");
    }
}
