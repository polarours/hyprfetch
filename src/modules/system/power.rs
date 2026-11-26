//! Chassis, wall power, Bluetooth and throughput rows.

use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct ChassisModule;

pub struct PowerAdapterModule;

pub struct BluetoothModule;

pub struct DiskIoModule;

pub struct NetIoModule;

impl Module for ChassisModule {
    fn id(&self) -> ModuleId {
        ModuleId::Chassis
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.chassis() {
            Some(name) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Chassis", name.to_string())],
                json!({ "type": name }),
            ),
            None => ModuleOutput::unavailable(self.id(), "chassis type unavailable".to_string()),
        }
    }
}

impl Module for PowerAdapterModule {
    fn id(&self) -> ModuleId {
        ModuleId::PowerAdapter
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.power_supplies() {
            Some(supplies) => {
                // A laptop with a dock has two; report the connected one first.
                let display = supplies
                    .iter()
                    .map(|supply| supply.display())
                    .collect::<Vec<_>>()
                    .join(", ");
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Power", display)],
                    json!({
                        "supplies": supplies.iter().map(|supply| json!({
                            "name": supply.name,
                            "online": supply.online,
                            "watts": supply.watts,
                        })).collect::<Vec<_>>(),
                    }),
                )
            }
            None => ModuleOutput::unavailable(self.id(), "no mains supply".to_string()),
        }
    }
}

impl Module for BluetoothModule {
    fn id(&self) -> ModuleId {
        ModuleId::Bluetooth
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.bluetooth_devices() {
            Some(devices) if devices.is_empty() => {
                ModuleOutput::unavailable(self.id(), "no device connected".to_string())
            }
            Some(devices) => {
                let value = if devices.len() <= 2 {
                    devices.join(", ")
                } else {
                    format!("{} (+{})", devices[..2].join(", "), devices.len() - 2)
                };
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Bluetooth", value)],
                    json!({ "devices": devices, "count": devices.len() }),
                )
            }
            None => ModuleOutput::unavailable(self.id(), "bluetoothctl unavailable".to_string()),
        }
    }
}

impl Module for DiskIoModule {
    fn id(&self) -> ModuleId {
        ModuleId::DiskIo
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.disk_throughput() {
            Some(rate) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Disk I/O", rate.display())],
                json!({ "read_bps": rate.read, "write_bps": rate.write }),
            ),
            None => ModuleOutput::unavailable(self.id(), "diskstats unavailable".to_string()),
        }
    }
}

impl Module for NetIoModule {
    fn id(&self) -> ModuleId {
        ModuleId::NetIo
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.network_throughput() {
            Some(rate) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Net I/O", rate.display())],
                json!({ "rx_bps": rate.read, "tx_bps": rate.write }),
            ),
            None => ModuleOutput::unavailable(self.id(), "netdev unavailable".to_string()),
        }
    }
}
