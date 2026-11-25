use std::io::Cursor;

use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder, RgbaImage};
use litho_core::generate;
use litho_core::mesh::{mesh_to_manifold, write_3mf, write_stl_ascii, write_stl_binary};
use litho_core::scene::{GenerationOptions, ImageInput, PartRole};
use litho_core::settings::{
    CurvatureDirection, EqualizeMode, LithophaneSettings, MatchDimensionMode, ReliefSide, StyleId,
    SupportOutput, ThicknessCurve,
};
use litho_core::styles::curved::panel_radial_bounds;
use zip::ZipArchive;

fn encode_rgb_png(w: u32, h: u32, mode: &str) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / (w.max(2) - 1) as f32;
            let fy = y as f32 / (h.max(2) - 1) as f32;
            let (r, g, b) = match mode {
                "checker" => {
                    let v = if ((x / 8) + (y / 8)) % 2 == 0 { 230 } else { 25 };
                    (v, v, v)
                }
                _ => (
                    (fx * 255.0).round() as u8,
                    (fy * 255.0).round() as u8,
                    (((1.0 - fx) * 0.5 + fy * 0.5) * 255.0).round() as u8,
                ),
            };
            raw.push(r);
            raw.push(g);
            raw.push(b);
        }
    }
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(&raw, w, h, ColorType::Rgb8.into())
        .unwrap();
    out
}

fn encode_rgba_png(w: u32, h: u32) -> Vec<u8> {
    let mut img = RgbaImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let alpha = if x < w / 2 { 0 } else { 255 };
        *px = image::Rgba([(x * 4) as u8, (y * 4) as u8, 200, alpha]);
    }
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(img.as_raw(), w, h, ColorType::Rgba8.into())
        .unwrap();
    out
}

fn fast_settings() -> LithophaneSettings {
    let mut s = LithophaneSettings::default();
    s.size.max_dimension_mm = 60.0;
    s.size.pixel_size_mm = 0.8;
    s
}

