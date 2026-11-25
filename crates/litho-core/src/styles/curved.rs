//! Cylindrical-arc curved lithophane style with optional filleted built-in base
//! and optional radial breakaway support fins.

use std::f64::consts::{FRAC_PI_2, PI};

use manifold_rust::linalg::Vec2;
use manifold_rust::manifold::Manifold;
use manifold_rust::types::Polygons;

use crate::error::LithoError;
use crate::heightmap::{build_thickness_map, ThicknessMap};
use crate::image_proc::process_image;
use crate::mesh::{
    compose_manifolds, manifold_to_mesh, mesh_to_manifold, union_manifolds, validate_manifold,
    MeshData,
};
use crate::preview::image_frame_previews;
use crate::scene::{
    CurvedReportInfo, DimensionsMm, GeneratedScene, GenerationOptions, GenerationReport,
    GridReportInfo, ImageInput, MaterialSpec, PartRole, ScenePart, SupportReportInfo,
    ThicknessRangeReport,
};
use crate::settings::{
    CurvatureDirection, CurvedBaseSettings, LithophaneSettings, ReliefSide, StyleId, SupportOutput,
};
use crate::styles::flat::{round2, round3, round4};
use crate::styles::{LithophaneStyle, StyleDescriptor};
use crate::supports::build_curved_supports;

const MAX_ARC_DEGREES: f64 = 340.0;
const MIN_SAFE_RADIUS_MM: f64 = 2.0;
const BASE_UNION_SKIRT_MM: f64 = 0.05;

/// Curved cylindrical-arc lithophane style.
pub struct CurvedStyle;

impl LithophaneStyle for CurvedStyle {
    fn descriptor(&self) -> StyleDescriptor {
        StyleDescriptor {
            id: StyleId::Curved,
            name: "Curved",
            tagline: "Cylindrical arc lithophane with optional filleted stand base",
            min_images: 1,
            max_images: Some(1),
        }
    }

    fn validate(
        &self,
        inputs: &[ImageInput],
        settings: &LithophaneSettings,
    ) -> Result<(), LithoError> {
        settings.validate()?;
        if inputs.len() != 1 {
            return Err(LithoError::Settings(format!(
                "curved style requires exactly 1 image, got {}",
                inputs.len()
            )));
        }
        Ok(())
    }

