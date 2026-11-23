# Contributing to hyprfetch

Thank you for your interest in contributing to hyprfetch! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/your-username/hyprfetch.git
   cd hyprfetch
   ```
3. Create a new branch for your feature:
   ```bash
   git checkout -b feature/my-feature
   ```

## Development Setup

### Prerequisites

- Rust 1.85 or later
- Cargo (comes with Rust)

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Formatting

```bash
cargo fmt
```

## Code Style

- Follow Rust conventions and idioms
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Keep functions focused and small
- Add documentation for public items
- Write tests for new functionality

## Commit Messages

- Use clear, descriptive commit messages
- Start with a verb in imperative mood (e.g., "Add", "Fix", "Update")
- Keep the subject line under 72 characters
- Use the body for detailed explanations if needed

Example:
```
Add temperature module for CPU thermal monitoring

- Read from /sys/class/thermal/thermal_zone*
- Display highest temperature zone
- Gracefully handle missing thermal data
```

## Pull Request Process

1. Update the README.md with details of changes if applicable
2. Update the CHANGELOG.md with your changes
3. Ensure all tests pass: `cargo test`
4. Ensure clippy passes: `cargo clippy -- -D warnings`
5. Ensure code is formatted: `cargo fmt --check`
6. Create a pull request with a clear description

## Adding New Modules

To add a new system module:

1. Add the module ID to `src/model.rs`:
   - Add variant to `ModuleId` enum
   - Add to `all()`, `default_order()`, `label()`, `Display`, and `FromStr`

2. Add provider method in `src/providers/system.rs`:
   - Create data struct with `#[derive(Serialize)]`
   - Add method to `SystemProvider`
   - Implement data reading logic

3. Add module implementation in `src/modules/system.rs`:
   - Create module struct
   - Implement `Module` trait
   - Register in `src/modules/mod.rs`

4. Update `src/render.rs`:
   - Add to `row_section()` for section classification
   - Add to `output_priority()` for narrow terminal priority

5. Add tests:
   - Unit tests for parsing functions
   - Integration test in `tests/cli.rs`

## Adding New Logos

The logo catalogue is a single table in `src/logo.rs`. Every entry generates the
`LogoPreset` variant, its `Display`/`FromStr` implementation and the logo definition, so
one entry is all that is needed.

To add a new distro logo:

1. Create a text file in `assets/logos/` (use existing logos as reference)
2. Add one entry to the `logo_catalog!` table in `src/logo.rs`:

   ```rust
   Linuxmint => "linuxmint", ["linuxmint", "linux mint"],
       include_str!("../assets/logos/linuxmint.txt"), Green, Cyan, White;
   ```

   The fields are the enum variant, the selectable name, the os-release values that
   select it, the artwork and the three tones.

3. Add the preset to the logo list in `README.md`

Notes:

- Aliases are matched case-insensitively and ignore non-alphanumeric characters, so
  `linux mint` and `linuxmint` are the same alias. Keep them unique across the table; the
  `aliases_are_unique_across_the_catalogue` test enforces this.
- Every preset name must be parseable, so the name is always accepted by `--logo`.
- Run `cargo test` after adding an entry. The catalogue tests check that every preset has
  artwork and that names round-trip.

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Include reproduction steps for bugs
- Include your environment details (OS, Rust version, etc.)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
