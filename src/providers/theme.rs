use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

use serde::Serialize;

use crate::providers::system::ProviderError;

/// How long a `--version` probe may take before it is given up on.
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Default, Serialize)]
pub struct ThemeInfo {
    pub gtk_theme: Option<String>,
    pub icon_theme: Option<String>,
    pub cursor_theme: Option<String>,
    pub font: Option<String>,
}

impl ThemeInfo {
    pub fn merge_missing_from(&mut self, fallback: ThemeInfo) {
        if self.gtk_theme.is_none() {
            self.gtk_theme = fallback.gtk_theme;
        }
        if self.icon_theme.is_none() {
            self.icon_theme = fallback.icon_theme;
        }
        if self.cursor_theme.is_none() {
            self.cursor_theme = fallback.cursor_theme;
        }
        if self.font.is_none() {
            self.font = fallback.font;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.gtk_theme.is_none()
            && self.icon_theme.is_none()
            && self.cursor_theme.is_none()
            && self.font.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EcosystemInfo {
    pub wallpaper: Option<(String, Option<String>)>,
    pub bar: Option<(String, Option<String>)>,
    pub launcher: Option<(String, Option<String>)>,
    pub notifications: Option<(String, Option<String>)>,
    pub clipboard: Option<(String, Option<String>)>,
    pub screenshot: Option<(String, Option<String>)>,
    pub idle: Option<(String, Option<String>)>,
    pub lock: Option<(String, Option<String>)>,
    pub portal: Option<(String, Option<String>)>,
    pub polkit: Option<(String, Option<String>)>,
    pub tray: Option<(String, Option<String>)>,
    pub picker: Option<(String, Option<String>)>,
    pub network: Option<(String, Option<String>)>,
    pub audio: Option<(String, Option<String>)>,
    pub bluetooth: Option<(String, Option<String>)>,
    pub greeter: Option<(String, Option<String>)>,
    pub ime: Option<(String, Option<String>)>,
    pub nightlight: Option<(String, Option<String>)>,
    pub power: Option<(String, Option<String>)>,
    pub automount: Option<(String, Option<String>)>,
}

impl EcosystemInfo {
    pub fn is_empty(&self) -> bool {
        self.wallpaper.is_none()
            && self.bar.is_none()
            && self.launcher.is_none()
            && self.notifications.is_none()
            && self.clipboard.is_none()
            && self.screenshot.is_none()
            && self.idle.is_none()
            && self.lock.is_none()
            && self.portal.is_none()
            && self.polkit.is_none()
            && self.tray.is_none()
            && self.picker.is_none()
            && self.network.is_none()
            && self.audio.is_none()
            && self.bluetooth.is_none()
            && self.greeter.is_none()
            && self.ime.is_none()
            && self.nightlight.is_none()
            && self.power.is_none()
            && self.automount.is_none()
    }

    pub fn display_label(&self, field: &str) -> Option<String> {
        let pair = match field {
            "wallpaper" => &self.wallpaper,
            "bar" => &self.bar,
            "launcher" => &self.launcher,
            "notifications" => &self.notifications,
            "clipboard" => &self.clipboard,
            "screenshot" => &self.screenshot,
            "idle" => &self.idle,
            "lock" => &self.lock,
            "portal" => &self.portal,
            "polkit" => &self.polkit,
            "tray" => &self.tray,
            "picker" => &self.picker,
            "network" => &self.network,
            "audio" => &self.audio,
            "bluetooth" => &self.bluetooth,
            "greeter" => &self.greeter,
            "ime" => &self.ime,
            "nightlight" => &self.nightlight,
            "power" => &self.power,
            "automount" => &self.automount,
            _ => return None,
        };
        pair.as_ref().map(|(label, version)| {
            if let Some(v) = version {
                format!("{label} ({v})")
            } else {
                label.clone()
            }
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ThemeProvider {
    processes: OnceLock<Result<Vec<Process>, ProviderError>>,
}

impl ThemeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn theme_info(&self) -> Result<ThemeInfo, ProviderError> {
        let bases = config_bases();
        let mut gtk4 = load_gtk_settings(&bases, "4.0");
        let gtk3 = load_gtk_settings(&bases, "3.0");
        gtk4.merge_missing_from(gtk3);
        if gtk4.is_empty() {
            return Err(ProviderError::not_found(
                "no GTK theme settings found in gtk-3.0 or gtk-4.0 settings.ini",
            ));
        }
        Ok(gtk4)
    }

    pub fn wallpaper_backend(&self) -> Result<String, ProviderError> {
        let ecosystem = self.ecosystem_info()?;
        ecosystem
            .wallpaper
            .map(|(label, _)| label)
            .ok_or_else(|| ProviderError::not_found("no known wallpaper backend detected"))
    }

    pub fn ecosystem_info(&self) -> Result<EcosystemInfo, ProviderError> {
        let processes = self.processes.get_or_init(read_processes).clone()?;
        let ecosystem = detect_ecosystem_info(&processes);
        if ecosystem.is_empty() {
            return Err(ProviderError::not_found(
                "no known Hyprland ecosystem tools detected",
            ));
        }
        Ok(ecosystem)
    }
}

/// The directories the configuration of the user may live in.
///
/// A machine that points its configuration somewhere else keeps its theme in
/// the home directory, so both places are looked at.
fn config_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();

    if let Some(directory) = env::var_os("XDG_CONFIG_HOME") {
        bases.push(PathBuf::from(directory));
    }
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home).join(".config");
        if !bases.contains(&home) {
            bases.push(home);
        }
    }

    bases
}

/// The settings of a GTK version, taken from the first file that holds them.
fn load_gtk_settings(bases: &[PathBuf], version: &str) -> ThemeInfo {
    for base in bases {
        let path = base.join(format!("gtk-{version}/settings.ini"));
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let info = parse_gtk_settings(&raw);
        // An empty file counts as no settings: keep looking in the next base
        // so a real theme in $HOME is not shadowed by an empty XDG file.
        if !info.is_empty() {
            return info;
        }
    }

    ThemeInfo::default()
}

fn parse_gtk_settings(raw: &str) -> ThemeInfo {
    let mut info = ThemeInfo::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with('[')
        {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "gtk-theme-name" => info.gtk_theme = Some(value),
            "gtk-icon-theme-name" => info.icon_theme = Some(value),
            "gtk-cursor-theme-name" => info.cursor_theme = Some(value),
            "gtk-font-name" => info.font = Some(normalize_font_name(&value)),
            _ => {}
        }
    }
    info
}

fn normalize_font_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A running process, with the name the kernel shows and the binary it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Process {
    /// The name from `/proc/<pid>/comm`, at most 15 characters long.
    comm: String,
    /// The path of the binary the process runs, if it can be read.
    exe: Option<String>,
}

impl Process {
    /// The file name of the binary the process runs.
    fn exe_name(&self) -> Option<&str> {
        self.exe.as_deref().and_then(|path| {
            Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
        })
    }
}

fn read_processes() -> Result<Vec<Process>, ProviderError> {
    let mut processes = Vec::new();
    for entry in fs::read_dir("/proc").map_err(ProviderError::io)? {
        let entry = entry.map_err(ProviderError::io)?;
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !file_name
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            continue;
        }
        let path = entry.path();
        let Ok(comm) = fs::read_to_string(path.join("comm")) else {
            continue;
        };
        let exe = fs::read_link(path.join("exe")).ok().and_then(|link| {
            let link = link.to_string_lossy().to_string();
            let link = link.strip_suffix(" (deleted)").unwrap_or(&link);
            (!link.is_empty()).then(|| link.to_string())
        });

        processes.push(Process {
            comm: comm.trim().to_ascii_lowercase(),
            exe,
        });
    }
    Ok(processes)
}

/// A desktop component to look for among the running processes.
#[derive(Debug, Clone, Copy)]
struct Backend {
    /// The process name as it appears in the process list.
    process: &'static str,
    /// The name to show in the output.
    label: &'static str,
    /// Whether the label only names the kind of the backend, so the name of
    /// the implementation that runs is shown instead.
    generic: bool,
    /// Whether running the process with `--version` is safe. Agents such as
    /// portal implementations would otherwise start a second instance.
    probe: bool,
}

/// The kernel keeps only the first 15 characters of a process name.
const COMM_NAME_LIMIT: usize = 15;

impl Backend {
    /// Whether a name from the process list belongs to this backend.
    ///
    /// The kernel truncates a process name to 15 characters, so `xdg-desktop-portal`
    /// is listed as `xdg-desktop-por` and is matched by that prefix.
    fn matches(self, process_name: &str) -> bool {
        if process_name == self.process {
            return true;
        }

        process_name.len() == COMM_NAME_LIMIT && self.process.starts_with(process_name)
    }
}

const fn backend(process: &'static str, label: &'static str) -> Backend {
    Backend {
        process,
        label,
        generic: false,
        probe: true,
    }
}

/// A backend whose label only names the kind of it, so the name of the
/// implementation that runs is shown instead.
const fn generic_backend(process: &'static str, label: &'static str) -> Backend {
    Backend {
        process,
        label,
        generic: true,
        probe: true,
    }
}

/// A backend that must not be started again and whose label only names the
/// kind of it.
const fn generic_passive_backend(process: &'static str, label: &'static str) -> Backend {
    Backend {
        process,
        label,
        generic: true,
        probe: false,
    }
}

const WALLPAPER_BACKENDS: &[Backend] = &[
    backend("hyprpaper", "hyprpaper"),
    backend("swww-daemon", "swww"),
    backend("swaybg", "swaybg"),
    backend("mpvpaper", "mpvpaper"),
    backend("waypaper", "waypaper"),
    backend("awww-daemon", "awww"),
    backend("matugen", "matugen"),
];

const BAR_BACKENDS: &[Backend] = &[
    backend("waybar", "waybar"),
    backend("quickshell", "quickshell"),
    backend("ags", "ags"),
    backend("eww", "eww"),
    backend("ironbar", "ironbar"),
    backend("yambar", "yambar"),
    backend("mypal", "mypal"),
];

const LAUNCHER_BACKENDS: &[Backend] = &[
    backend("fuzzel", "fuzzel"),
    backend("rofi", "rofi"),
    backend("wofi", "wofi"),
    backend("walker", "walker"),
    backend("tofi", "tofi"),
    backend("bemenu", "bemenu"),
];

const NOTIFICATION_BACKENDS: &[Backend] = &[
    backend("mako", "mako"),
    backend("swaync", "swaync"),
    backend("dunst", "dunst"),
];

const CLIPBOARD_BACKENDS: &[Backend] = &[
    backend("cliphist", "cliphist"),
    backend("wl-copy", "wl-clipboard"),
    backend("wl-paste", "wl-clipboard"),
];

const SCREENSHOT_BACKENDS: &[Backend] = &[
    backend("grim", "grim"),
    backend("slurp", "slurp"),
    backend("hyprshot", "hyprshot"),
    backend("flameshot", "flameshot"),
];

const IDLE_BACKENDS: &[Backend] = &[
    backend("hypridle", "hypridle"),
    backend("swayidle", "swayidle"),
];

const LOCK_BACKENDS: &[Backend] = &[
    backend("hyprlock", "hyprlock"),
    backend("swaylock", "swaylock"),
    backend("gtklock", "gtklock"),
];

const PICKER_BACKENDS: &[Backend] = &[
    backend("hyprpicker", "hyprpicker"),
    backend("gpick", "gpick"),
    backend("xcolorpicker", "xcolorpicker"),
    backend("wl-color-picker", "wl-color-picker"),
];

const PORTAL_BACKENDS: &[Backend] = &[
    generic_passive_backend("xdg-desktop-portal-hyprland", "portal"),
    generic_passive_backend("xdg-desktop-portal-hypr", "portal"),
    generic_passive_backend("xdg-desktop-portal-wlr", "portal"),
    generic_passive_backend("xdg-desktop-portal-gtk", "portal"),
    generic_passive_backend("xdg-desktop-portal-gnome", "portal"),
    generic_passive_backend("xdg-desktop-portal-kde", "portal"),
    generic_passive_backend("xdg-desktop-portal", "portal"),
];

const POLKIT_BACKENDS: &[Backend] = &[
    generic_passive_backend("hyprpolkitagent", "polkit"),
    generic_passive_backend("polkit_gnome_authentication_agent_1", "polkit"),
    generic_passive_backend("polkit-gnome-authentication-agent-1", "polkit"),
    generic_passive_backend("lxqt-policykit-agent", "polkit"),
    generic_passive_backend("mate-polkit", "polkit"),
    generic_passive_backend("polkit-mate", "polkit"),
    generic_passive_backend("xembeds-polkit-agent", "polkit"),
];

const TRAY_BACKENDS: &[Backend] = &[
    generic_backend("stalonetray", "tray"),
    generic_backend("trayion", "tray"),
    generic_backend("autotray", "tray"),
    generic_backend("ksnip-tray", "tray"),
];

const NETWORK_BACKENDS: &[Backend] = &[
    backend("NetworkManager", "NetworkManager"),
    backend("iwd", "iwd"),
    generic_passive_backend("connmand", "connman"),
    backend("systemd-networkd", "systemd-networkd"),
];

const AUDIO_BACKENDS: &[Backend] = &[
    backend("pipewire", "pipewire"),
    backend("pulseaudio", "pulseaudio"),
];

const BLUETOOTH_BACKENDS: &[Backend] = &[backend("bluetoothd", "bluetoothd")];

const GREETER_BACKENDS: &[Backend] = &[
    backend("sddm", "sddm"),
    backend("gdm3", "gdm"),
    backend("gdm", "gdm"),
    backend("lightdm", "lightdm"),
    backend("ly", "ly"),
    backend("greetd", "greetd"),
];

const IME_BACKENDS: &[Backend] = &[backend("fcitx5", "fcitx5"), backend("ibus-daemon", "ibus")];

const NIGHTLIGHT_BACKENDS: &[Backend] = &[
    backend("gammastep", "gammastep"),
    backend("wlsunset", "wlsunset"),
    backend("hyprsunset", "hyprsunset"),
    backend("redshift-gtk", "redshift"),
    backend("redshift", "redshift"),
];

const POWER_BACKENDS: &[Backend] = &[
    backend("power-profiles-daemon", "power-profiles"),
    backend("tuned", "tuned"),
];

const AUTOMOUNT_BACKENDS: &[Backend] = &[backend("udisksd", "udisks")];

/// A backend that is running.
#[derive(Debug, Clone)]
struct Detected {
    backend: Backend,
    /// The binary the backend runs, if it could be read.
    exe: Option<String>,
}

impl Detected {
    /// The name of the implementation that runs, if it is known.
    fn implementation(&self) -> Option<&str> {
        self.exe.as_deref().and_then(|path| {
            Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
        })
    }
}

/// The first backend of a category that is running.
fn pick_backend(backends: &[Backend], processes: &[Process]) -> Option<Detected> {
    // The name of the binary tells the implementations of a kind apart, while
    // the name of the process does not: the kernel keeps only the first 15
    // characters of it.
    for backend in backends {
        if let Some(process) = processes
            .iter()
            .find(|process| process.exe_name() == Some(backend.process))
        {
            return Some(Detected {
                backend: *backend,
                exe: process.exe.clone(),
            });
        }
    }

    for backend in backends {
        if let Some(process) = processes
            .iter()
            .find(|process| backend.matches(&process.comm))
        {
            return Some(Detected {
                backend: *backend,
                exe: process.exe.clone(),
            });
        }
    }

    None
}

fn detect_ecosystem_info(processes: &[Process]) -> EcosystemInfo {
    let wallpaper = pick_backend(WALLPAPER_BACKENDS, processes);
    let bar = pick_backend(BAR_BACKENDS, processes);
    let launcher = pick_backend(LAUNCHER_BACKENDS, processes);
    let notifications = pick_backend(NOTIFICATION_BACKENDS, processes);
    let clipboard = pick_backend(CLIPBOARD_BACKENDS, processes);
    let screenshot = pick_backend(SCREENSHOT_BACKENDS, processes);
    let idle = pick_backend(IDLE_BACKENDS, processes);
    let lock = pick_backend(LOCK_BACKENDS, processes);
    let portal = pick_backend(PORTAL_BACKENDS, processes);
    let polkit = pick_backend(POLKIT_BACKENDS, processes);
    let tray = pick_backend(TRAY_BACKENDS, processes);
    let picker = pick_backend(PICKER_BACKENDS, processes);
    let network = pick_backend(NETWORK_BACKENDS, processes);
    let audio = pick_backend(AUDIO_BACKENDS, processes);
    let bluetooth = pick_backend(BLUETOOTH_BACKENDS, processes);
    let greeter = pick_backend(GREETER_BACKENDS, processes);
    let ime = pick_backend(IME_BACKENDS, processes);
    let nightlight = pick_backend(NIGHTLIGHT_BACKENDS, processes);
    let power = pick_backend(POWER_BACKENDS, processes);
    let automount = pick_backend(AUTOMOUNT_BACKENDS, processes);

    // Reading a version starts a process per backend, so all probes run at
    // the same time instead of one after the other.
    let picks = [
        &wallpaper,
        &bar,
        &launcher,
        &notifications,
        &clipboard,
        &screenshot,
        &idle,
        &lock,
        &portal,
        &polkit,
        &tray,
        &picker,
        &network,
        &audio,
        &bluetooth,
        &greeter,
        &ime,
        &nightlight,
        &power,
        &automount,
    ];
    let versions = probe_versions(&picks);

    EcosystemInfo {
        wallpaper: ecosystem_entry(wallpaper, &versions),
        bar: ecosystem_entry(bar, &versions),
        launcher: ecosystem_entry(launcher, &versions),
        notifications: ecosystem_entry(notifications, &versions),
        clipboard: ecosystem_entry(clipboard, &versions),
        screenshot: ecosystem_entry(screenshot, &versions),
        idle: ecosystem_entry(idle, &versions),
        lock: ecosystem_entry(lock, &versions),
        portal: ecosystem_entry(portal, &versions),
        polkit: ecosystem_entry(polkit, &versions),
        tray: ecosystem_entry(tray, &versions),
        picker: ecosystem_entry(picker, &versions),
        network: ecosystem_entry(network, &versions),
        audio: ecosystem_entry(audio, &versions),
        bluetooth: ecosystem_entry(bluetooth, &versions),
        greeter: ecosystem_entry(greeter, &versions),
        ime: ecosystem_entry(ime, &versions),
        nightlight: ecosystem_entry(nightlight, &versions),
        power: ecosystem_entry(power, &versions),
        automount: ecosystem_entry(automount, &versions),
    }
}

/// Read the versions of the running backends, in parallel.
fn probe_versions(picks: &[&Option<Detected>]) -> HashMap<&'static str, Option<String>> {
    let active: Vec<&Detected> = picks.iter().filter_map(|pick| pick.as_ref()).collect();
    if active.is_empty() {
        return HashMap::new();
    }

