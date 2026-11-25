//! Typed settings, validation, presets, and CLI override application.
//!
//! All user-tunable parameters live here as a single source of truth shared by
//! `litho-core`, `litho-cli`, and the Tauri desktop app. All lengths are in millimetres.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::LithoError;

/// Registered lithophane generation modes.
///
/// New modes (e.g., `ColorFlat`, `Dome`, `CylinderLamp`) should be added here
/// and registered in [`crate::styles::StyleRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StyleId {
    /// Classic upright flat panel with removable breakaway support fins.
    #[default]
    Flat,
    /// Vertically extruded cylindrical arc panel with optional filleted base and radial supports.
    Curved,
    /// Multiple parallel upright flat panels joined by breakaway bridges with outer-only fins.
    MultiFlat,
}

impl StyleId {
    /// Human-readable short identifier used in CLI flags and presets.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Curved => "curved",
            Self::MultiFlat => "multi_flat",
        }
    }
}

/// How output panel dimensions are specified in the image frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SizeMode {
    /// Longest outer side equals `max_dimension_mm`; other side follows image aspect.
    #[default]
    MaxDimension,
    /// Outer width equals `width_mm`; height follows image aspect.
    Width,
    /// Outer height equals `height_mm`; width follows image aspect.
    Height,
    /// Both outer sides specified explicitly (`width_mm` × `height_mm`).
    Absolute,
}

/// How an image is fitted when `SizeMode::Absolute` has a differing aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    /// Center-crop the image to the target aspect ratio.
    #[default]
    Crop,
    /// Pad the image with `ImageSettings::background` to match aspect ratio.
    Pad,
    /// Stretch non-uniformly to fill the target size.
    Stretch,
}

/// Output size and sampling resolution settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SizeSettings {
    pub mode: SizeMode,
    pub max_dimension_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit: FitMode,
    pub pixel_size_mm: f64,
}

impl Default for SizeSettings {
    fn default() -> Self {
        Self {
            mode: SizeMode::MaxDimension,
            max_dimension_mm: 150.0,
            width_mm: 100.0,
            height_mm: 100.0,
            fit: FitMode::Crop,
            pixel_size_mm: 0.25,
        }
    }
}

/// Greyscale luminance weighting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GreyscaleMode {
    #[default]
    Rec601,
    Rec709,
    Average,
    Red,
    Green,
    Blue,
}

/// Background colour used when flattening alpha or padding images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundColor {
    #[default]
    White,
    Black,
}

/// Histogram equalization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EqualizeMode {
    #[default]
    None,
    Hist,
    Clahe,
}

/// Greyscale conversion and tonal adjustments (applied in documented pipeline order).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ImageSettings {
    pub greyscale: GreyscaleMode,
    pub background: BackgroundColor,
    /// Optional crop box `[left, top, right, bottom]` as fractions `0..1` of image.
    pub crop: Option<[f32; 4]>,
    /// Clockwise rotation in degrees before sizing (`0`, `90`, `180`, or `270`).
    pub rotate: u16,
    pub mirror: bool,
    pub flip: bool,
    pub auto_levels_percent: f32,
    pub equalize: EqualizeMode,
    pub clahe_clip: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub blur_sigma_mm: f32,
    pub sharpen_amount: f32,
    pub sharpen_radius_mm: f32,
    pub posterize_levels: Option<u32>,
    pub invert: bool,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            greyscale: GreyscaleMode::Rec601,
            background: BackgroundColor::White,
            crop: None,
            rotate: 0,
            mirror: false,
            flip: false,
            auto_levels_percent: 0.0,
            equalize: EqualizeMode::None,
            clahe_clip: 0.01,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            blur_sigma_mm: 0.1,
            sharpen_amount: 0.0,
            sharpen_radius_mm: 0.5,
            posterize_levels: None,
            invert: false,
        }
    }
}

/// Mapping function from normalized luminance to thickness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThicknessCurve {
    /// Thickness linear in darkness (`1 - luminance`).
    #[default]
    Linear,
    /// `darkness ^ curve_gamma`.
    Gamma,
    /// Exponential Beer-Lambert transmission compensation (`perceived backlit brightness` linear).
    Transmission,
    /// Piecewise-linear interpolation through `custom_points`.
    Custom,
}

