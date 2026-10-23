use serde_json::json;
use std::collections::BTreeMap;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::hyprland::HyprWorkspace;

use super::count_label;
use super::layers::summarize_namespaces;
use super::workspaces::workspace_name;

/// The layouts that the workspaces use.
///
/// Hyprland does not have an endpoint that lists the layouts it knows, but
/// every workspace reports the layout it is tiled with.
pub struct HyprLayoutsModule;
impl Module for HyprLayoutsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprLayouts
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.workspaces() {
            Ok(workspaces) => {
                let layouts = collect_layouts(&workspaces);
                if layouts.is_empty() {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "hyprctl reported no tiled layout for any workspace",
                    );
                }

                let compact = config.option_bool("compact", true);
                let mut rows = if compact {
                    vec![OutputRow::new("Layouts", summarize_layouts(&layouts))]
                } else {
                    layout_rows(&layouts)
                };

                // Every workspace on its own row, so a rice screenshot shows
                // which layout a workspace is tiled with.
                if config.option_bool("list", false) {
                    for row in listed_layouts(&workspaces) {
                        rows.push(row);
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "layouts": layouts }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

/// A layout and the workspaces that use it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct LayoutSummary {
    name: String,
    workspaces: Vec<String>,
}

fn collect_layouts(workspaces: &[HyprWorkspace]) -> Vec<LayoutSummary> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for workspace in workspaces {
        let Some(layout) = workspace
            .tiled_layout
            .as_deref()
            .map(str::trim)
            .filter(|layout| !layout.is_empty())
        else {
            continue;
        };

        grouped
            .entry(layout.to_string())
            .or_default()
            .push(workspace_name(workspace));
    }

    grouped
        .into_iter()
        .map(|(name, workspaces)| LayoutSummary { name, workspaces })
        .collect()
}

fn summarize_layouts(layouts: &[LayoutSummary]) -> String {
    let names = layout_names(layouts);
    if names.is_empty() {
        return count_label(0, "layout", "layouts");
    }

    summarize_namespaces(&names, 4, "/")
}

fn layout_rows(layouts: &[LayoutSummary]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Layouts",
        count_label(layouts.len(), "layout", "layouts"),
    )];
    let names = layout_names(layouts);

    if !names.is_empty() {
        rows.push(OutputRow::new(
            "Layout Names",
            summarize_namespaces(&names, 8, ", "),
        ));
    }

    for layout in layouts {
        rows.push(OutputRow::new(
            layout.name.clone(),
            count_label(layout.workspaces.len(), "workspace", "workspaces"),
        ));
    }

    rows
}

/// One row per workspace, with the layout it is tiled with.
fn listed_layouts(workspaces: &[HyprWorkspace]) -> Vec<OutputRow> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            let layout = workspace
                .tiled_layout
                .as_deref()
                .map(str::trim)
                .filter(|layout| !layout.is_empty())?;
            Some(OutputRow::new(
                workspace_name(workspace),
                layout.to_string(),
            ))
        })
        .collect()
}

fn layout_names(layouts: &[LayoutSummary]) -> Vec<String> {
    layouts
        .iter()
        .map(|layout| layout.name.clone())
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(id: i32, name: &str, layout: Option<&str>) -> HyprWorkspace {
        HyprWorkspace {
            id: Some(id),
            name: Some(name.to_string()),
            tiled_layout: layout.map(str::to_string),
            ..HyprWorkspace::default()
        }
    }

    #[test]
    fn summarizes_the_layouts_of_the_workspaces() {
        let workspaces = vec![
            workspace(1, "1", Some("dwindle")),
            workspace(2, "2", Some("dwindle")),
            workspace(3, "3", Some("master")),
        ];

        let layouts = collect_layouts(&workspaces);
        assert_eq!(
            layouts,
            vec![
                LayoutSummary {
                    name: "dwindle".to_string(),
                    workspaces: vec!["1".to_string(), "2".to_string()],
                },
                LayoutSummary {
                    name: "master".to_string(),
                    workspaces: vec!["3".to_string()],
                },
            ]
        );
        assert_eq!(summarize_layouts(&layouts), "dwindle/master");
        assert_eq!(
            layout_rows(&layouts),
            vec![
                OutputRow::new("Layouts", "2 layouts"),
                OutputRow::new("Layout Names", "dwindle, master"),
                OutputRow::new("dwindle", "2 workspaces"),
                OutputRow::new("master", "1 workspace"),
            ]
        );
    }

    #[test]
    fn lists_the_layout_of_every_workspace() {
        let workspaces = vec![
            workspace(1, "1", Some("dwindle")),
            workspace(-99, "special:music", Some("master")),
            workspace(4, "4", None),
        ];

        let rows = listed_layouts(&workspaces);
        assert_eq!(
            rows,
            vec![
                OutputRow::new("1", "dwindle"),
                OutputRow::new("[music]", "master"),
            ]
        );
    }

    #[test]
    fn handles_workspaces_without_a_layout() {
        let workspaces = vec![workspace(1, "1", None), workspace(2, "2", Some("  "))];

        let layouts = collect_layouts(&workspaces);
        assert!(layouts.is_empty());
        assert_eq!(summarize_layouts(&layouts), "0 layouts");
        assert!(listed_layouts(&workspaces).is_empty());
    }
}
