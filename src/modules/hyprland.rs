use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::hyprland::{HyprInstance, HyprOption, HyprSessionEnv};
use crate::providers::system::ProviderError;
use crate::util::{format_refresh_hz, truncate_visible};

pub struct HyprlandModule;
pub struct HyprHealthModule;
pub struct HyprInstancesModule;
pub struct HyprSplashModule;
pub struct HyprWorkspacesModule;
pub struct HyprMonitorsModule;
pub struct HyprActiveWindowModule;
pub struct HyprCursorModule;
pub struct HyprRiceModule;
pub struct HyprDevicesModule;
pub struct HyprBindsModule;
pub struct HyprLayersModule;
pub struct HyprConfigModule;
pub struct HyprRulesModule;
pub struct HyprShortcutsModule;
pub struct HyprLayoutsModule;
pub struct HyprEcosystemModule;

impl Module for HyprlandModule {
    fn id(&self) -> ModuleId {
        ModuleId::Hyprland
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let version = ctx.providers.hyprland.version();
        let workspace = ctx.providers.hyprland.active_workspace();
        let clients = ctx.providers.hyprland.clients();
        let monitors = ctx.providers.hyprland.monitors();
        let workspaces = ctx.providers.hyprland.workspaces();
        let session_env = ctx.providers.hyprland.session_env();

        let mut rows = Vec::new();
        let mut data = json!({});
        data["runtime"] = json!(&session_env);
        let mut failures = Vec::new();

        match version {
            Ok(version) => {
                data["version"] = json!(version);
                rows.push(OutputRow::new("Hyprland", version.display_name()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match workspace {
            Ok(workspace) => {
                data["active_workspace"] = json!(workspace);
                rows.push(OutputRow::new("Workspace", workspace.display_name()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match &clients {
            Ok(clients) => {
                data["client_count"] = json!(clients.len());
                rows.push(OutputRow::new("Clients", clients.len().to_string()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match &monitors {
            Ok(monitors) => {
                data["monitors"] = json!(monitors);
                if let Some(summary) = focused_monitor_summary(monitors) {
                    rows.push(OutputRow::new("Focused", summary));
                }
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match (&monitors, &workspaces, &clients) {
            (Ok(monitors), Ok(workspaces), Ok(clients)) => {
                data["workspaces"] = json!(workspaces);
                rows.push(OutputRow::new(
                    "Session",
                    session_overview(monitors, workspaces, clients.len()),
                ));
            }
            (_, Err(error), _) => push_failure(&mut failures, error),
            _ => {}
        }

        let has_hyprctl_rows = !rows.is_empty();
        if (config.option_bool("show_runtime", false)
            || !has_hyprctl_rows && session_env.is_hyprland_like())
            && let Some(summary) = summarize_hypr_runtime(&session_env)
        {
            rows.push(OutputRow::new("Runtime", summary));
        }

        if !failures.is_empty() {
            rows.push(OutputRow::new("Hyprctl", failure_summary(&failures)));
        }

        if rows.is_empty() {
            ModuleOutput::unavailable(self.id(), failure_summary(&failures))
        } else if failures.is_empty() {
            ModuleOutput::ok(self.id(), rows, data)
        } else {
            ModuleOutput::partial(self.id(), rows, data, failure_summary(&failures))
        }
    }
}

impl Module for HyprHealthModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprHealth
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        let session_env = ctx.providers.hyprland.session_env();
        let instances = ctx.providers.hyprland.instances().ok();
        let wallpaper = ctx.providers.theme.wallpaper_backend().ok();
        let ecosystem = ctx.providers.theme.ecosystem_info().ok();

        let mut warnings = Vec::new();
        let mut rows = Vec::new();

        if !session_env.socket_present {
            warnings.push("command socket missing".to_string());
        }
        if !session_env.event_socket_present {
            warnings.push("event socket missing".to_string());
        }

        if let Some(instances) = instances.as_deref() {
            warnings.extend(hypr_instance_warnings(instances));
        } else if session_env.is_hyprland_like() {
            warnings.push("instance metadata unavailable".to_string());
        }

        if wallpaper.is_none() {
            warnings.push("wallpaper backend missing".to_string());
        }

        let ecosystem = ecosystem.unwrap_or_default();
        if ecosystem.bar.is_none() {
            warnings.push("bar missing".to_string());
        }
        if ecosystem.notifications.is_none() {
            warnings.push("notifications missing".to_string());
        }
        if ecosystem.idle.is_none() {
            warnings.push("idle daemon missing".to_string());
        }
        if ecosystem.lock.is_none() {
            warnings.push("lock daemon missing".to_string());
        }

        let summary = if warnings.is_empty() {
            "ready".to_string()
        } else {
            summarize_health_warnings(&warnings)
        };
        rows.push(OutputRow::new("Health", summary));

        let data = json!({
            "runtime": session_env,
            "instances": instances,
            "wallpaper": wallpaper,
            "ecosystem": ecosystem,
            "warnings": warnings,
        });

        let warnings = data["warnings"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<_>>();

        if warnings.is_empty() {
            ModuleOutput::ok(self.id(), rows, data)
        } else {
            ModuleOutput::partial(self.id(), rows, data, failure_summary(&warnings))
        }
    }
}

impl Module for HyprInstancesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprInstances
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.instances() {
            Ok(instances) => {
                let compact = config.option_bool("compact", true);
                let warnings = hypr_instance_warnings(&instances);
                let rows = if compact {
                    vec![OutputRow::new(
                        "Instances",
                        summarize_hypr_instances(&instances),
                    )]
                } else {
                    let mut rows = vec![OutputRow::new(
                        "Instances",
                        count_label(instances.len(), "instance", "instances"),
                    )];
                    for instance in &instances {
                        rows.push(OutputRow::new(
                            hypr_instance_label(instance),
                            hypr_instance_details(instance),
                        ));
                    }
                    rows
                };

                if warnings.is_empty() {
                    ModuleOutput::ok(self.id(), rows, json!(instances))
                } else {
                    ModuleOutput::partial(
                        self.id(),
                        rows,
                        json!(instances),
                        failure_summary(&warnings),
                    )
                }
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprSplashModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprSplash
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.splash() {
            Ok(splash) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Splash", splash.clone())],
                json!({ "splash": splash }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprMonitorsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprMonitors
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.monitors() {
            Ok(monitors) => {
                let compact = config.option_bool("compact", true);
                if compact {
                    let summary = monitors
                        .iter()
                        .map(summarize_monitor)
                        .collect::<Vec<_>>()
                        .join(", ");
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new(
                            "Monitors",
                            format!("{} ({summary})", monitors.len()),
                        )],
                        json!({ "monitors": monitors }),
                    );
                }

                let mut rows = vec![OutputRow::new("Monitors", monitors.len().to_string())];
                for monitor in &monitors {
                    rows.push(OutputRow::new(
                        monitor_label(monitor),
                        monitor_details(monitor),
                    ));
                }

                ModuleOutput::ok(self.id(), rows, json!({ "monitors": monitors }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprWorkspacesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprWorkspaces
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let active_workspace = ctx.providers.hyprland.active_workspace();
        let workspaces = ctx.providers.hyprland.workspaces();

        match (active_workspace, workspaces) {
            (Ok(active_workspace), Ok(mut workspaces)) => {
                if workspaces.is_empty() {
                    return ModuleOutput::unavailable(self.id(), "hyprctl reported no workspaces");
                }

                workspaces.sort_by_key(workspace_sort_key);
                let compact = config.option_bool("compact", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new(
                            "Workspaces",
                            summarize_workspaces(&workspaces, active_workspace.id),
                        )],
                        json!({ "active_workspace": active_workspace, "workspaces": workspaces }),
                    );
                }

                let mut rows = vec![OutputRow::new("Workspaces", workspaces.len().to_string())];
                for workspace in &workspaces {
                    rows.push(OutputRow::new(
                        workspace_label(workspace, active_workspace.id),
                        workspace_details(workspace),
                    ));
                }

                ModuleOutput::ok(
                    self.id(),
                    rows,
                    json!({ "active_workspace": active_workspace, "workspaces": workspaces }),
                )
            }
            (Err(error), Ok(_)) | (Ok(_), Err(error)) => {
                ModuleOutput::unavailable(self.id(), error.to_string())
            }
            (Err(left), Err(right)) => {
                let mut failures = Vec::new();
                push_failure(&mut failures, left);
                push_failure(&mut failures, right);
                ModuleOutput::unavailable(self.id(), failure_summary(&failures))
            }
        }
    }
}

impl Module for HyprActiveWindowModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprActiveWindow
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.active_window() {
            Ok(window) => {
                let compact = config.option_bool("compact", true);
                if compact {
                    let display = summarize_active_window(&window, 72);
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Window", display)],
                        json!({ "window": window }),
                    );
                }

                let mut rows = Vec::new();
                if let Some(title) = window.title.as_deref().filter(|title| !title.is_empty()) {
                    rows.push(OutputRow::new("Title", title));
                }
                if let Some(class) = window.class.as_deref().filter(|class| !class.is_empty()) {
                    rows.push(OutputRow::new("Class", class));
                }
                if let Some(workspace) = window.workspace.as_ref() {
                    rows.push(OutputRow::new("Workspace", workspace.display_name()));
                }
                rows.push(OutputRow::new("State", window_state(&window)));

                ModuleOutput::ok(self.id(), rows, json!({ "window": window }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprCursorModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprCursor
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.cursor_position() {
            Ok(position) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new(
                    "Cursor",
                    format!("{}, {}", position.x, position.y),
                )],
                json!(position),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

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

impl Module for HyprDevicesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprDevices
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.devices() {
            Ok(devices) => {
                if devices.total_count() == 0 {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "hyprctl reported no input devices",
                    );
                }

                let compact = config.option_bool("compact", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Input", summarize_input_devices(&devices))],
                        json!({ "devices": devices }),
                    );
                }

                let mut rows = vec![OutputRow::new("Input", devices.total_count().to_string())];
                push_device_row(&mut rows, "Keyboards", devices.keyboards.len());
                push_device_row(&mut rows, "Mice", devices.mice.len());
                push_device_row(&mut rows, "Tablets", devices.tablets.len());
                push_device_row(&mut rows, "Touch", devices.touch.len());
                push_device_row(&mut rows, "Switches", devices.switches.len());
                if let Some(keyboard) = primary_keyboard(&devices) {
                    if let Some(name) = keyboard.name.as_deref().filter(|name| !name.is_empty()) {
                        rows.push(OutputRow::new("Main Keyboard", name));
                    }
                    if let Some(keymap) = keyboard
                        .active_keymap
                        .as_deref()
                        .filter(|keymap| !keymap.is_empty())
                    {
                        rows.push(OutputRow::new("Keymap", keymap));
                    }
                }

                ModuleOutput::ok(self.id(), rows, json!({ "devices": devices }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

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
                let rows = if compact {
                    vec![OutputRow::new("Binds", summarize_binds(&binds))]
                } else {
                    bind_rows(&binds)
                };

                ModuleOutput::ok(self.id(), rows, json!({ "binds": binds }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

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
                let rows = if compact {
                    vec![OutputRow::new("Layers", summarize_layers(&layers))]
                } else {
                    layer_rows(&layers)
                };

                ModuleOutput::ok(self.id(), rows, json!({ "layers": layers }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

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

impl Module for HyprEcosystemModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprEcosystem
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.theme.ecosystem_info() {
            Ok(ecosystem) => {
                let include_wallpaper = !ctx
                    .runtime
                    .modules
                    .iter()
                    .any(|module| module.name == ModuleId::Wallpaper);
                let ecosystem = prepare_ecosystem_for_display(ecosystem, include_wallpaper);
                if ecosystem.is_empty() {
                    return ModuleOutput::unavailable(
                        self.id(),
                        "no additional Hyprland ecosystem tools detected",
                    );
                }

                let compact = config.option_bool("compact", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Ecosystem", summarize_ecosystem(&ecosystem))],
                        json!(ecosystem),
                    );
                }

                let mut rows = Vec::new();
                if let Some(value) = ecosystem.wallpaper.as_deref() {
                    rows.push(OutputRow::new("Wallpaper", value));
                }
                if let Some(value) = ecosystem.bar.as_deref() {
                    rows.push(OutputRow::new("Bar", value));
                }
                if let Some(value) = ecosystem.launcher.as_deref() {
                    rows.push(OutputRow::new("Launcher", value));
                }
                if let Some(value) = ecosystem.notifications.as_deref() {
                    rows.push(OutputRow::new("Notifications", value));
                }
                if let Some(value) = ecosystem.clipboard.as_deref() {
                    rows.push(OutputRow::new("Clipboard", value));
                }
                if let Some(value) = ecosystem.screenshot.as_deref() {
                    rows.push(OutputRow::new("Screenshot", value));
                }
                if let Some(value) = ecosystem.idle.as_deref() {
                    rows.push(OutputRow::new("Idle", value));
                }
                if let Some(value) = ecosystem.lock.as_deref() {
                    rows.push(OutputRow::new("Lock", value));
                }

                ModuleOutput::ok(self.id(), rows, json!(ecosystem))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
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

fn push_failure(failures: &mut Vec<String>, error: impl ToString) {
    let message = error.to_string();
    if !message.is_empty() && !failures.iter().any(|existing| existing == &message) {
        failures.push(message);
    }
}

fn failure_summary(failures: &[String]) -> String {
    failures.join("; ")
}

fn summarize_health_warnings(warnings: &[String]) -> String {
    let mut parts = Vec::new();
    if warnings.iter().any(|warning| warning.contains("stale")) {
        parts.push("stale".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("socket")) {
        parts.push("socket issue".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("wallpaper")) {
        parts.push("wallpaper missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("bar")) {
        parts.push("bar missing".to_string());
    }
    if warnings
        .iter()
        .any(|warning| warning.contains("notifications"))
    {
        parts.push("notifications missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("idle")) {
        parts.push("idle missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("lock")) {
        parts.push("lock missing".to_string());
    }

    if parts.is_empty() {
        return count_label(warnings.len(), "warning", "warnings");
    }

    parts.join(", ")
}

fn summarize_hypr_instances(instances: &[HyprInstance]) -> String {
    let mut parts = vec![count_label(instances.len(), "instance", "instances")];

    if let Some(current) = instances.iter().find(|instance| instance.current) {
        parts.push(format!(
            "current:{}",
            truncate_visible(&current.instance, 8)
        ));
        if let Some(pid) = current.pid {
            parts.push(format!("pid:{pid}"));
        }
        if let Some(wl_socket) = current.wl_socket.as_deref() {
            parts.push(format!("wl:{wl_socket}"));
        }
        if !current.process_running {
            parts.push("stale".to_string());
        }
    }

    parts.join(", ")
}

fn hypr_instance_label(instance: &HyprInstance) -> String {
    let prefix = if instance.current { "*" } else { "" };
    format!("{prefix}{}", truncate_visible(&instance.instance, 8))
}

fn hypr_instance_details(instance: &HyprInstance) -> String {
    let mut parts = Vec::new();

    if let Some(pid) = instance.pid {
        parts.push(format!("pid:{pid}"));
    }
    if let Some(wl_socket) = instance.wl_socket.as_deref() {
        parts.push(format!("wl:{wl_socket}"));
    }
    parts.push(format!(
        "socket:{}",
        if instance.socket_present {
            "ok"
        } else {
            "missing"
        }
    ));
    parts.push(format!(
        "events:{}",
        if instance.event_socket_present {
            "ok"
        } else {
            "missing"
        }
    ));
    if !instance.process_running {
        parts.push("stale".to_string());
    }

    parts.join(" ")
}

fn hypr_instance_warnings(instances: &[HyprInstance]) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(current) = instances.iter().find(|instance| instance.current) else {
        return warnings;
    };

    if !current.process_running {
        warnings.push("current Hyprland instance is stale or its PID is missing".to_string());
    }
    if !current.socket_present {
        warnings.push("current Hyprland command socket is missing".to_string());
    }
    if !current.event_socket_present {
        warnings.push("current Hyprland event socket is missing".to_string());
    }

    warnings
}

fn summarize_hypr_runtime(session: &HyprSessionEnv) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(signature) = session.instance_signature.as_deref() {
        parts.push(format!("sig:{}", truncate_visible(signature, 8)));
    } else if session.is_hyprland_like() {
        parts.push("sig:missing".to_string());
    }

    if session.socket_path.is_some() {
        parts.push(format!(
            "socket:{}",
            if session.socket_present {
                "ok"
            } else {
                "missing"
            }
        ));
    }

    if session.event_socket_path.is_some() {
        parts.push(format!(
            "events:{}",
            if session.event_socket_present {
                "ok"
            } else {
                "missing"
            }
        ));
    }

    if let Some(display) = session.wayland_display.as_deref() {
        parts.push(format!("display:{display}"));
    }

    if parts.is_empty() {
        return session
            .is_hyprland_like()
            .then(|| "hyprland env detected".to_string());
    }

    Some(parts.join(" "))
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
}

fn hypr_option_bool(option: &HyprOption) -> Option<bool> {
    option
        .data
        .as_ref()
        .and_then(json_value_bool)
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

fn json_value_bool(value: &Value) -> Option<bool> {
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

fn bool_label(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn format_float(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn summarize_input_devices(devices: &crate::providers::hyprland::HyprDevices) -> String {
    let mut parts = Vec::new();
    push_summary_part(&mut parts, devices.keyboards.len(), "keyboard", "keyboards");
    push_summary_part(&mut parts, devices.mice.len(), "mouse", "mice");
    push_summary_part(&mut parts, devices.tablets.len(), "tablet", "tablets");
    push_summary_part(&mut parts, devices.touch.len(), "touch", "touch");
    push_summary_part(&mut parts, devices.switches.len(), "switch", "switches");
    if let Some(keymap) = primary_keyboard_keymap(devices) {
        parts.push(format!("keymap:{keymap}"));
    }
    parts.join(", ")
}

fn primary_keyboard(
    devices: &crate::providers::hyprland::HyprDevices,
) -> Option<&crate::providers::hyprland::HyprInputDevice> {
    devices
        .keyboards
        .iter()
        .find(|keyboard| keyboard.main.unwrap_or(false))
        .or_else(|| devices.keyboards.first())
}

fn primary_keyboard_keymap(devices: &crate::providers::hyprland::HyprDevices) -> Option<&str> {
    primary_keyboard(devices).and_then(|keyboard| {
        keyboard
            .active_keymap
            .as_deref()
            .filter(|keymap| !keymap.is_empty())
    })
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

fn count_label(count: usize, singular: &str, plural: &str) -> String {
    let label = if count == 1 { singular } else { plural };
    format!("{count} {label}")
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

fn summarize_namespaces(namespaces: &[String], limit: usize, separator: &str) -> String {
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

fn clean_config_error_text(value: &str) -> String {
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct WorkspaceRuleSummary {
    target: Option<String>,
    monitor: Option<String>,
    is_default: bool,
    persistent: bool,
    has_gaps: bool,
}

fn summarize_workspace_rules(rules: &[WorkspaceRuleSummary]) -> String {
    let mut parts = vec![count_label(rules.len(), "rule", "rules")];
    let targets = workspace_rule_targets(rules);
    let monitors = workspace_rule_monitors(rules);
    let default_count = rules.iter().filter(|rule| rule.is_default).count();
    let persistent_count = rules.iter().filter(|rule| rule.persistent).count();
    let gap_count = rules.iter().filter(|rule| rule.has_gaps).count();

    if !targets.is_empty() {
        parts.push(format!(
            "targets:{}",
            summarize_namespaces(&targets, 4, "/")
        ));
    }
    if !monitors.is_empty() {
        parts.push(format!(
            "monitors:{}",
            summarize_namespaces(&monitors, 3, "/")
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

fn workspace_rule_rows(rules: &[WorkspaceRuleSummary]) -> Vec<OutputRow> {
    let mut rows = vec![OutputRow::new(
        "Workspace Rules",
        count_label(rules.len(), "rule", "rules"),
    )];
    let targets = workspace_rule_targets(rules);
    let monitors = workspace_rule_monitors(rules);
    let flags = workspace_rule_flags(rules);

    if !targets.is_empty() {
        rows.push(OutputRow::new(
            "Rule Targets",
            summarize_namespaces(&targets, 8, ", "),
        ));
    }
    if !monitors.is_empty() {
        rows.push(OutputRow::new(
            "Rule Monitors",
            summarize_namespaces(&monitors, 6, ", "),
        ));
    }
    if !flags.is_empty() {
        rows.push(OutputRow::new("Rule Flags", flags));
    }

    rows
}

fn collect_workspace_rules(value: &Value) -> Vec<WorkspaceRuleSummary> {
    let mut rules = Vec::new();
    collect_workspace_rules_into(value, &mut rules);
    rules
}

fn collect_workspace_rules_into(value: &Value, rules: &mut Vec<WorkspaceRuleSummary>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_workspace_rules_into(item, rules);
            }
        }
        Value::Object(object) => {
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
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn workspace_rule_from_object(object: &Map<String, Value>) -> WorkspaceRuleSummary {
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

fn object_looks_like_workspace_rule(object: &Map<String, Value>) -> bool {
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
        .collect::<BTreeSet<_>>()
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
        .collect::<BTreeSet<_>>()
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

fn first_object_value_string(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(value_to_rule_string))
        .map(|value| clean_config_error_text(&value))
        .find(|value| !value.is_empty())
}

fn value_to_rule_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(bool_label(*value).to_string()),
        Value::Object(object) => first_object_value_string(
            object,
            &["name", "id", "workspaceString", "workspace", "value"],
        ),
        Value::Array(_) | Value::Null => None,
    }
}

fn first_object_bool(object: &Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(json_value_bool))
        .unwrap_or(false)
}

fn object_has_any_non_empty_key_containing(object: &Map<String, Value>, needle: &str) -> bool {
    object
        .iter()
        .any(|(key, value)| key.to_ascii_lowercase().contains(needle) && value_is_not_empty(value))
}

fn value_is_not_empty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
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

fn focused_monitor_summary(monitors: &[crate::providers::hyprland::HyprMonitor]) -> Option<String> {
    let monitor = monitors.iter().find(|monitor| monitor.focused)?;
    let mut summary = monitor.name.clone();

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        summary.push_str(&format!(" (ws:{})", workspace.display_name()));
    }

    Some(summary)
}

fn session_overview(
    monitors: &[crate::providers::hyprland::HyprMonitor],
    workspaces: &[crate::providers::hyprland::HyprWorkspace],
    _client_count: usize,
) -> String {
    let total_workspaces = workspaces.len();
    let special_count = workspaces
        .iter()
        .filter(|workspace| workspace_is_special(workspace))
        .count();
    let fullscreen = workspaces
        .iter()
        .any(|workspace| workspace.has_fullscreen.unwrap_or(false));
    let total_windows: i32 = workspaces
        .iter()
        .filter_map(|workspace| workspace.windows)
        .sum();

    let mut summary = format!(
        "{} outputs, {} ws, {} wins",
        monitors.len(),
        total_workspaces,
        total_windows
    );

    if special_count > 0 {
        summary.push_str(&format!(" ({} special)", special_count));
    }

    if fullscreen {
        summary.push_str(", fullscreen");
    }

    summary
}

fn summarize_workspaces(
    workspaces: &[crate::providers::hyprland::HyprWorkspace],
    active_id: Option<i32>,
) -> String {
    workspaces
        .iter()
        .map(|workspace| workspace_compact_token(workspace, active_id))
        .collect::<Vec<_>>()
        .join(" ")
}

fn workspace_label(
    workspace: &crate::providers::hyprland::HyprWorkspace,
    active_id: Option<i32>,
) -> String {
    let base = workspace_name(workspace);
    if workspace.id == active_id {
        format!("*{base}")
    } else {
        base
    }
}

fn workspace_details(workspace: &crate::providers::hyprland::HyprWorkspace) -> String {
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

fn workspace_sort_key(workspace: &crate::providers::hyprland::HyprWorkspace) -> (i32, String) {
    let order = match workspace.id {
        Some(id) if id >= 0 => id,
        _ => i32::MAX,
    };
    (order, workspace.display_name())
}

fn workspace_compact_token(
    workspace: &crate::providers::hyprland::HyprWorkspace,
    active_id: Option<i32>,
) -> String {
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

fn workspace_name(workspace: &crate::providers::hyprland::HyprWorkspace) -> String {
    let display = workspace.display_name();
    if workspace_is_special(workspace) {
        let name = display.strip_prefix("special:").unwrap_or(display.as_str());
        format!("[{name}]")
    } else {
        display
    }
}

fn workspace_is_special(workspace: &crate::providers::hyprland::HyprWorkspace) -> bool {
    workspace.id.map(|id| id < 0).unwrap_or(false)
        || workspace
            .name
            .as_deref()
            .map(|name| name.starts_with("special:"))
            .unwrap_or(false)
}

fn prepare_ecosystem_for_display(
    mut ecosystem: crate::providers::theme::EcosystemInfo,
    include_wallpaper: bool,
) -> crate::providers::theme::EcosystemInfo {
    if !include_wallpaper {
        ecosystem.wallpaper = None;
    }
    ecosystem
}

fn summarize_ecosystem(ecosystem: &crate::providers::theme::EcosystemInfo) -> String {
    let mut parts = Vec::new();
    if let Some(value) = ecosystem.wallpaper.as_deref() {
        parts.push(format!("wall:{value}"));
    }
    if let Some(value) = ecosystem.bar.as_deref() {
        parts.push(format!("bar:{value}"));
    }
    if let Some(value) = ecosystem.launcher.as_deref() {
        parts.push(format!("launch:{value}"));
    }
    if let Some(value) = ecosystem.notifications.as_deref() {
        parts.push(format!("notify:{value}"));
    }
    if let Some(value) = ecosystem.clipboard.as_deref() {
        parts.push(format!("clip:{value}"));
    }
    if let Some(value) = ecosystem.screenshot.as_deref() {
        parts.push(format!("shot:{value}"));
    }
    if let Some(value) = ecosystem.idle.as_deref() {
        parts.push(format!("idle:{value}"));
    }
    if let Some(value) = ecosystem.lock.as_deref() {
        parts.push(format!("lock:{value}"));
    }
    parts.join(" ")
}

fn summarize_monitor(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut summary = monitor_label(monitor);

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        summary.push(':');
        summary.push_str(&workspace_name(workspace));
    }
    summary.push(' ');
    summary.push_str(&monitor_mode_summary(monitor));

    if matches!(monitor.dpms_status, Some(false)) {
        summary.push_str(" off");
    }

    summary
}

fn monitor_label(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    if monitor.focused {
        format!("*{}", monitor.name)
    } else {
        monitor.name.clone()
    }
}

fn monitor_details(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut parts = Vec::new();

    if let Some(description) = monitor
        .description
        .as_deref()
        .filter(|description| !description.is_empty())
    {
        parts.push(description.to_string());
    }

    parts.push(format!(
        "{}x{} @ {}",
        monitor.width,
        monitor.height,
        format_refresh_hz(monitor.refresh_rate)
    ));

    if !scale_is_default(monitor.scale) {
        parts.push(format!("scale:{:.2}", monitor.scale));
    }

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        parts.push(format!("ws:{}", workspace_name(workspace)));
    }
    if matches!(monitor.dpms_status, Some(false)) {
        parts.push("dpms:off".to_string());
    }

    parts.join(", ")
}

fn monitor_mode_summary(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut summary = format!(
        "{}x{}@{}",
        monitor.width,
        monitor.height,
        format_refresh_hz(monitor.refresh_rate)
    );

    if !scale_is_default(monitor.scale) {
        summary.push_str(&format!(" x{:.2}", monitor.scale));
    }

    summary
}

fn scale_is_default(scale: f64) -> bool {
    (scale - 1.0).abs() < 0.01
}

fn summarize_active_window(
    window: &crate::providers::hyprland::HyprWindow,
    max_chars: usize,
) -> String {
    let title = window
        .title
        .as_deref()
        .filter(|title| !title.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| window.display_name());
    let mut metadata = Vec::new();

    if let Some(class) = window.class.as_deref().filter(|class| !class.is_empty())
        && window
            .title
            .as_deref()
            .filter(|title| !title.is_empty())
            .is_some()
    {
        metadata.push(class.to_string());
    }
    if let Some(workspace) = window.workspace.as_ref() {
        metadata.push(format!("ws:{}", workspace.display_name()));
    }

    let state = window_state(window);
    if state != "tiled" {
        metadata.push(state);
    }

    if metadata.is_empty() {
        return truncate_visible(&title, max_chars);
    }

    let metadata = format!(" [{}]", metadata.join(", "));
    let metadata_width = metadata.chars().count();
    if metadata_width >= max_chars {
        return truncate_visible(&metadata, max_chars);
    }

    let title_budget = max_chars.saturating_sub(metadata_width);
    format!("{}{}", truncate_visible(&title, title_budget), metadata)
}

fn window_state(window: &crate::providers::hyprland::HyprWindow) -> String {
    let mut states = Vec::new();

    if window.is_fullscreen() {
        states.push("fullscreen");
    }
    if window.floating {
        states.push("floating");
    }
    if window.pinned {
        states.push("pinned");
    }
    if window.hidden {
        states.push("hidden");
    }
    if !window.mapped {
        states.push("unmapped");
    }

    if states.is_empty() {
        return "tiled".to_string();
    }

    states.join(", ")
}

fn push_device_row(rows: &mut Vec<OutputRow>, label: &str, count: usize) {
    if count > 0 {
        rows.push(OutputRow::new(label, count.to_string()));
    }
}

fn push_summary_part(parts: &mut Vec<String>, count: usize, singular: &str, plural: &str) {
    if count == 0 {
        return;
    }

    let label = if count == 1 { singular } else { plural };
    parts.push(format!("{count} {label}"));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        GlobalShortcutSummary, HyprRiceSnapshot, LayoutSummary, WorkspaceRuleSummary, bind_rows,
        collect_config_error_messages, collect_global_shortcuts, collect_layouts,
        collect_workspace_rules, config_error_rows, focused_monitor_summary, global_shortcut_rows,
        hypr_instance_warnings, hypr_option_bool, hypr_option_value, hypr_rice_rows, layer_rows,
        layout_rows, monitor_details, prepare_ecosystem_for_display, primary_keyboard_keymap,
        session_overview, summarize_active_window, summarize_binds, summarize_config_errors,
        summarize_ecosystem, summarize_global_shortcuts, summarize_hypr_instances,
        summarize_hypr_rice, summarize_hypr_runtime, summarize_input_devices, summarize_layers,
        summarize_layouts, summarize_monitor, summarize_workspace_rules, summarize_workspaces,
        top_dispatchers, window_state, workspace_compact_token, workspace_details, workspace_label,
        workspace_rule_rows, workspace_sort_key,
    };
    use crate::model::OutputRow;
    use crate::providers::hyprland::{
        HyprBind, HyprDevices, HyprInputDevice, HyprInstance, HyprLayerMonitor, HyprLayerSurface,
        HyprLayers, HyprMonitor, HyprOption, HyprSessionEnv, HyprWindow, HyprWorkspace,
    };
    use crate::providers::theme::EcosystemInfo;

    #[test]
    fn marks_active_workspace_in_summary() {
        let workspaces = vec![
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                windows: Some(1),
                ..HyprWorkspace::default()
            },
        ];

        assert_eq!(summarize_workspaces(&workspaces, Some(2)), "1 *2:1");
        assert_eq!(workspace_label(&workspaces[0], Some(1)), "*1");
    }

    #[test]
    fn formats_workspace_details() {
        let workspace = HyprWorkspace {
            id: Some(1),
            name: Some("1".to_string()),
            monitor: Some("DP-1".to_string()),
            windows: Some(2),
            has_fullscreen: Some(true),
        };

        assert_eq!(workspace_details(&workspace), "DP-1, 2 windows, fullscreen");
    }

    #[test]
    fn formats_special_and_empty_workspaces() {
        let workspace = HyprWorkspace {
            id: Some(-99),
            name: Some("special:music".to_string()),
            monitor: Some("HDMI-A-1".to_string()),
            windows: Some(0),
            ..HyprWorkspace::default()
        };

        assert_eq!(workspace_label(&workspace, Some(1)), "[music]");
        assert_eq!(workspace_compact_token(&workspace, Some(1)), "[music]:0");
        assert_eq!(workspace_details(&workspace), "HDMI-A-1, empty, special");
    }

    #[test]
    fn sorts_special_workspaces_last() {
        let mut workspaces = [
            HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            },
        ];

        workspaces.sort_by_key(workspace_sort_key);
        assert_eq!(workspaces[0].display_name(), "1");
        assert_eq!(workspaces[1].display_name(), "special:music");
    }

    #[test]
    fn formats_monitor_details_with_workspace_and_dpms() {
        let monitor = HyprMonitor {
            name: "HDMI-A-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            scale: 1.0,
            dpms_status: Some(false),
            active_workspace: Some(HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                ..HyprWorkspace::default()
            }),
            ..HyprMonitor::default()
        };

        assert_eq!(
            monitor_details(&monitor),
            "1920x1080 @ 60Hz, ws:2, dpms:off"
        );
    }

    #[test]
    fn summarizes_monitor_compact_output() {
        let monitor = HyprMonitor {
            name: "DP-1".to_string(),
            width: 2560,
            height: 1440,
            refresh_rate: 165.0,
            scale: 1.25,
            focused: true,
            active_workspace: Some(HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                ..HyprWorkspace::default()
            }),
            dpms_status: Some(false),
            ..HyprMonitor::default()
        };

        assert_eq!(
            summarize_monitor(&monitor),
            "*DP-1:[music] 2560x1440@165Hz x1.25 off"
        );
    }

    #[test]
    fn summarizes_focused_monitor_and_session() {
        let monitors = vec![
            HyprMonitor {
                name: "DP-1".to_string(),
                focused: true,
                active_workspace: Some(HyprWorkspace {
                    id: Some(1),
                    name: Some("1".to_string()),
                    ..HyprWorkspace::default()
                }),
                ..HyprMonitor::default()
            },
            HyprMonitor {
                name: "HDMI-A-1".to_string(),
                ..HyprMonitor::default()
            },
        ];
        let workspaces = vec![
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                windows: Some(2),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                windows: Some(1),
                has_fullscreen: Some(true),
                ..HyprWorkspace::default()
            },
        ];

        assert_eq!(
            focused_monitor_summary(&monitors).as_deref(),
            Some("DP-1 (ws:1)")
        );
        assert_eq!(
            session_overview(&monitors, &workspaces, 3),
            "2 outputs, 2 ws, 3 wins (1 special), fullscreen"
        );
    }

    #[test]
    fn summarizes_hypr_runtime_status() {
        let session = HyprSessionEnv {
            instance_signature: Some("abcdef12".to_string()),
            wayland_display: Some("wayland-1".to_string()),
            socket_path: Some("/run/user/1000/hypr/abcdef12/.socket.sock".to_string()),
            event_socket_path: Some("/run/user/1000/hypr/abcdef12/.socket2.sock".to_string()),
            socket_present: true,
            event_socket_present: false,
            ..HyprSessionEnv::default()
        };

        assert_eq!(
            summarize_hypr_runtime(&session).as_deref(),
            Some("sig:abcdef12 socket:ok events:missing display:wayland-1")
        );
    }

    #[test]
    fn summarizes_missing_hypr_runtime_signature() {
        let session = HyprSessionEnv {
            current_desktop: Some("Hyprland".to_string()),
            wayland_display: Some("wayland-1".to_string()),
            ..HyprSessionEnv::default()
        };

        assert_eq!(
            summarize_hypr_runtime(&session).as_deref(),
            Some("sig:missing display:wayland-1")
        );
    }

    #[test]
    fn summarizes_hypr_instances() {
        let instances = vec![
            HyprInstance {
                instance: "abcdef12_1777474148_1543042573".to_string(),
                pid: Some(6089),
                wl_socket: Some("wayland-1".to_string()),
                current: true,
                socket_present: true,
                event_socket_present: true,
                process_running: true,
                ..HyprInstance::default()
            },
            HyprInstance {
                instance: "stale_1777470000_1".to_string(),
                process_running: false,
                ..HyprInstance::default()
            },
        ];

        assert_eq!(
            summarize_hypr_instances(&instances),
            "2 instances, current:abcdef1…, pid:6089, wl:wayland-1"
        );

        let stale_current = vec![HyprInstance {
            instance: "stale".to_string(),
            current: true,
            process_running: false,
            ..HyprInstance::default()
        }];
        assert_eq!(
            summarize_hypr_instances(&stale_current),
            "1 instance, current:stale, stale"
        );
        assert_eq!(
            hypr_instance_warnings(&stale_current),
            vec![
                "current Hyprland instance is stale or its PID is missing".to_string(),
                "current Hyprland command socket is missing".to_string(),
                "current Hyprland event socket is missing".to_string(),
            ]
        );
    }

    #[test]
    fn summarizes_ecosystem_compact_output() {
        let ecosystem = EcosystemInfo {
            wallpaper: Some("hyprpaper".to_string()),
            bar: Some("waybar".to_string()),
            launcher: Some("fuzzel".to_string()),
            notifications: Some("mako".to_string()),
            clipboard: Some("cliphist".to_string()),
            screenshot: Some("grim".to_string()),
            idle: Some("hypridle".to_string()),
            lock: Some("hyprlock".to_string()),
        };

        assert_eq!(
            summarize_ecosystem(&ecosystem),
            "wall:hyprpaper bar:waybar launch:fuzzel notify:mako clip:cliphist shot:grim idle:hypridle lock:hyprlock"
        );
    }

    #[test]
    fn drops_wallpaper_from_ecosystem_when_wallpaper_module_is_enabled() {
        let ecosystem = EcosystemInfo {
            wallpaper: Some("hyprpaper".to_string()),
            bar: Some("waybar".to_string()),
            ..EcosystemInfo::default()
        };

        let filtered = prepare_ecosystem_for_display(ecosystem, false);
        assert_eq!(summarize_ecosystem(&filtered), "bar:waybar");
    }

    #[test]
    fn formats_active_window_state_summary() {
        let window = HyprWindow {
            class: Some("kitty".to_string()),
            title: Some("codex - hyprfetch".to_string()),
            workspace: Some(HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            }),
            fullscreen: Some(1),
            mapped: true,
            ..HyprWindow::default()
        };

        assert_eq!(window_state(&window), "fullscreen");
        assert_eq!(
            summarize_active_window(&window, 72),
            "codex - hyprfetch [kitty, ws:1, fullscreen]"
        );
    }

    #[test]
    fn preserves_window_metadata_when_title_is_long() {
        let window = HyprWindow {
            class: Some("firefox".to_string()),
            title: Some(
                "This is a very long browser title that should not hide workspace state"
                    .to_string(),
            ),
            workspace: Some(HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                ..HyprWorkspace::default()
            }),
            floating: true,
            mapped: true,
            ..HyprWindow::default()
        };

        let summary = summarize_active_window(&window, 48);
        assert!(summary.ends_with("[firefox, ws:2, floating]"));
        assert!(summary.chars().count() <= 48);
    }

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
    fn summarizes_primary_keyboard_keymap() {
        let devices = HyprDevices {
            keyboards: vec![
                HyprInputDevice {
                    name: Some("Fallback Keyboard".to_string()),
                    active_keymap: Some("English (US)".to_string()),
                    ..HyprInputDevice::default()
                },
                HyprInputDevice {
                    name: Some("Keychron Q1".to_string()),
                    main: Some(true),
                    active_keymap: Some("Colemak".to_string()),
                    ..HyprInputDevice::default()
                },
            ],
            mice: vec![HyprInputDevice {
                name: Some("Logitech G Pro Wireless".to_string()),
                ..HyprInputDevice::default()
            }],
            ..HyprDevices::default()
        };

        assert_eq!(primary_keyboard_keymap(&devices), Some("Colemak"));
        assert_eq!(
            summarize_input_devices(&devices),
            "2 keyboards, 1 mouse, keymap:Colemak"
        );
    }

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