/// Which face of the panel carries the relief texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReliefSide {
    /// Textured face points toward `-Y` (viewer).
    #[default]
    Front,
    /// Textured face points toward `+Y` (away from viewer).
    Back,
}

/// Luminance-to-thickness mapping settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ThicknessSettings {
    pub min_thickness_mm: f64,
    pub max_thickness_mm: f64,
    pub curve: ThicknessCurve,
    pub curve_gamma: f64,
    pub attenuation_per_mm: f64,
    pub custom_points: Vec<[f64; 2]>,
    pub relief_side: ReliefSide,
}

impl Default for ThicknessSettings {
    fn default() -> Self {
        Self {
            min_thickness_mm: 0.6,
            max_thickness_mm: 3.6,
            curve: ThicknessCurve::Linear,
            curve_gamma: 1.0,
            attenuation_per_mm: 0.7,
            custom_points: vec![[0.0, 0.0], [1.0, 1.0]],
            relief_side: ReliefSide::Front,
        }
    }
}

/// Optional flat border around the image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FrameSettings {
    pub enabled: bool,
    pub width_mm: f64,
    /// Border thickness in mm; defaults to `thickness.max_thickness_mm` when `None`.
    pub thickness_mm: Option<f64>,
}

impl Default for FrameSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            width_mm: 3.0,
            thickness_mm: None,
        }
    }
}

/// Optional flat bottom extension for slotting into an external holder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct BottomExtensionSettings {
    pub enabled: bool,
    pub height_mm: f64,
    pub thickness_mm: f64,
}

impl Default for BottomExtensionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            height_mm: 10.0,
            thickness_mm: 2.0,
        }
    }
}

/// How the image is oriented for vertical 3D printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrientationMode {
    /// Narrowest dimension points up (`Z`), longest sits along bed (`X`).
    #[default]
    AutoShortUp,
    /// Longest dimension points up (`Z`).
    LongUp,
    /// Keep supplied image upright (no 90° print rotation).
    ImageUp,
}

/// Orientation settings (used by flat style; curved style has its own upright-default orientation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct OrientationSettings {
    pub mode: OrientationMode,
}

impl Default for OrientationSettings {
    fn default() -> Self {
        Self {
            mode: OrientationMode::AutoShortUp,
        }
    }
}

/// Which faces receive breakaway support fins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SupportSides {
    #[default]
    Both,
    Front,
    Back,
}

/// How panel and support meshes are packaged on export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SupportOutput {
    /// Boolean-unioned single watertight manifold shell.
    #[default]
    Merged,
    /// Disjoint shells concatenated into a single file.
    SeparateBodies,
    /// Panel and supports exported to separate files.
    SeparateFiles,
}

/// Removable triangular fins with bed pads and tapered breakaway tabs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SupportSettings {
    pub enabled: bool,
    pub sides: SupportSides,
    pub output: SupportOutput,
    pub fin_count: usize,
    pub fin_height_mm: Option<f64>,
    pub fin_height_fraction: f64,
    pub fin_base_length_mm: Option<f64>,
    pub fin_thickness_mm: f64,
    pub fin_inset_mm: Option<f64>,
    pub interface_gap_mm: f64,
    pub tab_count: usize,
    pub tab_height_mm: f64,
    pub tab_neck_height_mm: f64,
    pub tab_width_mm: Option<f64>,
    pub tab_overlap_mm: f64,
    pub tab_margin_mm: f64,
    pub pad_enabled: bool,
    pub pad_thickness_mm: f64,
    pub pad_side_margin_mm: f64,
    pub pad_end_margin_mm: f64,
}

impl Default for SupportSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sides: SupportSides::Both,
            output: SupportOutput::Merged,
            fin_count: 2,
            fin_height_mm: None,
            fin_height_fraction: 0.5,
            fin_base_length_mm: None,
            fin_thickness_mm: 1.2,
            fin_inset_mm: None,
            interface_gap_mm: 0.25,
            tab_count: 3,
            tab_height_mm: 1.2,
            tab_neck_height_mm: 0.5,
            tab_width_mm: None,
            tab_overlap_mm: 0.1,
            tab_margin_mm: 2.0,
            pad_enabled: true,
            pad_thickness_mm: 0.6,
            pad_side_margin_mm: 4.0,
            pad_end_margin_mm: 2.0,
        }
    }
}