    std::thread::scope(|scope| {
        let handles: Vec<_> = active
            .iter()
            .map(|detected| {
                scope.spawn(move || {
                    let version = if detected.backend.probe {
                        detect_version_with_timeout(
                            &version_command(detected),
                            detected
                                .implementation()
                                .unwrap_or(detected.backend.process),
                            VERSION_PROBE_TIMEOUT,
                        )
                    } else {
                        None
                    };
                    (detected.backend.label, version)
                })
            })
            .collect();

        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    })
}

/// The command that reads the version of a running backend.
///
/// The binary that runs is used when it can be found, because a backend does
/// not have to be in the search path to run.
fn version_command(detected: &Detected) -> String {
    match &detected.exe {
        Some(path) if Path::new(path).is_file() => path.clone(),
        _ => detected.backend.process.to_string(),
    }
}

/// The label and version of a detected backend.
fn ecosystem_entry(
    detected: Option<Detected>,
    versions: &HashMap<&'static str, Option<String>>,
) -> Option<(String, Option<String>)> {
    let detected = detected?;
    let version = versions.get(detected.backend.label).cloned().flatten();
    let label = if detected.backend.generic {
        detected
            .implementation()
            .unwrap_or(detected.backend.label)
            .to_string()
    } else {
        detected.backend.label.to_string()
    };
    Some((label, version))
}

