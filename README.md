# hyprfetch

`hyprfetch` is a Hyprland-native fetch utility for people who care about live session state, desktop rice, customization, and terminal presentation.

Unlike generic fetch tools, it treats Hyprland runtime data as first-class output. The MVP combines classic system facts with current Hyprland state, GTK theme detection, wallpaper daemon detection, and a terminal-friendly renderer that degrades cleanly when parts of the environment are unavailable.

## Current MVP

- Basic system modules: OS, host, kernel, uptime, packages, shell, terminal, CPU (with realtime usage), GPU (with VRAM and usage), memory, swap, disk, battery, locale, network (with WiFi SSID/signal), audio, temperature, datetime
- Hyprland modules: version, health summary, runtime instance summary, current splash, active workspace, focused monitor and session summary, workspace history, workspace list with active/special/fullscreen state, active window state, cursor position, monitor summary, config health, workspace rules, input summary, keybind summary, global shortcut summary, layout summary, layer-shell surface summary, hypr-rice (layout, gaps, border, rounding, blur, animations)
- Rice modules: GTK theme, icon theme, cursor theme, font, wallpaper backend
- Ecosystem detection: wallpaper, bar, launcher, notification, clipboard, screenshot, idle, and lock daemons (with version info) when present
- Output modes: styled split layout, plain mode, minimal mode, JSON mode, diagnostics mode, configurable module order, width-aware layout fallback for narrow terminals, share preset for rice screenshots, `--watch` continuous refresh mode
- Customization: TOML-driven module order, per-module options, user-defined display labels, and command plugins
- Palettes: hypr, mono, catppuccin, dracula
- Logo system: bundled distro-aware text logos stored in `assets/logos/`, with `os-release` auto detection, family fallback, and explicit override support (128 curated presets)
- Package managers: pacman, dpkg, rpm, nix, homebrew
- Config: `~/.config/hyprfetch/config.toml` or `$XDG_CONFIG_HOME/hyprfetch/config.toml`

## Why It Exists

- Hyprland users often want current session context, not just hardware specs.
- Rice screenshots benefit from fetch output that understands the compositor and desktop stack.
- A focused tool can stay fast and maintainable instead of becoming a generic “detect everything” binary.

## Build

```bash
cargo build --release
```

## Install Locally

```bash
cargo install --path . --locked
```

Or install the built binary manually:

```bash
install -Dm755 target/release/hyprfetch ~/.local/bin/hyprfetch
```

## Quick Start

Default output:

```bash
hyprfetch
```

This uses `os-release` to choose a built-in distro logo automatically. If the distro is not recognized, `hyprfetch` falls back to a generic Linux logo.
Styled output uses `key: value` rows, a session title, and optional terminal color bars similar to classic fetch screenshots.
On narrow terminals, `hyprfetch` automatically drops from the logo split layout to denser text layouts instead of relying on hard wrapping.
On very narrow terminals, it also prioritizes core system and Hyprland session modules before auxiliary modules like wallpaper or device summaries.

Plain output:

```bash
hyprfetch --plain
```

Share preset (optimized for rice screenshots):

```bash
hyprfetch --share
```

Minimal style (key:value only, no header or color bars):

```bash
hyprfetch --style minimal
```

Continuous refresh (redraws in place when attached to a terminal):

```bash
hyprfetch --watch --watch-interval 1
```

Create a starter config:

```bash
hyprfetch --init-config
```

Preview the default config without writing a file:

```bash
hyprfetch --print-config
```

Choose modules explicitly:

```bash
hyprfetch --modules os,kernel,uptime,hyprland,hypr-health,hypr-instances,hypr-splash,hypr-active-window,hypr-cursor,hypr-workspaces,hypr-monitors,hypr-config,hypr-rules,hypr-binds,hypr-shortcuts,hypr-layouts,hypr-layers,theme,wallpaper,hypr-ecosystem
```

Machine-readable output:

```bash
hyprfetch --json
```

Diagnose missing or partial modules:

```bash
hyprfetch --diagnostics
```

Show built-in logo presets:

```bash
hyprfetch --list-logos
```

Force a specific logo:

```bash
hyprfetch --logo arch
hyprfetch --logo hypr
hyprfetch --logo windows
hyprfetch --logo macos
```

## Example Config

