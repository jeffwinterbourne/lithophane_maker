//! Modular scene, part, material, input, and report definitions.
//!
//! Every [`crate::styles::LithophaneStyle`] returns a [`GeneratedScene`], keeping
//! individual geometry components (`Panel`, `Base`, `Support`, `Bridge`, and future
//! `ColorLayer` / `Structural` parts) cleanly separated for 3D shader rendering and
//! multi-material 3MF export while also providing the final watertight combined mesh.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::LithoError;
use crate::mesh::{write_3mf, write_stl_ascii, write_stl_binary, MeshData};
use crate::settings::{
    CurvatureDirection, ExportFormat, LithophaneSettings, MatchDimensionMode, StyleId,
    SupportOutput,
};

/// Input image source passed to the core library.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// Path to an image on disk.
    Path(PathBuf),
    /// In-memory encoded image bytes (PNG, JPEG, WebP, etc.) with a display stem.
    Bytes { bytes: Vec<u8>, stem: String },
}

impl ImageSource {
    /// Base stem used when naming exported files and reports.
    pub fn stem(&self) -> String {
        match self {
            Self::Path(p) => p
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("lithophane")
                .to_string(),
            Self::Bytes { stem, .. } => {
                if stem.is_empty() {
                    "lithophane".to_string()
                } else {
                    stem.clone()
                }
            }
        }
    }
}

/// One input image entry with optional per-item layout parameters.
#[derive(Debug, Clone)]
pub struct ImageInput {
    pub source: ImageSource,
    /// Optional per-image bed-dimension mapping (used by `MultiFlat` and future lamp cylinders).
    pub mapping_mode: Option<MatchDimensionMode>,
}

impl ImageInput {
    /// Wrap a path with default mapping options.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ImageSource::Path(path.into()),
            mapping_mode: None,
        }
    }

    /// Wrap raw encoded image bytes with default mapping options.
    pub fn from_bytes(bytes: Vec<u8>, stem: impl Into<String>) -> Self {
        Self {
            source: ImageSource::Bytes {
                bytes,
                stem: stem.into(),
            },
            mapping_mode: None,
        }
    }
}

/// Semantic role of a mesh part inside a [`GeneratedScene`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartRole {
    /// The lithophane relief panel itself (carries per-vertex thickness for backlit preview).
    Panel,
    /// Built-in stand or base foot.
    Base,
    /// Breakaway support fins, pads, and tabs.
    Support,
    /// Breakaway inter-panel bridges in multi-panel styles.
    Bridge,
    /// Future CMY / colour sublayer for multi-material flat colour lithophanes.
    ColorLayer,
    /// Future structural collars, lamp top/bottom rings, or dome frames.
    Structural,
}

/// Material metadata for 3D preview and 3MF `<basematerials>` export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaterialSpec {
    pub name: String,
    pub rgba: [u8; 4],
}

impl MaterialSpec {
    /// Standard translucent white/ivory lithophane PLA.
    pub fn ivory_pla() -> Self {
        Self {
            name: "Ivory Litho PLA".into(),
            rgba: [246, 241, 232, 255],
        }
    }

    /// Built-in base material.
    pub fn alabaster_base() -> Self {
        Self {
            name: "Stand Base PLA".into(),
            rgba: [228, 222, 212, 255],
        }
    }

    /// Breakaway support fin & bridge material.
    pub fn support_amber() -> Self {
        Self {
            name: "Breakaway Support".into(),
            rgba: [217, 119, 6, 255],
        }
    }
}

/// An individual solid component within a generated scene.
#[derive(Debug, Clone)]
pub struct ScenePart {
    pub id: String,
    pub name: String,
    pub role: PartRole,
    pub material: MaterialSpec,
    pub mesh: MeshData,
}

/// Options controlling speed vs. final manifold union behaviour.
#[derive(Debug, Clone, Default)]
pub struct GenerationOptions {
    /// If set, enforces `pixel_size_mm >= preview_pixel_size_mm` for fast interactive preview.
    pub preview_pixel_size_mm: Option<f64>,
    /// If true, skips expensive boolean unions for interactive 3D preview (still builds every part).
    pub skip_final_boolean_merge: bool,
}

/// 2D diagnostic PNG previews (upright image frame) for one input image.
#[derive(Debug, Clone)]
pub struct ImagePreviews {
    pub stem: String,
    pub luminance_png: Vec<u8>,
    pub heightmap_png: Vec<u8>,
    pub backlit_png: Vec<u8>,
    /// Cached upright thickness grid `(rows × cols)` for fast lighting-only 2D refreshes.
    pub thickness_grid: Vec<f32>,
    pub grid_rows: usize,
    pub grid_cols: usize,
}

/// 3D axis extent in millimetres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionsMm {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Grid spacing and vertex dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridReportInfo {
    pub columns: usize,
    pub rows: usize,
    pub dx_mm: f64,
    pub dz_mm: f64,
}

/// Min/max thickness summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThicknessRangeReport {
    pub min: f64,
    pub max: f64,
}

/// Curved geometry summary reported to UI & CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvedReportInfo {
    pub radius_mm: f64,
    pub arc_angle_deg: f64,
    pub chord_width_mm: f64,
    pub arc_depth_mm: f64,
    pub direction: CurvatureDirection,
    pub base_enabled: bool,
    pub base_thickness_mm: Option<f64>,
    pub base_effective_fillet_in_mm: Option<f64>,
    pub base_effective_fillet_out_mm: Option<f64>,
}

/// Support plan summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportReportInfo {
    pub fin_height_mm: f64,
    pub fin_base_length_mm: f64,
    pub fin_centres_x_mm: Vec<f64>,
    pub sides: String,
}

