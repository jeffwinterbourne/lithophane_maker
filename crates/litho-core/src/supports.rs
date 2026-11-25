//! Breakaway support fins, bed pads, and tapered snap-off tabs (flat & curved).

use manifold_rust::manifold::Manifold;

use crate::error::LithoError;
use crate::heightmap::ThicknessMap;
use crate::mesh::{hull_manifold, manifold_to_mesh, union_manifolds, MeshData};
use crate::settings::{CurvatureDirection, LithophaneSettings, ReliefSide, SupportSides};

/// Resolved automatic support dimensions reported back to callers.
#[derive(Debug, Clone)]
pub struct SupportPlan {
    pub fin_height: f64,
    pub fin_base_length: f64,
    pub fin_thickness: f64,
    pub fin_centres_x: Vec<f64>,
    pub sides: Vec<i32>,
    pub warnings: Vec<String>,
}

/// Resolve automatic fin height, footprint, and lateral stations.
pub fn plan_supports(
    tm: &ThicknessMap,
    settings: &LithophaneSettings,
) -> Result<SupportPlan, LithoError> {
    let s = &settings.supports;
    let hz = tm.height_z();
    let height = s
        .fin_height_mm
        .unwrap_or(s.fin_height_fraction * hz)
        .min(hz);
    let base = s.fin_base_length_mm.unwrap_or((0.6 * height).max(12.0));
    let inset = s.fin_inset_mm.unwrap_or_else(|| {
        if settings.frame.enabled && tm.frame_px_x > 0 {
            tm.frame_px_x as f64 * tm.dx / 2.0
        } else {
            s.fin_thickness_mm / 2.0 + 1.0
        }
    });

    let half = tm.length_x() / 2.0;
    let centres = if s.fin_count == 1 {
        vec![0.0]
    } else {
        if inset >= half {
            return Err(LithoError::Settings(
                "supports.fin_inset_mm places the fins beyond the panel centre".into(),
            ));
        }
        linspace(-half + inset, half - inset, s.fin_count)
    };

    let sides = match s.sides {
        SupportSides::Both => vec![-1, 1],
        SupportSides::Front => vec![-1],
        SupportSides::Back => vec![1],
    };

    let mut warnings = Vec::new();
    if base < 0.25 * hz {
        warnings.push(format!(
            "Support footprint ({base:.1} mm) is under 25% of panel height ({hz:.1} mm); may be unstable."
        ));
    }
    if s.tab_height_mm < s.tab_neck_height_mm {
        warnings.push(
            "supports.tab_height_mm is smaller than tab_neck_height_mm; tabs will not taper."
                .into(),
        );
    }

    Ok(SupportPlan {
        fin_height: height,
        fin_base_length: base,
        fin_thickness: s.fin_thickness_mm,
        fin_centres_x: centres,
        sides,
        warnings,
    })
}

pub(crate) fn linspace(start: f64, end: f64, count: usize) -> Vec<f64> {
    if count <= 1 {
        return vec![start];
    }
    let step = (end - start) / (count - 1) as f64;
    (0..count).map(|i| start + i as f64 * step).collect()
}

/// Minimum panel thickness within an `(x, z)` window (in panel local coordinates where `z=0` is bottom of grid).
pub(crate) fn surface_thickness(
    tm: &ThicknessMap,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
) -> f64 {
    let half_x = tm.length_x() / 2.0;
    let c0 = ((x0 + half_x) / tm.dx).floor().clamp(0.0, (tm.cols - 1) as f64) as usize;
    let c1 = ((x1 + half_x) / tm.dx).ceil().clamp(0.0, (tm.cols - 1) as f64) as usize;
    let r0 = ((tm.rows - 1) as f64 - z1 / tm.dz)
        .floor()
        .clamp(0.0, (tm.rows - 1) as f64) as usize;
    let r1 = ((tm.rows - 1) as f64 - z0 / tm.dz)
        .ceil()
        .clamp(0.0, (tm.rows - 1) as f64) as usize;

    let mut min_t = f64::INFINITY;
    for r in r0.min(r1)..=r0.max(r1) {
        for c in c0.min(c1)..=c0.max(c1) {
            min_t = min_t.min(tm.get(r, c));
        }
    }
    min_t
}

/// Maximum panel thickness along a vertical strip `[x0, x1]`.
pub(crate) fn max_thickness_strip(tm: &ThicknessMap, x0: f64, x1: f64) -> f64 {
    let half_x = tm.length_x() / 2.0;
    let c0 = ((x0 + half_x) / tm.dx).floor().clamp(0.0, (tm.cols - 1) as f64) as usize;
    let c1 = ((x1 + half_x) / tm.dx).ceil().clamp(0.0, (tm.cols - 1) as f64) as usize;

    let mut max_t = 0.0f64;
    for r in 0..tm.rows {
        for c in c0.min(c1)..=c0.max(c1) {
            max_t = max_t.max(tm.get(r, c));
        }
    }
    max_t
}

