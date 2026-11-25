//! Image loading, EXIF orientation, resizing, greyscale conversion, and tonal adjustments.
//!
//! Future colour styles (such as flat CMY multi-layer lithophanes) can call
//! [`load_prepared_rgb`] and [`fit_and_resize_rgb`] directly to obtain the aligned,
//! Lanczos-resampled RGB image prior to greyscale conversion.

use std::io::Cursor;

use image::imageops::{self, FilterType};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, Rgb, RgbImage};

use crate::error::LithoError;
use crate::scene::ImageSource;
use crate::settings::{
    BackgroundColor, EqualizeMode, FitMode, GreyscaleMode, ImageSettings, LithophaneSettings,
    SizeMode,
};

/// Resolved geometry in the upright image frame (before optional print orientation rotation).
#[derive(Debug, Clone, Copy)]
pub struct SizePlan {
    pub outer_w_mm: f64,
    pub outer_h_mm: f64,
    pub inner_w_mm: f64,
    pub inner_h_mm: f64,
    pub frame_mm: f64,
    /// Outer grid vertices horizontally (`X` in upright image frame).
    pub nx: usize,
    /// Outer grid vertices vertically (`Y` in upright image frame).
    pub ny: usize,
    pub frame_px_x: usize,
    pub frame_px_y: usize,
}

impl SizePlan {
    /// Inner image width in grid vertices.
    pub fn inner_nx(&self) -> usize {
        self.nx - 2 * self.frame_px_x
    }

    /// Inner image height in grid vertices.
    pub fn inner_ny(&self) -> usize {
        self.ny - 2 * self.frame_px_y
    }
}

/// Resampled RGB image paired with its [`SizePlan`].
#[derive(Debug, Clone)]
pub struct PreparedImage {
    pub rgb: RgbImage,
    pub plan: SizePlan,
}

fn bg_rgb(bg: BackgroundColor) -> [u8; 3] {
    match bg {
        BackgroundColor::White => [255, 255, 255],
        BackgroundColor::Black => [0, 0, 0],
    }
}

/// Load any supported image source into an EXIF-oriented, alpha-flattened [`RgbImage`].
pub fn load_image_rgb(source: &ImageSource, bg: BackgroundColor) -> Result<RgbImage, LithoError> {
    let dyn_img = match source {
        ImageSource::Path(path) => {
            let reader = ImageReader::open(path)
                .map_err(|e| LithoError::Image(format!("could not open {}: {e}", path.display())))?
                .with_guessed_format()
                .map_err(|e| LithoError::Image(format!("format probe failed: {e}")))?;
            decode_with_exif(reader)?
        }
        ImageSource::Bytes { bytes, .. } => {
            let reader = ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|e| LithoError::Image(format!("format probe failed: {e}")))?;
            decode_with_exif(reader)?
        }
    };

    Ok(flatten_alpha(dyn_img, bg))
}

fn decode_with_exif<R: std::io::BufRead + std::io::Seek>(
    reader: ImageReader<R>,
) -> Result<DynamicImage, LithoError> {
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| LithoError::Image(format!("could not create decoder: {e}")))?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)
        .map_err(|e| LithoError::Image(format!("could not decode image: {e}")))?;
    img.apply_orientation(orientation);
    Ok(img)
}

fn flatten_alpha(img: DynamicImage, bg: BackgroundColor) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let [br, bg_g, bb] = bg_rgb(bg);
    let mut out = RgbImage::new(w, h);
    for (x, y, px) in rgba.enumerate_pixels() {
        let a = px[3] as f32 / 255.0;
        let inv = 1.0 - a;
        let r = (px[0] as f32 * a + br as f32 * inv).round().clamp(0.0, 255.0) as u8;
        let g = (px[1] as f32 * a + bg_g as f32 * inv).round().clamp(0.0, 255.0) as u8;
        let b = (px[2] as f32 * a + bb as f32 * inv).round().clamp(0.0, 255.0) as u8;
        out.put_pixel(x, y, Rgb([r, g, b]));
    }
    out
}

