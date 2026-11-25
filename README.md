# Lithophane Studio

Fast, watertight 3D lithophane generator written in **Rust**, featuring a minimal canvas-first **Tauri + Three.js** desktop application and the standalone **`litho` CLI**.

## Features

- **Flat, Curved & Multi-Panel Modes**
  - **Flat**: Upright panels with configurable frame borders, optional holder extensions, and removable breakaway triangular support fins, bed adhesion pads, and tapered tabs.
  - **Curved**: Concave or convex cylindrical-arc lithophanes with exact arc angle and chord telemetry, an optional **built-in filleted stand base** (default `1.6 mm` thick, projecting forward and backward only with smoothly rounded corners), and optional radial supports.
  - **Multi**: Two or more parallel flat lithophanes linked by flat-bottomed breakaway bridges and outer support fins.
- **Modular Geometry Architecture**: Extensible `LithophaneStyle` trait and typed multi-part scene model (`Panel`, `Base`, `Support`, `Bridge`, `ColorLayer`, `Structural`) designed for future colour sublayer prints, domes, and multi-image cylindrical lamps.
- **Fast Watertight Meshes**: Pure-Rust `manifold-rust` boolean union engine with automatic manifold verification and `rayon`-parallel binary/ASCII STL and multi-part 3MF export.
- **Physical Backlit Simulation**: Real-time Beer-Lambert optical simulation driven by filament colour, Transmission Distance (`TD`), and backlight spectrum (defaulting to white PLA `TD = 5.0 mm` in daylight).
- **Restorable Self-Contained Projects**: Save and reopen complete workspaces (`project.json`, copied source images, thumbnails, and export folders), plus persistent Application Settings including full-resolution default preview and optional low-resolution scrubbing mode.
- **Clean Studio UI**: Full-screen backlit WebGL viewport, mode-aware floating quick bar, and a collapsible drawer containing every advanced parameter and JSON preset import/export.

## Installation

Pre-built desktop installers and standalone CLI archives are published on the [GitHub Releases](https://github.com/jeffwinterbourne/lithophane_maker/releases) page:

- **Windows**: `.exe` (NSIS installer) or `.msi` installer (`x86_64`)
- **macOS**: `.dmg` disk image (`Apple Silicon` and `Intel`)
- **Linux**: `.AppImage`, `.deb`, or `.rpm` (`x86_64`)
- **CLI (`litho`)**: Standalone compressed archives (`zip` / `tar.gz`) for Windows, macOS, and Linux

> **Note:** Release binaries are currently unsigned. On macOS (Gatekeeper) or Windows (SmartScreen), you may need to explicitly allow launching the downloaded installer or executable on first run.

## CLI (`litho`) Usage

Extract the `litho` archive (or build from source) and run:

```bash
# Classic flat lithophane
litho make photo.jpg -o out --max 150

# Curved lithophane with 110 mm radius and 1.6 mm filleted built-in base
litho curved photo.jpg -o out --radius 110 --base-thickness 1.6 --format 3mf

# Multi-lithophane assembly with common 150 mm bed width
litho multi a.jpg b.jpg c.jpg -o out --common-dim 150 --spacing 12
```

Run `litho --help` or `litho <subcommand> --help` for all options, presets, and `--set group.field=value` overrides.

## Building from Source & Packaging Installers

### Prerequisites

- **Rust toolchain** (stable, edition 2021) via [rustup](https://rustup.rs/)
- **Node.js** (LTS, e.g. 22+) and `npm`
- **Linux only** (Ubuntu/Debian desktop build dependencies):
  ```bash
  sudo apt-get update && sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
  ```

### Build Binaries

```bash
npm --prefix ui ci
npm --prefix ui run build
cargo build --release --workspace
```

Compiled binaries are placed in `target/release/` (`lithophane-studio` desktop app and `litho` CLI).

### Package Native Installers

```bash
npm --prefix ui ci
npx --yes @tauri-apps/cli@2 build
```

Platform installers are generated under `target/release/bundle/`.

## License

Licensed under the GNU General Public License v3.0. See [LICENSE](LICENSE) for details.
