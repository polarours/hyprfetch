# Changelog

All notable changes to hyprfetch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--logo-image PATH` (`logo_image` in the config, `--logo-image-cols` for the
  width ceiling): draws a PNG as the logo using the kitty graphics protocol,
  which kitty, ghostty and WezTerm implement. The picture is scaled to the pixel
  size of the cell box it occupies — the protocol's own `c`/`r` only move the
  cursor, they do not resize the image — and sized so it fills the height of the
  information block, capped at half the terminal. The rows are positioned beside
  it with absolute cursor moves, and the arcade view switches off: a picture and
  a HUD cannot share the same space.
  Anything unusable (no protocol support, missing file, non-PNG, over 8 MB)
  falls back to the text logo instead of printing raw escapes, and the payload
  is never emitted when stdout is not a terminal.
### Added

- `display`: the focused monitor as `2560x1600 @165Hz (eDP-1)`. Hyprland is
  asked first because it knows the refresh rate; outside the compositor the
  row falls back to `/sys/class/drm` and simply omits the rate instead of
  guessing.
- `weather` and `publicip`, both **opt-in**. They shell out to `curl` with a
  three-second budget, because a fetch tool should only send traffic when the
  user asks for it. A failed lookup reports "module unavailable" rather than a
  stale or invented value.
### Added

- `bios` and `board`: firmware vendor/version and mainboard model from
  `/sys/class/dmi/id`, with the SMBIOS placeholders (`None`,
  `To Be Filled By O.E.M.`) treated as absent so no row fills with noise.
- `initsystem`: the init that owns PID 1, normalised to `systemd`, `OpenRC`,
  `runit`, `s6` or `sysvinit`.
- `editor`: `$VISUAL`, then `$EDITOR` (reduced to the binary name), then the
  first of `nvim`/`vim`/`hx`/`nano`/`emacs` found on `PATH`.
- `brightness`: average backlight level across `/sys/class/backlight` devices.
- `bios`, `board` and `initsystem` join the default order; `editor` and
  `brightness` stay opt-in.
## [0.2.1] - 2026-11-25

### Added

- Four local system modules: `localip` (the address the kernel would route
  with, via a connected UDP socket — no packets sent), `loadavg` (1/5/15-minute
  load), `processes` (live process count) and `users` (logged-in users from
  `who`). All are cheap local reads and join the default module order after
  `network`.
- `--icons` (config `icons = true`): prefixes every row with a Nerd Font glyph
  (` CPU`, ` MEMORY`) while keeping the text label, so the layout still reads
  without a patched font. Opt-in — terminals without a Nerd Font would show
  tofu. Only widely-covered Font Awesome codepoints are used.

## [0.2.0] - 2026-11-25

### Changed

- Keys are padded to a common width, so every value now starts in the same
  column; previously each value began immediately after its own key and the
  column drifted with the label length.
- Arcade usage meters are wide LED energy cells: fourteen `▮` cells for the
  load and `▯` for the remainder, so the bar carries the same visual weight as
  the logo and rows. The meter hugs its value with a single space, and it keeps
  the same visible width at any load.
- Keys render as upper-case arcade button labels (`[CPU]`, `[MEMORY]`), padded
  to one width so values still align in a single column. Plain ASCII brackets
  keep the width exact on every terminal, unlike ambiguous-width arrows.
- Figures inside values (`53%`, `8.1 GiB`, `72°C`) are rendered in the theme
  accent, leaving prose and units in the bright value tone. Version strings
  (`7.3.5-arch1-1`) stay plain.
- The `arcade` palette's session-group tones are muted periwinkle and steel
  blue instead of two reds, so the Hyprland block no longer dominates the
  screenshot.
- Distro logos are now drawn as solid pixel blocks (`█` cells) instead of a
  thin `·` dot outline, giving the fetch a pixel-font look while every distro
  keeps its own mark. The running distro's logo stays the default; an optional
  `hypr-pixel` preset adds a solid-block `HYPRFETCH` wordmark.
- Arcade styling is now the default: the `arcade` palette and `retro = true`
  (CRT rules, pixel meters) are on out of the box, while the
  normal fetch layout is preserved. `--no-retro` restores the plain look.
- An `arcade` theme in an ice-blue coin-op palette: light blue `#8ecbff`,
  electric aqua `#7ef5e0`, ice white on deep navy `#0a1020`; locked in the
  ground-truth test.
- `--logo-file <path>` renders user-supplied ASCII artwork with the same
  dot-outline treatment and active palette as built-in logos, without adding it
  to the catalogue.
- `--doctor`: a deep diagnostics report that classifies each failing module's
  message and prints an actionable fix, plus general tips. `--diagnostics` remains
  the terse status summary.