    fn generate(
        &self,
        inputs: &[ImageInput],
        settings: &LithophaneSettings,
        opts: &GenerationOptions,
        progress: &mut dyn FnMut(&str, f32),
    ) -> Result<GeneratedScene, LithoError> {
        self.validate(inputs, settings)?;

        let mut eff_settings = settings.clone();
        if let Some(preview_px) = opts.preview_pixel_size_mm {
            eff_settings.size.pixel_size_mm = eff_settings.size.pixel_size_mm.max(preview_px);
        }

        let input = &inputs[0];
        let stem = input.source.stem();

        progress("Processing image", 0.10);
        let (lum, plan) = process_image(&input.source, &eff_settings)?;
        let tm = build_thickness_map(lum, plan, &eff_settings, eff_settings.curved.orientation);

        let curved = eff_settings.curved.clone();
        let base = curved.base.clone();
        let length_x = tm.length_x();
        let radius = curved.radius_mm;
        let theta = length_x / radius;
        let max_theta = MAX_ARC_DEGREES.to_radians();
        if theta > max_theta {
            let min_r = length_x / max_theta;
            return Err(LithoError::Geometry(format!(
                "curved radius {radius:.1} mm wraps {:.1}° (exceeds {MAX_ARC_DEGREES:.0}° max). Minimum radius for a {length_x:.1} mm arc is {min_r:.1} mm.",
                theta.to_degrees()
            )));
        }

        let max_t = tm.max_thickness();
        let (r_panel_min, _r_panel_max) =
            panel_radial_bounds(radius, max_t, curved.direction, eff_settings.thickness.relief_side);
        if r_panel_min < MIN_SAFE_RADIUS_MM {
            return Err(LithoError::Geometry(format!(
                "curved radius {radius:.1} mm leaves inner panel radius {r_panel_min:.2} mm (< {MIN_SAFE_RADIUS_MM:.1} mm)."
            )));
        }

        let z_offset = if base.enabled { base.thickness_mm } else { 0.0 };
        let skirt = if base.enabled {
            BASE_UNION_SKIRT_MM.min(base.thickness_mm * 0.5)
        } else {
            0.0
        };

        progress("Building curved panel", 0.35);
        let panel_mesh_union = build_curved_panel_mesh(
            &tm,
            radius,
            curved.direction,
            eff_settings.thickness.relief_side,
            z_offset,
            skirt,
        );
        // Keep clean non-skirt panel mesh for preview shading if base is enabled
        let panel_mesh_display = if skirt > 0.0 && opts.skip_final_boolean_merge {
            build_curved_panel_mesh(
                &tm,
                radius,
                curved.direction,
                eff_settings.thickness.relief_side,
                z_offset,
                0.0,
            )
        } else {
            panel_mesh_union.clone()
        };

        let preview_2d = image_frame_previews(&stem, &tm, &eff_settings)?;

        let mut parts = vec![ScenePart {
            id: "panel".into(),
            name: format!("{stem} Curved Panel"),
            role: PartRole::Panel,
            material: MaterialSpec::ivory_pla(),
            mesh: panel_mesh_display,
        }];

        let mut base_manifold: Option<Manifold> = None;
        let mut eff_fillet_in: Option<f64> = None;
        let mut eff_fillet_out: Option<f64> = None;
        let mut warnings = Vec::new();

        if base.enabled {
            progress("Building filleted curved base", 0.52);
            let (bm, f_in, f_out, base_warn) = build_curved_base_manifold(
                &tm,
                radius,
                curved.direction,
                eff_settings.thickness.relief_side,
                &base,
            )?;
            warnings.extend(base_warn);
            eff_fillet_in = Some(round2(f_in));
            eff_fillet_out = Some(round2(f_out));
            let base_mesh = manifold_to_mesh(&bm);
            base_manifold = Some(bm);
            parts.push(ScenePart {
                id: "base".into(),
                name: "Curved Stand Base".into(),
                role: PartRole::Base,
                material: MaterialSpec::alabaster_base(),
                mesh: base_mesh,
            });
        }

        let mut sup_report = None;
        let mut sup_manifold: Option<Manifold> = None;
        if curved.supports_enabled {
            progress("Building radial supports", 0.65);
            let (fp, bp) = if base.enabled {
                (base.front_projection_mm, base.back_projection_mm)
            } else {
                (0.0, 0.0)
            };
            let (sm, sup_mesh, sup_plan) = build_curved_supports(
                &tm,
                &eff_settings,
                radius,
                curved.direction,
                z_offset,
                fp,
                bp,
            )?;
            warnings.extend(sup_plan.warnings);
            sup_report = Some(SupportReportInfo {
                fin_height_mm: round2(sup_plan.fin_height),
                fin_base_length_mm: round2(sup_plan.fin_base_length),
                fin_centres_x_mm: sup_plan.fin_centres_x.iter().map(|&x| round2(x)).collect(),
                sides: format!("{:?}", eff_settings.supports.sides).to_lowercase(),
            });
            sup_manifold = Some(sm);
            parts.push(ScenePart {
                id: "supports".into(),
                name: "Radial Supports".into(),
                role: PartRole::Support,
                material: MaterialSpec::support_amber(),
                mesh: sup_mesh,
            });
        }

        let (combined_mesh, volume_mm3, bodies) = if opts.skip_final_boolean_merge {
            let refs: Vec<&MeshData> = parts.iter().map(|p| &p.mesh).collect();
            let cat = MeshData::concatenate(&refs);
            let approx_vol = cat.signed_volume().max(0.0);
            (None, approx_vol, None)
        } else {
            progress("Validating & joining manifold", 0.80);
            let panel_manifold = mesh_to_manifold(&panel_mesh_union, "curved panel")?;
            let panel_plus_base = if let Some(ref bm) = base_manifold {
                union_manifolds(&[panel_manifold, bm.clone()], "curved panel + base")?
            } else {
                panel_manifold
            };

            let final_manifold = match (&sup_manifold, eff_settings.supports.output) {
                (Some(sm), SupportOutput::Merged) => {
                    union_manifolds(&[panel_plus_base, sm.clone()], "merged curved model")?
                }
                (Some(sm), SupportOutput::SeparateBodies | SupportOutput::SeparateFiles) => {
                    compose_manifolds(&[panel_plus_base, sm.clone()])
                }
                (None, _) => panel_plus_base,
            };

            let vol = final_manifold.volume();
            let body_count = if final_manifold.num_tri() < 400_000 {
                let b = final_manifold.decompose().len();
                if eff_settings.supports.output == SupportOutput::Merged && b != 1 {
                    warnings.push(format!("Merged curved model has {b} bodies; expected 1."));
                }
                Some(b)
            } else {
                None
            };
            (Some(manifold_to_mesh(&final_manifold)), vol, body_count)
        };

        let all_refs: Vec<&MeshData> = parts.iter().map(|p| &p.mesh).collect();
        let (bmin, bmax) = if let Some(ref c) = combined_mesh {
            c.bounds()
        } else {
            MeshData::concatenate(&all_refs).bounds()
        };
        let triangles = if let Some(ref c) = combined_mesh {
            c.indices.len()
        } else {
            parts.iter().map(|p| p.mesh.indices.len()).sum()
        };
        if triangles > 5_000_000 {
            warnings.push(format!(
                "{triangles} triangles: very large file; consider a larger pixel size."
            ));
        }

        let half_theta = theta * 0.5;
        let chord_width = 2.0 * radius * half_theta.sin();
        let sagitta = radius * (1.0 - half_theta.cos());
        let filament_g = (volume_mm3 / 1000.0) * eff_settings.export.pla_density_g_cm3;
        let min_t = tm.min_thickness();

        progress("Done", 1.0);
        Ok(GeneratedScene {
            style: StyleId::Curved,
            settings: eff_settings,
            parts,
            combined: combined_mesh,
            previews: vec![preview_2d],
            report: GenerationReport {
                style: StyleId::Curved,
                dimensions_mm: DimensionsMm {
                    x: round3(length_x),
                    y: round3(max_t),
                    z: round3(tm.height_z()),
                },
                overall_dimensions_mm: DimensionsMm {
                    x: round3(bmax[0] - bmin[0]),
                    y: round3(bmax[1] - bmin[1]),
                    z: round3(bmax[2] - bmin[2]),
                },
                grid: Some(GridReportInfo {
                    columns: tm.cols,
                    rows: tm.rows,
                    dx_mm: round4(tm.dx),
                    dz_mm: round4(tm.dz),
                }),
                rotated_for_print: tm.rot_k != 0,
                thickness_mm: ThicknessRangeReport {
                    min: round4(min_t),
                    max: round4(max_t),
                },
                triangles,
                volume_mm3: round2(volume_mm3),
                filament_g_estimate: round2(filament_g),
                bodies,
                curved_info: Some(CurvedReportInfo {
                    radius_mm: round2(radius),
                    arc_angle_deg: round2(theta.to_degrees()),
                    chord_width_mm: round2(chord_width),
                    arc_depth_mm: round2(sagitta),
                    direction: curved.direction,
                    base_enabled: base.enabled,
                    base_thickness_mm: if base.enabled {
                        Some(round2(base.thickness_mm))
                    } else {
                        None
                    },
                    base_effective_fillet_in_mm: eff_fillet_in,
                    base_effective_fillet_out_mm: eff_fillet_out,
                }),
                supports: sup_report,
                bridges: None,
                panels: Vec::new(),
                warnings,
            },
            source_stems: vec![stem],
        })
    }
}

