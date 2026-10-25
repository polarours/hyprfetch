//! The modules that report on the machine rather than on the
//! compositor.

mod basics;
mod hardware;
mod media;
mod session;
mod storage;

pub use basics::{
    HostModule, KernelModule, OsModule, PackagesModule, ShellModule, TerminalModule, UptimeModule,
};
pub use hardware::{CpuModule, GpuModule, MemoryModule, SwapModule};
pub use media::MediaModule;
pub use session::{
    AudioModule, ColorschemeModule, DatetimeModule, LocaleModule, NetworkModule, TemperatureModule,
};
pub use storage::{BatteryModule, DiskModule};