/// Apply user crop, clockwise rotation, mirror, and flip in the upright image frame.
pub fn prepare_source(mut img: RgbImage, s: &ImageSettings) -> Result<RgbImage, LithoError> {
    if let Some([l, t, r, b]) = s.crop {
        if !(0.0 <= l && l < r && r <= 1.0 && 0.0 <= t && t < b && b <= 1.0) {
            return Err(LithoError::Settings(
                "crop must satisfy 0 <= left < right <= 1 and 0 <= top < bottom <= 1".into(),
            ));
        }
        let (w, h) = img.dimensions();
        let x0 = (l * w as f32).round() as u32;
        let y0 = (t * h as f32).round() as u32;
        let x1 = ((r * w as f32).round() as u32).min(w);
        let y1 = ((b * h as f32).round() as u32).min(h);
        let cw = x1.saturating_sub(x0).max(1);
        let ch = y1.saturating_sub(y0).max(1);
        img = imageops::crop_imm(&img, x0, y0, cw, ch).to_image();
    }
    match s.rotate {
        0 => {}
        90 => img = imageops::rotate90(&img),
        180 => img = imageops::rotate180(&img),
        270 => img = imageops::rotate270(&img),
        other => {
            return Err(LithoError::Settings(format!(
                "image.rotate must be 0, 90, 180, or 270, got {other}"
            )));
        }
    }
    if s.mirror {
        img = imageops::flip_horizontal(&img);
    }
    if s.flip {
        img = imageops::flip_vertical(&img);
    }
    Ok(img)
}

/// Make `img` match target `width / height` aspect via crop, pad, or stretch.
pub fn fit_to_aspect(img: RgbImage, aspect: f64, fit: FitMode, bg: BackgroundColor) -> RgbImage {
    let (w, h) = img.dimensions();
    let cur = w as f64 / h as f64;
    if (cur - aspect).abs() < 1e-6 || fit == FitMode::Stretch {
        return img;
    }
    match fit {
        FitMode::Crop => {
            if cur > aspect {
                let nw = ((h as f64 * aspect).round() as u32).max(1).min(w);
                let x0 = (w - nw) / 2;
                imageops::crop_imm(&img, x0, 0, nw, h).to_image()
            } else {
                let nh = ((w as f64 / aspect).round() as u32).max(1).min(h);
                let y0 = (h - nh) / 2;
                imageops::crop_imm(&img, 0, y0, w, nh).to_image()
            }
        }
        FitMode::Pad => {
            let (nw, nh) = if cur > aspect {
                (w, ((w as f64 / aspect).round() as u32).max(h))
            } else {
                (((h as f64 * aspect).round() as u32).max(w), h)
            };
            let mut out = RgbImage::from_pixel(nw, nh, Rgb(bg_rgb(bg)));
            let ox = ((nw - w) / 2) as i64;
            let oy = ((nh - h) / 2) as i64;
            imageops::overlay(&mut out, &img, ox, oy);
            out
        }
        FitMode::Stretch => img,
    }
}

/// Resolve outer/inner dimensions and exact vertex grid counts from size settings.
pub fn plan_size(src_aspect: f64, settings: &LithophaneSettings) -> Result<SizePlan, LithoError> {
    let sz = &settings.size;
    let fw = if settings.frame.enabled {
        settings.frame.width_mm
    } else {
        0.0
    };
    let a = src_aspect;
    let (ow, oh) = match sz.mode {
        SizeMode::MaxDimension => {
            let m = sz.max_dimension_mm;
            if a >= 1.0 {
                let iw = m - 2.0 * fw;
                let ih = if iw > 0.0 { iw / a } else { 0.0 };
                (m, ih + 2.0 * fw)
            } else {
                let ih = m - 2.0 * fw;
                let iw = if ih > 0.0 { ih * a } else { 0.0 };
                (iw + 2.0 * fw, m)
            }
        }
        SizeMode::Width => {
            let ow = sz.width_mm;
            let iw = ow - 2.0 * fw;
            let ih = if iw > 0.0 { iw / a } else { 0.0 };
            (ow, ih + 2.0 * fw)
        }
        SizeMode::Height => {
            let oh = sz.height_mm;
            let ih = oh - 2.0 * fw;
            let iw = if ih > 0.0 { ih * a } else { 0.0 };
            (iw + 2.0 * fw, oh)
        }
        SizeMode::Absolute => (sz.width_mm, sz.height_mm),
    };

    plan_size_explicit(ow, oh, fw, sz.pixel_size_mm)
}

