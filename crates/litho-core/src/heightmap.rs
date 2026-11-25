//! Luminance-to-thickness mapping, framing, bottom extension, and print orientation.

use crate::image_proc::SizePlan;
use crate::settings::{LithophaneSettings, OrientationMode, ThicknessCurve, ThicknessSettings};

/// Map normalized luminance (`1.0` = white) to thickness in mm (`white -> min`, `black -> max`).
pub fn luminance_to_thickness(lum: &[f32], s: &ThicknessSettings) -> Vec<f32> {
    let lo = s.min_thickness_mm;
    let hi = s.max_thickness_mm;
    let mut custom_pts = s.custom_points.clone();
    if s.curve == ThicknessCurve::Custom {
        custom_pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    }

    lum.iter()
        .map(|&l| {
            let clamped_l = (l as f64).clamp(0.0, 1.0);
            let dark = 1.0 - clamped_l;
            let thick = match s.curve {
                ThicknessCurve::Linear => lo + (hi - lo) * dark,
                ThicknessCurve::Gamma => {
                    let f = dark.powf(s.curve_gamma).clamp(0.0, 1.0);
                    lo + (hi - lo) * f
                }
                ThicknessCurve::Custom => {
                    let f = interp_custom(dark, &custom_pts).clamp(0.0, 1.0);
                    lo + (hi - lo) * f
                }
                ThicknessCurve::Transmission => {
                    let k = s.attenuation_per_mm;
                    let t_hi = (-k * lo).exp();
                    let t_lo = (-k * hi).exp();
                    let target = t_lo + clamped_l * (t_hi - t_lo);
                    (-target.max(1e-12).ln() / k).clamp(lo, hi)
                }
            };
            thick as f32
        })
        .collect()
}

fn interp_custom(x: f64, sorted_pts: &[[f64; 2]]) -> f64 {
    if sorted_pts.is_empty() {
        return x;
    }
    if x <= sorted_pts[0][0] {
        return sorted_pts[0][1];
    }
    let last = sorted_pts.len() - 1;
    if x >= sorted_pts[last][0] {
        return sorted_pts[last][1];
    }
    for win in sorted_pts.windows(2) {
        let [x0, y0] = win[0];
        let [x1, y1] = win[1];
        if x >= x0 && x <= x1 {
            let span = x1 - x0;
            if span <= 1e-12 {
                return y1;
            }
            return y0 + (y1 - y0) * ((x - x0) / span);
        }
    }
    sorted_pts[last][1]
}

/// Final oriented thickness grid ready for meshing.
///
/// `thickness[r * cols + c]`: row `0` is the top (`max Z`), col `0` is left (`min X` / `-theta/2`)
/// as seen by a viewer standing at `-Y`.
#[derive(Debug, Clone)]
pub struct ThicknessMap {
    pub thickness: Vec<f32>,
    pub rows: usize,
    pub cols: usize,
    pub dx: f64,
    pub dz: f64,
    pub frame_px_x: usize,
    pub frame_px_z: usize,
    /// `0` (unchanged) or `-1` (rotated 90° clockwise for printing).
    pub rot_k: i32,
    /// Inner luminance grid in image-upright orientation (`plan.inner_ny()` × `plan.inner_nx()`).
    pub luminance: Vec<f32>,
    pub lum_rows: usize,
    pub lum_cols: usize,
    pub plan: SizePlan,
    pub frame_thickness_mm: f64,
}

impl ThicknessMap {
    /// Total panel width along X (or arc length along curve) in mm.
    pub fn length_x(&self) -> f64 {
        self.dx * (self.cols - 1) as f64
    }

    /// Total panel height along Z in mm.
    pub fn height_z(&self) -> f64 {
        self.dz * (self.rows - 1) as f64
    }

    /// Maximum thickness across the grid in mm.
    pub fn max_thickness(&self) -> f64 {
        self.thickness
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max) as f64
    }

    /// Minimum thickness across the grid in mm.
    pub fn min_thickness(&self) -> f64 {
        self.thickness
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min) as f64
    }

    /// Thickness grid un-rotated back to the supplied image's upright orientation.
    pub fn thickness_image_frame(&self) -> (Vec<f32>, usize, usize) {
        if self.rot_k == -1 {
            rot90_ccw(&self.thickness, self.rows, self.cols)
        } else {
            (self.thickness.clone(), self.rows, self.cols)
        }
    }

    /// Read cell `(row, col)`.
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.thickness[row * self.cols + col] as f64
    }
}