- Standalone theme files: `--theme-file <path>` loads an `[appearance]`-only
  TOML that changes the look (palette, style, bars, padding, separator) without
  touching the module set — so a look can be shared independently of behavior.
- Built-in presets: `--preset all|rice|minimal` select a ready-made module set
  and look, with explicit CLI flags still taking precedence.
- Per-module `format` templates: `format = "{name}"` / `{usage_percent:.0}` reshape
  a module's value from its JSON data, with numeric rounding and meters preserved.
- A GitHub Pages landing page at `https://polarours.github.io/hyprfetch/` with the
  wordmark, live demo GIF, feature cards, a three-theme gallery and install commands.
  Deployed from `docs/` by `.github/workflows/pages.yml`.
- hyprfetch is now on the AUR as `hyprfetch-git`; the README Packaging section
  links the package and shows `yay -S` / `paru -S` install commands. The stale
  "before submitting to the AUR" note is gone.
- A multi-theme screenshot gallery in the README (hypr, catppuccin, gruvbox),
  rendered from real PTY captures so users can compare palettes at a glance.
- A clear [!IMPORTANT] notice at the top of the README states that 0.1.0 is an
  early, testing-stage release with no stability guarantees.

### Fixed

- A latent race in command output draining: buffered stdout could occasionally be
  lost when a reader thread was slow to schedule, making a test flaky on CI. The
  reader pipes are now non-blocking, so the reader drains every available byte
  before observing the stop signal — no data loss, and no descendant can pin it.
- The reveal animation felt like a startup stall: its 450 ms budget made output
  appear in slow stages. The budget is now 180 ms (split/wave phases and per-line
  delays scaled accordingly), cutting the real-terminal run from ~520 ms to ~250 ms.
- The README demo GIF had a stacked-duration bug (one frame lasted ~156 s, making
  it look frozen). Re-encoded from the real PTY capture at 900px, 43 frames,
  ~2.9 s total, ~66 KB — loads and loops promptly.

### Changed

- The README is redesigned around a full-width screenshot of the default output,
  with a badge row, a navigation bar and grouped feature sections. The former
  text-only output dump is gone; detailed reference material (config, plugins,
  module IDs, data sources, logo families) is preserved and the logo families
  now live in a collapsible section.

### Changed

- The README hero is now a fresh recording of the current arcade build:
  ice-blue distro logo in solid pixel blocks, `[BRACKET]` labels and the
  14-cell LED usage meters.

## [0.1.0] - 2026-11-21

### Added

- `--fast` (or `fast = true` in the config) skips expensive probes: CPU usage
  sampling, the GPU vendor tool, audio/WiFi/media external commands. Every
  module row stays but values needing those probes are omitted, never guessed.
  Fast mode renders in ~25 ms versus ~50 ms in normal mode.

### Performance

- Startup is now ~50 ms in release mode (down from ~2.3 s), close to fastfetch's
  range while keeping the live Hyprland state that fastfetch does not collect.
  The remaining big wins:
  - The command helper's reader threads now stop when the child exits instead of
    waiting a fixed grace period for a pipe held open by a background descendant
    (each such command used to add 100 ms; a run has several).
  - The Hyprland ecosystem is detected once and cached; several modules asked for
    it separately and each paid for a full process scan plus version probes.
  - `--version` probes are opt-in: the default compact output and normal JSON
    render skip them; they run only for `versions=true` and full JSON, where the
    version is actually shown. Probing cost ~100 ms on a normal desktop.
  - The CPU usage module runs last so its 50 ms `/proc/stat` sampling window
    overlaps the other modules' work (hyprctl, GPU, audio, network) instead of
    adding its own sleep. Output order is unchanged and the sample remains a
    real 50 ms window; this alone cut startup from ~80 ms to ~50 ms.

- The "Collecting system information" spinner waits 500 ms before drawing, so it
  never flashes in a healthy run; it only appears when a probe or plugin stalls.

### Fixed

- The `Wallpaper` key was missing from the theme-row color group, so it rendered
  in the muted gray while Theme/Ecosystem/Input shared a colored accent. It now
  shares that accent; a regression test locks all four keys together across
  every palette.

- NetworkManager is detected again: the process list carries lowercased names and a root
  daemon offers no readable executable, so the name comparison is now case-insensitive (it
  reported `iwd` on a machine running both).
- Keys in the minimal layout keep their accent colors: the right-aligned padding made the
  label miss its color match and every short key rendered in the muted gray.
- Three unbounded waits on external commands are bounded: the shared helper now drains the
  pipes under its deadline (a background descendant holding the pipe can no longer hang the
  run), `rpm -qa` gives up while the database is locked by another transaction, and
  `nvidia-smi` no longer stalls behind a wedged driver.
