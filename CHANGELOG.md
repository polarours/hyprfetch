# Changelog

All notable changes to hyprfetch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-23

### Added

- A narrow terminal keeps the logo by falling back to a narrower drawing of it, so the split layout survives down to about 55 columns instead of 76
- A man page at `contrib/hyprfetch.1`, shipped with the releases and with the Arch package

- **System modules**: realtime CPU utilisation, GPU utilisation for NVIDIA, AMD and Intel
- **Hyprland modules**: hypr-workspace-history (the window that was last focused in each workspace)
- **Ecosystem detection**: version info for detected daemons, now covering portal, polkit, tray and colour picker backends as well
- **Disk module**: reports several mount points at once, configured with `mounts = ["/", "/home"]`
- **Input devices**: the caps and num lock state of the main keyboard, mentioned in the summary while engaged
- **Output modes**: `--watch` and `--watch-interval` for continuous refresh
- **Distro logos**: the catalogue grew from 23 to 225 presets, covering the Arch, Fedora, Debian, Ubuntu, SUSE and Gentoo families plus their derivatives, the BSDs and several appliance distributions, with artwork for Windows 95, 8, 11 and 2025
- **Logo detection**: `VARIANT` is read from `os-release`, so Ubuntu flavours and Fedora immutable variants resolve to their own logo

### Fixed

- CPU utilisation no longer blocks the run for 500ms; it is sampled across the collection window instead
- GPU utilisation is queried once per run and matched to each device by PCI bus id, so multi-GPU systems report per-GPU values
- `--watch` redraws in place on a terminal instead of scrolling past
- `--logo pop` selected the Ubuntu logo, and Linux Mint, Rocky, Kali and other derivatives fell back to the logo of the distribution they derive from
- The integration test asserting the OS row hardcoded the name of the distribution it was written on, so it failed on every CI runner that is not Arch
- The disk module called `statvfs`, which macOS does not have, so the macOS build never compiled
- Reading the version of a detected backend could hang the run, because a backend that ignores `--version` never exits
- Backends whose name is longer than 15 characters were never detected, because the kernel truncates a process name in the process list
- An endpoint or option the running Hyprland does not know was reported as a JSON parse error, because hyprctl answers it with a plain message and a zero exit code
- The workspace history module read a config option that no Hyprland version has, so it could never report anything
- The split and column layouts rendered one character wider than the terminal, so the longest row wrapped
- The SSID, signal and link speed of a wireless interface were missing on drivers that do not publish the wireless files in `/sys`, such as iwlwifi
- The amount of video memory of an NVIDIA card came from the size of a PCI bar, so a card with 8 GiB of memory was reported as having 16 MiB
- The default audio sink was never reported, because the parser of the `wpctl` output skipped every line
- The awww wallpaper daemon and the bemenu launcher were not recognised
- The health check did not look for a desktop portal or a polkit agent, so a missing portal or a missing authentication agent went unnoticed
- Every Hyprland module is now covered by a test that runs it without a compositor, so a missing `hyprctl` can no longer pass unnoticed
- A plugin that failed was dropped from the output without a trace, so a broken plugin looked like a plugin that produced nothing

### Changed

- README module and logo lists now match the shipped catalog
- The Hyprland module directory is split per domain: session, window, workspaces, monitors, rice, input, binds, layers, config, rules, shortcuts, layouts and ecosystem
- The system provider is split per domain as well: os, cpu, memory, storage, graphics and session
- The Hyprland provider is split per domain too: session, workspace, window, display, input and config
- The renderer is split into the layout, the styling and the public entry points
- The system modules are split per domain: basics, hardware, storage and session
- The configuration is split into the types, the shipped defaults, loading the file and merging it with the command line
- The disk module reports its numbers under `disks` in the JSON output instead of at the top level, because it can now report more than one mount point
- The share preset now shows the desktop, not only the machine: the theme, the wallpaper backend, the ecosystem, the active window, the workspaces, the monitors and the input devices
- The logo catalogue is generated from a single table, so adding a logo no longer needs changes in several places
- Ecosystem versions are read in parallel instead of one backend after the other, and each probe is given up on after 500ms
- Selecting modules with `--modules` or `--share` keeps the options configured for them instead of discarding them

## [0.2.0] - 2025-06-23

### Added

- **System modules**: temperature, datetime, colorscheme, network (with WiFi SSID/signal), audio, GPU VRAM
- **Hyprland modules**: hypr-rice (layout, gaps, border, rounding, blur, animations) now enabled by default
- **Render styles**: minimal mode for key:value output without headers
- **Palettes**: catppuccin, dracula (in addition to hypr, mono)
- **CLI flags**: `--share` (screenshot preset), `--style` (render style selection)
- **Package managers**: dpkg, rpm, nix, homebrew (in addition to pacman)
- **Ecosystem detection**: clipboard (cliphist, wl-clipboard), screenshot (grim, slurp, hyprshot, flameshot)
- **Distro logos**: Garuda, Nobara, Artix, Asahi (21 curated presets)
- **CI/CD**: GitHub Actions with multi-platform build (Linux/macOS, amd64/arm64) and release automation
- **Integration tests**: 7 new tests for temperature, datetime, network, audio, share preset, minimal style

### Changed

- Refactored hyprland.rs into module directory with workspace submodule
- Updated PKGBUILD with check() function and optdepends
- Session overview now shows total window count instead of client count

## [0.1.0] - 2025-06-20

### Added

- Initial release
- System modules: OS, host, kernel, uptime, packages, shell, terminal, CPU, GPU, memory, swap, disk, battery, locale
- Hyprland modules: version, health, instances, splash, workspaces, monitors, active window, cursor, config, rules, devices, binds, shortcuts, layouts, layers, ecosystem
- Rice modules: GTK theme, wallpaper backend
- Render styles: split, compact
- Palettes: hypr, mono
- Logo system with auto-detection and 17 curated presets
- TOML configuration with module order, labels, and command plugins
