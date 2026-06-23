# Changelog

All notable changes to hyprfetch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