/// Minimum and maximum radii `(r_min, r_max)` occupied by the curved lithophane panel.
pub fn panel_radial_bounds(
    radius: f64,
    max_t: f64,
    direction: CurvatureDirection,
    relief_side: ReliefSide,
) -> (f64, f64) {
    let relief_smaller = matches!(
        (direction, relief_side),
        (CurvatureDirection::Concave, ReliefSide::Front)
            | (CurvatureDirection::Convex, ReliefSide::Back)
    );
    if relief_smaller {
        (radius - max_t, radius)
    } else {
        (radius, radius + max_t)
    }
}

/// Convert polar `(r, phi)` around the curvature center into printed world `(x, y)` mm.
#[inline]
pub fn polar_to_xy(r: f64, phi: f64, radius: f64, direction: CurvatureDirection) -> [f64; 2] {
    match direction {
        CurvatureDirection::Concave => [r * phi.sin(), r * phi.cos() - radius],
        CurvatureDirection::Convex => [r * phi.sin(), radius - r * phi.cos()],
    }
}

/// Build the watertight cylindrical-arc panel shell with zero T-junctions.
///
/// The smooth back/front cylinder uses only boundary vertices plus 2 vertices per interior
/// column (top/bottom) along straight cylinder rulings, while side caps and relief grid
/// share exact 1-to-1 perimeter edges. When `bottom_skirt_mm > 0`, adds one extra vertical
/// row below `z_offset_mm` so the solid embeds cleanly into the built-in base.
pub fn build_curved_panel_mesh(
    tm: &ThicknessMap,
    radius: f64,
    direction: CurvatureDirection,
    relief_side: ReliefSide,
    z_offset_mm: f64,
    bottom_skirt_mm: f64,
) -> MeshData {
    let img_rows = tm.rows;
    let cols = tm.cols;
    let has_skirt = bottom_skirt_mm > 0.0;
    let rows = if has_skirt { img_rows + 1 } else { img_rows };
    let length_x = tm.length_x();
    let hz = tm.height_z();

    let relief_smaller = matches!(
        (direction, relief_side),
        (CurvatureDirection::Concave, ReliefSide::Front)
            | (CurvatureDirection::Convex, ReliefSide::Back)
    );

    let phis: Vec<f64> = (0..cols)
        .map(|j| (j as f64 * tm.dx - length_x * 0.5) / radius)
        .collect();

    let row_z = |i: usize| -> f64 {
        if i < img_rows {
            z_offset_mm + (img_rows - 1 - i) as f64 * tm.dz
        } else {
            z_offset_mm - bottom_skirt_mm
        }
    };
    let row_t = |i: usize, j: usize| -> f64 {
        let src_r = i.min(img_rows - 1);
        tm.get(src_r, j)
    };
    let relief_r = |t: f64| -> f64 {
        if relief_smaller {
            radius - t
        } else {
            radius + t
        }
    };

    let mut positions = Vec::with_capacity(rows * cols + 2 * rows + 2 * cols);
    let mut thickness = Vec::with_capacity(rows * cols + 2 * rows + 2 * cols);

    // 1. Relief grid vertices (0 .. rows * cols)
    for i in 0..rows {
        let z = row_z(i);
        for (j, &phi) in phis.iter().enumerate() {
            let t = row_t(i, j);
            let r = relief_r(t);
            let [x, y] = polar_to_xy(r, phi, radius, direction);
            positions.push([x, y, z]);
            thickness.push(t as f32);
        }
    }

    // 2. Smooth cylinder vertices (at r = radius)
    // Left column (rows vertices, top->bottom)
    let smooth_left_start = positions.len() as u32;
    for i in 0..rows {
        let [x, y] = polar_to_xy(radius, phis[0], radius, direction);
        positions.push([x, y, row_z(i)]);
        thickness.push(row_t(i, 0) as f32);
    }
    // Right column (rows vertices, top->bottom)
    let smooth_right_start = positions.len() as u32;
    for i in 0..rows {
        let [x, y] = polar_to_xy(radius, phis[cols - 1], radius, direction);
        positions.push([x, y, row_z(i)]);
        thickness.push(row_t(i, cols - 1) as f32);
    }
    // Interior top & bottom vertices (j = 1 .. cols-2)
    let smooth_int_top_start = positions.len() as u32;
    for (j, &phi) in phis.iter().enumerate().take(cols - 1).skip(1) {
        let [x, y] = polar_to_xy(radius, phi, radius, direction);
        positions.push([x, y, z_offset_mm + hz]);
        thickness.push(row_t(0, j) as f32);
    }
    let smooth_int_bot_start = positions.len() as u32;
    let z_bot = row_z(rows - 1);
    for (j, &phi) in phis.iter().enumerate().take(cols - 1).skip(1) {
        let [x, y] = polar_to_xy(radius, phi, radius, direction);
        positions.push([x, y, z_bot]);
        thickness.push(row_t(rows - 1, j) as f32);
    }

    let smooth_top = |j: usize| -> u32 {
        if j == 0 {
            smooth_left_start
        } else if j == cols - 1 {
            smooth_right_start
        } else {
            smooth_int_top_start + (j - 1) as u32
        }
    };
    let smooth_bot = |j: usize| -> u32 {
        if j == 0 {
            smooth_left_start + (rows - 1) as u32
        } else if j == cols - 1 {
            smooth_right_start + (rows - 1) as u32
        } else {
            smooth_int_bot_start + (j - 1) as u32
        }
    };
    let smooth_left = |i: usize| -> u32 { smooth_left_start + i as u32 };
    let smooth_right = |i: usize| -> u32 { smooth_right_start + i as u32 };

    let mut indices = Vec::with_capacity(2 * (rows - 1) * (cols - 1) + 4 * rows + 4 * cols);
    let r_idx = |i: usize, j: usize| -> u32 { (i * cols + j) as u32 };

    // Relief surface faces
    for i in 0..rows - 1 {
        for j in 0..cols - 1 {
            let a = r_idx(i, j);
            let b = r_idx(i, j + 1);
            let c = r_idx(i + 1, j);
            let d = r_idx(i + 1, j + 1);
            indices.push([a, c, d]);
            indices.push([a, d, b]);
        }
    }

    // Smooth surface faces (ruled strips matching all perimeter edges 1-to-1)
    if cols == 2 {
        for i in 0..rows - 1 {
            let a = smooth_left(i);
            let b = smooth_right(i);
            let c = smooth_left(i + 1);
            let d = smooth_right(i + 1);
            indices.push([a, b, d]);
            indices.push([a, d, c]);
        }
    } else {
        // Leftmost strip (j = 0 .. 1): fan from smooth_top(1) along left edge + bottom cap
        let t1 = smooth_top(1);
        let b1 = smooth_bot(1);
        for i in 0..rows - 1 {
            indices.push([smooth_left(i), t1, smooth_left(i + 1)]);
        }
        indices.push([smooth_left(rows - 1), t1, b1]);

        // Interior ruling strips (j = 1 .. cols-3)
        for j in 1..(cols - 2) {
            let tl = smooth_top(j);
            let bl = smooth_bot(j);
            let tr = smooth_top(j + 1);
            let br = smooth_bot(j + 1);
            indices.push([tl, tr, br]);
            indices.push([tl, br, bl]);
        }

        // Rightmost strip (j = cols-2 .. cols-1): top cap + fan from smooth_bot(cols-2) along right edge
        let tm2 = smooth_top(cols - 2);
        let bm2 = smooth_bot(cols - 2);
        indices.push([tm2, smooth_right(0), bm2]);
        for i in 0..rows - 1 {
            indices.push([bm2, smooth_right(i), smooth_right(i + 1)]);
        }
    }

    // Perimeter walls (CCW when seen from relief side)
    let mut relief_perim = Vec::with_capacity(2 * rows + 2 * cols - 4);
    let mut smooth_perim = Vec::with_capacity(2 * rows + 2 * cols - 4);
    for i in 0..rows {
        relief_perim.push(r_idx(i, 0));
        smooth_perim.push(smooth_left(i));
    }
    for j in 1..cols {
        relief_perim.push(r_idx(rows - 1, j));
        smooth_perim.push(smooth_bot(j));
    }
    for i in (0..rows - 1).rev() {
        relief_perim.push(r_idx(i, cols - 1));
        smooth_perim.push(smooth_right(i));
    }
    for j in (1..cols - 1).rev() {
        relief_perim.push(r_idx(0, j));
        smooth_perim.push(smooth_top(j));
    }

    let n_per = relief_perim.len();
    for k in 0..n_per {
        let k1 = (k + 1) % n_per;
        let f0 = relief_perim[k];
        let f1 = relief_perim[k1];
        let s0 = smooth_perim[k];
        let s1 = smooth_perim[k1];
        indices.push([f1, f0, s0]);
        indices.push([f1, s0, s1]);
    }

    if relief_side == ReliefSide::Back {
        for tri in &mut indices {
            tri.swap(1, 2);
        }
    }

    MeshData {
        positions,
        indices,
        thickness: Some(thickness),
    }
}