/// Whether a curved lithophane bends toward or away from the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CurvatureDirection {
    /// Center of curvature is toward the viewer (`-Y`); edges curve toward viewer.
    #[default]
    Concave,
    /// Center of curvature is behind the lithophane (`+Y`); center bulges toward viewer.
    Convex,
}

/// Optional thin, filleted curved foot under a curved lithophane.
///
/// The base sits from `Z = 0` to `Z = thickness_mm` with the full lithophane panel rising
/// cleanly above it. It projects forward (`-Y` radial) and backward (`+Y` radial) by the
/// configured margins, and matches the panel's exact angular arc `[-theta/2, +theta/2]`
/// without projecting sideways.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CurvedBaseSettings {
    pub enabled: bool,
    pub thickness_mm: f64,
    pub front_projection_mm: f64,
    pub back_projection_mm: f64,
    pub corner_fillet_mm: f64,
}

impl Default for CurvedBaseSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            thickness_mm: 1.6,
            front_projection_mm: 8.0,
            back_projection_mm: 8.0,
            corner_fillet_mm: 4.0,
        }
    }
}

/// Settings for cylindrical-arc curved lithophanes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CurvedSettings {
    pub radius_mm: f64,
    pub direction: CurvatureDirection,
    pub orientation: OrientationMode,
    pub supports_enabled: bool,
    pub base: CurvedBaseSettings,
}

impl Default for CurvedSettings {
    fn default() -> Self {
        Self {
            radius_mm: 120.0,
            direction: CurvatureDirection::Concave,
            orientation: OrientationMode::ImageUp,
            supports_enabled: false,
            base: CurvedBaseSettings::default(),
        }
    }
}

/// How an image maps its dimension onto the shared bed width in multi-lithophane mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatchDimensionMode {
    #[default]
    LongSide,
    ShortSide,
    Width,
    Height,
}

/// Multi-lithophane settings: shared bed length, parallel spacing, and breakaway bridges.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct MultiSettings {
    pub common_dimension_mm: f64,
    pub panel_spacing_mm: f64,
    pub item_modes: Vec<MatchDimensionMode>,
    pub bridge_count: usize,
    pub bridge_width_mm: f64,
    pub bridge_height_mm: f64,
    pub bridge_overlap_mm: f64,
    pub bridge_inset_mm: Option<f64>,
    pub margin_z_mm: f64,
    pub add_center_bridge: bool,
}

impl Default for MultiSettings {
    fn default() -> Self {
        Self {
            common_dimension_mm: 150.0,
            panel_spacing_mm: 12.0,
            item_modes: vec![MatchDimensionMode::LongSide, MatchDimensionMode::LongSide],
            bridge_count: 3,
            bridge_width_mm: 2.0,
            bridge_height_mm: 1.2,
            bridge_overlap_mm: 0.2,
            bridge_inset_mm: None,
            margin_z_mm: 3.0,
            add_center_bridge: false,
        }
    }
}

/// Output 3D file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    #[default]
    Stl,
    #[serde(rename = "3mf")]
    ThreeMf,
}

/// Physical filament, backlight, and optical transmission parameters for 2D/3D previews.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LightingSettings {
    /// Filament sRGB hex colour (default `#f6f6f2`, white PLA).
    pub filament_color: String,
    /// HueForge-style Transmission Distance (TD) in mm where light drops to 1% (default `5.0`).
    pub transmission_distance_mm: f64,
    /// Backlight sRGB hex colour (default `#ffffff`, 6500K daylight).
    pub backlight_color: String,
    /// Relative backlight brightness multiplier.
    pub backlight_intensity: f64,
    /// Subtle front ambient light illuminating surface relief in shadows.
    pub ambient_front_light: f64,
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            filament_color: "#f6f6f2".to_string(),
            transmission_distance_mm: 5.0,
            backlight_color: "#ffffff".to_string(),
            backlight_intensity: 1.8,
            ambient_front_light: 0.025,
        }
    }
}