/// Build the union of flat fins, bed pads, and breakaway tabs as a validated [`Manifold`] + [`MeshData`].
pub fn build_flat_supports(
    tm: &ThicknessMap,
    settings: &LithophaneSettings,
) -> Result<(Manifold, MeshData, SupportPlan), LithoError> {
    let s = &settings.supports;
    let relief: i32 = match settings.thickness.relief_side {
        ReliefSide::Front => -1,
        ReliefSide::Back => 1,
    };
    let plan = plan_supports(tm, settings)?;
    let ft = s.fin_thickness_mm;
    let tw = s.tab_width_mm.unwrap_or(ft);
    let mut parts: Vec<Manifold> = Vec::new();

    for &side_i in &plan.sides {
        let side = side_i as f64;
        for &xc in &plan.fin_centres_x {
            let xl = xc - ft / 2.0;
            let xr = xc + ft / 2.0;
            let m = if side_i == relief {
                max_thickness_strip(tm, xl, xr)
            } else {
                0.0
            };
            let y_e = side * (m + s.interface_gap_mm);
            let y_out = y_e + side * plan.fin_base_length;
            let h = plan.fin_height;

            parts.push(hull_manifold(
                &[
                    [xl, y_e, 0.0],
                    [xl, y_out, 0.0],
                    [xl, y_e, h],
                    [xr, y_e, 0.0],
                    [xr, y_out, 0.0],
                    [xr, y_e, h],
                ],
                "support fin",
            )?);

            if s.pad_enabled {
                let pt = s.pad_thickness_mm;
                let px0 = xl - s.pad_side_margin_mm;
                let px1 = xr + s.pad_side_margin_mm;
                let py_out = y_out + side * s.pad_end_margin_mm;
                parts.push(hull_manifold(
                    &[
                        [px0, y_e, 0.0],
                        [px1, y_e, 0.0],
                        [px0, py_out, 0.0],
                        [px1, py_out, 0.0],
                        [px0, y_e, pt],
                        [px1, y_e, pt],
                        [px0, py_out, pt],
                        [px1, py_out, pt],
                    ],
                    "support pad",
                )?);
            }

            let z_lo = s.tab_margin_mm + s.tab_height_mm / 2.0;
            let z_hi = h - s.tab_margin_mm - s.tab_height_mm / 2.0;
            let zs = if s.tab_count == 1 || z_hi <= z_lo {
                vec![h / 2.0]
            } else {
                linspace(z_lo, z_hi, s.tab_count)
            };
            for zc in zs {
                let t_loc = if side_i == relief {
                    surface_thickness(
                        tm,
                        xc - tw / 2.0,
                        xc + tw / 2.0,
                        zc - s.tab_height_mm / 2.0,
                        zc + s.tab_height_mm / 2.0,
                    )
                } else {
                    0.0
                };
                let y_start = y_e + side * 0.2;
                let y_end = side * (t_loc - s.tab_overlap_mm);
                let hh = s.tab_height_mm / 2.0;
                let nh = s.tab_neck_height_mm / 2.0;
                let xa = xc - tw / 2.0;
                let xb = xc + tw / 2.0;
                parts.push(hull_manifold(
                    &[
                        [xa, y_start, zc - hh],
                        [xb, y_start, zc - hh],
                        [xa, y_start, zc + hh],
                        [xb, y_start, zc + hh],
                        [xa, y_end, zc - nh],
                        [xb, y_end, zc - nh],
                        [xa, y_end, zc + nh],
                        [xb, y_end, zc + nh],
                    ],
                    "support tab",
                )?);
            }
        }
    }

    let sup_manifold = union_manifolds(&parts, "flat supports")?;
    let sup_mesh = manifold_to_mesh(&sup_manifold);
    Ok((sup_manifold, sup_mesh, plan))
}