/// Bridge connector summary for multi-lithophane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeReportInfo {
    pub stations_x_mm: Vec<f64>,
    pub bridges_per_station: usize,
    pub total_bridges: usize,
}

/// Per-panel summary inside multi-lithophane runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelReportInfo {
    pub index: usize,
    pub source_stem: String,
    pub mode: MatchDimensionMode,
    pub dimensions_mm: DimensionsMm,
    pub y_ref_mm: f64,
    pub triangles: usize,
}

/// Structured generation report returned alongside meshes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub style: StyleId,
    pub dimensions_mm: DimensionsMm,
    pub overall_dimensions_mm: DimensionsMm,
    pub grid: Option<GridReportInfo>,
    pub rotated_for_print: bool,
    pub thickness_mm: ThicknessRangeReport,
    pub triangles: usize,
    pub volume_mm3: f64,
    pub filament_g_estimate: f64,
    pub bodies: Option<usize>,
    pub curved_info: Option<CurvedReportInfo>,
    pub supports: Option<SupportReportInfo>,
    pub bridges: Option<BridgeReportInfo>,
    pub panels: Vec<PanelReportInfo>,
    pub warnings: Vec<String>,
}

/// Complete output of a generation run across any [`crate::styles::LithophaneStyle`].
#[derive(Debug, Clone)]
pub struct GeneratedScene {
    pub style: StyleId,
    pub settings: LithophaneSettings,
    pub parts: Vec<ScenePart>,
    /// Merged / combined printable mesh when generated (populated on full export runs).
    pub combined: Option<MeshData>,
    pub previews: Vec<ImagePreviews>,
    pub report: GenerationReport,
    pub source_stems: Vec<String>,
}

impl GeneratedScene {
    /// Formatted base stem for output files.
    pub fn stem(&self) -> String {
        let joined = if self.source_stems.is_empty() {
            "lithophane".to_string()
        } else {
            self.source_stems.join("_")
        };
        self.settings.format_stem(
            &joined,
            self.report.overall_dimensions_mm.x,
            self.report.overall_dimensions_mm.z,
            self.style,
        )
    }

    /// Save the generated scene to a file path or directory.
    ///
    /// Supports `.stl` (binary or ASCII) and `.3mf` (multi-part OPC ZIP with materials).
    /// When `supports.output == SeparateFiles` and `format == Stl`, writes one file per role/panel.
    pub fn save_to_path(&self, target: &Path) -> Result<Vec<PathBuf>, LithoError> {
        let ext = target
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        let is_explicit_file = matches!(ext.as_deref(), Some("stl") | Some("3mf"));
        let format = match ext.as_deref() {
            Some("3mf") => ExportFormat::ThreeMf,
            Some("stl") => ExportFormat::Stl,
            _ => self.settings.export.format,
        };

        let (out_dir, base_stem) = if is_explicit_file {
            let parent = target
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let stem = target
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("lithophane")
                .to_string();
            (parent.to_path_buf(), stem)
        } else {
            (target.to_path_buf(), self.stem())
        };

        std::fs::create_dir_all(&out_dir)?;

        match format {
            ExportFormat::ThreeMf => {
                let path = out_dir.join(format!("{base_stem}.3mf"));
                let file = File::create(&path)?;
                let writer = BufWriter::new(file);
                write_3mf(&self.parts, writer)?;
                Ok(vec![path])
            }
            ExportFormat::Stl => {
                let binary = self.settings.export.binary;
                let has_supports = self
                    .parts
                    .iter()
                    .any(|p| matches!(p.role, PartRole::Support | PartRole::Bridge));

                if self.settings.supports.output == SupportOutput::SeparateFiles
                    && (has_supports || self.parts.len() > 1)
                {
                    let mut paths = Vec::new();
                    let panel_parts: Vec<&ScenePart> = self
                        .parts
                        .iter()
                        .filter(|p| !matches!(p.role, PartRole::Support | PartRole::Bridge))
                        .collect();
                    let support_parts: Vec<&ScenePart> = self
                        .parts
                        .iter()
                        .filter(|p| matches!(p.role, PartRole::Support | PartRole::Bridge))
                        .collect();

                    if panel_parts.len() == 1 {
                        let p = out_dir.join(format!("{base_stem}_panel.stl"));
                        write_stl_file(&panel_parts[0].mesh, &p, binary)?;
                        paths.push(p);
                    } else {
                        for part in panel_parts {
                            let p = out_dir.join(format!("{base_stem}_{}.stl", part.id));
                            write_stl_file(&part.mesh, &p, binary)?;
                            paths.push(p);
                        }
                    }

                    if !support_parts.is_empty() {
                        let sup_mesh = if support_parts.len() == 1 {
                            support_parts[0].mesh.clone()
                        } else {
                            MeshData::concatenate(
                                &support_parts.iter().map(|s| &s.mesh).collect::<Vec<_>>(),
                            )
                        };
                        let p = out_dir.join(format!("{base_stem}_supports.stl"));
                        write_stl_file(&sup_mesh, &p, binary)?;
                        paths.push(p);
                    }
                    return Ok(paths);
                }

                let mesh = self.combined.as_ref().ok_or_else(|| {
                    LithoError::Export(
                        "combined mesh not present; generate with skip_final_boolean_merge=false before STL export"
                            .into(),
                    )
                })?;
                let path = out_dir.join(format!("{base_stem}.stl"));
                write_stl_file(mesh, &path, binary)?;
                Ok(vec![path])
            }
        }
    }
}

fn write_stl_file(mesh: &MeshData, path: &Path, binary: bool) -> Result<(), LithoError> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    if binary {
        write_stl_binary(mesh, writer)
    } else {
        write_stl_ascii(mesh, writer)
    }
}