/// Build a [`SizePlan`] directly from explicit outer dimensions `(ow, oh)`, frame width, and pitch.
pub fn plan_size_explicit(
    ow: f64,
    oh: f64,
    fw: f64,
    pixel_size_mm: f64,
) -> Result<SizePlan, LithoError> {
    let iw = ow - 2.0 * fw;
    let ih = oh - 2.0 * fw;
    if iw <= 0.0 || ih <= 0.0 {
        return Err(LithoError::Settings(format!(
            "size {ow:.1} x {oh:.1} mm leaves no room for the image inside a {fw} mm frame"
        )));
    }
    let nx = ((ow / pixel_size_mm).round() as usize + 1).max(3);
    let ny = ((oh / pixel_size_mm).round() as usize + 1).max(3);
    let dx = ow / (nx - 1) as f64;
    let dy = oh / (ny - 1) as f64;
    let fpx = if fw > 0.0 {
        (fw / dx).round() as usize
    } else {
        0
    };
    let fpy = if fw > 0.0 {
        (fw / dy).round() as usize
    } else {
        0
    };
    if nx <= 2 * fpx + 1 || ny <= 2 * fpy + 1 {
        return Err(LithoError::Settings(
            "pixel_size_mm is too coarse for this size/frame combination".into(),
        ));
    }
    Ok(SizePlan {
        outer_w_mm: ow,
        outer_h_mm: oh,
        inner_w_mm: iw,
        inner_h_mm: ih,
        frame_mm: fw,
        nx,
        ny,
        frame_px_x: fpx,
        frame_px_y: fpy,
    })
}

/// Load, orient, transform, fit, and Lanczos3-resample an input image to inner grid dimensions.
pub fn load_prepared_rgb(
    source: &ImageSource,
    settings: &LithophaneSettings,
) -> Result<PreparedImage, LithoError> {
    let s = &settings.image;
    let raw = load_image_rgb(source, s.background)?;
    let mut img = prepare_source(raw, s)?;
    let aspect = img.width() as f64 / img.height() as f64;
    let plan = plan_size(aspect, settings)?;
    if settings.size.mode == SizeMode::Absolute {
        img = fit_to_aspect(
            img,
            plan.inner_w_mm / plan.inner_h_mm,
            settings.size.fit,
            s.background,
        );
    }
    let resized = imageops::resize(
        &img,
        plan.inner_nx() as u32,
        plan.inner_ny() as u32,
        FilterType::Lanczos3,
    );
    Ok(PreparedImage { rgb: resized, plan })
}

/// Load, resize, greyscale-convert, and tone-adjust an image into row-major luminance `[0, 1]`.
pub fn process_image(
    source: &ImageSource,
    settings: &LithophaneSettings,
) -> Result<(Vec<f32>, SizePlan), LithoError> {
    let prepared = load_prepared_rgb(source, settings)?;
    let dx = prepared.plan.outer_w_mm / (prepared.plan.nx - 1) as f64;
    let dy = prepared.plan.outer_h_mm / (prepared.plan.ny - 1) as f64;
    let (w, h) = prepared.rgb.dimensions();
    let lum = to_greyscale(&prepared.rgb, settings.image.greyscale);
    let adjusted = adjust_luminance(&lum, h as usize, w as usize, &settings.image, dx, dy);
    Ok((adjusted, prepared.plan))
}