- The spinner respects `NO_COLOR` like the rendered output does.
- The two-phase and wave reveals fall back to the plain line-by-line reveal when the output
  is as tall as the terminal window, whose cursor cannot step back to the first row.
- Twenty-seven bundled artworks lost ink where a literal `$` was mistaken for a fastfetch
  colour marker; literal dollars are now escaped in the assets, so every ink dot survives.
- The material theme's officially verified foreground/background is locked in the ground-truth
  test with the rest of the catalogue.

## [0.0.1] - 2026-11-19

The squashed initial baseline (`init commit`) of hyprfetch, a Hyprland-native fetch utility.

### Added

- 40 built-in modules covering the classic system rows — OS, host, kernel, uptime, packages,
  shell, terminal, CPU with live usage, GPU with VRAM and utilization, memory, swap, disk with
  several mount points, battery, locale, network with WiFi SSID and signal, temperature, audio
  with the default sink's volume and mute state, media through MPRIS, datetime and colorscheme —
  plus the Hyprland session rows fastfetch cannot show: compositor version, health summary,
  runtime instances, splash, active workspace, workspace history with monitor and window
  counts, the workspace list, monitors, active window, cursor position, config health,
  workspace rules, input devices, keybinds, global shortcuts, per-workspace layouts,
  layer-shell surfaces, hypr-rice (layout, gaps, border, rounding, blur, animations), GTK/Qt
  theme and wallpaper backend.
- Ecosystem detection across twenty daemon categories, each with its version when the process
  can be probed: wallpaper, bar, launcher, notifications, clipboard, screenshot, idle, lock,
  portal, polkit, tray, colour picker, network (NetworkManager/iwd/ConnMan/systemd-networkd),
  audio (PipeWire/PulseAudio), bluetooth, greeter (SDDM/GDM/LightDM/ly/greetd), IME
  (fcitx5/ibus), night light (gammastep/wlsunset/hyprsunset/redshift), power
  (power-profiles/tuned) and automount (udisks). The compact row stays short with a `+N`
  remainder, a fixed name width and an opt-in `versions = true`.
- Thirty-four alternate drawings of covered distros — numbered takes (`arch2`, `artix2`,
  `macos2`, `void2`, ...), older artwork (`fedora_old`, `ubuntu_old`, ...) and retro variants —
  are selectable by name without joining the auto detection (`--logo arch2`), and the `minimal`
  preset now renders the bundled wordmark artwork instead of a bare string.
- A logo catalogue of 429 presets drawn from the bundled artwork and rendered in a minimal
  dot-outline house style with a brand-matched tone gradient: `os-release` auto detection with
  family fallback, `--logo` override, and coverage of the Arch, Fedora, Debian, Ubuntu, SUSE and
  Gentoo families, their derivatives, the BSDs, niche Linux distros and alternate platforms.
- 17 built-in color themes — hypr, mono, catppuccin, dracula, gruvbox, gruvbox-light, nord,
  one-dark, solarized-dark, solarized-light, rose-pine, ayu, everforest, kanagawa, monokai,
  material, github — selectable with `--theme` or `palette` in the config; values are drawn in
  a bright near-white tone derived per theme so they stay readable next to the colored keys.
- Usage bars (green → gold → red by load) on the CPU, GPU, memory and swap rows, with the
  value width budgeted so long lines never overflow the terminal.
- An animated reveal on every run, and a personal choice of story: `--animation split` (the
  dot-outline logo skeleton grows first, then the session rows light up behind it), `wave` (the
  logo cells light up column by column from the left edge), `lines` (one row after another) or
  `static` for no motion at all — also settable as `animation = "..."` in the config file. The
  reveal lands inside roughly half a second whatever the output length, only runs on a real
  terminal, and the finished frame is byte-identical to the static output.
- TOML-driven configuration (`--init-config` to create one): module order, per-module options,
  display labels, theme, logo and animation; command plugins that turn any script's output
  into rows; and output modes for every consumer — styled split layout, plain, minimal, JSON,
  diagnostics, a share preset that leads with the theme and the live session state, and
  `--watch` continuous refresh with in-place redraw.
- Width-aware layout fallback: a narrow terminal first switches to a compact logo drawing,
  then to denser text layouts that prioritize the core system and Hyprland rows, so nothing
  wraps or overflows.
- A man page (`contrib/hyprfetch.1`) and an Arch PKGBUILD.

- Every theme's foreground and background has been verified against the palettes of the
  theme projects themselves (Tokyo Night, Catppuccin Mocha, Rosé Pine, Dracula, Atom One Dark,
  Solarized, Nord, Everforest, Kanagawa and the rest), and the verified values are locked in a
  test so a future edit cannot drift from the official palettes. The kanagawa, ayu and github
  entries deliberately use the classic tones their terminal ports are built on.

### License

- hyprfetch is released under GPL-3.0; see `LICENSE`.