/// Parse `#RRGGBB` hex string into normalized sRGB `[0.0..1.0]` channels.
pub fn parse_hex_rgb(hex: &str) -> Result<[f64; 3], LithoError> {
    let trimmed = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
    if trimmed.len() != 6 || !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(LithoError::Settings(format!(
            "invalid hex colour {hex:?}; expected #RRGGBB"
        )));
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16)
        .map_err(|_| LithoError::Settings(format!("invalid hex colour {hex:?}")))?;
    let g = u8::from_str_radix(&trimmed[2..4], 16)
        .map_err(|_| LithoError::Settings(format!("invalid hex colour {hex:?}")))?;
    let b = u8::from_str_radix(&trimmed[4..6], 16)
        .map_err(|_| LithoError::Settings(format!("invalid hex colour {hex:?}")))?;
    Ok([r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0])
}

/// Convert a single sRGB channel `[0..1]` to linear light.
#[inline]
pub fn srgb_to_linear(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// File export options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub binary: bool,
    pub filename_template: String,
    pub pla_density_g_cm3: f64,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: ExportFormat::Stl,
            binary: true,
            filename_template: "{stem}_{w}x{h}mm".to_string(),
            pla_density_g_cm3: 1.24,
        }
    }
}

/// Root settings object shared across core, CLI, and desktop UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LithophaneSettings {
    pub size: SizeSettings,
    pub image: ImageSettings,
    pub thickness: ThicknessSettings,
    pub frame: FrameSettings,
    pub bottom_extension: BottomExtensionSettings,
    pub orientation: OrientationSettings,
    pub supports: SupportSettings,
    pub curved: CurvedSettings,
    pub multi: MultiSettings,
    pub lighting: LightingSettings,
    pub export: ExportSettings,
}

