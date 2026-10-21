use std::env;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::HyprlandProvider;
use super::ProviderError;

pub(super) fn has_hyprland_session_values(
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

pub(super) fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

pub(super) fn process_is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
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

pub(super) fn parse_hypr_lock(raw: &str) -> (Option<u32>, Option<String>) {
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

impl HyprlandProvider {
    pub fn version(&self) -> Result<HyprVersion, ProviderError> {
        self.version
            .get_or_init(|| self.fetch_version_with_fallback())
            .clone()
    }

    pub fn instances(&self) -> Result<Vec<HyprInstance>, ProviderError> {
        self.instances.get_or_init(detect_hypr_instances).clone()
    }

    pub fn session_env(&self) -> HyprSessionEnv {
        HyprSessionEnv::detect()
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
#[cfg(test)]
mod tests {
    use super::super::HyprlandProvider;
    use super::super::MockRunner;
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

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