`~/.config/hyprfetch/config.toml`

```toml
version = 1

[appearance]
style = "split"
palette = "hypr"
color = "auto"
plain = false
color_bars = true
padding = 1
separator = "  "

[logo]
preset = "auto"

[[modules]]
name = "os"
label = "Distro"

[[modules]]
name = "host"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "packages"

[[modules]]
name = "shell"

[[modules]]
name = "terminal"

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"

[[modules]]
name = "swap"

[[modules]]
name = "disk"

[[modules]]
name = "battery"

[[modules]]
name = "locale"

[[modules]]
name = "network"

[[modules]]
name = "temperature"

[[modules]]
name = "audio"

[[modules]]
name = "datetime"

[[modules]]
name = "hyprland"
show_runtime = false
labels = { Workspace = "WS", Clients = "Windows", Focused = "Output" }

[[modules]]
name = "hypr-health"

[[modules]]
name = "hypr-instances"
compact = true

[[modules]]
name = "hypr-splash"

[[modules]]
name = "hypr-active-window"
compact = true

[[modules]]
name = "hypr-cursor"

[[modules]]
name = "hypr-workspaces"
compact = true

[[modules]]
name = "hypr-monitors"
compact = true

[[modules]]
name = "hypr-config"
compact = true

[[modules]]
name = "hypr-rules"
compact = true

[[modules]]
name = "hypr-devices"
compact = true

[[modules]]
name = "hypr-binds"
compact = true

[[modules]]
name = "hypr-shortcuts"
compact = true

[[modules]]
name = "hypr-layouts"
compact = true

[[modules]]
name = "hypr-layers"
compact = true

[[modules]]
name = "hypr-rice"
compact = true

[[modules]]
name = "theme"
compact = false
show_font = true

[[modules]]
name = "wallpaper"

[[modules]]
name = "hypr-ecosystem"
compact = true

[[plugins]]
name = "battery"
command = "sh"
args = ["-c", "printf 'Battery\t88%%\n'"]
timeout_ms = 1000
```

## Command Plugins

Command plugins are the first step toward a richer plugin system. They let users add custom rows without recompiling `hyprfetch`.

Each plugin runs a command and reads stdout:

- A single plain line becomes one row using `label` or `name`.
- `Key<TAB>Value` lines become explicit rows.
- `Key=Value` lines also become explicit rows.

Example:

```toml
[[plugins]]
name = "music"
command = "sh"
args = ["-c", "printf 'Track\t%s\n' \"Lo-fi radio\""]
timeout_ms = 1000
```

## Module IDs

- `os`
- `host`
- `kernel`
- `uptime`
- `packages`
- `shell`
- `terminal`
- `cpu`
- `gpu`
- `memory`
- `swap`
- `disk`
- `battery`
- `locale`
- `network`
- `temperature`
- `audio`
- `datetime`
- `colorscheme`
- `hyprland`
- `hypr-health`
- `hypr-instances`
- `hypr-splash`
- `hypr-workspaces`
- `hypr-monitors`
- `hypr-active-window`
- `hypr-cursor`
- `hypr-workspace-history`
- `hypr-rice`
- `hypr-config`
- `hypr-rules`
- `hypr-devices`
- `hypr-binds`
- `hypr-shortcuts`
- `hypr-layouts`
- `hypr-layers`
- `hypr-ecosystem`
- `theme`
- `wallpaper`

## Data Sources

- `/etc/os-release`
- `/proc`
- `/sys`
- `/proc/stat` for realtime CPU utilisation, sampled across the collection window
- `nvidia-smi` for NVIDIA GPU utilisation, `/sys/class/drm/*/device/gpu_busy_percent` and `gt_perf_status` for AMD and Intel
- `/var/lib/pacman/local`
- `/var/lib/dpkg/status`
- `/sys/class/power_supply`
- `/proc/net/route`, `ip` command for network detection
- PipeWire/PulseAudio runtime sockets for audio detection
- environment variables, including Hyprland runtime variables and socket paths
- Hyprland runtime instance directories under `$XDG_RUNTIME_DIR/hypr`
- GTK settings files under `$XDG_CONFIG_HOME`
- `hyprctl`, including `splash` for the current Hyprland splash and `-j` endpoints such as `cursorpos` for pointer location, `configerrors` for config health, `workspacerules` for workspace rule summaries, `binds` for keybind summaries, `globalshortcuts` for portal shortcut summaries, `layouts` for available compositor layouts, `layers` for layer-shell surfaces, and `getoption` for the opt-in `hypr-rice` module
- `rpm -qa` for RPM-based package counting
- `/nix/store` for Nix package counting
- `/opt/homebrew/Cellar` or `/usr/local/Cellar` for Homebrew package counting