impl LithophaneSettings {
    /// Validate numeric invariants and field bounds.
    pub fn validate(&self) -> Result<(), LithoError> {
        if self.size.max_dimension_mm <= 1.0
            || self.size.width_mm <= 1.0
            || self.size.height_mm <= 1.0
        {
            return Err(LithoError::Settings(
                "size dimensions must be greater than 1.0 mm".into(),
            ));
        }
        if !(0.01..=5.0).contains(&self.size.pixel_size_mm) {
            return Err(LithoError::Settings(
                "size.pixel_size_mm must be between 0.01 and 5.0 mm".into(),
            ));
        }
        if let Some([l, t, r, b]) = self.image.crop {
            if !(0.0 <= l && l < r && r <= 1.0 && 0.0 <= t && t < b && b <= 1.0) {
                return Err(LithoError::Settings(
                    "image.crop must satisfy 0 <= left < right <= 1 and 0 <= top < bottom <= 1"
                        .into(),
                ));
            }
        }
        if !matches!(self.image.rotate, 0 | 90 | 180 | 270) {
            return Err(LithoError::Settings(
                "image.rotate must be 0, 90, 180, or 270".into(),
            ));
        }
        if !(0.0..=20.0).contains(&self.image.auto_levels_percent) {
            return Err(LithoError::Settings(
                "image.auto_levels_percent must be in 0..=20".into(),
            ));
        }
        if !(0.0 < self.image.clahe_clip && self.image.clahe_clip <= 1.0) {
            return Err(LithoError::Settings(
                "image.clahe_clip must be in (0, 1]".into(),
            ));
        }
        if !(-1.0..=1.0).contains(&self.image.brightness) {
            return Err(LithoError::Settings(
                "image.brightness must be in [-1, 1]".into(),
            ));
        }
        if !(0.0 < self.image.contrast && self.image.contrast <= 5.0) {
            return Err(LithoError::Settings(
                "image.contrast must be in (0, 5]".into(),
            ));
        }
        if !(0.0 < self.image.gamma && self.image.gamma <= 5.0) {
            return Err(LithoError::Settings(
                "image.gamma must be in (0, 5]".into(),
            ));
        }
        if !(0.0..=10.0).contains(&self.image.blur_sigma_mm) {
            return Err(LithoError::Settings(
                "image.blur_sigma_mm must be in [0, 10]".into(),
            ));
        }
        if !(0.0..=10.0).contains(&self.image.sharpen_amount) {
            return Err(LithoError::Settings(
                "image.sharpen_amount must be in [0, 10]".into(),
            ));
        }
        if !(0.0 < self.image.sharpen_radius_mm && self.image.sharpen_radius_mm <= 10.0) {
            return Err(LithoError::Settings(
                "image.sharpen_radius_mm must be in (0, 10]".into(),
            ));
        }
        if let Some(levels) = self.image.posterize_levels {
            if !(2..=256).contains(&levels) {
                return Err(LithoError::Settings(
                    "image.posterize_levels must be in 2..=256".into(),
                ));
            }
        }

        if self.thickness.min_thickness_mm <= 0.0 || self.thickness.max_thickness_mm <= 0.0 {
            return Err(LithoError::Settings(
                "thickness values must be positive".into(),
            ));
        }
        if self.thickness.max_thickness_mm <= self.thickness.min_thickness_mm {
            return Err(LithoError::Settings(
                "thickness.max_thickness_mm must be greater than min_thickness_mm".into(),
            ));
        }
        if !(0.0 < self.thickness.curve_gamma && self.thickness.curve_gamma <= 5.0) {
            return Err(LithoError::Settings(
                "thickness.curve_gamma must be in (0, 5]".into(),
            ));
        }
        if !(0.0 < self.thickness.attenuation_per_mm && self.thickness.attenuation_per_mm <= 10.0) {
            return Err(LithoError::Settings(
                "thickness.attenuation_per_mm must be in (0, 10]".into(),
            ));
        }

        if !(0.0 < self.frame.width_mm && self.frame.width_mm <= 50.0) {
            return Err(LithoError::Settings(
                "frame.width_mm must be in (0, 50]".into(),
            ));
        }
        if let Some(ft) = self.frame.thickness_mm {
            if ft <= 0.0 {
                return Err(LithoError::Settings(
                    "frame.thickness_mm must be positive".into(),
                ));
            }
        }

        if !(0.0..=100.0).contains(&self.bottom_extension.height_mm)
            || !(0.0 < self.bottom_extension.thickness_mm
                && self.bottom_extension.thickness_mm <= 50.0)
        {
            return Err(LithoError::Settings(
                "bottom_extension dimensions are out of valid bounds".into(),
            ));
        }

        if self.supports.fin_count == 0 || self.supports.fin_count > 20 {
            return Err(LithoError::Settings(
                "supports.fin_count must be in 1..=20".into(),
            ));
        }
        if self.supports.tab_count == 0 || self.supports.tab_count > 20 {
            return Err(LithoError::Settings(
                "supports.tab_count must be in 1..=20".into(),
            ));
        }
        if !(0.0 < self.supports.fin_height_fraction && self.supports.fin_height_fraction <= 1.0) {
            return Err(LithoError::Settings(
                "supports.fin_height_fraction must be in (0, 1]".into(),
            ));
        }
        if self.supports.fin_thickness_mm <= 0.0
            || self.supports.interface_gap_mm < 0.05
            || self.supports.tab_height_mm <= 0.0
            || self.supports.tab_neck_height_mm <= 0.0
            || self.supports.tab_overlap_mm < 0.0
            || self.supports.tab_margin_mm < 0.0
            || self.supports.pad_thickness_mm <= 0.0
            || self.supports.pad_side_margin_mm < 0.0
            || self.supports.pad_end_margin_mm < 0.0
        {
            return Err(LithoError::Settings(
                "supports geometry parameters must be valid positive bounds".into(),
            ));
        }

        if self.curved.radius_mm <= 5.0 {
            return Err(LithoError::Settings(
                "curved.radius_mm must be greater than 5.0 mm".into(),
            ));
        }
        if !(0.2..=20.0).contains(&self.curved.base.thickness_mm) {
            return Err(LithoError::Settings(
                "curved.base.thickness_mm must be in [0.2, 20.0]".into(),
            ));
        }
        if !(0.0..=80.0).contains(&self.curved.base.front_projection_mm)
            || !(0.0..=80.0).contains(&self.curved.base.back_projection_mm)
        {
            return Err(LithoError::Settings(
                "curved.base projections must be in [0.0, 80.0]".into(),
            ));
        }
        if !(0.0..=40.0).contains(&self.curved.base.corner_fillet_mm) {
            return Err(LithoError::Settings(
                "curved.base.corner_fillet_mm must be in [0.0, 40.0]".into(),
            ));
        }

        if self.multi.common_dimension_mm <= 1.0
            || !(2.0..=100.0).contains(&self.multi.panel_spacing_mm)
            || self.multi.bridge_count == 0
            || self.multi.bridge_count > 20
            || self.multi.bridge_width_mm <= 0.0
            || self.multi.bridge_height_mm <= 0.0
            || self.multi.bridge_overlap_mm < 0.0
            || self.multi.margin_z_mm < 0.0
        {
            return Err(LithoError::Settings(
                "multi settings are out of valid bounds".into(),
            ));
        }

        if self.export.pla_density_g_cm3 <= 0.0 {
            return Err(LithoError::Settings(
                "export.pla_density_g_cm3 must be positive".into(),
            ));
        }

        parse_hex_rgb(&self.lighting.filament_color)?;
        parse_hex_rgb(&self.lighting.backlight_color)?;
        if !(0.2..=30.0).contains(&self.lighting.transmission_distance_mm) {
            return Err(LithoError::Settings(
                "lighting.transmission_distance_mm must be in [0.2, 30.0]".into(),
            ));
        }
        if !(0.1..=5.0).contains(&self.lighting.backlight_intensity) {
            return Err(LithoError::Settings(
                "lighting.backlight_intensity must be in [0.1, 5.0]".into(),
            ));
        }
        if !(0.0..=0.5).contains(&self.lighting.ambient_front_light) {
            return Err(LithoError::Settings(
                "lighting.ambient_front_light must be in [0.0, 0.5]".into(),
            ));
        }

        Ok(())
    }