/// Convert an [`RgbImage`] into row-major `f32` luminance in `[0, 1]` (`1.0` = white).
pub fn to_greyscale(img: &RgbImage, mode: GreyscaleMode) -> Vec<f32> {
    let (wr, wg, wb): (f32, f32, f32) = match mode {
        GreyscaleMode::Rec601 => (0.299, 0.587, 0.114),
        GreyscaleMode::Rec709 => (0.2126, 0.7152, 0.0722),
        GreyscaleMode::Average => (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
        GreyscaleMode::Red => (1.0, 0.0, 0.0),
        GreyscaleMode::Green => (0.0, 1.0, 0.0),
        GreyscaleMode::Blue => (0.0, 0.0, 1.0),
    };
    let inv255 = 1.0 / 255.0;
    img.pixels()
        .map(|p| (p[0] as f32 * wr + p[1] as f32 * wg + p[2] as f32 * wb) * inv255)
        .collect()
}

/// Apply all 9 tonal adjustments in the exact documented order.
pub fn adjust_luminance(
    input: &[f32],
    rows: usize,
    cols: usize,
    s: &ImageSettings,
    pixel_mm_x: f64,
    pixel_mm_y: f64,
) -> Vec<f32> {
    let mut lum = input.to_vec();
    let pmm = ((pixel_mm_x + pixel_mm_y) * 0.5) as f32;

    // 1. Auto-levels percentile clip & stretch
    if s.auto_levels_percent > 0.0 && !lum.is_empty() {
        let mut sorted = lum.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let lo = percentile_sorted(&sorted, s.auto_levels_percent as f64 / 100.0);
        let hi = percentile_sorted(&sorted, 1.0 - s.auto_levels_percent as f64 / 100.0);
        if hi > lo + 1e-7 {
            let inv = 1.0 / (hi - lo);
            for v in &mut lum {
                *v = ((*v - lo) * inv).clamp(0.0, 1.0);
            }
        }
    }

    // 2. Histogram equalization (global or adaptive CLAHE)
    match s.equalize {
        EqualizeMode::None => {}
        EqualizeMode::Hist => equalize_hist(&mut lum),
        EqualizeMode::Clahe => equalize_clahe(&mut lum, rows, cols, s.clahe_clip),
    }

    // 3. Brightness & contrast
    if s.brightness != 0.0 || (s.contrast - 1.0).abs() > 1e-7 {
        for v in &mut lum {
            *v = ((*v - 0.5) * s.contrast + 0.5 + s.brightness).clamp(0.0, 1.0);
        }
    } else {
        for v in &mut lum {
            *v = v.clamp(0.0, 1.0);
        }
    }

    // 4. Gamma
    if (s.gamma - 1.0).abs() > 1e-7 {
        for v in &mut lum {
            *v = v.clamp(0.0, 1.0).powf(s.gamma);
        }
    }

    // 5. Gaussian blur
    if s.blur_sigma_mm > 0.0 && pmm > 0.0 {
        let sigma_px = s.blur_sigma_mm / pmm;
        if sigma_px > 1e-4 {
            lum = gaussian_blur_2d(&lum, rows, cols, sigma_px);
        }
    }

    // 6. Unsharp mask sharpen
    if s.sharpen_amount > 0.0 && pmm > 0.0 {
        let sigma_px = s.sharpen_radius_mm / pmm;
        if sigma_px > 1e-4 {
            let blurred = gaussian_blur_2d(&lum, rows, cols, sigma_px);
            for (v, b) in lum.iter_mut().zip(blurred.iter()) {
                *v = *v + s.sharpen_amount * (*v - *b);
            }
        }
    }

    // 7. Clamp
    for v in &mut lum {
        *v = v.clamp(0.0, 1.0);
    }

    // 8. Posterize
    if let Some(levels) = s.posterize_levels {
        if levels >= 2 {
            let n_minus_1 = (levels - 1) as f32;
            for v in &mut lum {
                *v = (*v * n_minus_1).round() / n_minus_1;
            }
        }
    }

    // 9. Invert
    if s.invert {
        for v in &mut lum {
            *v = 1.0 - *v;
        }
    }

    lum
}

fn percentile_sorted(sorted: &[f32], q: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    let frac = (idx - lo as f64) as f32;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

fn equalize_hist(lum: &mut [f32]) {
    const BINS: usize = 256;
    if lum.is_empty() {
        return;
    }
    let mut hist = [0u32; BINS];
    for &v in lum.iter() {
        let b = (v.clamp(0.0, 1.0) * (BINS - 1) as f32).round() as usize;
        hist[b] += 1;
    }
    let mut cdf = [0u32; BINS];
    let mut acc = 0u32;
    for i in 0..BINS {
        acc += hist[i];
        cdf[i] = acc;
    }
    let cdf_min = cdf.iter().copied().find(|&c| c > 0).unwrap_or(0);
    let total = lum.len() as u32;
    if total <= cdf_min {
        return;
    }
    let denom = (total - cdf_min) as f32;
    let mut lut = [0.0f32; BINS];
    for i in 0..BINS {
        lut[i] = (cdf[i].saturating_sub(cdf_min) as f32 / denom).clamp(0.0, 1.0);
    }
    for v in lum.iter_mut() {
        let b = (v.clamp(0.0, 1.0) * (BINS - 1) as f32).round() as usize;
        *v = lut[b];
    }
}

fn equalize_clahe(lum: &mut [f32], rows: usize, cols: usize, clip_limit_norm: f32) {
    const BINS: usize = 256;
    if rows < 2 || cols < 2 {
        equalize_hist(lum);
        return;
    }
    let tiles_y = 8usize.min(rows);
    let tiles_x = 8usize.min(cols);

    let mut tile_luts = vec![[0.0f32; BINS]; tiles_y * tiles_x];
    let mut tile_centers_y = vec![0.0f32; tiles_y];
    let mut tile_centers_x = vec![0.0f32; tiles_x];

    for ty in 0..tiles_y {
        let r0 = ty * rows / tiles_y;
        let r1 = ((ty + 1) * rows / tiles_y).max(r0 + 1);
        tile_centers_y[ty] = (r0 + r1 - 1) as f32 * 0.5;
        for tx in 0..tiles_x {
            let c0 = tx * cols / tiles_x;
            let c1 = ((tx + 1) * cols / tiles_x).max(c0 + 1);
            if ty == 0 {
                tile_centers_x[tx] = (c0 + c1 - 1) as f32 * 0.5;
            }

            let mut hist = [0u32; BINS];
            let count = ((r1 - r0) * (c1 - c0)) as u32;
            for r in r0..r1 {
                let row_off = r * cols;
                for c in c0..c1 {
                    let b = (lum[row_off + c].clamp(0.0, 1.0) * (BINS - 1) as f32).round() as usize;
                    hist[b] += 1;
                }
            }

            let clip = ((clip_limit_norm * count as f32).round() as u32).max(1);
            let mut excess = 0u32;
            for bin in &mut hist {
                if *bin > clip {
                    excess += *bin - clip;
                    *bin = clip;
                }
            }
            let add_each = excess / BINS as u32;
            let rem = (excess % BINS as u32) as usize;
            for (i, bin) in hist.iter_mut().enumerate() {
                *bin += add_each + if i < rem { 1 } else { 0 };
            }

            let mut cdf = 0u32;
            let inv_count = 1.0 / count.max(1) as f32;
            let lut = &mut tile_luts[ty * tiles_x + tx];
            for i in 0..BINS {
                cdf += hist[i];
                lut[i] = (cdf as f32 * inv_count).clamp(0.0, 1.0);
            }
        }
    }

    for r in 0..rows {
        let (ty0, ty1, wy) = interp_indices(r as f32, &tile_centers_y);
        let row_off = r * cols;
        for c in 0..cols {
            let (tx0, tx1, wx) = interp_indices(c as f32, &tile_centers_x);
            let b = (lum[row_off + c].clamp(0.0, 1.0) * (BINS - 1) as f32).round() as usize;
            let v00 = tile_luts[ty0 * tiles_x + tx0][b];
            let v10 = tile_luts[ty0 * tiles_x + tx1][b];
            let v01 = tile_luts[ty1 * tiles_x + tx0][b];
            let v11 = tile_luts[ty1 * tiles_x + tx1][b];
            let top = v00 * (1.0 - wx) + v10 * wx;
            let bot = v01 * (1.0 - wx) + v11 * wx;
            lum[row_off + c] = (top * (1.0 - wy) + bot * wy).clamp(0.0, 1.0);
        }
    }
}

fn interp_indices(pos: f32, centers: &[f32]) -> (usize, usize, f32) {
    if centers.len() <= 1 || pos <= centers[0] {
        return (0, 0, 0.0);
    }
    let last = centers.len() - 1;
    if pos >= centers[last] {
        return (last, last, 0.0);
    }
    let mut lo = 0;
    for i in 0..last {
        if pos >= centers[i] && pos <= centers[i + 1] {
            lo = i;
            break;
        }
    }
    let hi = lo + 1;
    let span = (centers[hi] - centers[lo]).max(1e-6);
    (lo, hi, ((pos - centers[lo]) / span).clamp(0.0, 1.0))
}

/// Separable 2D Gaussian filter with replicate border padding.
pub fn gaussian_blur_2d(input: &[f32], rows: usize, cols: usize, sigma: f32) -> Vec<f32> {
    let radius = (3.0 * sigma).ceil().max(1.0) as isize;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut sum = 0.0f32;
    for i in -radius..=radius {
        let w = (-(i * i) as f32 / two_sigma_sq).exp();
        kernel.push(w);
        sum += w;
    }
    for w in &mut kernel {
        *w /= sum;
    }

    // Horizontal pass
    let mut horiz = vec![0.0f32; rows * cols];
    for r in 0..rows {
        let row_off = r * cols;
        for c in 0..cols {
            let mut acc = 0.0f32;
            for (k_idx, &kw) in kernel.iter().enumerate() {
                let offset = k_idx as isize - radius;
                let sc = (c as isize + offset).clamp(0, cols as isize - 1) as usize;
                acc += input[row_off + sc] * kw;
            }
            horiz[row_off + c] = acc;
        }
    }

    // Vertical pass
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let mut acc = 0.0f32;
            for (k_idx, &kw) in kernel.iter().enumerate() {
                let offset = k_idx as isize - radius;
                let sr = (r as isize + offset).clamp(0, rows as isize - 1) as usize;
                acc += horiz[sr * cols + c] * kw;
            }
            out[r * cols + c] = acc;
        }
    }
    out
}
