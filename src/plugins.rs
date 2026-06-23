use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::config::PluginConfig;
use crate::model::{ModuleOutput, OutputRow};

pub fn run_plugin(config: &PluginConfig) -> ModuleOutput {
    let id = plugin_output_id(&config.name);
    let mut command = Command::new(&config.command);
    command
        .args(&config.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return ModuleOutput::unavailable_custom(
                id,
                format!("failed to spawn plugin '{}': {error}", config.name),
            );
        }
    };

    match wait_for_child(&mut child, config.timeout_ms.max(1)) {
        Ok(true) => match child.wait_with_output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let rows = parse_plugin_rows(&stdout, config);
                if rows.is_empty() {
                    return ModuleOutput::unavailable_custom(
                        id,
                        format!("plugin '{}' produced no displayable output", config.name),
                    );
                }

                ModuleOutput::ok_custom(
                    id,
                    rows,
                    json!({
                        "name": config.name,
                        "command": config.command,
                    }),
                )
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let message = stderr.trim();
                let message = if message.is_empty() {
                    format!("plugin '{}' exited with {}", config.name, output.status)
                } else {
                    format!(
                        "plugin '{}' exited with {}: {message}",
                        config.name, output.status
                    )
                };
                ModuleOutput::unavailable_custom(id, message)
            }
            Err(error) => ModuleOutput::unavailable_custom(
                id,
                format!("failed to collect plugin '{}' output: {error}", config.name),
            ),
        },
        Ok(false) => {
            let _ = child.kill();
            let _ = child.wait();
            ModuleOutput::unavailable_custom(
                id,
                format!(
                    "plugin '{}' timed out after {}ms",
                    config.name, config.timeout_ms
                ),
            )
        }
        Err(error) => ModuleOutput::unavailable_custom(
            id,
            format!("failed to wait for plugin '{}': {error}", config.name),
        ),
    }
}

fn wait_for_child(child: &mut std::process::Child, timeout_ms: u64) -> std::io::Result<bool> {
    let timeout = Duration::from_millis(timeout_ms);
    let start = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn parse_plugin_rows(stdout: &str, config: &PluginConfig) -> Vec<OutputRow> {
    let lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.len() == 1 && !lines[0].contains('\t') && !lines[0].contains('=') {
        return vec![OutputRow::new(plugin_label(config), lines[0])];
    }

    lines
        .into_iter()
        .map(|line| {
            if let Some((key, value)) = line.split_once('\t') {
                return OutputRow::new(key.trim(), value.trim());
            }
            if let Some((key, value)) = line.split_once('=') {
                return OutputRow::new(key.trim(), value.trim());
            }
            OutputRow::new(plugin_label(config), line)
        })
        .collect()
}

fn plugin_label(config: &PluginConfig) -> String {
    config.label.clone().unwrap_or_else(|| config.name.clone())
}

fn plugin_output_id(name: &str) -> String {
    format!("plugin:{name}")
}

#[cfg(test)]
mod tests {
    use super::parse_plugin_rows;
    use crate::config::PluginConfig;

    #[test]
    fn parses_single_line_plugin_output_with_configured_label() {
        let config = PluginConfig {
            name: "battery".to_string(),
            label: Some("Battery".to_string()),
            ..PluginConfig::default()
        };

        let rows = parse_plugin_rows("88%\n", &config);
        assert_eq!(rows[0].key, "Battery");
        assert_eq!(rows[0].value, "88%");
    }

    #[test]
    fn parses_multi_row_plugin_protocol() {
        let config = PluginConfig {
            name: "session-extra".to_string(),
            ..PluginConfig::default()
        };

        let rows = parse_plugin_rows("Music\tLo-fi\nVPN=up\n", &config);
        assert_eq!(rows[0].key, "Music");
        assert_eq!(rows[0].value, "Lo-fi");
        assert_eq!(rows[1].key, "VPN");
        assert_eq!(rows[1].value, "up");
    }
}