    /// Apply `group.field=value` (or nested `curved.base.field=value`) overrides.
    ///
    /// Values parse as JSON first, falling back to plain JSON strings
    /// (so `--set image.equalize=clahe` works without extra quoting).
    pub fn apply_overrides(&self, overrides: &[String]) -> Result<Self, LithoError> {
        let mut root = serde_json::to_value(self)
            .map_err(|e| LithoError::Settings(format!("failed to serialize settings: {e}")))?;

        for item in overrides {
            let Some((raw_key, raw_val)) = item.split_once('=') else {
                return Err(LithoError::Settings(format!(
                    "override must look like group.field=value, got {item:?}"
                )));
            };
            let key = raw_key.trim();
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() < 2 {
                return Err(LithoError::Settings(format!(
                    "override key must include group and field (e.g. size.max_dimension_mm), got {key:?}"
                )));
            }

            let parsed_val: Value = match serde_json::from_str(raw_val) {
                Ok(v) => v,
                Err(_) => Value::String(raw_val.to_string()),
            };

            let mut cursor = &mut root;
            for group in &parts[..parts.len() - 1] {
                let obj = cursor.as_object_mut().ok_or_else(|| {
                    LithoError::Settings(format!("cannot traverse non-object at {group:?}"))
                })?;
                cursor = obj.get_mut(*group).ok_or_else(|| {
                    LithoError::Settings(format!("unknown settings group {group:?} in {key:?}"))
                })?;
            }
            let leaf = parts[parts.len() - 1];
            let obj = cursor.as_object_mut().ok_or_else(|| {
                LithoError::Settings(format!("target group for {key:?} is not an object"))
            })?;
            if !obj.contains_key(leaf) {
                return Err(LithoError::Settings(format!("unknown setting {key:?}")));
            }
            obj.insert(leaf.to_string(), parsed_val);
        }

        let updated: Self = serde_json::from_value(root)
            .map_err(|e| LithoError::Settings(format!("invalid override value: {e}")))?;
        updated.validate()?;
        Ok(updated)
    }

    /// Format the configured filename stem using outer printed dimensions.
    pub fn format_stem(&self, source_stem: &str, width_x: f64, height_z: f64, style: StyleId) -> String {
        let w_round = format!("{:.0}", width_x);
        let h_round = format!("{:.0}", height_z);
        self.export
            .filename_template
            .replace("{stem}", source_stem)
            .replace("{w:.0f}", &w_round)
            .replace("{h:.0f}", &h_round)
            .replace("{w}", &w_round)
            .replace("{h}", &h_round)
            .replace("{style}", style.as_str())
    }
}

/// Bundled preset names.
pub fn list_presets() -> Vec<&'static str> {
    vec![
        "default",
        "fast_draft",
        "fine_detail",
        "transmission_photo",
        "curved_stand",
    ]
}

/// Metadata + recommended default style for a bundled preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledPreset {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub recommended_style: StyleId,
    pub settings: LithophaneSettings,
}

/// Return all bundled presets with metadata.
pub fn bundled_presets() -> Vec<BundledPreset> {
    list_presets()
        .into_iter()
        .filter_map(|name| load_preset_with_style(name).ok())
        .collect()
}