/// Build radial breakaway fins, pads, and tabs along a curved lithophane arc.
///
/// When a built-in base is enabled (`z_offset_mm > 0`), fins stand outside the base projection
/// on the bed at `Z = 0` and tabs reach horizontally across to the panel above the base.
pub fn build_curved_supports(
    tm: &ThicknessMap,
    settings: &LithophaneSettings,
    radius_mm: f64,
    direction: CurvatureDirection,
    z_offset_mm: f64,
    front_base_proj_mm: f64,
    back_base_proj_mm: f64,
) -> Result<(Manifold, MeshData, SupportPlan), LithoError> {
    let s = &settings.supports;
    let relief: i32 = match settings.thickness.relief_side {
        ReliefSide::Front => -1,
        ReliefSide::Back => 1,
    };
    let mut plan = plan_supports(tm, settings)?;
    let ft = s.fin_thickness_mm;
    let tw = s.tab_width_mm.unwrap_or(ft);
    let mut parts: Vec<Manifold> = Vec::new();
    let total_z_top = z_offset_mm + tm.height_z();

    for &side_i in &plan.sides {
        let side = side_i as f64;
        let base_clear = if side_i == -1 {
            front_base_proj_mm
        } else {
            back_base_proj_mm
        };

        // Determine whether this side extends radially toward the curvature center (inner side)
        let goes_inward = matches!(
            (direction, side_i),
            (CurvatureDirection::Concave, -1) | (CurvatureDirection::Convex, 1)
        );

        for &sc in &plan.fin_centres_x {
            let sl = sc - ft / 2.0;
            let sr = sc + ft / 2.0;
            let m = if side_i == relief {
                max_thickness_strip(tm, sl, sr)
            } else {
                0.0
            };

            let v_e = side * (m + base_clear + s.interface_gap_mm);
            let mut fin_len = plan.fin_base_length;
            if goes_inward {
                // Keep inner radial fin foot well clear of the center of curvature
                let r_edge = radius_mm - v_e.abs();
                let max_inward = (r_edge - s.pad_end_margin_mm - 3.0).max(4.0);
                if fin_len > max_inward {
                    fin_len = max_inward;
                    let warn = format!(
                        "Inner curved support footprint clamped to {fin_len:.1} mm to stay clear of curvature center."
                    );
                    if !plan.warnings.contains(&warn) {
                        plan.warnings.push(warn);
                    }
                }
            }
            let v_out = v_e + side * fin_len;
            let h_top = (z_offset_mm + plan.fin_height).min(total_z_top);

            let phi = sc / radius_mm;
            let (px, py, tx, ty, nx, ny) = match direction {
                CurvatureDirection::Concave => (
                    radius_mm * phi.sin(),
                    radius_mm * phi.cos() - radius_mm,
                    phi.cos(),
                    -phi.sin(),
                    phi.sin(),
                    phi.cos(),
                ),
                CurvatureDirection::Convex => (
                    radius_mm * phi.sin(),
                    radius_mm - radius_mm * phi.cos(),
                    phi.cos(),
                    phi.sin(),
                    -phi.sin(),
                    phi.cos(),
                ),
            };

            let map = |u: f64, v: f64, z: f64| -> [f64; 3] {
                [px + u * tx + v * nx, py + u * ty + v * ny, z]
            };

            let u_l = -ft / 2.0;
            let u_r = ft / 2.0;
            parts.push(hull_manifold(
                &[
                    map(u_l, v_e, 0.0),
                    map(u_l, v_out, 0.0),
                    map(u_l, v_e, h_top),
                    map(u_r, v_e, 0.0),
                    map(u_r, v_out, 0.0),
                    map(u_r, v_e, h_top),
                ],
                "curved support fin",
            )?);

            if s.pad_enabled {
                let pt = s.pad_thickness_mm;
                let pu0 = u_l - s.pad_side_margin_mm;
                let pu1 = u_r + s.pad_side_margin_mm;
                let pv_out = v_out + side * s.pad_end_margin_mm;
                parts.push(hull_manifold(
                    &[
                        map(pu0, v_e, 0.0),
                        map(pu1, v_e, 0.0),
                        map(pu0, pv_out, 0.0),
                        map(pu1, pv_out, 0.0),
                        map(pu0, v_e, pt),
                        map(pu1, v_e, pt),
                        map(pu0, pv_out, pt),
                        map(pu1, pv_out, pt),
                    ],
                    "curved support pad",
                )?);
            }

            let z_lo = z_offset_mm + s.tab_margin_mm + s.tab_height_mm / 2.0;
            let z_hi = h_top - s.tab_margin_mm - s.tab_height_mm / 2.0;
            let zs = if s.tab_count == 1 || z_hi <= z_lo {
                vec![(z_offset_mm + h_top) / 2.0]
            } else {
                linspace(z_lo, z_hi, s.tab_count)
            };

            for zc in zs {
                let z_panel = (zc - z_offset_mm).max(0.0);
                let t_loc = if side_i == relief {
                    surface_thickness(
                        tm,
                        sc - tw / 2.0,
                        sc + tw / 2.0,
                        (z_panel - s.tab_height_mm / 2.0).max(0.0),
                        (z_panel + s.tab_height_mm / 2.0).min(tm.height_z()),
                    )
                } else {
                    0.0
                };
                let v_start = v_e + side * 0.2;
                let v_end = side * (t_loc - s.tab_overlap_mm);
                let hh = s.tab_height_mm / 2.0;
                let nh = s.tab_neck_height_mm / 2.0;
                let ua = -tw / 2.0;
                let ub = tw / 2.0;
                parts.push(hull_manifold(
                    &[
                        map(ua, v_start, zc - hh),
                        map(ub, v_start, zc - hh),
                        map(ua, v_start, zc + hh),
                        map(ub, v_start, zc + hh),
                        map(ua, v_end, zc - nh),
                        map(ub, v_end, zc - nh),
                        map(ua, v_end, zc + nh),
                        map(ub, v_end, zc + nh),
                    ],
                    "curved support tab",
                )?);
            }
        }
    }

    let sup_manifold = union_manifolds(&parts, "curved supports")?;
    let sup_mesh = manifold_to_mesh(&sup_manifold);
    Ok((sup_manifold, sup_mesh, plan))
}