The binary stays usable outside Hyprland. Hyprland-specific modules degrade to unavailable instead of aborting the run.

## Built-in Logos

`hyprfetch` ships with a bundled logo catalogue in `assets/logos/`. It covers the major
families and their well known derivatives, so a machine running a niche distro lands on
its own artwork instead of a generic fallback:

- **Arch family**: `arch`, `arcolinux`, `archcraft`, `archlabs`, `blackarch`, `hyperbola`, `steamos`, `steamdeck`, `chimera`, `biglinux`, `manjaro`, `garuda`, `cachyos`, `endeavouros`, `artix`, `asahi`
- **Fedora and RHEL family**: `fedora`, `rocky`, `almalinux`, `centos`, `oracle`, `silverblue`, `kinoite`, `sericea`, `nobara`
- **Debian family**: `debian`, `devuan`, `kali`, `parrot`, `raspbian`, `mx`, `antix`, `dietpi`, `deepin`, `q4os`, `proxmox`, `truenas`, `emmabuntus`, `lilidog`, `locos`
- **Ubuntu family**: `ubuntu`, `pop`, `elementary`, `zorin`, `linuxmint`, `tuxedo`, `vanilla`, `peppermint`, `linuxlite`, `lliurex`, plus the `kubuntu`, `xubuntu`, `lubuntu`, `ubuntu-studio`, `ubuntu-cinnamon`, `ubuntu-budgie`, `ubuntu-mate`, `ubuntu-unity`, `ubuntu-kylin`, `ubuntu-sway`, `ubuntu-touch` and `ubuntu-gnome` flavours
- **SUSE family**: `opensuse`, `opensuse-leap`, `opensuse-tumbleweed`, `opensuse-microos`, `opensuse-slowroll`
- **Gentoo family**: `gentoo`, `funtoo`, `calculate`, `redcore`, `pentoo`
- **Other Linux**: `nixos`, `guix`, `void`, `alpine`, `postmarketos`, `solus`, `chakra`, `pureos`, `openmandriva`, `openeuler`, `antergos`, `kaos`, `altlinux`, `rosa`, `lunar`, `source-mage`, `slackware`, `mageia`, `mandriva`, `exherbo`, `scientific`, `lfs`, `puppy`
- **Enterprise and appliance**: `clear-linux-os`, `amazon`, `openwrt`, `opnsense`, `haiku`, `qubes`, `openkylin`, `kylin`, `uos`, `astra`, `cbl-mariner`, `android`, `harmonyos`, `sailfish`, `bredos`, `smartos`
- **BSD and Unix**: `freebsd`, `netbsd`, `openbsd`, `dragonfly`, `ghostbsd`, `midnightbsd`, `nomadbsd`, `bsd`, `solaris`, `zos`, `aix`
- **Platform and generic**: `hypr`, `linux`, `windows`, `macos`

Two extra presets are also available: `minimal`, which renders the `hyprfetch` wordmark instead of distro art, and `none`, which hides the logo entirely. Run `hyprfetch --list-logos` for the authoritative list.

Auto detection follows the same basic idea that makes `fastfetch` strong here: check `VARIANT`, then `ID`, then `NAME`, then `ID_LIKE`, and only then fall back to the platform family logo. That means unsupported derivatives still land on a sensible family logo in many cases instead of dropping straight to a generic fallback.

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Packaging

The release profile enables thin LTO and symbol stripping, so `cargo build --release --locked` produces a compact single binary.

An Arch packaging template lives at `contrib/PKGBUILD`:

```bash
cd contrib
makepkg -si
```

Before submitting to the AUR, confirm the GitHub repository URL in `Cargo.toml` and `contrib/PKGBUILD`, then tag releases as `v0.1.0`, `v0.2.0`, and so on.

## Roadmap

- richer Hyprland ecosystem detection
- deeper workspace history and input detail modules
- additional distro logos