#[test]
fn image_pipeline_and_tonal_adjustments_run_cleanly() {
    let png = encode_rgb_png(64, 48, "grad");
    let mut s = fast_settings();
    s.image.auto_levels_percent = 2.0;
    s.image.equalize = EqualizeMode::Clahe;
    s.image.brightness = 0.05;
    s.image.contrast = 1.15;
    s.image.gamma = 0.9;
    s.image.blur_sigma_mm = 0.3;
    s.image.sharpen_amount = 0.4;
    s.image.posterize_levels = Some(12);
    s.image.rotate = 90;
    s.image.mirror = true;
    s.thickness.curve = ThicknessCurve::Transmission;

    let mut prog = |_: &str, _: f32| {};
    let scene = generate(
        StyleId::Flat,
        &[ImageInput::from_bytes(png, "grad")],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect("tonal pipeline flat generation should succeed");

    let combined = scene.combined.as_ref().unwrap();
    assert!(combined.signed_volume() > 0.0);
    assert_eq!(scene.report.bodies, Some(1));
    assert!(!scene.previews[0].backlit_png.is_empty());
}

#[test]
fn rgba_transparency_flattens_cleanly() {
    let rgba = encode_rgba_png(48, 36);
    let mut s = fast_settings();
    s.supports.enabled = false;
    let mut prog = |_: &str, _: f32| {};
    let scene = generate(
        StyleId::Flat,
        &[ImageInput::from_bytes(rgba, "alpha")],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect("rgba image should flatten and mesh cleanly");
    assert!(scene.combined.as_ref().unwrap().signed_volume() > 0.0);
}

#[test]
fn flat_front_back_and_support_output_modes_are_manifold() {
    let png = encode_rgb_png(60, 40, "checker");
    let mut prog = |_: &str, _: f32| {};

    for relief in [ReliefSide::Front, ReliefSide::Back] {
        let mut s = fast_settings();
        s.thickness.relief_side = relief;
        s.supports.enabled = true;
        s.supports.output = SupportOutput::Merged;

        let scene = generate(
            StyleId::Flat,
            &[ImageInput::from_bytes(png.clone(), "flat")],
            &s,
            &GenerationOptions::default(),
            &mut prog,
        )
        .expect("flat merged should succeed");
        let combined = scene.combined.as_ref().unwrap();
        let m = mesh_to_manifold(combined, "flat merged").expect("must be manifold");
        assert!(m.volume() > 0.0);
        assert_eq!(scene.report.bodies, Some(1));
    }

    let mut s_sep = fast_settings();
    s_sep.supports.output = SupportOutput::SeparateBodies;
    let scene_sep = generate(
        StyleId::Flat,
        &[ImageInput::from_bytes(png, "flat_sep")],
        &s_sep,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect("separate bodies should succeed");
    assert!(scene_sep.report.bodies.unwrap_or(0) >= 2);
}

#[test]
fn curved_all_directions_and_relief_sides_are_watertight_with_exact_base_bounds() {
    let png = encode_rgb_png(64, 48, "grad");
    let mut prog = |_: &str, _: f32| {};

    for direction in [CurvatureDirection::Concave, CurvatureDirection::Convex] {
        for relief in [ReliefSide::Front, ReliefSide::Back] {
            for base_enabled in [true, false] {
                let mut s = fast_settings();
                s.size.max_dimension_mm = 70.0;
                s.size.pixel_size_mm = 0.8;
                s.curved.radius_mm = 60.0;
                s.curved.direction = direction;
                s.thickness.relief_side = relief;
                s.curved.base.enabled = base_enabled;
                s.curved.base.thickness_mm = 1.6;
                s.curved.base.front_projection_mm = 6.0;
                s.curved.base.back_projection_mm = 6.0;
                s.curved.base.corner_fillet_mm = 3.5;

                let scene = generate(
                    StyleId::Curved,
                    &[ImageInput::from_bytes(png.clone(), "curved")],
                    &s,
                    &GenerationOptions::default(),
                    &mut prog,
                )
                .expect("curved generation must succeed");

                let combined = scene.combined.as_ref().unwrap();
                let m = mesh_to_manifold(combined, "curved combined").expect("must be manifold");
                assert!(m.volume() > 0.0);
                assert_eq!(scene.report.bodies, Some(1));

                let (bmin, bmax) = combined.bounds();
                assert!(bmin[2].abs() < 0.05, "bottom should sit on Z=0 bed, got {}", bmin[2]);

                if base_enabled {
                    let base_part = scene
                        .parts
                        .iter()
                        .find(|p| p.role == PartRole::Base)
                        .expect("base part must exist");
                    let (base_min, base_max) = base_part.mesh.bounds();
                    assert!(base_min[2].abs() < 1e-4);
                    assert!((base_max[2] - 1.6).abs() < 1e-4);

                    // Verify base does NOT project sideways beyond [-theta/2, +theta/2]
                    let length_x = scene.report.dimensions_mm.x;
                    let radius = s.curved.radius_mm;
                    let half_theta = (length_x / radius) * 0.5;
                    let (r_pmin, r_pmax) =
                        panel_radial_bounds(radius, s.thickness.max_thickness_mm, direction, relief);
                    let (inner_proj, outer_proj) = match direction {
                        CurvatureDirection::Concave => (
                            s.curved.base.front_projection_mm,
                            s.curved.base.back_projection_mm,
                        ),
                        CurvatureDirection::Convex => (
                            s.curved.base.back_projection_mm,
                            s.curved.base.front_projection_mm,
                        ),
                    };
                    let expected_r_in = r_pmin - inner_proj;
                    let expected_r_out = r_pmax + outer_proj;
                    let mut min_r = f64::INFINITY;
                    let mut max_r = 0.0f64;

                    for &[x, y, _z] in &base_part.mesh.positions {
                        let (rx, ry) = match direction {
                            CurvatureDirection::Concave => (x, y + radius),
                            CurvatureDirection::Convex => (x, radius - y),
                        };
                        let phi = rx.atan2(ry);
                        let r = rx.hypot(ry);
                        min_r = min_r.min(r);
                        max_r = max_r.max(r);
                        assert!(
                            phi.abs() <= half_theta + 1e-3,
                            "base projected sideways: |phi|={} > half_theta={}",
                            phi.abs(),
                            half_theta
                        );
                        assert!(
                            r >= expected_r_in - 1e-3 && r <= expected_r_out + 1e-3,
                            "base radial vertex r={r} outside [{expected_r_in}, {expected_r_out}]"
                        );
                    }
                    assert!(min_r < r_pmin, "base must project inward past panel");
                    assert!(max_r > r_pmax, "base must project outward past panel");

                    let unfilleted_area = 0.5
                        * (length_x / radius)
                        * (expected_r_out * expected_r_out - expected_r_in * expected_r_in);
                    let base_vol = base_part.mesh.signed_volume();
                    assert!(
                        base_vol < unfilleted_area * 1.6,
                        "filleted base volume ({base_vol}) should be slightly less than unfilleted ({})",
                        unfilleted_area * 1.6
                    );
                    assert!(
                        base_vol > (unfilleted_area - 2.0 * 3.5 * 3.5) * 1.6,
                        "filleted base volume ({base_vol}) too small vs unfilleted ({}) - check fillet sweep",
                        unfilleted_area * 1.6
                    );

                    assert!(
                        bmax[2] > 1.6 + 10.0,
                        "panel must rise above 1.6 mm base, got zmax={}",
                        bmax[2]
                    );
                }
            }
        }
    }
}

#[test]
fn curved_with_radial_supports_merges_cleanly() {
    let png = encode_rgb_png(64, 48, "grad");
    let mut s = fast_settings();
    s.size.max_dimension_mm = 70.0;
    s.size.pixel_size_mm = 0.8;
    s.curved.radius_mm = 80.0;
    s.curved.base.enabled = true;
    s.curved.supports_enabled = true;

    let mut prog = |_: &str, _: f32| {};
    let scene = generate(
        StyleId::Curved,
        &[ImageInput::from_bytes(png, "curved_sup")],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect("curved with supports must merge into single manifold");
    assert_eq!(scene.report.bodies, Some(1));
}

#[test]
fn curved_rejects_impossibly_tight_radius() {
    let png = encode_rgb_png(64, 48, "grad");
    let mut s = fast_settings();
    s.size.max_dimension_mm = 150.0;
    s.curved.radius_mm = 10.0;

    let mut prog = |_: &str, _: f32| {};
    let err = generate(
        StyleId::Curved,
        &[ImageInput::from_bytes(png, "too_tight")],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect_err("over-340-deg arc must return a geometry error");
    assert!(err.to_string().contains("Minimum radius"));
}

#[test]
fn multi_flat_aligns_bed_width_and_merges_bridges_into_one_body() {
    let land = encode_rgb_png(60, 40, "grad");
    let port = encode_rgb_png(40, 60, "checker");

    let mut s = fast_settings();
    s.size.pixel_size_mm = 0.8;
    s.multi.common_dimension_mm = 70.0;
    s.multi.panel_spacing_mm = 10.0;
    s.multi.add_center_bridge = true;

    let mut in1 = ImageInput::from_bytes(land, "land");
    in1.mapping_mode = Some(MatchDimensionMode::LongSide);
    let mut in2 = ImageInput::from_bytes(port, "port");
    in2.mapping_mode = Some(MatchDimensionMode::LongSide);

    let mut prog = |_: &str, _: f32| {};
    let scene = generate(
        StyleId::MultiFlat,
        &[in1, in2],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .expect("multi_flat generation must succeed");

    assert_eq!(scene.report.panels.len(), 2);
    assert!((scene.report.panels[0].dimensions_mm.x - 70.0).abs() < 0.05);
    assert!((scene.report.panels[1].dimensions_mm.x - 70.0).abs() < 0.05);
    assert_eq!(scene.report.bodies, Some(1));
    assert!(scene.report.bridges.as_ref().unwrap().total_bridges >= 6);
}

#[test]
fn stl_and_3mf_exporters_produce_valid_archives() {
    let png = encode_rgb_png(48, 36, "grad");
    let mut s = fast_settings();
    s.curved.radius_mm = 80.0;
    s.curved.base.enabled = true;

    let mut prog = |_: &str, _: f32| {};
    let scene = generate(
        StyleId::Curved,
        &[ImageInput::from_bytes(png, "exp")],
        &s,
        &GenerationOptions::default(),
        &mut prog,
    )
    .unwrap();

    let combined = scene.combined.as_ref().unwrap();
    let mut bin_stl = Vec::new();
    write_stl_binary(combined, &mut bin_stl).unwrap();
    assert_eq!(bin_stl.len(), 84 + 50 * combined.indices.len());

    let mut asc_stl = Vec::new();
    write_stl_ascii(combined, &mut asc_stl).unwrap();
    let asc_text = String::from_utf8(asc_stl).unwrap();
    assert!(asc_text.starts_with("solid lithophane"));
    assert!(asc_text.trim_end().ends_with("endsolid lithophane"));

    let mut buf_3mf = Cursor::new(Vec::new());
    write_3mf(&scene.parts, &mut buf_3mf).unwrap();
    buf_3mf.set_position(0);
    let mut zip = ZipArchive::new(buf_3mf).unwrap();
    assert!(zip.by_name("[Content_Types].xml").is_ok());
    assert!(zip.by_name("_rels/.rels").is_ok());
    let mut model_file = zip.by_name("3D/3dmodel.model").unwrap();
    let mut model_xml = String::new();
    std::io::Read::read_to_string(&mut model_file, &mut model_xml).unwrap();
    assert!(model_xml.contains("basematerials"));
    assert!(model_xml.contains("<components>"));
}

#[test]
fn backlit_preview_reflects_td_and_filament_backlight_colours() {
    use image::ImageReader;
    use litho_core::preview::backlit_png;

    let grid = vec![1.0f32, 2.5f32];

    let mut s_low_td = LithophaneSettings::default();
    s_low_td.lighting.transmission_distance_mm = 2.0;
    let low_bytes = backlit_png(&grid, 1, 2, &s_low_td).unwrap();

    let mut s_high_td = LithophaneSettings::default();
    s_high_td.lighting.transmission_distance_mm = 8.0;
    let high_bytes = backlit_png(&grid, 1, 2, &s_high_td).unwrap();

    let low_img = ImageReader::new(Cursor::new(low_bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();
    let high_img = ImageReader::new(Cursor::new(high_bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();

    // Higher TD transmits more light at the same thickness
    assert!(high_img.get_pixel(1, 0)[0] > low_img.get_pixel(1, 0)[0]);

    // Warm/amber backlight shifts red strictly above blue
    let mut s_warm = LithophaneSettings::default();
    s_warm.lighting.backlight_color = "#ffb86b".to_string();
    let warm_bytes = backlit_png(&grid, 1, 2, &s_warm).unwrap();
    let warm_img = ImageReader::new(Cursor::new(warm_bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();
    let px = warm_img.get_pixel(0, 0);
    assert!(px[0] > px[2], "warm backlight should keep R > B, got {:?}", px);
}
