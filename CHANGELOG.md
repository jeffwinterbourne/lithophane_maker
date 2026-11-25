# Changelog

All notable changes to **Lithophane Studio** are documented in this file.

## [1.0.0] - 2026-11-25

### Added
- **Desktop App & CLI (`Lithophane Studio` + `litho`)**: Pure-Rust geometry, image processing, and boolean union engine (`litho-core`), paired with a standalone CLI (`litho`) and a canvas-first Tauri + Three.js desktop studio.
- **Flat, Curved & Multi-Panel Lithophane Modes**:
  - **Flat**: Upright panels with configurable borders, optional bottom holder extensions, and removable breakaway triangular support fins, bed pads, and tapered tabs.
  - **Curved**: Concave or convex cylindrical-arc lithophanes with exact arc angle and chord telemetry, optional built-in filleted stand base (default `1.6 mm` thickness projecting forward and backward only with smoothly rounded corners), and optional radial supports.
  - **Multi**: Two or more parallel flat panels joined along a common bed length with configurable Y spacing, flat-bottomed breakaway bridge connectors, and outer support fins.
- **Watertight Geometry & Multi-Format Export**: Pure-Rust `manifold-rust` CSG union engine verifying watertight positive-volume meshes before exporting binary/ASCII STL files or multi-part assembly 3MF packages (`<basematerials>`).
- **Physical Backlit Simulation**: Real-time Beer-Lambert optical preview driven by filament colour, Transmission Distance (`TD`), backlight colour/intensity, and front ambient light (shared across the 3D WebGL shader and 2D backlit PNG export), defaulting to white PLA (`#f6f6f2`, `TD = 5.0 mm`) in daylight (`#ffffff`).
- **Restorable Projects**: Self-contained project folders (`project.json`, copied `assets/`, `thumbnail.png`, `exports/`) with a Projects Manager modal (create, open, duplicate, rename/notes, delete), quick save (`Ctrl+S`), unsaved indicator, and optional automatic reopen on launch.
- **Application Settings & Full-Resolution Default Preview**: Persistent preferences (`Low-resolution preview mode` disabled by default so previews match final output resolution, debounce timing, configurable print-bed grid, default export format, filament density, and project/export folder settings).
- **Image Processing & Presets**: EXIF-aware loader with crop, rotate, mirror/flip, auto-levels, histogram equalization, adaptive CLAHE, brightness/contrast/gamma, Gaussian blur (default `0.1 mm`), unsharp mask, posterize, invert, and JSON preset import/export.
- **Cross-Platform Packaging**: Automated Windows (`.exe` NSIS / `.msi`), macOS (`.dmg`), Linux (`.AppImage` / `.deb` / `.rpm`), and `litho` CLI release artefacts.