/// Build the filleted curved foot base extruded from `Z = 0` to `Z = base.thickness_mm`.
///
/// The base extends radially forward and backward by `front_projection_mm` and
/// `back_projection_mm` without projecting sideways beyond `[-theta/2, +theta/2]`,
/// and clamps inner and outer corner fillets independently so panel corners never overhang.
/// Returns `(base_manifold, effective_fillet_in, effective_fillet_out, warnings)`.
pub fn build_curved_base_manifold(
    tm: &ThicknessMap,
    radius: f64,
    direction: CurvatureDirection,
    relief_side: ReliefSide,
    base: &CurvedBaseSettings,
) -> Result<(Manifold, f64, f64, Vec<String>), LithoError> {
    let length_x = tm.length_x();
    let max_t = tm.max_thickness();
    let theta = length_x / radius;
    let alpha = theta * 0.5;
    let (r_panel_min, r_panel_max) = panel_radial_bounds(radius, max_t, direction, relief_side);

    let (inner_proj, outer_proj) = match direction {
        CurvatureDirection::Concave => (base.front_projection_mm, base.back_projection_mm),
        CurvatureDirection::Convex => (base.back_projection_mm, base.front_projection_mm),
    };

    let r_in = r_panel_min - inner_proj;
    let r_out = r_panel_max + outer_proj;
    if r_in < MIN_SAFE_RADIUS_MM {
        let min_req = radius - r_in + MIN_SAFE_RADIUS_MM;
        return Err(LithoError::Geometry(format!(
            "curved base inner radius {r_in:.2} mm is too small (< {MIN_SAFE_RADIUS_MM:.1} mm). Increase radius to at least {min_req:.1} mm or reduce front/back base projection."
        )));
    }

    let radial_span = r_out - r_in;
    let mut r_f_in = base
        .corner_fillet_mm
        .min(0.9 * inner_proj)
        .min(0.45 * radial_span);

    // Exact tangent guarantee so panel outer corner at r_panel_max never overhangs outer fillet:
    // r_t_out = sqrt(r_out^2 - 2*r_out*r_f_out) >= r_panel_max + 0.05*outer_proj
    let r_safe_out = r_panel_max + 0.05 * outer_proj;
    let max_f_out_tangent = ((r_out * r_out - r_safe_out * r_safe_out) / (2.0 * r_out)).max(0.0);
    let mut r_f_out = base
        .corner_fillet_mm
        .min(0.9 * outer_proj)
        .min(0.45 * radial_span)
        .min(max_f_out_tangent);

    if alpha < FRAC_PI_2 {
        let s_max = (0.95 * alpha).sin();
        if s_max < 0.999 {
            r_f_in = r_f_in.min(r_in * s_max / (1.0 - s_max));
        }
        r_f_out = r_f_out.min(r_out * s_max / (1.0 + s_max));
    }
    r_f_in = r_f_in.max(0.0);
    r_f_out = r_f_out.max(0.0);

    let mut warnings = Vec::new();
    if base.corner_fillet_mm > 0.0 && (r_f_in + 0.05 < base.corner_fillet_mm || r_f_out + 0.05 < base.corner_fillet_mm) {
        warnings.push(format!(
            "Curved base corner fillet clamped from {:.1} mm to ({:.2} mm inner, {:.2} mm outer) to keep panel corners fully supported.",
            base.corner_fillet_mm, r_f_in, r_f_out
        ));
    }

    // Build closed 2D outline in (u, v) = (r*cos(phi), r*sin(phi)) where |phi| <= alpha
    let mut uv_pts: Vec<[f64; 2]> = Vec::new();

    let (phi_c_in, c_in_plus, c_in_minus) = if r_f_in > 1e-4 {
        let rc = r_in + r_f_in;
        let dphi = (r_f_in / rc).clamp(-1.0, 1.0).asin();
        let pc = alpha - dphi;
        (
            pc,
            [rc * pc.cos(), rc * pc.sin()],
            [rc * (-pc).cos(), rc * (-pc).sin()],
        )
    } else {
        (alpha, [0.0, 0.0], [0.0, 0.0])
    };

    let (phi_c_out, c_out_plus, c_out_minus) = if r_f_out > 1e-4 {
        let rc = r_out - r_f_out;
        let dphi = (r_f_out / rc).clamp(-1.0, 1.0).asin();
        let pc = alpha - dphi;
        (
            pc,
            [rc * pc.cos(), rc * pc.sin()],
            [rc * (-pc).cos(), rc * (-pc).sin()],
        )
    } else {
        (alpha, [0.0, 0.0], [0.0, 0.0])
    };

    let arc_steps_main_in = ((2.0 * phi_c_in).to_degrees().ceil() as usize).max(16);
    let arc_steps_main_out = ((2.0 * phi_c_out).to_degrees().ceil() as usize).max(16);
    const FILLET_STEPS: usize = 16;

    // 1. Inner main arc: phi from -phi_c_in to +phi_c_in
    for k in 0..=arc_steps_main_in {
        let phi = -phi_c_in + (2.0 * phi_c_in) * (k as f64 / arc_steps_main_in as f64);
        uv_pts.push([r_in * phi.cos(), r_in * phi.sin()]);
    }

    // 2. Corner 1 (+alpha, inner -> radial side, sweeping CW)
    if r_f_in > 1e-4 {
        let a0 = phi_c_in + PI;
        let a1 = unwrap_cw(a0, alpha + FRAC_PI_2);
        sample_circle_arc(&mut uv_pts, c_in_plus, r_f_in, a0, a1, FILLET_STEPS);
    }

    // 3. Corner 2 (+alpha, radial side -> outer, sweeping CW)
    if r_f_out > 1e-4 {
        let a0 = alpha + FRAC_PI_2;
        let a1 = unwrap_cw(a0, phi_c_out);
        sample_circle_arc(&mut uv_pts, c_out_plus, r_f_out, a0, a1, FILLET_STEPS);
    }

    // 4. Outer main arc: phi from +phi_c_out down to -phi_c_out
    for k in 0..=arc_steps_main_out {
        let phi = phi_c_out - (2.0 * phi_c_out) * (k as f64 / arc_steps_main_out as f64);
        uv_pts.push([r_out * phi.cos(), r_out * phi.sin()]);
    }

    // 5. Corner 3 (-alpha, outer -> radial side, sweeping CW)
    if r_f_out > 1e-4 {
        let a0 = -phi_c_out;
        let a1 = unwrap_cw(a0, -alpha - FRAC_PI_2);
        sample_circle_arc(&mut uv_pts, c_out_minus, r_f_out, a0, a1, FILLET_STEPS);
    }

    // 6. Corner 4 (-alpha, radial side -> inner, sweeping CW)
    if r_f_in > 1e-4 {
        let a0 = -alpha - FRAC_PI_2;
        let a1 = unwrap_cw(a0, -phi_c_in - PI);
        sample_circle_arc(&mut uv_pts, c_in_minus, r_f_in, a0, a1, FILLET_STEPS);
    }

    // Convert (u, v) = (r*cos(phi), r*sin(phi)) -> world (x, y)
    let mut contour: Vec<Vec2> = Vec::with_capacity(uv_pts.len());
    for [u, v] in uv_pts {
        let (x, y) = match direction {
            CurvatureDirection::Concave => (v, u - radius),
            CurvatureDirection::Convex => (v, radius - u),
        };
        if let Some(last) = contour.last() {
            if (last.x - x).hypot(last.y - y) < 1e-6 {
                continue;
            }
        }
        contour.push(Vec2::new(x, y));
    }
    if let (Some(first), Some(last)) = (contour.first(), contour.last()) {
        if (first.x - last.x).hypot(first.y - last.y) < 1e-6 {
            contour.pop();
        }
    }

    if contour.len() < 3 {
        return Err(LithoError::Geometry(
            "curved base contour has fewer than 3 points".into(),
        ));
    }

    // Ensure strictly CCW winding for Manifold::extrude
    if polygon_signed_area(&contour) < 0.0 {
        contour.reverse();
    }

    let polys: Polygons = vec![contour];
    let base_m = Manifold::extrude(&polys, base.thickness_mm, 0, 0.0, Vec2::new(1.0, 1.0));
    validate_manifold(&base_m, "curved base")?;

    Ok((base_m, r_f_in, r_f_out, warnings))
}

#[inline]
fn unwrap_cw(a0: f64, mut a1: f64) -> f64 {
    while a1 > a0 {
        a1 -= 2.0 * PI;
    }
    while a0 - a1 > 2.0 * PI {
        a1 += 2.0 * PI;
    }
    a1
}

fn sample_circle_arc(
    dst: &mut Vec<[f64; 2]>,
    center: [f64; 2],
    radius: f64,
    a0: f64,
    a1: f64,
    steps: usize,
) {
    for k in 0..=steps {
        let a = a0 + (a1 - a0) * (k as f64 / steps as f64);
        dst.push([center[0] + radius * a.cos(), center[1] + radius * a.sin()]);
    }
}

fn polygon_signed_area(pts: &[Vec2]) -> f64 {
    let mut area2 = 0.0;
    let n = pts.len();
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        area2 += a.x * b.y - b.x * a.y;
    }
    area2 * 0.5
}
