use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// The distribution name the OS module reports, read the same way the binary
/// does. Assertions use this instead of a fixed name so the tests pass on
/// every distribution the CI happens to run on.
fn system_os_name() -> String {
    os_name_from_release(&fs::read_to_string("/etc/os-release").unwrap_or_default())
}

fn os_name_from_release(raw: &str) -> String {
    let mut pretty_name = None;
    let mut name = None;
    let mut version = None;
    for line in raw.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        match key {
            "PRETTY_NAME" => pretty_name = Some(value.to_string()),
            "NAME" => name = Some(value.to_string()),
            "VERSION" => version = Some(value.to_string()),
            _ => {}
        }
    }

    pretty_name
        .or_else(|| match (name, version) {
            (Some(name), Some(version)) => Some(format!("{name} {version}")),
            (name, _) => name,
        })
        .unwrap_or_default()
}

#[test]
fn reads_os_name_from_release() {
    assert_eq!(
        os_name_from_release("PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\n"),
        "Ubuntu 24.04.1 LTS"
    );
    assert_eq!(
        os_name_from_release(
            "NAME=\"AlmaLinux\"\nVERSION=\"9.4 (Seafoam Ocelot)\"\nID=almalinux\n"
        ),
        "AlmaLinux 9.4 (Seafoam Ocelot)"
    );
    assert_eq!(
        os_name_from_release("NAME=\"Rocky Linux\"\nID=rocky\n"),
        "Rocky Linux"
    );
    assert_eq!(os_name_from_release(""), "");
}

#[test]
fn lists_available_modules() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-modules")
        .assert()
        .success()
        .stdout(predicate::str::contains("host"))
        .stdout(predicate::str::contains("disk"))
        .stdout(predicate::str::contains("battery"))
        .stdout(predicate::str::contains("hyprland"))
        .stdout(predicate::str::contains("packages"))
        .stdout(predicate::str::contains("swap"))
        .stdout(predicate::str::contains("hypr-health"))
        .stdout(predicate::str::contains("hypr-instances"))
        .stdout(predicate::str::contains("hypr-splash"))
        .stdout(predicate::str::contains("hypr-workspaces"))
        .stdout(predicate::str::contains("hypr-cursor"))
        .stdout(predicate::str::contains("hypr-rice"))
        .stdout(predicate::str::contains("hypr-config"))
        .stdout(predicate::str::contains("hypr-rules"))
        .stdout(predicate::str::contains("hypr-devices"))
        .stdout(predicate::str::contains("hypr-binds"))
        .stdout(predicate::str::contains("hypr-shortcuts"))
        .stdout(predicate::str::contains("hypr-layouts"))
        .stdout(predicate::str::contains("hypr-layers"))
        .stdout(predicate::str::contains("hypr-ecosystem"))
        .stdout(predicate::str::contains("wallpaper"));
}

#[test]
fn lists_available_themes() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("hypr"))
        .stdout(predicate::str::contains("mono"));
}

#[test]
fn lists_available_logos() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-logos")
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"))
        .stdout(predicate::str::contains("arch"))
        .stdout(predicate::str::contains("ubuntu"))
        .stdout(predicate::str::contains("windows"))
        .stdout(predicate::str::contains("macos"))
        .stdout(predicate::str::contains("hypr"));
}

#[test]
fn prints_default_config() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--print-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("version = 1"))
        .stdout(predicate::str::contains("[appearance]"))
        .stdout(predicate::str::contains("hyprland"));
}

#[test]
fn initializes_config_at_requested_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("hyprfetch/config.toml");

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--init-config", "--config", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("created config"));

    let config = fs::read_to_string(path).unwrap();
    assert!(config.contains("version = 1"));
    assert!(config.contains("hyprland"));
}

#[test]
fn init_config_refuses_to_overwrite_existing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "version = 1\n").unwrap();

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--init-config", "--config", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn rejects_invalid_config() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        r#"
version = 2

[[modules]]
name = "os"
"#,
    )
    .unwrap();

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--config", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported config version"));
}

#[test]
fn renders_plain_os_module() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--plain", "--modules", "os"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"));
}

#[test]
fn emits_json_output() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--json", "--modules", "os"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"os\""))
        .stdout(predicate::str::contains("\"rows\""));
}

