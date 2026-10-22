use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::humanize_kib;

pub struct DiskModule;

pub struct BatteryModule;

impl Module for DiskModule {
    fn id(&self) -> ModuleId {
        ModuleId::Disk
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let mut mounts = config.option_string_list("mounts");
        if mounts.is_empty() {
            mounts.push("/".to_string());
        }

        let mut rows = Vec::new();
        let mut disks = Vec::new();
        for mount in &mounts {
            let Ok(disk) = ctx.providers.system.disk_info(mount) else {
                continue;
            };

            let used_bytes = disk.total_bytes.saturating_sub(disk.available_bytes);
            let mut rendered = format!(
                "{} / {} ({:.0}%)",
                humanize_kib(used_bytes / 1024),
                humanize_kib(disk.total_bytes / 1024),
                disk_usage_percent(used_bytes, disk.total_bytes)
            );
            if let Some(filesystem) = disk.filesystem.as_deref() {
                rendered.push_str(&format!(" - {filesystem}"));
            }

            rows.push(OutputRow::new(format!("Disk ({mount})"), rendered));
            disks.push(json!({
                "mount": disk.mount,
                "filesystem": disk.filesystem,
                "used_bytes": used_bytes,
                "available_bytes": disk.available_bytes,
                "total_bytes": disk.total_bytes
            }));
        }

        if rows.is_empty() {
            return ModuleOutput::unavailable(self.id(), "no readable mount points");
        }

        ModuleOutput::ok(self.id(), rows, json!({ "disks": disks }))
    }
}

impl Module for BatteryModule {
    fn id(&self) -> ModuleId {
        ModuleId::Battery
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.battery_info() {
            Ok(batteries) => {
                let rows = batteries
                    .iter()
                    .map(|battery| {
                        OutputRow::new(
                            format!("Battery ({})", battery.name),
                            render_battery_value(
                                battery.capacity_percent,
                                battery.status.as_deref(),
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                ModuleOutput::ok(self.id(), rows, json!(batteries))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn disk_usage_percent(used_bytes: u64, total_bytes: u64) -> f64 {
    if total_bytes == 0 {
        0.0
    } else {
        used_bytes as f64 / total_bytes as f64 * 100.0
    }
}

fn render_battery_value(capacity_percent: Option<u8>, status: Option<&str>) -> String {
    match (capacity_percent, status) {
        (Some(capacity), Some(status)) => format!("{capacity}% [{status}]"),
        (Some(capacity), None) => format!("{capacity}%"),
        (None, Some(status)) => status.to_string(),
        (None, None) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_disk_usage_percent() {
        assert_eq!(disk_usage_percent(9, 10), 90.0);
        assert_eq!(disk_usage_percent(9, 0), 0.0);
    }

    #[test]
    fn formats_battery_value() {
        assert_eq!(
            render_battery_value(Some(49), Some("Charging")),
            "49% [Charging]"
        );
        assert_eq!(render_battery_value(Some(49), None), "49%");
    }
}