/// Load a bundled preset along with its recommended style.
pub fn load_preset_with_style(name: &str) -> Result<BundledPreset, LithoError> {
    let mut s = LithophaneSettings::default();
    match name {
        "default" => Ok(BundledPreset {
            name: "default",
            label: "Standard (Balanced)",
            description: "0.25 mm detail, linear mapping, 0.6-3.6 mm range.",
            recommended_style: StyleId::Flat,
            settings: s,
        }),
        "fast_draft" => {
            s.size.pixel_size_mm = 0.5;
            Ok(BundledPreset {
                name: "fast_draft",
                label: "Fast Draft",
                description: "Coarse 0.50 mm pitch for rapid test prints.",
                recommended_style: StyleId::Flat,
                settings: s,
            })
        }
        "fine_detail" => {
            s.size.pixel_size_mm = 0.1;
            Ok(BundledPreset {
                name: "fine_detail",
                label: "Fine Detail",
                description: "High-resolution 0.10 mm mesh pitch for crisp portraits.",
                recommended_style: StyleId::Flat,
                settings: s,
            })
        }
        "transmission_photo" => {
            s.image.auto_levels_percent = 1.0;
            s.image.blur_sigma_mm = 0.15;
            s.thickness.curve = ThicknessCurve::Transmission;
            Ok(BundledPreset {
                name: "transmission_photo",
                label: "Transmission Photo",
                description: "Beer-Lambert optical compensation with gentle denoising and auto-levels.",
                recommended_style: StyleId::Flat,
                settings: s,
            })
        }
        "curved_stand" => {
            s.image.auto_levels_percent = 0.5;
            s.thickness.curve = ThicknessCurve::Transmission;
            s.curved.radius_mm = 110.0;
            s.curved.direction = CurvatureDirection::Concave;
            s.curved.base.enabled = true;
            s.curved.base.thickness_mm = 1.6;
            s.curved.base.front_projection_mm = 8.0;
            s.curved.base.back_projection_mm = 8.0;
            s.curved.base.corner_fillet_mm = 4.5;
            Ok(BundledPreset {
                name: "curved_stand",
                label: "Curved Desk Stand",
                description: "Self-standing concave curved lithophane with 1.6 mm filleted base.",
                recommended_style: StyleId::Curved,
                settings: s,
            })
        }
        _ => Err(LithoError::Settings(format!(
            "unknown bundled preset {name:?}. Available: {}",
            list_presets().join(", ")
        ))),
    }
}

/// Load a bundled preset's settings by name.
pub fn load_preset(name: &str) -> Result<LithophaneSettings, LithoError> {
    Ok(load_preset_with_style(name)?.settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_update_nested_and_flat_fields() {
        let s = LithophaneSettings::default();
        let updated = s
            .apply_overrides(&[
                "size.max_dimension_mm=180".to_string(),
                "image.equalize=clahe".to_string(),
                "curved.base.thickness_mm=2.2".to_string(),
            ])
            .expect("overrides should apply");
        assert!((updated.size.max_dimension_mm - 180.0).abs() < 1e-9);
        assert_eq!(updated.image.equalize, EqualizeMode::Clahe);
        assert!((updated.curved.base.thickness_mm - 2.2).abs() < 1e-9);
    }

    #[test]
    fn presets_all_validate() {
        for name in list_presets() {
            let s = load_preset(name).expect("preset loads");
            s.validate().expect("preset validates");
        }
    }

    #[test]
    fn defaults_and_lighting_backwards_compat() {
        let s = LithophaneSettings::default();
        assert!((s.image.blur_sigma_mm - 0.1).abs() < 1e-6);
        assert_eq!(s.lighting.filament_color, "#f6f6f2");
        assert!((s.lighting.transmission_distance_mm - 5.0).abs() < 1e-9);
        assert_eq!(s.lighting.backlight_color, "#ffffff");

        // Legacy preset JSON omitting `lighting` still deserializes cleanly
        let mut json = serde_json::to_value(&s).unwrap();
        json.as_object_mut().unwrap().remove("lighting");
        let parsed: LithophaneSettings = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.lighting, LightingSettings::default());
        assert!(parse_hex_rgb("invalid").is_err());
    }
}
