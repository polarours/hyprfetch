//! The modules that report on the machine rather than on the
//! compositor.

mod basics;
mod extras;
mod firmware;
mod hardware;
mod media;
mod network_ext;
mod power;
mod session;
mod storage;

pub use basics::{
    HostModule, KernelModule, OsModule, PackagesModule, ShellModule, TerminalModule, UptimeModule,
};
pub use extras::{LoadAverageModule, LocalIpModule, ProcessesModule, UsersModule};
pub use firmware::{BiosModule, BoardModule, BrightnessModule, EditorModule, InitSystemModule};
pub use hardware::{CpuModule, GpuModule, MemoryModule, SwapModule};
pub use media::MediaModule;
pub use network_ext::{DisplayModule, PublicIpModule, WeatherModule};
pub use power::{BluetoothModule, ChassisModule, DiskIoModule, NetIoModule, PowerAdapterModule};
pub use session::{
    AudioModule, ColorschemeModule, DatetimeModule, LocaleModule, NetworkModule, TemperatureModule,
};
pub use storage::{BatteryModule, DiskModule};