#[test]
fn emits_diagnostics_output() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--diagnostics", "--modules", "os,hyprland"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hyprfetch diagnostics"))
        .stdout(predicate::str::contains("summary:"))
        .stdout(predicate::str::contains("os: ok"))
        .stdout(predicate::str::contains("hyprland:"));
}

#[test]
fn applies_module_label_overrides_from_config() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        r#"
version = 1

[appearance]
plain = true

[[modules]]
name = "os"
label = "Distro"
"#,
    )
    .unwrap();

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--config", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Distro"))
        .stdout(predicate::str::contains("OS").not());
}

#[test]
fn renders_configured_command_plugin() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        r#"
version = 1

[appearance]
plain = true

[[plugins]]
name = "battery"
command = "sh"
args = ["-c", "printf '88%%\n'"]
label = "Battery"
"#,
    )
    .unwrap();

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--config", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Battery"))
        .stdout(predicate::str::contains("88%"));
}

#[test]
fn accepts_windows_logo_override() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--color", "never", "--logo", "windows", "--modules", "os"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("$1").not())
        .stdout(predicate::str::contains("$2").not());
}

#[test]
fn strips_fastfetch_logo_color_markers_from_output() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--color", "never", "--logo", "arch", "--modules", "os"])
        .assert()
        .success()
        .stdout(predicate::str::contains(system_os_name()))
        .stdout(predicate::str::contains("$1").not())
        .stdout(predicate::str::contains("$2").not());
}

#[test]
fn renders_plain_temperature_module() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--plain", "--modules", "temperature"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Temp"));
}

#[test]
fn renders_plain_datetime_module() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--plain", "--modules", "datetime"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DateTime"));
}

#[test]
fn renders_plain_network_module() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--plain", "--modules", "network"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Network"));
}

#[test]
fn renders_plain_audio_module() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--plain", "--modules", "audio"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Audio"));
}

#[test]
fn renders_share_preset() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--share", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("Kernel"));
}

#[test]
fn renders_minimal_style() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--style", "minimal", "--modules", "os,memory"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("Memory"));
}

#[test]
fn lists_new_module_ids() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-modules")
        .assert()
        .success()
        .stdout(predicate::str::contains("temperature"))
        .stdout(predicate::str::contains("datetime"))
        .stdout(predicate::str::contains("network"))
        .stdout(predicate::str::contains("audio"));
}

#[test]
fn json_output_contains_module_ids() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--json", "--modules", "os,kernel,memory"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"os\""))
        .stdout(predicate::str::contains("\"id\": \"kernel\""))
        .stdout(predicate::str::contains("\"id\": \"memory\""));
}

#[test]
fn diagnostics_shows_module_status() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--diagnostics", "--modules", "os,temperature,datetime"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hyprfetch diagnostics"))
        .stdout(predicate::str::contains("os: ok"))
        .stdout(predicate::str::contains("temperature:"))
        .stdout(predicate::str::contains("datetime:"));
}

#[test]
fn share_preset_includes_core_modules() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--share", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("Kernel"))
        .stdout(predicate::str::contains("Memory"));
}

#[test]
fn minimal_style_without_header() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--style", "minimal", "--color", "never", "--modules", "os"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"));
}

#[test]
fn list_logos_includes_new_presets() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-logos")
        .assert()
        .success()
        .stdout(predicate::str::contains("garuda"))
        .stdout(predicate::str::contains("nobara"))
        .stdout(predicate::str::contains("artix"))
        .stdout(predicate::str::contains("asahi"));
}

#[test]
fn list_themes_includes_new_palettes() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("catppuccin"))
        .stdout(predicate::str::contains("dracula"));
}

/// The media module reads the players over MPRIS, which `playerctl` does for
/// it. A stand-in for the tool keeps the test independent of the machine it
/// runs on.
#[test]
#[cfg(unix)]
fn reports_the_track_of_a_media_player() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let tool = dir.path().join("playerctl");
    fs::write(
        &tool,
        "#!/bin/sh\nprintf 'spotify\\tNever Gonna Give You Up\\tRick Astley\\tWhenever You Need Somebody\\tPlaying\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();

    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    Command::cargo_bin("hyprfetch")
        .unwrap()
        .env("PATH", path)
        .args(["--modules", "media", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rick Astley - Never Gonna Give You Up",
        ))
        .stdout(predicate::str::contains("playing"));
}
