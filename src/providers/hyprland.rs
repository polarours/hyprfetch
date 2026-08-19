use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::providers::system::ProviderError;

const HYPRCTL_COMMAND_TIMEOUT: Duration = Duration::from_millis(1_200);
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ProviderError>;
}

#[derive(Debug, Default)]
pub struct RealCommandRunner {
    hyprctl_preflight: OnceLock<Option<ProviderError>>,
}

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, ProviderError> {
        if let Some(error) = self.preflight_error(program) {
            return Err(error);
        }

        let output = run_command_with_timeout(program, args, HYPRCTL_COMMAND_TIMEOUT)?;

        if output.status.success() {
            return String::from_utf8(output.stdout)
                .map_err(|error| ProviderError::parse(format!("invalid UTF-8: {error}")));
        }

        Err(ProviderError::command(command_failure_message(
            program, args, &output,
        )))
    }
}

impl RealCommandRunner {
    fn preflight_error(&self, program: &str) -> Option<ProviderError> {
        if program != "hyprctl" {
            return None;
        }

        self.hyprctl_preflight
            .get_or_init(hyprctl_preflight_error)
            .clone()
    }
}

fn hyprctl_preflight_error() -> Option<ProviderError> {
    let Some(signature) = env_value("HYPRLAND_INSTANCE_SIGNATURE") else {
        return Some(ProviderError::not_found(
            "hyprctl requires HYPRLAND_INSTANCE_SIGNATURE, but it is missing",
        ));
    };
    let Some(runtime_dir) = env_value("XDG_RUNTIME_DIR") else {
        return Some(ProviderError::not_found(
            "hyprctl requires XDG_RUNTIME_DIR, but it is missing",
        ));
    };

    let instance_dir = Path::new(&runtime_dir).join("hypr").join(&signature);
    let socket_path = instance_dir.join(".socket.sock");
    if !socket_path.exists() {
        return Some(ProviderError::not_found(format!(
            "Hyprland command socket is missing: {}",
            socket_path.display()
        )));
    }

    let lock_path = instance_dir.join("hyprland.lock");
    if let Ok(lock) = fs::read_to_string(lock_path) {
        let (pid, _) = parse_hypr_lock(&lock);
        if let Some(pid) = pid
            && !process_is_running(pid)
        {
            return Some(ProviderError::not_found(format!(
                "Hyprland instance is stale; PID {pid} from hyprland.lock is not running"
            )));
        }
    }

    None
}

fn has_hyprland_session_values(
    signature: Option<&str>,
    current_desktop: Option<&str>,
    desktop_session: Option<&str>,
) -> bool {
    if signature.is_some_and(|value| !value.trim().is_empty()) {
        return true;
    }

    current_desktop
        .into_iter()
        .chain(desktop_session)
        .flat_map(|value| value.split([':', ';', ',']))
        .any(|part| part.trim().eq_ignore_ascii_case("hyprland"))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn process_is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, ProviderError> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ProviderError::io)?;

    let started = Instant::now();
    loop {
        if child.try_wait().map_err(ProviderError::io)?.is_some() {
            return child.wait_with_output().map_err(ProviderError::io);
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProviderError::command(format!(
                "{program} {:?} timed out after {}ms",
                args,
                timeout.as_millis()
            )));
        }

        thread::sleep(COMMAND_POLL_INTERVAL);
    }
}