fn detect_version_with_timeout(command: &str, name: &str, timeout: Duration) -> Option<String> {
    let command = command.to_string();
    let name = name.to_string();
    with_timeout(timeout, move || detect_version(&command, &name)).flatten()
}

/// Run `work` on its own thread and give up on it after `timeout`.
///
/// The work outlives the timeout on purpose: a backend that ignores
/// `--version` must not be able to hang the whole run.
fn with_timeout<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });

    receiver.recv_timeout(timeout).ok()
}

fn detect_version(command: &str, name: &str) -> Option<String> {
    let output = std::process::Command::new(command)
        .args(["--version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&output.stdout), name)
}

/// The version inside a `--version` output.
///
/// Every non-empty line is reduced to the first version-looking token; a line
/// without one is a banner, not a version. Tools such as pipewire print their
/// invocation path first (`/usr/bin/pipewire`), which must not end up as the
/// reported version.
fn parse_version(stdout: &str, name: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Tools print their name in front of the version, sometimes in a
        // different case, so drop it before looking for the number.
        let rest = strip_name_prefix(trimmed, name)
            .or_else(|| strip_name_prefix(trimmed, "version"))
            .unwrap_or(trimmed)
            .trim_start_matches([' ', ':', '='])
            .trim();

        if let Some(number) = first_version_number(rest) {
            return Some(number.to_string());
        }
    }
    None
}

/// Remove a leading name from a version line, ignoring case.
fn strip_name_prefix<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line.get(..name.len())?;
    if !rest.eq_ignore_ascii_case(name) {
        return None;
    }
    Some(&line[name.len()..])
}

