//! 2D diagnostic preview PNG generators (greyscale, thickness map, simulated backlit).

use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};

use crate::error::LithoError;
use crate::heightmap::ThicknessMap;
use crate::scene::ImagePreviews;
use crate::settings::{parse_hex_rgb, srgb_to_linear, LithophaneSettings};

const VIRIDIS_STOPS: [(f64, [f64; 3]); 5] = [
    (0.00, [253.0, 231.0, 37.0]),
    (0.25, [94.0, 201.0, 98.0]),
    (0.50, [33.0, 145.0, 140.0]),
    (0.75, [59.0, 82.0, 139.0]),
    (1.00, [68.0, 1.0, 84.0]),
];

fn encode_png(data: &[u8], width: u32, height: u32, color: ColorType) -> Result<Vec<u8>, LithoError> {
    let mut out = Vec::new();
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(data, width, height, color.into())
        .map_err(|e| LithoError::Image(format!("PNG encode failed: {e}")))?;
    Ok(out)
}

/// Encode normalized luminance `[0, 1]` as a greyscale PNG.
pub fn luminance_png(lum: &[f32], rows: usize, cols: usize) -> Result<Vec<u8>, LithoError> {
    let bytes: Vec<u8> = lum
        .iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect();
    encode_png(&bytes, cols as u32, rows as u32, ColorType::L8)
}

/// Encode thickness grid with viridis colour map (`bright = thin`, `dark = thick`).
pub fn thickness_png(
    thickness: &[f32],
    rows: usize,
    cols: usize,
    lo: f64,
    hi: f64,
) -> Result<Vec<u8>, LithoError> {
    let span = (hi - lo).max(1e-9);
    let mut rgb = Vec::with_capacity(rows * cols * 3);
    for &t in thickness {
        let f = ((t as f64 - lo) / span).clamp(0.0, 1.0);
        let [r, g, b] = sample_viridis(f);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    encode_png(&rgb, cols as u32, rows as u32, ColorType::Rgb8)
}

fn sample_viridis(f: f64) -> [u8; 3] {
    for win in VIRIDIS_STOPS.windows(2) {
        let (p0, c0) = win[0];
        let (p1, c1) = win[1];
        if f >= p0 && f <= p1 {
            let t = (f - p0) / (p1 - p0);
            return [
                (c0[0] + (c1[0] - c0[0]) * t).round() as u8,
                (c0[1] + (c1[1] - c0[1]) * t).round() as u8,
                (c0[2] + (c1[2] - c0[2]) * t).round() as u8,
            ];
        }
    }
    let last = VIRIDIS_STOPS[4].1;
    [last[0] as u8, last[1] as u8, last[2] as u8]
}

#[inline]
fn linear_to_srgb(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Encode physical Beer-Lambert backlit preview using filament colour, TD (mm), and backlight.
pub fn backlit_png(
    thickness: &[f32],
    rows: usize,
    cols: usize,
    settings: &LithophaneSettings,
) -> Result<Vec<u8>, LithoError> {
    let lit = &settings.lighting;
    let fil_srgb = parse_hex_rgb(&lit.filament_color).unwrap_or([0.965, 0.965, 0.949]);
    let light_srgb = parse_hex_rgb(&lit.backlight_color).unwrap_or([1.0, 1.0, 1.0]);
    let fil_lin = [
        srgb_to_linear(fil_srgb[0]),
        srgb_to_linear(fil_srgb[1]),
        srgb_to_linear(fil_srgb[2]),
    ];
    let light_lin = [
        srgb_to_linear(light_srgb[0]),
        srgb_to_linear(light_srgb[1]),
        srgb_to_linear(light_srgb[2]),
    ];

    let k = 100.0_f64.ln() / lit.transmission_distance_mm.max(0.1);
    let mut rgb = Vec::with_capacity(rows * cols * 3);
    for &th in thickness {
        let thick = th as f64;
        let atten = (-k * thick).exp();
        for c in 0..3 {
            let trans_c = atten * fil_lin[c].clamp(0.02, 1.0).powf(thick / 1.5);
            let lin = lit.backlight_intensity * light_lin[c] * trans_c
                + lit.ambient_front_light * fil_lin[c] * 0.75;
            // Filmic ACES-like shoulder curve matching Three.js ACESFilmic tone mapping
            let mapped = ((lin * (2.51 * lin + 0.03)) / (lin * (2.43 * lin + 0.59) + 0.14))
                .clamp(0.0, 1.0);
            rgb.push((linear_to_srgb(mapped) * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    encode_png(&rgb, cols as u32, rows as u32, ColorType::Rgb8)
}

/// Generate all three image-frame 2D diagnostic PNGs for one [`ThicknessMap`].
pub fn image_frame_previews(
    stem: &str,
    tm: &ThicknessMap,
    settings: &LithophaneSettings,
) -> Result<ImagePreviews, LithoError> {
    let (thick_img, t_rows, t_cols) = tm.thickness_image_frame();
    Ok(ImagePreviews {
        stem: stem.to_string(),
        luminance_png: luminance_png(&tm.luminance, tm.lum_rows, tm.lum_cols)?,
        heightmap_png: thickness_png(
            &thick_img,
            t_rows,
            t_cols,
            settings.thickness.min_thickness_mm,
            settings.thickness.max_thickness_mm,
        )?,
        backlit_png: backlit_png(&thick_img, t_rows, t_cols, settings)?,
        thickness_grid: thick_img,
        grid_rows: t_rows,
        grid_cols: t_cols,
    })
}