fn command_failure_message(program: &str, args: &[&str], output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }

    format!("{program} {:?} exited with {}", args, output.status)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprVersion {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub tag: Option<String>,
    pub dirty: Option<bool>,
    #[serde(default)]
    pub source: HyprVersionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HyprVersionSource {
    #[default]
    Hyprctl,
    Binary,
}

impl HyprVersion {
    pub fn display_name(&self) -> String {
        let display = if let Some(tag) = self.tag.as_deref().filter(|tag| !tag.is_empty()) {
            tag.to_string()
        } else {
            let branch = self.branch.as_deref().unwrap_or("unknown");
            let commit = self
                .commit
                .as_deref()
                .map(|commit| commit.chars().take(7).collect::<String>())
                .unwrap_or_else(|| "unknown".to_string());
            format!("{branch} ({commit})")
        };

        match self.source {
            HyprVersionSource::Hyprctl => display,
            HyprVersionSource::Binary => format!("{display} (binary)"),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprWorkspace {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub monitor: Option<String>,
    pub windows: Option<i32>,
    #[serde(rename = "hasfullscreen")]
    pub has_fullscreen: Option<bool>,
}

impl HyprWorkspace {
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.id.map(|id| id.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprWindow {
    pub class: Option<String>,
    pub title: Option<String>,
    pub workspace: Option<HyprWorkspace>,
    #[serde(default)]
    pub floating: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default = "default_true")]
    pub mapped: bool,
    #[serde(default)]
    pub hidden: bool,
    pub fullscreen: Option<i32>,
}

impl HyprWindow {
    pub fn display_name(&self) -> String {
        match (self.class.as_deref(), self.title.as_deref()) {
            (Some(class), Some(title)) if !title.is_empty() => format!("{class} - {title}"),
            (Some(class), _) => class.to_string(),
            (_, Some(title)) if !title.is_empty() => title.to_string(),
            _ => "none".to_string(),
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen.unwrap_or_default() > 0
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprMonitor {
    pub name: String,
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    #[serde(rename = "refreshRate")]
    pub refresh_rate: f64,
    pub scale: f64,
    #[serde(default)]
    pub focused: bool,
    #[serde(rename = "activeWorkspace")]
    pub active_workspace: Option<HyprWorkspace>,
    #[serde(rename = "dpmsStatus")]
    pub dpms_status: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprClient {
    pub address: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprCursorPosition {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HyprInstance {
    pub instance: String,
    pub time: Option<i64>,
    pub pid: Option<u32>,
    pub wl_socket: Option<String>,
    pub socket_path: Option<String>,
    pub event_socket_path: Option<String>,
    pub socket_present: bool,
    pub event_socket_present: bool,
    pub current: bool,
    pub process_running: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprBind {
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub mouse: bool,
    #[serde(default)]
    pub release: bool,
    #[serde(default)]
    pub repeat: bool,
    #[serde(rename = "nonConsuming", default)]
    pub non_consuming: bool,
    #[serde(rename = "hasDescription", default)]
    pub has_description: bool,
    pub description: Option<String>,
    pub modmask: Option<i64>,
    pub submap: Option<String>,
    pub key: Option<String>,
    pub keycode: Option<i64>,
    #[serde(rename = "catchAll", default)]
    pub catch_all: bool,
    pub dispatcher: Option<String>,
    pub arg: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprLayerSurface {
    pub address: Option<String>,
    pub namespace: Option<String>,
    pub layer: Option<i32>,
    pub monitor: Option<i32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub keyboard_interactive: Option<Value>,
    pub exclusive: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprLayerMonitor {
    #[serde(default)]
    pub levels: BTreeMap<String, Vec<HyprLayerSurface>>,
}

pub type HyprLayers = BTreeMap<String, HyprLayerMonitor>;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprInputDevice {
    pub name: Option<String>,
    pub main: Option<bool>,
    pub layout: Option<String>,
    pub variant: Option<String>,
    pub options: Option<String>,
    pub active_keymap: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprDevices {
    #[serde(default)]
    pub keyboards: Vec<HyprInputDevice>,
    #[serde(default)]
    pub mice: Vec<HyprInputDevice>,
    #[serde(default)]
    pub tablets: Vec<HyprInputDevice>,
    #[serde(default)]
    pub touch: Vec<HyprInputDevice>,
    #[serde(default)]
    pub switches: Vec<HyprInputDevice>,
}

impl HyprDevices {
    pub fn total_count(&self) -> usize {
        self.keyboards.len()
            + self.mice.len()
            + self.tablets.len()
            + self.touch.len()
            + self.switches.len()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprOption {
    pub option: Option<String>,
    pub int: Option<i64>,
    pub float: Option<f64>,
    #[serde(rename = "str")]
    pub str_value: Option<String>,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HyprSessionEnv {
    pub instance_signature: Option<String>,
    pub current_desktop: Option<String>,
    pub desktop_session: Option<String>,
    pub wayland_display: Option<String>,
    pub runtime_dir: Option<String>,
    pub socket_path: Option<String>,
    pub event_socket_path: Option<String>,
    pub socket_present: bool,
    pub event_socket_present: bool,
}

impl HyprSessionEnv {
    pub fn detect() -> Self {
        let instance_signature = env_value("HYPRLAND_INSTANCE_SIGNATURE");
        let current_desktop = env_value("XDG_CURRENT_DESKTOP");
        let desktop_session = env_value("DESKTOP_SESSION");
        let wayland_display = env_value("WAYLAND_DISPLAY");
        let runtime_dir = env_value("XDG_RUNTIME_DIR");

        let socket_path = hypr_socket_path(
            runtime_dir.as_deref(),
            instance_signature.as_deref(),
            ".socket.sock",
        );
        let event_socket_path = hypr_socket_path(
            runtime_dir.as_deref(),
            instance_signature.as_deref(),
            ".socket2.sock",
        );

        let socket_present = socket_path
            .as_deref()
            .is_some_and(|path| Path::new(path).exists());
        let event_socket_present = event_socket_path
            .as_deref()
            .is_some_and(|path| Path::new(path).exists());

        Self {
            instance_signature,
            current_desktop,
            desktop_session,
            wayland_display,
            runtime_dir,
            socket_path,
            event_socket_path,
            socket_present,
            event_socket_present,
        }
    }

    pub fn is_hyprland_like(&self) -> bool {
        has_hyprland_session_values(
            self.instance_signature.as_deref(),
            self.current_desktop.as_deref(),
            self.desktop_session.as_deref(),
        )
    }
}

fn hypr_socket_path(
    runtime_dir: Option<&str>,
    instance_signature: Option<&str>,
    socket_name: &str,
) -> Option<String> {
    let runtime_dir = runtime_dir?;
    let instance_signature = instance_signature?;
    Some(
        Path::new(runtime_dir)
            .join("hypr")
            .join(instance_signature)
            .join(socket_name)
            .to_string_lossy()
            .to_string(),
    )
}

#[derive(Clone)]
pub struct HyprlandProvider {
    runner: Arc<dyn CommandRunner>,
    instances: OnceLock<Result<Vec<HyprInstance>, ProviderError>>,
    splash: OnceLock<Result<String, ProviderError>>,
    version: OnceLock<Result<HyprVersion, ProviderError>>,
    active_workspace: OnceLock<Result<HyprWorkspace, ProviderError>>,
    workspaces: OnceLock<Result<Vec<HyprWorkspace>, ProviderError>>,
    active_window: OnceLock<Result<HyprWindow, ProviderError>>,
    monitors: OnceLock<Result<Vec<HyprMonitor>, ProviderError>>,
    clients: OnceLock<Result<Vec<HyprClient>, ProviderError>>,
    cursor_position: OnceLock<Result<HyprCursorPosition, ProviderError>>,
    binds: OnceLock<Result<Vec<HyprBind>, ProviderError>>,
    layers: OnceLock<Result<HyprLayers, ProviderError>>,
    config_errors: OnceLock<Result<Value, ProviderError>>,
    workspace_rules: OnceLock<Result<Value, ProviderError>>,
    global_shortcuts: OnceLock<Result<Value, ProviderError>>,
    layouts: OnceLock<Result<Value, ProviderError>>,
    devices: OnceLock<Result<HyprDevices, ProviderError>>,
}

impl std::fmt::Debug for HyprlandProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyprlandProvider").finish_non_exhaustive()
    }
}

impl HyprlandProvider {
    pub fn with_runner(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            instances: OnceLock::new(),
            splash: OnceLock::new(),
            version: OnceLock::new(),
            active_workspace: OnceLock::new(),
            workspaces: OnceLock::new(),
            active_window: OnceLock::new(),
            monitors: OnceLock::new(),
            clients: OnceLock::new(),
            cursor_position: OnceLock::new(),
            binds: OnceLock::new(),
            layers: OnceLock::new(),
            config_errors: OnceLock::new(),
            workspace_rules: OnceLock::new(),
            global_shortcuts: OnceLock::new(),
            layouts: OnceLock::new(),
            devices: OnceLock::new(),
        }
    }

    pub fn version(&self) -> Result<HyprVersion, ProviderError> {
        self.version
            .get_or_init(|| self.fetch_version_with_fallback())
            .clone()
    }

    pub fn instances(&self) -> Result<Vec<HyprInstance>, ProviderError> {
        self.instances.get_or_init(detect_hypr_instances).clone()
    }

    pub fn splash(&self) -> Result<String, ProviderError> {
        self.splash.get_or_init(|| self.fetch_splash()).clone()
    }

    pub fn active_workspace(&self) -> Result<HyprWorkspace, ProviderError> {
        self.cached(&self.active_workspace, "activeworkspace")
    }

    pub fn workspaces(&self) -> Result<Vec<HyprWorkspace>, ProviderError> {
        self.cached(&self.workspaces, "workspaces")
    }

    pub fn active_window(&self) -> Result<HyprWindow, ProviderError> {
        self.cached(&self.active_window, "activewindow")
    }

    pub fn monitors(&self) -> Result<Vec<HyprMonitor>, ProviderError> {
        self.cached(&self.monitors, "monitors")
    }

    pub fn clients(&self) -> Result<Vec<HyprClient>, ProviderError> {
        self.cached(&self.clients, "clients")
    }

    pub fn cursor_position(&self) -> Result<HyprCursorPosition, ProviderError> {
        self.cached(&self.cursor_position, "cursorpos")
    }

    pub fn binds(&self) -> Result<Vec<HyprBind>, ProviderError> {
        self.cached(&self.binds, "binds")
    }

    pub fn layers(&self) -> Result<HyprLayers, ProviderError> {
        self.cached(&self.layers, "layers")
    }

    pub fn config_errors(&self) -> Result<Value, ProviderError> {
        self.cached(&self.config_errors, "configerrors")
    }

    pub fn workspace_rules(&self) -> Result<Value, ProviderError> {
        self.cached(&self.workspace_rules, "workspacerules")
    }

    pub fn global_shortcuts(&self) -> Result<Value, ProviderError> {
        self.cached(&self.global_shortcuts, "globalshortcuts")
    }

    pub fn layouts(&self) -> Result<Value, ProviderError> {
        self.cached(&self.layouts, "layouts")
    }

    pub fn devices(&self) -> Result<HyprDevices, ProviderError> {
        self.cached(&self.devices, "devices")
    }

    pub fn prefetch(&self) {
        std::thread::scope(|s| {
            s.spawn(|| self.version());
            s.spawn(|| self.instances());
            s.spawn(|| self.splash());
            s.spawn(|| self.active_workspace());
            s.spawn(|| self.workspaces());
            s.spawn(|| self.active_window());
            s.spawn(|| self.monitors());
            s.spawn(|| self.clients());
            s.spawn(|| self.cursor_position());
            s.spawn(|| self.binds());
            s.spawn(|| self.layers());
            s.spawn(|| self.config_errors());
            s.spawn(|| self.workspace_rules());
            s.spawn(|| self.global_shortcuts());
            s.spawn(|| self.layouts());
            s.spawn(|| self.devices());
        });
    }

    pub fn session_env(&self) -> HyprSessionEnv {
        HyprSessionEnv::detect()
    }

    pub fn option(&self, name: &str) -> Result<HyprOption, ProviderError> {
        let raw = self.runner.run("hyprctl", &["-j", "getoption", name])?;
        serde_json::from_str::<HyprOption>(&raw).map_err(|error| {
            ProviderError::parse(format!(
                "failed to parse hyprctl getoption {name} JSON: {error}"
            ))
        })
    }

    fn cached<T>(
        &self,
        cache: &OnceLock<Result<T, ProviderError>>,
        endpoint: &str,
    ) -> Result<T, ProviderError>
    where
        T: Clone + DeserializeOwned,
    {
        cache.get_or_init(|| self.fetch(endpoint)).clone()
    }

    fn fetch<T>(&self, endpoint: &str) -> Result<T, ProviderError>
    where
        T: DeserializeOwned,
    {
        let raw = self.runner.run("hyprctl", &["-j", endpoint])?;
        serde_json::from_str::<T>(&raw).map_err(|error| {
            ProviderError::parse(format!("failed to parse hyprctl {endpoint} JSON: {error}"))
        })
    }

    fn fetch_splash(&self) -> Result<String, ProviderError> {
        let raw = self.runner.run("hyprctl", &["splash"])?;
        let splash = raw.trim();
        if splash.is_empty() {
            return Err(ProviderError::not_found("hyprctl returned empty splash"));
        }
        Ok(splash.to_string())
    }

    fn fetch_version_with_fallback(&self) -> Result<HyprVersion, ProviderError> {
        match self.fetch("version") {
            Ok(version) => Ok(version),
            Err(hyprctl_error) => match self.runner.run("Hyprland", &["--version"]) {
                Ok(raw) => parse_hyprland_version_output(&raw)
                    .map(|mut version| {
                        version.source = HyprVersionSource::Binary;
                        version
                    })
                    .ok_or_else(|| {
                        ProviderError::parse(format!(
                            "failed to parse Hyprland --version output after hyprctl failed: {hyprctl_error}"
                        ))
                    }),
                Err(binary_error) => Err(ProviderError::not_found(format!(
                    "hyprctl version unavailable: {hyprctl_error}; Hyprland --version unavailable: {binary_error}"
                ))),
            },
        }
    }
}

fn default_true() -> bool {
    true
}

fn detect_hypr_instances() -> Result<Vec<HyprInstance>, ProviderError> {
    let runtime_dir = env_value("XDG_RUNTIME_DIR")
        .ok_or_else(|| ProviderError::not_found("XDG_RUNTIME_DIR is not set"))?;
    let root = Path::new(&runtime_dir).join("hypr");
    if !root.exists() {
        return Err(ProviderError::not_found(format!(
            "{} does not exist",
            root.display()
        )));
    }

    let current_signature = env_value("HYPRLAND_INSTANCE_SIGNATURE");
    let mut instances = Vec::new();
    for entry in fs::read_dir(&root).map_err(ProviderError::io)? {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let Some(instance) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let path = entry.path();
        let lock = fs::read_to_string(path.join("hyprland.lock")).unwrap_or_default();
        let (pid, wl_socket) = parse_hypr_lock(&lock);
        let socket_path = path.join(".socket.sock").to_string_lossy().to_string();
        let event_socket_path = path.join(".socket2.sock").to_string_lossy().to_string();
        let process_running = pid.is_some_and(process_is_running);

        instances.push(HyprInstance {
            time: parse_instance_time(&instance),
            current: current_signature.as_deref() == Some(instance.as_str()),
            instance,
            pid,
            wl_socket,
            socket_present: Path::new(&socket_path).exists(),
            event_socket_present: Path::new(&event_socket_path).exists(),
            socket_path: Some(socket_path),
            event_socket_path: Some(event_socket_path),
            process_running,
        });
    }

    if instances.is_empty() {
        return Err(ProviderError::not_found(format!(
            "no Hyprland instances found under {}",
            root.display()
        )));
    }

    instances.sort_by(|left, right| {
        right
            .current
            .cmp(&left.current)
            .then_with(|| right.time.cmp(&left.time))
            .then_with(|| left.instance.cmp(&right.instance))
    });
    Ok(instances)
}

fn parse_hypr_lock(raw: &str) -> (Option<u32>, Option<String>) {
    let mut lines = raw.lines().map(str::trim).filter(|line| !line.is_empty());
    let pid = lines.next().and_then(|line| line.parse::<u32>().ok());
    let wl_socket = lines.next().map(str::to_string);
    (pid, wl_socket)
}

fn parse_instance_time(instance: &str) -> Option<i64> {
    instance.split('_').nth(1)?.parse::<i64>().ok()
}

fn parse_hyprland_version_output(raw: &str) -> Option<HyprVersion> {
    let first_line = raw.lines().next()?.trim();
    let mut version = HyprVersion::default();

    if let Some(rest) = first_line.strip_prefix("Hyprland ") {
        version.tag = rest
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }

    if let Some((_, rest)) = first_line.split_once(" built from branch ") {
        if let Some((branch, commit_rest)) = rest.split_once(" at commit ") {
            version.branch = (!branch.trim().is_empty()).then(|| branch.trim().to_string());
            version.commit = commit_rest
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        } else {
            version.branch = rest
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        }
    } else if let Some((_, rest)) = first_line.split_once(" at commit ") {
        version.commit = rest
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }

    if first_line.contains(" dirty ") {
        version.dirty = Some(true);
    } else if first_line.contains(" clean ") {
        version.dirty = Some(false);
    }

    if let Some(tag) = raw
        .lines()
        .find_map(|line| line.trim().strip_prefix("Tag: "))
        .and_then(|tag_line| tag_line.split(',').next())
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        version.tag = Some(tag.to_string());
    }

    (version.tag.is_some() || version.branch.is_some() || version.commit.is_some())
        .then_some(version)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;

    use super::{
        CommandRunner, HyprVersionSource, HyprlandProvider, RealCommandRunner,
        has_hyprland_session_values, hypr_socket_path, parse_hypr_lock,
        parse_hyprland_version_output, parse_instance_time, run_command_with_timeout,
    };
    use crate::providers::system::ProviderError;

    #[derive(Debug)]
    struct MockRunner {
        payloads: HashMap<String, String>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, _program: &str, args: &[&str]) -> Result<String, ProviderError> {
            let endpoint = args.last().copied().unwrap_or_default();
            self.payloads
                .get(endpoint)
                .cloned()
                .ok_or_else(|| ProviderError::not_found(endpoint))
        }
    }

    #[test]
    fn parses_hyprctl_fixtures() {
        let fixture =
            |name: &str| fs::read_to_string(format!("tests/fixtures/hyprctl/{name}.json")).unwrap();

        let provider = HyprlandProvider::with_runner(Arc::new(MockRunner {
            payloads: HashMap::from([
                ("version".to_string(), fixture("version")),
                ("splash".to_string(), "Keep moving forward.".to_string()),
                ("activeworkspace".to_string(), fixture("activeworkspace")),
                ("workspaces".to_string(), fixture("workspaces")),
                ("activewindow".to_string(), fixture("activewindow")),
                ("monitors".to_string(), fixture("monitors")),
                ("clients".to_string(), fixture("clients")),
                ("cursorpos".to_string(), fixture("cursorpos")),
                ("binds".to_string(), fixture("binds")),
                ("layers".to_string(), fixture("layers")),
                ("configerrors".to_string(), fixture("configerrors")),
                ("workspacerules".to_string(), fixture("workspacerules")),
                ("globalshortcuts".to_string(), fixture("globalshortcuts")),
                ("layouts".to_string(), fixture("layouts")),
                ("devices".to_string(), fixture("devices")),
            ]),
        }));

        let version = provider.version().unwrap();
        assert!(version.display_name().contains("v0.47.2"));

        let splash = provider.splash().unwrap();
        assert_eq!(splash, "Keep moving forward.");

        let workspace = provider.active_workspace().unwrap();
        assert_eq!(workspace.display_name(), "1");

        let workspaces = provider.workspaces().unwrap();
        assert_eq!(workspaces.len(), 4);
        assert_eq!(workspaces[0].windows, Some(2));
        assert_eq!(workspaces[1].has_fullscreen, Some(true));
        assert_eq!(workspaces[2].windows, Some(0));

        let window = provider.active_window().unwrap();
        assert!(window.display_name().contains("kitty"));
        assert!(window.is_fullscreen());
        assert_eq!(
            window.workspace.as_ref().and_then(|workspace| workspace.id),
            Some(1)
        );

        let monitors = provider.monitors().unwrap();
        assert_eq!(monitors.len(), 2);
        assert!(monitors[0].focused);
        assert_eq!(
            monitors[1]
                .active_workspace
                .as_ref()
                .and_then(|workspace| workspace.id),
            Some(2)
        );

        let clients = provider.clients().unwrap();
        assert_eq!(clients.len(), 3);

        let cursor_position = provider.cursor_position().unwrap();
        assert_eq!(cursor_position.x, 1432);
        assert_eq!(cursor_position.y, 860);

        let binds = provider.binds().unwrap();
        assert_eq!(binds.len(), 4);
        assert_eq!(binds[0].description.as_deref(), Some("Open terminal"));
        assert!(binds[2].mouse);
        assert_eq!(binds[3].submap.as_deref(), Some("resize"));

        let layers = provider.layers().unwrap();
        assert_eq!(layers.len(), 2);
        let laptop_layers = layers.get("eDP-1").unwrap();
        assert_eq!(laptop_layers.levels["0"].len(), 1);
        assert_eq!(laptop_layers.levels["2"].len(), 2);
        assert_eq!(
            laptop_layers.levels["2"][0].namespace.as_deref(),
            Some("waybar")
        );

        let config_errors = provider.config_errors().unwrap();
        assert_eq!(config_errors.as_array().unwrap().len(), 2);

        let workspace_rules = provider.workspace_rules().unwrap();
        assert_eq!(workspace_rules.as_array().unwrap().len(), 3);

        let global_shortcuts = provider.global_shortcuts().unwrap();
        assert_eq!(global_shortcuts.as_array().unwrap().len(), 3);

        let layouts = provider.layouts().unwrap();
        assert_eq!(layouts.as_array().unwrap().len(), 3);

        let devices = provider.devices().unwrap();
        assert_eq!(devices.keyboards.len(), 2);
        assert_eq!(
            devices.keyboards[0].active_keymap.as_deref(),
            Some("English (US)")
        );
        assert_eq!(devices.keyboards[0].main, Some(true));
        assert_eq!(devices.mice.len(), 1);
        assert_eq!(devices.total_count(), 5);
    }

    #[test]
    fn command_runner_times_out_stalled_processes() {
        let result = run_command_with_timeout("sleep", &["1"], Duration::from_millis(10));

        assert!(matches!(
            result,
            Err(ProviderError::Command(message)) if message.contains("timed out")
        ));
    }

    #[test]
    fn command_runner_uses_stdout_failure_message_when_stderr_is_empty() {
        let runner = RealCommandRunner::default();
        let result = runner.run("sh", &["-c", "printf 'socket unavailable'; exit 2"]);

        assert!(matches!(
            result,
            Err(ProviderError::Command(message)) if message == "socket unavailable"
        ));
    }

    #[test]
    fn detects_hyprland_session_from_environment_values() {
        assert!(has_hyprland_session_values(Some("abc123"), None, None));
        assert!(has_hyprland_session_values(None, Some("Hyprland"), None));
        assert!(has_hyprland_session_values(
            None,
            Some("sway:Hyprland"),
            None
        ));
        assert!(has_hyprland_session_values(None, None, Some("hyprland")));

        assert!(!has_hyprland_session_values(None, None, None));
        assert!(!has_hyprland_session_values(Some("  "), None, None));
        assert!(!has_hyprland_session_values(
            None,
            Some("GNOME"),
            Some("plasma")
        ));
    }

    #[test]
    fn builds_hypr_socket_paths_from_runtime_dir_and_signature() {
        assert_eq!(
            hypr_socket_path(Some("/run/user/1000"), Some("abc123"), ".socket.sock").as_deref(),
            Some("/run/user/1000/hypr/abc123/.socket.sock")
        );
        assert!(hypr_socket_path(None, Some("abc123"), ".socket.sock").is_none());
        assert!(hypr_socket_path(Some("/run/user/1000"), None, ".socket.sock").is_none());
    }

    #[test]
    fn parses_hypr_instance_lock_file() {
        let (pid, wl_socket) = parse_hypr_lock("6089\nwayland-1\n");
        assert_eq!(pid, Some(6089));
        assert_eq!(wl_socket.as_deref(), Some("wayland-1"));

        let (pid, wl_socket) = parse_hypr_lock("");
        assert_eq!(pid, None);
        assert_eq!(wl_socket, None);
    }

    #[test]
    fn parses_hypr_instance_time_from_signature() {
        assert_eq!(
            parse_instance_time("521ece463c4a9d3d128670688a34756805a4328f_1777474148_1543042573"),
            Some(1777474148)
        );
        assert_eq!(parse_instance_time("invalid"), None);
    }

    #[test]
    fn hyprctl_preflight_ignores_non_hyprctl_commands() {
        let runner = RealCommandRunner::default();
        assert!(runner.preflight_error("sleep").is_none());
    }

    #[test]
    fn parses_hyprctl_getoption() {
        let provider = HyprlandProvider::with_runner(Arc::new(MockRunner {
            payloads: HashMap::from([(
                "general:layout".to_string(),
                r#"{ "option": "general:layout", "int": 0, "float": 0.0, "str": "dwindle", "data": "dwindle" }"#
                    .to_string(),
            )]),
        }));

        let option = provider.option("general:layout").unwrap();
        assert_eq!(option.option.as_deref(), Some("general:layout"));
        assert_eq!(option.str_value.as_deref(), Some("dwindle"));
        assert_eq!(option.data, Some(serde_json::json!("dwindle")));
    }

    #[test]
    fn parses_hyprland_binary_version_output() {
        let raw = r#"Hyprland 0.54.3 built from branch v0.54.3 at commit 521ece463c4a9d3d128670688a34756805a4328f clean (version: bump to 0.54.3).
Date: Fri Mar 27 18:17:50 2026
Tag: v0.54.3, commits: 7014
"#;

        let version = parse_hyprland_version_output(raw).unwrap();
        assert_eq!(version.tag.as_deref(), Some("v0.54.3"));
        assert_eq!(version.branch.as_deref(), Some("v0.54.3"));
        assert_eq!(
            version.commit.as_deref(),
            Some("521ece463c4a9d3d128670688a34756805a4328f")
        );
        assert_eq!(version.dirty, Some(false));
        assert_eq!(version.source, HyprVersionSource::Hyprctl);
    }

    #[test]
    fn falls_back_to_hyprland_binary_version() {
        let provider = HyprlandProvider::with_runner(Arc::new(MockRunner {
            payloads: HashMap::from([(
                "--version".to_string(),
                "Hyprland 0.54.3 built from branch v0.54.3 at commit abcdef123 clean\n".to_string(),
            )]),
        }));

        let version = provider.version().unwrap();
        assert_eq!(version.display_name(), "0.54.3 (binary)");
        assert_eq!(version.branch.as_deref(), Some("v0.54.3"));
        assert_eq!(version.commit.as_deref(), Some("abcdef123"));
        assert_eq!(version.source, HyprVersionSource::Binary);
    }
}