/// The first token that looks like a version number.
fn first_version_number(text: &str) -> Option<&str> {
    text.split_whitespace().find(|token| {
        token
            .strip_prefix('v')
            .unwrap_or(token)
            .starts_with(|character: char| character.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Process, detect_ecosystem_info, first_version_number, load_gtk_settings,
        parse_gtk_settings, parse_version, strip_name_prefix, with_timeout,
    };

    /// A process the way the process list shows it.
    fn process(comm: &str) -> Process {
        Process {
            comm: comm.to_string(),
            exe: None,
        }
    }

    /// A process together with the binary it runs.
    fn running(comm: &str, exe: &str) -> Process {
        Process {
            comm: comm.to_string(),
            exe: Some(exe.to_string()),
        }
    }

    #[test]
    fn parses_gtk_settings() {
        let raw = r#"
[Settings]
gtk-theme-name=catppuccin-mocha-blue-standard+default
gtk-icon-theme-name=Papirus-Dark
gtk-cursor-theme-name=Bibata-Modern-Ice
gtk-font-name=JetBrainsMono Nerd Font 11
"#;
        let info = parse_gtk_settings(raw);
        assert_eq!(
            info.gtk_theme.as_deref(),
            Some("catppuccin-mocha-blue-standard+default")
        );
        assert_eq!(info.icon_theme.as_deref(), Some("Papirus-Dark"));
        assert_eq!(info.cursor_theme.as_deref(), Some("Bibata-Modern-Ice"));
        assert_eq!(info.font.as_deref(), Some("JetBrainsMono Nerd Font 11"));
    }

    #[test]
    fn normalizes_gtk_font_spacing() {
        let raw = r#"
[Settings]
gtk-font-name=Noto Sans,  10
"#;
        let info = parse_gtk_settings(raw);
        assert_eq!(info.font.as_deref(), Some("Noto Sans, 10"));
    }

    #[test]
    fn takes_the_theme_from_the_first_place_that_holds_it() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join("gtk-4.0")).unwrap();
        std::fs::write(
            home.join("gtk-4.0/settings.ini"),
            "[Settings]\ngtk-theme-name=FromHome\n",
        )
        .unwrap();

        let bases = vec![dir.path().to_path_buf(), home.clone()];
        let theme = load_gtk_settings(&bases, "4.0");
        assert_eq!(theme.gtk_theme.as_deref(), Some("FromHome"));
    }

    #[test]
    fn skips_an_empty_settings_file_and_keeps_looking() {
        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().join("xdg");
        let home = dir.path().join("home");
        std::fs::create_dir_all(xdg.join("gtk-4.0")).unwrap();
        std::fs::create_dir_all(home.join("gtk-4.0")).unwrap();
        // The XDG file exists but holds no theme keys.
        std::fs::write(xdg.join("gtk-4.0/settings.ini"), "[Settings]\n").unwrap();
        std::fs::write(
            home.join("gtk-4.0/settings.ini"),
            "gtk-theme-name=Adwaita\n",
        )
        .unwrap();

        let bases = vec![xdg, home];
        let theme = load_gtk_settings(&bases, "4.0");
        assert_eq!(theme.gtk_theme.as_deref(), Some("Adwaita"));
    }

    #[test]
    fn has_no_theme_when_no_place_holds_one() {
        let dir = tempfile::tempdir().unwrap();
        let bases = vec![dir.path().to_path_buf()];
        assert!(load_gtk_settings(&bases, "4.0").is_empty());
    }

    #[test]
    fn detects_hyprland_ecosystem_processes() {
        let processes = vec![
            process("hyprpaper"),
            process("waybar"),
            process("fuzzel"),
            process("mako"),
            process("hypridle"),
            process("hyprlock"),
        ];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.wallpaper.as_ref().map(|(l, _)| l.as_str()),
            Some("hyprpaper")
        );
        assert_eq!(
            ecosystem.bar.as_ref().map(|(l, _)| l.as_str()),
            Some("waybar")
        );
        assert_eq!(
            ecosystem.launcher.as_ref().map(|(l, _)| l.as_str()),
            Some("fuzzel")
        );
        assert_eq!(
            ecosystem.notifications.as_ref().map(|(l, _)| l.as_str()),
            Some("mako")
        );
        assert_eq!(
            ecosystem.idle.as_ref().map(|(l, _)| l.as_str()),
            Some("hypridle")
        );
        assert_eq!(
            ecosystem.lock.as_ref().map(|(l, _)| l.as_str()),
            Some("hyprlock")
        );
    }

    #[test]
    fn detects_system_service_backends() {
        // Roughtly what a real session looks like: user daemons with an
        // executable and root daemons that only expose their process name.
        let processes = vec![
            // The path only needs its file name for the binary-name match; it
            // must not point at the real system service, so the version
            // probe of the test never starts it.
            running("NetworkManager", "/nonexistent/NetworkManager"),
            process("systemd-network"), // 15-character comm of systemd-networkd
            process("pipewire"),
            process("bluetoothd"),
            process("sddm"),
            process("fcitx5"),
            process("wlsunset"),
            process("power-profiles-"), // 15-character comm of power-profiles-daemon
            process("udisksd"),
        ];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.network.as_ref().map(|(l, _)| l.as_str()),
            Some("NetworkManager"),
            "network must match the exact binary name first"
        );
        assert_eq!(
            ecosystem.audio.as_ref().map(|(l, _)| l.as_str()),
            Some("pipewire")
        );
        assert_eq!(
            ecosystem.bluetooth.as_ref().map(|(l, _)| l.as_str()),
            Some("bluetoothd")
        );
        assert_eq!(
            ecosystem.greeter.as_ref().map(|(l, _)| l.as_str()),
            Some("sddm")
        );
        assert_eq!(
            ecosystem.ime.as_ref().map(|(l, _)| l.as_str()),
            Some("fcitx5")
        );
        assert_eq!(
            ecosystem.nightlight.as_ref().map(|(l, _)| l.as_str()),
            Some("wlsunset")
        );
        assert_eq!(
            ecosystem.power.as_ref().map(|(l, _)| l.as_str()),
            Some("power-profiles"),
            "a 15-character comm must still match its longer binary name"
        );
        assert_eq!(
            ecosystem.automount.as_ref().map(|(l, _)| l.as_str()),
            Some("udisks")
        );
    }

    #[test]
    fn network_detection_falls_back_to_truncated_names() {
        // iwd is matched through the comm name when systemd-networkd is absent.
        let ecosystem = detect_ecosystem_info(&[process("iwd")]);
        assert_eq!(
            ecosystem.network.as_ref().map(|(l, _)| l.as_str()),
            Some("iwd")
        );
    }

    #[test]
    fn greeter_detection_reports_the_display_manager() {
        for (comm, label) in [
            ("gdm3", "gdm"),
            ("gdm", "gdm"),
            ("lightdm", "lightdm"),
            ("ly", "ly"),
            ("greetd", "greetd"),
        ] {
            let ecosystem = detect_ecosystem_info(&[process(comm)]);
            assert_eq!(
                ecosystem.greeter.as_ref().map(|(l, _)| l.as_str()),
                Some(label),
                "{comm} must be reported as {label}"
            );
        }
    }

    #[test]
    fn detects_portal_polkit_and_tray_backends() {
        let processes = vec![
            process("xdg-desktop-portal-hypr"),
            process("hyprpolkitagent"),
            process("stalonetray"),
        ];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.portal.as_ref().map(|(label, _)| label.as_str()),
            Some("portal")
        );
        assert_eq!(
            ecosystem.polkit.as_ref().map(|(label, _)| label.as_str()),
            Some("polkit")
        );
        assert_eq!(
            ecosystem.tray.as_ref().map(|(label, _)| label.as_str()),
            Some("tray")
        );
    }

    #[test]
    fn passive_backends_are_not_started_for_their_version() {
        let processes = vec![process("xdg-desktop-portal-hypr")];

        let ecosystem = detect_ecosystem_info(&processes);
        let (_, version) = ecosystem.portal.expect("portal detected");
        assert_eq!(version, None);
    }

    #[test]
    fn gives_up_on_a_hanging_probe() {
        let started = Instant::now();
        let result = with_timeout(Duration::from_millis(100), || {
            std::thread::sleep(Duration::from_secs(30));
            42
        });

        assert_eq!(result, None);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn returns_the_probe_result_when_it_finishes_in_time() {
        let result = with_timeout(Duration::from_secs(5), || "version 1.2.3");

        assert_eq!(result, Some("version 1.2.3"));
    }

    #[test]
    fn matches_a_process_name_the_kernel_truncated() {
        let processes = vec![process("xdg-desktop-por")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.portal.as_ref().map(|(label, _)| label.as_str()),
            Some("portal")
        );
    }

    #[test]
    fn does_not_match_a_process_name_that_is_merely_shorter() {
        let processes = vec![process("polkit")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert!(ecosystem.polkit.is_none());
    }

    #[test]
    fn strips_the_name_in_front_of_a_version() {
        assert_eq!(
            strip_name_prefix("Quickshell 0.3.2 (revision)", "quickshell"),
            Some(" 0.3.2 (revision)")
        );
        assert_eq!(strip_name_prefix("2.17.2", "walker"), None);
    }

    #[test]
    fn finds_the_version_number_in_a_version_line() {
        assert_eq!(
            first_version_number("0.3.2 (revision , distributed by Arch Linux)"),
            Some("0.3.2")
        );
        assert_eq!(first_version_number("v1.2.3"), Some("v1.2.3"));
        assert_eq!(first_version_number("no number here"), None);
    }

    #[test]
    fn skips_banner_lines_without_a_version_number() {
        // pipewire prints its invocation path first, then the real version.
        let stdout =
            "/usr/bin/pipewire\nCompiled with libpipewire 1.6.11\nLinked with libpipewire 1.6.11\n";
        assert_eq!(
            parse_version(stdout, "pipewire"),
            Some("1.6.11".to_string())
        );
        // A name-only banner yields no version at all.
        assert_eq!(parse_version("pipewire\n", "pipewire"), None);
    }

    #[test]
    fn parses_versions_from_the_first_line_with_a_number() {
        // The usual single-line form with the tool name in front.
        assert_eq!(
            parse_version("Quickshell 0.3.2 (revision)\n", "quickshell"),
            Some("0.3.2".to_string())
        );
        // A bare version with no name prefix.
        assert_eq!(
            parse_version("2.17.2\n", "walker"),
            Some("2.17.2".to_string())
        );
    }

    #[test]
    fn detects_the_wallpaper_and_launcher_backends() {
        let processes = vec![process("awww-daemon"), process("bemenu")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem
                .wallpaper
                .as_ref()
                .map(|(label, _)| label.as_str()),
            Some("awww")
        );
        assert_eq!(
            ecosystem.launcher.as_ref().map(|(label, _)| label.as_str()),
            Some("bemenu")
        );
    }

    #[test]
    fn shows_the_implementation_of_a_generic_backend() {
        let processes = vec![running("stalonetray", "/usr/bin/stalonetray")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.tray.as_ref().map(|(label, _)| label.as_str()),
            Some("stalonetray")
        );
    }

    #[test]
    fn tells_the_portal_implementations_apart() {
        // the kernel keeps only the first 15 characters of a process name, so
        // every portal implementation looks the same in the process list
        let processes = vec![
            running("xdg-desktop-por", "/usr/lib/xdg-desktop-portal-hyprland"),
            running("xdg-desktop-por", "/usr/lib/xdg-desktop-portal-gtk"),
            running("xdg-desktop-por", "/usr/lib/xdg-desktop-portal"),
        ];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.portal.as_ref().map(|(label, _)| label.as_str()),
            Some("xdg-desktop-portal-hyprland")
        );
    }

    #[test]
    fn finds_the_only_portal_implementation_that_runs() {
        let processes = vec![running(
            "xdg-desktop-por",
            "/usr/lib/xdg-desktop-portal-gtk",
        )];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.portal.as_ref().map(|(label, _)| label.as_str()),
            Some("xdg-desktop-portal-gtk")
        );
    }

    #[test]
    fn keeps_the_label_of_a_backend_that_names_itself() {
        let processes = vec![running("waybar", "/usr/bin/waybar")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.bar.as_ref().map(|(label, _)| label.as_str()),
            Some("waybar")
        );
    }

    #[test]
    fn recognises_a_polkit_agent_with_a_truncated_name() {
        // the kernel keeps only the first 15 characters of a process name
        let processes = vec![process("polkit_gnome_au")];

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(
            ecosystem.polkit.as_ref().map(|(label, _)| label.as_str()),
            Some("polkit")
        );
    }
}