/// Decide print rotation (`0` or `-1` = 90° clockwise) for a rectangle of size `w × h`.
pub fn decide_rotation(w: f64, h: f64, mode: OrientationMode) -> i32 {
    if mode == OrientationMode::ImageUp || (w - h).abs() < 1e-9 {
        return 0;
    }
    match mode {
        OrientationMode::AutoShortUp => {
            if w < h {
                -1
            } else {
                0
            }
        }
        OrientationMode::LongUp => {
            if w > h {
                -1
            } else {
                0
            }
        }
        OrientationMode::ImageUp => 0,
    }
}

/// Rotate a row-major `(rows × cols)` grid 90° clockwise (`np.rot90(..., k=-1)`).
pub fn rot90_cw(grid: &[f32], rows: usize, cols: usize) -> (Vec<f32>, usize, usize) {
    let new_rows = cols;
    let new_cols = rows;
    let mut out = vec![0.0f32; new_rows * new_cols];
    for r in 0..rows {
        for c in 0..cols {
            let nr = c;
            let nc = rows - 1 - r;
            out[nr * new_cols + nc] = grid[r * cols + c];
        }
    }
    (out, new_rows, new_cols)
}

/// Rotate a row-major `(rows × cols)` grid 90° counter-clockwise (`np.rot90(..., k=1)`).
pub fn rot90_ccw(grid: &[f32], rows: usize, cols: usize) -> (Vec<f32>, usize, usize) {
    let new_rows = cols;
    let new_cols = rows;
    let mut out = vec![0.0f32; new_rows * new_cols];
    for r in 0..rows {
        for c in 0..cols {
            let nr = cols - 1 - c;
            let nc = r;
            out[nr * new_cols + nc] = grid[r * cols + c];
        }
    }
    (out, new_rows, new_cols)
}

/// Convert inner luminance into a framed, bottom-extended, print-oriented [`ThicknessMap`].
pub fn build_thickness_map(
    lum: Vec<f32>,
    plan: SizePlan,
    settings: &LithophaneSettings,
    orientation_mode: OrientationMode,
) -> ThicknessMap {
    let t = &settings.thickness;
    let thick = luminance_to_thickness(&lum, t);
    let ft = settings.frame.thickness_mm.unwrap_or(t.max_thickness_mm);

    let inner_nx = plan.inner_nx();
    let inner_ny = plan.inner_ny();

    let (mut grid, mut rows, mut cols) = if settings.frame.enabled && plan.frame_mm > 0.0 {
        let mut full = vec![ft as f32; plan.ny * plan.nx];
        for r in 0..inner_ny {
            let dst_r = plan.frame_px_y + r;
            let dst_off = dst_r * plan.nx + plan.frame_px_x;
            let src_off = r * inner_nx;
            full[dst_off..dst_off + inner_nx].copy_from_slice(&thick[src_off..src_off + inner_nx]);
        }
        (full, plan.ny, plan.nx)
    } else {
        (thick, plan.ny, plan.nx)
    };

    let mut dx = plan.outer_w_mm / (plan.nx - 1) as f64;
    let mut dz = plan.outer_h_mm / (plan.ny - 1) as f64;

    let ext = &settings.bottom_extension;
    if ext.enabled && ext.height_mm > 0.0 {
        let ext_rows = ((ext.height_mm / dz).round() as usize).max(1);
        let ext_val = ext.thickness_mm as f32;
        grid.resize((rows + ext_rows) * cols, ext_val);
        rows += ext_rows;
    }

    let mut fpx = plan.frame_px_x;
    let mut fpz = plan.frame_px_y;
    let total_w = (cols - 1) as f64 * dx;
    let total_h = (rows - 1) as f64 * dz;
    let k = decide_rotation(total_w, total_h, orientation_mode);
    if k != 0 {
        let (rot, nr, nc) = rot90_cw(&grid, rows, cols);
        grid = rot;
        rows = nr;
        cols = nc;
        std::mem::swap(&mut dx, &mut dz);
        std::mem::swap(&mut fpx, &mut fpz);
    }

    ThicknessMap {
        thickness: grid,
        rows,
        cols,
        dx,
        dz,
        frame_px_x: fpx,
        frame_px_z: fpz,
        rot_k: k,
        luminance: lum,
        lum_rows: inner_ny,
        lum_cols: inner_nx,
        plan,
        frame_thickness_mm: ft,
    }
}
