//! Talking to Hyprland.
//!
//! The provider keeps one file per part of the compositor state and
//! runs `hyprctl` through a command runner that can be replaced in tests.

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::providers::system::ProviderError;

mod config;
mod display;
mod input;
mod session;
mod window;
mod workspace;

use serde::de::DeserializeOwned;

use session::{env_value, parse_hypr_lock, process_is_running};

pub use config::{HyprBind, HyprOption};
pub use display::{HyprLayerMonitor, HyprLayerSurface, HyprLayers, HyprMonitor};
pub use input::{HyprDevices, HyprInputDevice};
pub use session::{HyprInstance, HyprSessionEnv, HyprVersion, HyprVersionSource};
pub use window::{HyprClient, HyprCursorPosition, HyprWindow};
pub use workspace::HyprWorkspace;

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

/// The message hyprctl prints when it does not know a request.
///
/// It answers an unknown endpoint or an unknown option with a plain line and
/// a zero exit code, so the JSON parser would otherwise report a confusing
/// error instead of what hyprctl actually said.
pub(super) fn hyprctl_message(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('[') {
        return None;
    }

    // only a short single line is a message, not data
    if trimmed.lines().count() > 1 {
        return None;
    }
    Some(trimmed)
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
    devices: OnceLock<Result<HyprDevices, ProviderError>>,
}

impl std::fmt::Debug for HyprlandProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyprlandProvider").finish_non_exhaustive()
    }
}

fn default_true() -> bool {
    true
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
            devices: OnceLock::new(),
        }
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
            s.spawn(|| self.devices());
        });
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
            if let Some(message) = hyprctl_message(&raw) {
                return ProviderError::command(format!("hyprctl {endpoint}: {message}"));
            }
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
}
#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
pub(crate) struct MockRunner {
    pub(crate) payloads: HashMap<String, String>,
}

#[cfg(test)]
impl CommandRunner for MockRunner {
    fn run(&self, _program: &str, args: &[&str]) -> Result<String, ProviderError> {
        let endpoint = args.last().copied().unwrap_or_default();
        self.payloads
            .get(endpoint)
            .cloned()
            .ok_or_else(|| ProviderError::not_found(endpoint))
    }
}

#[cfg(test)]
mod tests {
    use super::CommandRunner;
    use super::HyprlandProvider;
    use super::MockRunner;
    use super::RealCommandRunner;
    use super::hyprctl_message;
    use super::run_command_with_timeout;
    use super::*;
    use crate::providers::system::ProviderError;
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;

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
        assert_eq!(workspaces[0].tiled_layout.as_deref(), Some("dwindle"));
        assert_eq!(workspaces[2].tiled_layout.as_deref(), Some("master"));

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
    fn hyprctl_preflight_ignores_non_hyprctl_commands() {
        let runner = RealCommandRunner::default();
        assert!(runner.preflight_error("sleep").is_none());
    }

    #[test]
    fn recognises_the_message_hyprctl_prints_for_an_unknown_request() {
        assert_eq!(
            hyprctl_message("unknown request\n"),
            Some("unknown request")
        );
        assert_eq!(hyprctl_message("no such option\n"), Some("no such option"));
    }

    #[test]
    fn does_not_treat_data_as_a_message() {
        assert_eq!(hyprctl_message("{\"id\": 1}"), None);
        assert_eq!(hyprctl_message("[1, 2]"), None);
        assert_eq!(hyprctl_message(""), None);
        assert_eq!(hyprctl_message("first line\nsecond line\n"), None);
    }
}
