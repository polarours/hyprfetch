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
        .stdout(predicate::str::contains("KERNEL"));
}

#[test]
fn renders_minimal_style() {
    Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--style", "minimal", "--modules", "os,memory"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        // The minimal layout has no header labels, so keys keep their casing.
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
        .stdout(predicate::str::contains("KERNEL"))
        .stdout(predicate::str::contains("MEMORY"));
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
        "#!/bin/sh\nprintf 'spotify	Never Gonna Give You Up	Rick Astley	Whenever You Need Somebody	Playing\n'\n",
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

#[test]
fn preset_minimal_selects_a_short_module_set() {
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--preset",
            "minimal",
            "--no-config",
            "--json",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let ids: Vec<&str> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"os"));
    assert!(ids.contains(&"cpu"));
    assert!(
        !ids.contains(&"wallpaper"),
        "minimal must not include wallpaper: {ids:?}"
    );
    assert!(ids.len() <= 10, "minimal should stay short: {ids:?}");
}

#[test]
fn preset_all_selects_every_module() {
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--preset",
            "all",
            "--no-config",
            "--json",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let count = json.as_array().unwrap().len();
    // `all` uses the full default order (hypr-rice is opt-in, so not counted).
    assert!(
        count >= 39,
        "all preset should include every default module, got {count}"
    );
}

#[test]
fn explicit_modules_override_the_preset() {
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--preset",
            "minimal",
            "--modules",
            "os,kernel",
            "--no-config",
            "--json",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let ids: Vec<&str> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["os", "kernel"],
        "explicit --modules must win over preset: {ids:?}"
    );
}

#[test]
fn theme_file_overrides_look_but_not_modules() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "[appearance]").unwrap();
    writeln!(f, "palette = \"catppuccin\"").unwrap();
    writeln!(f, "style = \"minimal\"").unwrap();
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--theme-file",
            f.path().to_str().unwrap(),
            "--no-config",
            "--json",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    // The module set is unchanged by the theme file: still the curated default
    // (a theme file carries looks, never module lists).
    let count = json.as_array().unwrap().len();
    assert!(
        (18..=24).contains(&count),
        "expected the curated default module set, got {count}"
    );
}

#[test]
fn logo_file_renders_custom_artwork() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "  /\\").unwrap();
    writeln!(f, " /  \\").unwrap();
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--logo-file",
            f.path().to_str().unwrap(),
            "--no-config",
            "--animation",
            "static",
            "--color",
            "never",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("/\\"),
        "custom logo art should appear: {text}"
    );
    assert!(text.contains("[OS]"), "system rows should still render");
}

#[test]
fn retro_adds_arcade_flair_without_replacing_fetch_layout() {
    let retro = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--retro",
            "--color",
            "always",
            "--animation",
            "static",
            "--no-config",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let t = String::from_utf8(retro).unwrap();
    // fetch shape preserved: key: value rows still exist
    assert!(t.contains("[OS]"), "fetch rows must remain: {t}");
    assert!(t.contains("[CPU]"), "CPU row must remain: {t}");
    // arcade elements layered on top
    assert!(
        t.contains('\u{25ae}') && t.contains('\u{25af}'),
        "arcade LED usage meter expected in the output: {t}"
    );

    let normal = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--no-retro",
            "--color",
            "never",
            "--animation",
            "static",
            "--no-config",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let n = String::from_utf8(normal).unwrap();
    assert!(!n.contains("═"), "--no-retro must drop the CRT rule");
}

#[test]
fn arcade_style_is_the_default() {
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--no-config", "--color", "never", "--animation", "static"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let t = String::from_utf8(output).unwrap();
    assert!(t.contains("═"), "CRT rule should be on by default: {t}");

    // --no-retro restores the plain fetch look
    let plain = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--no-config",
            "--no-retro",
            "--color",
            "never",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let p = String::from_utf8(plain).unwrap();
    assert!(p.contains("---"), "--no-retro restores plain dash rule");
    assert!(!p.contains("═"), "--no-retro drops the CRT rule");
}

#[test]
fn distro_logo_is_default_and_drawn_in_pixel_blocks() {
    let output = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--no-config", "--color", "never", "--animation", "static"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let t = String::from_utf8(output).unwrap();
    // The running distro still supplies the artwork; it is just drawn with
    // solid block cells instead of thin dots.
    assert!(
        t.contains("██"),
        "distro logo must render in solid blocks: {t}"
    );
    assert!(!t.contains("·"), "the old dot outline must be gone: {t}");
    // The wordmark's first line is unique: H Y P R with single-cell strokes.
    // A short run like "█████ █████" can occur inside real distro artwork
    // (Ubuntu's mark does), so match the whole line instead.
    assert!(
        !t.contains("█   █ █   █ ████  ████"),
        "the HYPRFETCH wordmark must not replace the distro logo: {t}"
    );

    // The wordmark stays available as an explicit choice.
    let wordmark = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--no-config",
            "--logo",
            "hypr-pixel",
            "--color",
            "never",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let w = String::from_utf8(wordmark).unwrap();
    assert!(
        w.contains("█   █ █   █ ████  ████"),
        "--logo hypr-pixel must work"
    );

    // auto keeps meaning distro detection
    let auto = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--no-config",
            "--logo",
            "auto",
            "--color",
            "never",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let a = String::from_utf8(auto).unwrap();
    assert!(a.contains("██"), "--logo auto must render the distro logo");
    assert!(
        !a.contains("█   █ █   █ ████  ████"),
        "--logo auto must not render the wordmark"
    );
}

#[test]
fn icons_are_on_by_default_and_no_icons_turns_them_off() {
    let with_icons = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args(["--no-config", "--color", "never", "--animation", "static"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let t = String::from_utf8(with_icons).unwrap();
    assert!(
        t.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
        "icons should be the default: {t}"
    );
    assert!(t.contains("[CPU]"), "labels must survive icon mode: {t}");

    let plain = Command::cargo_bin("hyprfetch")
        .unwrap()
        .args([
            "--no-config",
            "--no-icons",
            "--color",
            "never",
            "--animation",
            "static",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let p = String::from_utf8(plain).unwrap();
    assert!(
        !p.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
        "--no-icons must strip the private-use glyphs: {p}"
    );
    assert!(p.contains("[CPU]"));
}
