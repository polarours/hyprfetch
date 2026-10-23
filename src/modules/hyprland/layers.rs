use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

use super::count_label;

pub struct HyprLayersModule;
impl Module for HyprLayersModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprLayers
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.layers() {
            Ok(layers) => {
                if layer_surface_count(&layers) == 0 {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "hyprctl reported no layer surfaces",
                    );
                }

                let compact = config.option_bool("compact", true);
                let mut rows = if compact {
                    vec![OutputRow::new("Layers", summarize_layers(&layers))]
                } else {
                    layer_rows(&layers)
                };

                // Every surface on its own row, so a rice screenshot shows
                // which program draws where.
                if config.option_bool("list", false) {
                    for surface in layer_surface_rows(&layers) {
                        rows.push(surface);
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "layers": layers }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_layers(layers: &crate::providers::hyprland::HyprLayers) -> String {
    let surface_count = layer_surface_count(layers);
    let mut parts = vec![
        count_label(surface_count, "surface", "surfaces"),
        count_label(layers.len(), "output", "outputs"),
    ];

    let namespaces = layer_namespaces(layers);
    if !namespaces.is_empty() {
        parts.push(format!("ns:{}", summarize_namespaces(&namespaces, 4, "/")));
    }
    if let Some(levels) = layer_level_summary(layers) {
        parts.push(levels);
    }

    parts.join(", ")
}

/// One row per layer surface.
fn layer_surface_rows(layers: &crate::providers::hyprland::HyprLayers) -> Vec<OutputRow> {
    let mut rows = Vec::new();

    for (monitor_name, monitor) in layers {
        for (level, surfaces) in &monitor.levels {
            for surface in surfaces {
                let namespace = surface
                    .namespace
                    .as_deref()
                    .filter(|namespace| !namespace.is_empty())
                    .unwrap_or("unknown");

                let mut value = match (surface.x, surface.y) {
                    (Some(x), Some(y)) => format!("{x},{y} "),
                    _ => String::new(),
                };
                value.push_str(layer_level_label(level));
                if !monitor_name.is_empty() {
                    value.push_str(&format!(" on {monitor_name}"));
                }

                rows.push(OutputRow::new(namespace.to_string(), value));
            }
        }
    }

    rows
}

fn layer_rows(layers: &crate::providers::hyprland::HyprLayers) -> Vec<OutputRow> {
    let mut rows = vec![
        OutputRow::new(
            "Layer Surfaces",
            count_label(layer_surface_count(layers), "surface", "surfaces"),
        ),
        OutputRow::new(
            "Layer Outputs",
            count_label(layers.len(), "output", "outputs"),
        ),
    ];

    let namespaces = layer_namespaces(layers);
    if !namespaces.is_empty() {
        rows.push(OutputRow::new(
            "Namespaces",
            summarize_namespaces(&namespaces, 8, ", "),
        ));
    }
    if let Some(levels) = layer_level_summary(layers) {
        rows.push(OutputRow::new("Layer Levels", levels));
    }

    for (monitor, monitor_layers) in layers {
        let Some(summary) = monitor_layer_summary(monitor_layers) else {
            continue;
        };
        rows.push(OutputRow::new(monitor, summary));
    }

    rows
}

fn layer_surface_count(layers: &crate::providers::hyprland::HyprLayers) -> usize {
    layers
        .values()
        .map(|monitor| {
            monitor
                .levels
                .values()
                .map(std::vec::Vec::len)
                .sum::<usize>()
        })
        .sum()
}

fn layer_namespaces(layers: &crate::providers::hyprland::HyprLayers) -> Vec<String> {
    layers
        .values()
        .flat_map(|monitor| monitor.levels.values())
        .flatten()
        .filter_map(|surface| surface.namespace.as_deref())
        .map(str::trim)
        .filter(|namespace| !namespace.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn summarize_namespaces(namespaces: &[String], limit: usize, separator: &str) -> String {
    let mut visible = namespaces.iter().take(limit).cloned().collect::<Vec<_>>();
    let hidden_count = namespaces.len().saturating_sub(visible.len());
    if hidden_count > 0 {
        visible.push(format!("+{hidden_count}"));
    }
    visible.join(separator)
}

fn layer_level_summary(layers: &crate::providers::hyprland::HyprLayers) -> Option<String> {
    let counts = layer_level_counts(layers);
    if counts.is_empty() {
        return None;
    }

    Some(
        counts
            .into_iter()
            .map(|(level, count)| format!("{}:{count}", layer_level_label(&level)))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn monitor_layer_summary(monitor: &crate::providers::hyprland::HyprLayerMonitor) -> Option<String> {
    let counts = monitor_layer_counts(monitor);
    if counts.is_empty() {
        return None;
    }

    Some(
        counts
            .into_iter()
            .map(|(level, count)| format!("{}:{count}", layer_level_label(&level)))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn layer_level_counts(layers: &crate::providers::hyprland::HyprLayers) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for monitor in layers.values() {
        for (level, surfaces) in &monitor.levels {
            if surfaces.is_empty() {
                continue;
            }
            *counts.entry(level.clone()).or_insert(0) += surfaces.len();
        }
    }
    counts
}

fn monitor_layer_counts(
    monitor: &crate::providers::hyprland::HyprLayerMonitor,
) -> BTreeMap<String, usize> {
    monitor
        .levels
        .iter()
        .filter_map(|(level, surfaces)| {
            (!surfaces.is_empty()).then_some((level.clone(), surfaces.len()))
        })
        .collect()
}

fn layer_level_label(level: &str) -> &str {
    match level {
        "0" => "bg",
        "1" => "bottom",
        "2" => "top",
        "3" => "overlay",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::{HyprLayerMonitor, HyprLayerSurface, HyprLayers};

    #[test]
    fn summarizes_hypr_layers() {
        let mut laptop_levels = BTreeMap::new();
        laptop_levels.insert(
            "0".to_string(),
            vec![HyprLayerSurface {
                namespace: Some("hyprpaper".to_string()),
                ..HyprLayerSurface::default()
            }],
        );
        laptop_levels.insert(
            "2".to_string(),
            vec![
                HyprLayerSurface {
                    namespace: Some("waybar".to_string()),
                    ..HyprLayerSurface::default()
                },
                HyprLayerSurface {
                    namespace: Some("mako".to_string()),
                    ..HyprLayerSurface::default()
                },
            ],
        );

        let mut external_levels = BTreeMap::new();
        external_levels.insert("0".to_string(), Vec::new());

        let layers = HyprLayers::from([
            (
                "eDP-1".to_string(),
                HyprLayerMonitor {
                    levels: laptop_levels,
                },
            ),
            (
                "HDMI-A-1".to_string(),
                HyprLayerMonitor {
                    levels: external_levels,
                },
            ),
        ]);

        assert_eq!(
            summarize_layers(&layers),
            "3 surfaces, 2 outputs, ns:hyprpaper/mako/waybar, bg:1 top:2"
        );
        assert_eq!(
            layer_rows(&layers),
            vec![
                OutputRow::new("Layer Surfaces", "3 surfaces"),
                OutputRow::new("Layer Outputs", "2 outputs"),
                OutputRow::new("Namespaces", "hyprpaper, mako, waybar"),
                OutputRow::new("Layer Levels", "bg:1 top:2"),
                OutputRow::new("eDP-1", "bg:1 top:2"),
            ]
        );
    }

    #[test]
    fn lists_every_layer_surface() {
        let layers = crate::providers::hyprland::HyprLayers::from([(
            "eDP-1".to_string(),
            crate::providers::hyprland::HyprLayerMonitor {
                levels: BTreeMap::from([(
                    "2".to_string(),
                    vec![crate::providers::hyprland::HyprLayerSurface {
                        namespace: Some("quickshell".to_string()),
                        x: Some(0),
                        y: Some(1279),
                        ..Default::default()
                    }],
                )]),
            },
        )]);

        let rows = layer_surface_rows(&layers);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "quickshell");
        assert_eq!(rows[0].value, "0,1279 top on eDP-1");
    }
}
