//! Multi-lithophane style: multiple parallel panels along Y sharing outer supports
//! and flat-bottomed breakaway bridge connectors.

use image::imageops::{self, FilterType};
use manifold_rust::manifold::Manifold;

use crate::error::LithoError;
use crate::heightmap::{luminance_to_thickness, rot90_cw, ThicknessMap};
use crate::image_proc::{
    adjust_luminance, load_image_rgb, plan_size_explicit, prepare_source, to_greyscale,
};
use crate::mesh::{
    compose_manifolds, hull_manifold, manifold_to_mesh, mesh_to_manifold, union_manifolds, MeshData,
};
use crate::preview::image_frame_previews;
use crate::scene::{
    BridgeReportInfo, DimensionsMm, GeneratedScene, GenerationOptions, GenerationReport,
    ImageInput, ImagePreviews, MaterialSpec, PanelReportInfo, PartRole, ScenePart,
    SupportReportInfo, ThicknessRangeReport,
};
use crate::settings::{LithophaneSettings, MatchDimensionMode, ReliefSide, StyleId, SupportOutput};
use crate::styles::flat::{build_flat_panel_mesh, round2, round3, round4};
use crate::styles::{LithophaneStyle, StyleDescriptor};
use crate::supports::{
    linspace, max_thickness_strip, plan_supports, surface_thickness, SupportPlan,
};

/// Multi-lithophane parallel panel style with shared outer fins and breakaway bridges.
pub struct MultiFlatStyle;

impl LithophaneStyle for MultiFlatStyle {
    fn descriptor(&self) -> StyleDescriptor {
        StyleDescriptor {
            id: StyleId::MultiFlat,
            name: "Multi",
            tagline: "Multiple parallel panels linked by bridges & outer fins",
            min_images: 2,
            max_images: None,
        }
    }

    fn validate(
        &self,
        inputs: &[ImageInput],
        settings: &LithophaneSettings,
    ) -> Result<(), LithoError> {
        settings.validate()?;
        if inputs.len() < 2 {
            return Err(LithoError::Settings(format!(
                "multi_flat style requires at least 2 images, got {}",
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

        let n = inputs.len();
        let mut tms = Vec::with_capacity(n);
        let mut previews: Vec<ImagePreviews> = Vec::with_capacity(n);
        let mut stems = Vec::with_capacity(n);
        let mut modes = Vec::with_capacity(n);

        for (i, item) in inputs.iter().enumerate() {
            progress(
                &format!("Processing image {}/{n}", i + 1),
                0.05 + 0.25 * (i as f32 / n as f32),
            );
            let mode = item
                .mapping_mode
                .or_else(|| eff_settings.multi.item_modes.get(i).copied())
                .unwrap_or(MatchDimensionMode::LongSide);
            let stem = item.source.stem();
            let tm = build_multi_thickness_map(&item.source, &eff_settings, mode)?;
            previews.push(image_frame_previews(&stem, &tm, &eff_settings)?);
            stems.push(stem);
            modes.push(mode);
            tms.push(tm);
        }

        let y_positions = compute_panel_y_positions(&tms, &eff_settings);

        progress("Building panel meshes", 0.35);
        let mut parts = Vec::new();
        let mut panel_meshes = Vec::with_capacity(n);
        let mut panel_reports = Vec::with_capacity(n);

        for (i, tm) in tms.iter().enumerate() {
            let mut p_mesh = build_flat_panel_mesh(tm, eff_settings.thickness.relief_side);
            p_mesh.translate_y(y_positions[i]);
            let role_label = if i == 0 {
                "Front".to_string()
            } else if i == n - 1 {
                "Back".to_string()
            } else {
                format!("Middle {i}")
            };
            panel_reports.push(PanelReportInfo {
                index: i,
                source_stem: stems[i].clone(),
                mode: modes[i],
                dimensions_mm: DimensionsMm {
                    x: round3(tm.length_x()),
                    y: round3(tm.max_thickness()),
                    z: round3(tm.height_z()),
                },
                y_ref_mm: round3(y_positions[i]),
                triangles: p_mesh.indices.len(),
            });
            parts.push(ScenePart {
                id: format!("panel_{i}"),
                name: format!("Panel {} ({role_label}): {}", i + 1, stems[i]),
                role: PartRole::Panel,
                material: MaterialSpec::ivory_pla(),
                mesh: p_mesh.clone(),
            });
            panel_meshes.push(p_mesh);
        }

        let mut warnings = Vec::new();
        let mut sup_report = None;
        let mut bridge_report = None;
        let mut sup_manifold: Option<Manifold> = None;

        if eff_settings.supports.enabled {
            progress("Building outer supports & bridges", 0.58);
            let (sm, sup_mesh, bridges_mesh, plan, b_info) =
                build_multi_supports(&tms, &y_positions, &eff_settings)?;
            warnings.extend(plan.warnings);
            sup_report = Some(SupportReportInfo {
                fin_height_mm: round2(plan.fin_height),
                fin_base_length_mm: round2(plan.fin_base_length),
                fin_centres_x_mm: plan.fin_centres_x.iter().map(|&x| round2(x)).collect(),
                sides: "front & back only".into(),
            });
            bridge_report = Some(b_info);
            sup_manifold = Some(sm);
            parts.push(ScenePart {
                id: "supports".into(),
                name: "Outer Support Fins".into(),
                role: PartRole::Support,
                material: MaterialSpec::support_amber(),
                mesh: sup_mesh,
            });
            parts.push(ScenePart {
                id: "bridges".into(),
                name: "Breakaway Bridges".into(),
                role: PartRole::Bridge,
                material: MaterialSpec::support_amber(),
                mesh: bridges_mesh,
            });
        }

        let (combined_mesh, volume_mm3, bodies) = if opts.skip_final_boolean_merge {
            let refs: Vec<&MeshData> = parts.iter().map(|p| &p.mesh).collect();
            let cat = MeshData::concatenate(&refs);
            let approx_vol = cat.signed_volume().max(0.0);
            (None, approx_vol, None)
        } else {
            progress("Validating & joining multi-lithophane", 0.78);
            let mut panel_manifolds = Vec::with_capacity(n);
            for (i, pm) in panel_meshes.iter().enumerate() {
                panel_manifolds.push(mesh_to_manifold(pm, &format!("panel_{i}"))?);
            }

            let final_manifold = match (&sup_manifold, eff_settings.supports.output) {
                (Some(sm), SupportOutput::Merged) => {
                    let mut all = panel_manifolds;
                    all.push(sm.clone());
                    union_manifolds(&all, "merged multi-lithophane")?
                }
                (Some(sm), SupportOutput::SeparateBodies | SupportOutput::SeparateFiles) => {
                    let mut all = panel_manifolds;
                    all.push(sm.clone());
                    compose_manifolds(&all)
                }
                (None, _) => compose_manifolds(&panel_manifolds),
            };
            let vol = final_manifold.volume();
            let body_count = if final_manifold.num_tri() < 400_000 {
                let b = final_manifold.decompose().len();
                if eff_settings.supports.enabled
                    && eff_settings.supports.output == SupportOutput::Merged
                    && b != 1
                {
                    warnings.push(format!("Merged multi-model has {b} bodies; expected 1."));
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

        let min_t = tms
            .iter()
            .map(|t| t.min_thickness())
            .fold(f64::INFINITY, f64::min);
        let max_t = tms
            .iter()
            .map(|t| t.max_thickness())
            .fold(0.0f64, f64::max);
        let filament_g = (volume_mm3 / 1000.0) * eff_settings.export.pla_density_g_cm3;

        progress("Done", 1.0);
        Ok(GeneratedScene {
            style: StyleId::MultiFlat,
            settings: eff_settings,
            parts,
            combined: combined_mesh,
            previews,
            report: GenerationReport {
                style: StyleId::MultiFlat,
                dimensions_mm: DimensionsMm {
                    x: round3(tms[0].length_x()),
                    y: round3(bmax[1] - bmin[1]),
                    z: round3(
                        tms.iter()
                            .map(|t| t.height_z())
                            .fold(0.0f64, f64::max),
                    ),
                },
                overall_dimensions_mm: DimensionsMm {
                    x: round3(bmax[0] - bmin[0]),
                    y: round3(bmax[1] - bmin[1]),
                    z: round3(bmax[2] - bmin[2]),
                },
                grid: None,
                rotated_for_print: tms.iter().any(|t| t.rot_k != 0),
                thickness_mm: ThicknessRangeReport {
                    min: round4(min_t),
                    max: round4(max_t),
                },
                triangles,
                volume_mm3: round2(volume_mm3),
                filament_g_estimate: round2(filament_g),
                bodies,
                curved_info: None,
                supports: sup_report,
                bridges: bridge_report,
                panels: panel_reports,
                warnings,
            },
            source_stems: stems,
        })
    }
}

/// Resolve outer `(w, h)` in the upright image frame and rotation `k` (`0` or `-1`)
/// so the selected dimension sits along the print bed (`X`).
pub fn resolve_item_dimensions(
    aspect: f64,
    common_dim: f64,
    mode: MatchDimensionMode,
) -> (f64, f64, i32) {
    match mode {
        MatchDimensionMode::LongSide => {
            if aspect >= 1.0 {
                (common_dim, common_dim / aspect, 0)
            } else {
                (common_dim * aspect, common_dim, -1)
            }
        }
        MatchDimensionMode::ShortSide => {
            if aspect >= 1.0 {
                (common_dim * aspect, common_dim, -1)
            } else {
                (common_dim, common_dim / aspect, 0)
            }
        }
        MatchDimensionMode::Width => (common_dim, common_dim / aspect, 0),
        MatchDimensionMode::Height => (common_dim * aspect, common_dim, -1),
    }
}

/// Build the oriented [`ThicknessMap`] for one panel inside a multi-lithophane set.
pub fn build_multi_thickness_map(
    source: &crate::scene::ImageSource,
    settings: &LithophaneSettings,
    mode: MatchDimensionMode,
) -> Result<ThicknessMap, LithoError> {
    let raw = load_image_rgb(source, settings.image.background)?;
    let img = prepare_source(raw, &settings.image)?;
    let aspect = img.width() as f64 / img.height() as f64;
    let common_dim = settings.multi.common_dimension_mm;
    let (ow, oh, rot_k) = resolve_item_dimensions(aspect, common_dim, mode);

    let fw = if settings.frame.enabled {
        settings.frame.width_mm
    } else {
        0.0
    };
    let plan = plan_size_explicit(ow, oh, fw, settings.size.pixel_size_mm)?;
    let dx = plan.outer_w_mm / (plan.nx - 1) as f64;
    let mut dy = plan.outer_h_mm / (plan.ny - 1) as f64;

    let resized = imageops::resize(
        &img,
        plan.inner_nx() as u32,
        plan.inner_ny() as u32,
        FilterType::Lanczos3,
    );
    let raw_lum = to_greyscale(&resized, settings.image.greyscale);
    let lum = adjust_luminance(
        &raw_lum,
        plan.inner_ny(),
        plan.inner_nx(),
        &settings.image,
        dx,
        dy,
    );

    let thick = luminance_to_thickness(&lum, &settings.thickness);
    let ft = settings
        .frame
        .thickness_mm
        .unwrap_or(settings.thickness.max_thickness_mm);

    let (mut grid, mut rows, mut cols) = if settings.frame.enabled && plan.frame_mm > 0.0 {
        let mut full = vec![ft as f32; plan.ny * plan.nx];
        for r in 0..plan.inner_ny() {
            let dst_r = plan.frame_px_y + r;
            let dst_off = dst_r * plan.nx + plan.frame_px_x;
            let src_off = r * plan.inner_nx();
            full[dst_off..dst_off + plan.inner_nx()]
                .copy_from_slice(&thick[src_off..src_off + plan.inner_nx()]);
        }
        (full, plan.ny, plan.nx)
    } else {
        (thick, plan.ny, plan.nx)
    };

    let ext = &settings.bottom_extension;
    if ext.enabled && ext.height_mm > 0.0 {
        let ext_rows = ((ext.height_mm / dy).round() as usize).max(1);
        grid.resize((rows + ext_rows) * cols, ext.thickness_mm as f32);
        rows += ext_rows;
    }

    let mut dx_final = dx;
    let mut fpx = plan.frame_px_x;
    let mut fpy = plan.frame_px_y;
    if rot_k != 0 {
        let (rot, nr, nc) = rot90_cw(&grid, rows, cols);
        grid = rot;
        rows = nr;
        cols = nc;
        std::mem::swap(&mut dx_final, &mut dy);
        std::mem::swap(&mut fpx, &mut fpy);
    }

    Ok(ThicknessMap {
        thickness: grid,
        rows,
        cols,
        dx: dx_final,
        dz: dy,
        frame_px_x: fpx,
        frame_px_z: fpy,
        rot_k,
        luminance: lum,
        lum_rows: plan.inner_ny(),
        lum_cols: plan.inner_nx(),
        plan,
        frame_thickness_mm: ft,
    })
}

/// Compute reference Y coordinates for each parallel panel, centered around `Y = 0`.
pub fn compute_panel_y_positions(
    tms: &[ThicknessMap],
    settings: &LithophaneSettings,
) -> Vec<f64> {
    let n = tms.len();
    let spacing = settings.multi.panel_spacing_mm;
    let relief_side = settings.thickness.relief_side;

    let mut y_refs = Vec::with_capacity(n);
    y_refs.push(0.0f64);
    for i in 0..(n - 1) {
        let t_prev = tms[i].max_thickness();
        let t_next = tms[i + 1].max_thickness();
        let y_next = match relief_side {
            ReliefSide::Front => y_refs[i] + t_next + spacing,
            ReliefSide::Back => y_refs[i] + t_prev + spacing,
        };
        y_refs.push(y_next);
    }

    let (y_min, y_max) = match relief_side {
        ReliefSide::Front => (y_refs[0] - tms[0].max_thickness(), *y_refs.last().unwrap()),
        ReliefSide::Back => (y_refs[0], *y_refs.last().unwrap() + tms[n - 1].max_thickness()),
    };
    let y_mid = (y_min + y_max) * 0.5;
    y_refs.into_iter().map(|y| y - y_mid).collect()
}

/// Build outer front/back support fins and flat-bottomed inter-panel breakaway bridges.
pub fn build_multi_supports(
    tms: &[ThicknessMap],
    y_positions: &[f64],
    settings: &LithophaneSettings,
) -> Result<(Manifold, MeshData, MeshData, SupportPlan, BridgeReportInfo), LithoError> {
    let s = &settings.supports;
    let m_cfg = &settings.multi;
    let relief_side = settings.thickness.relief_side;
    let n = tms.len();

    let front_tm = &tms[0];
    let back_tm = &tms[n - 1];
    let y_front_ref = y_positions[0];
    let y_back_ref = y_positions[n - 1];

    let plan = plan_supports(front_tm, settings)?;
    let ft = s.fin_thickness_mm;
    let tw = s.tab_width_mm.unwrap_or(ft);

    let mut fin_parts: Vec<Manifold> = Vec::new();
    let mut bridge_parts: Vec<Manifold> = Vec::new();

    // 1. Front Support Fins (Panel 0, facing -Y)
    for &xc in &plan.fin_centres_x {
        let xl = xc - ft / 2.0;
        let xr = xc + ft / 2.0;
        let m = if relief_side == ReliefSide::Front {
            max_thickness_strip(front_tm, xl, xr)
        } else {
            0.0
        };
        let y_e = (y_front_ref - m) - s.interface_gap_mm;
        let y_out = y_e - plan.fin_base_length;
        let h = plan.fin_height.min(front_tm.height_z());

        fin_parts.push(hull_manifold(
            &[
                [xl, y_e, 0.0],
                [xl, y_out, 0.0],
                [xl, y_e, h],
                [xr, y_e, 0.0],
                [xr, y_out, 0.0],
                [xr, y_e, h],
            ],
            "multi front fin",
        )?);

        if s.pad_enabled {
            let pt = s.pad_thickness_mm;
            let px0 = xl - s.pad_side_margin_mm;
            let px1 = xr + s.pad_side_margin_mm;
            let py_out = y_out - s.pad_end_margin_mm;
            fin_parts.push(hull_manifold(
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
                "multi front pad",
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
            let t_loc = if relief_side == ReliefSide::Front {
                surface_thickness(
                    front_tm,
                    xc - tw / 2.0,
                    xc + tw / 2.0,
                    zc - s.tab_height_mm / 2.0,
                    zc + s.tab_height_mm / 2.0,
                )
            } else {
                0.0
            };
            let y_start = y_e - 0.2;
            let y_end = (y_front_ref - t_loc) + s.tab_overlap_mm;
            let hh = s.tab_height_mm / 2.0;
            let nh = s.tab_neck_height_mm / 2.0;
            let xa = xc - tw / 2.0;
            let xb = xc + tw / 2.0;
            fin_parts.push(hull_manifold(
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
                "multi front tab",
            )?);
        }
    }

    // 2. Back Support Fins (Panel N-1, facing +Y)
    let back_h = plan.fin_height.min(back_tm.height_z());
    for &xc in &plan.fin_centres_x {
        let xl = xc - ft / 2.0;
        let xr = xc + ft / 2.0;
        let m = if relief_side == ReliefSide::Back {
            max_thickness_strip(back_tm, xl, xr)
        } else {
            0.0
        };
        let y_e = (y_back_ref + m) + s.interface_gap_mm;
        let y_out = y_e + plan.fin_base_length;
        let h = back_h;

        fin_parts.push(hull_manifold(
            &[
                [xl, y_e, 0.0],
                [xl, y_out, 0.0],
                [xl, y_e, h],
                [xr, y_e, 0.0],
                [xr, y_out, 0.0],
                [xr, y_e, h],
            ],
            "multi back fin",
        )?);

        if s.pad_enabled {
            let pt = s.pad_thickness_mm;
            let px0 = xl - s.pad_side_margin_mm;
            let px1 = xr + s.pad_side_margin_mm;
            let py_out = y_out + s.pad_end_margin_mm;
            fin_parts.push(hull_manifold(
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
                "multi back pad",
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
            let t_loc = if relief_side == ReliefSide::Back {
                surface_thickness(
                    back_tm,
                    xc - tw / 2.0,
                    xc + tw / 2.0,
                    zc - s.tab_height_mm / 2.0,
                    zc + s.tab_height_mm / 2.0,
                )
            } else {
                0.0
            };
            let y_start = y_e + 0.2;
            let y_end = (y_back_ref + t_loc) - s.tab_overlap_mm;
            let hh = s.tab_height_mm / 2.0;
            let nh = s.tab_neck_height_mm / 2.0;
            let xa = xc - tw / 2.0;
            let xb = xc + tw / 2.0;
            fin_parts.push(hull_manifold(
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
                "multi back tab",
            )?);
        }
    }

    // 3. Inter-panel flat-bottomed rectangular breakaway bridge connectors
    let mut bridge_stations = if let Some(inset) = m_cfg.bridge_inset_mm {
        let half_x = front_tm.length_x() * 0.5;
        vec![-half_x + inset, half_x - inset]
    } else {
        plan.fin_centres_x.clone()
    };
    if m_cfg.add_center_bridge && bridge_stations.iter().all(|&x| x.abs() > 1.0) {
        bridge_stations.push(0.0);
    }
    bridge_stations.sort_by(|a, b| a.total_cmp(b));

    let mut total_bridges = 0usize;
    let bw = m_cfg.bridge_width_mm * 0.5;
    let bh = m_cfg.bridge_height_mm * 0.5;
    let overlap = m_cfg.bridge_overlap_mm;

    for i in 0..(n - 1) {
        let tm_a = &tms[i];
        let tm_b = &tms[i + 1];
        let y_ref_a = y_positions[i];
        let y_ref_b = y_positions[i + 1];

        let h_common = tm_a.height_z().min(tm_b.height_z());
        let z_min = m_cfg.margin_z_mm + bh;
        let z_max = h_common - m_cfg.margin_z_mm - bh;
        let z_bridges = if z_max <= z_min {
            vec![h_common * 0.5]
        } else {
            linspace(z_min, z_max, m_cfg.bridge_count)
        };

        for &xc in &bridge_stations {
            for &zc in &z_bridges {
                total_bridges += 1;
                let y_near = if relief_side == ReliefSide::Front {
                    y_ref_a
                } else {
                    let t_a = surface_thickness(tm_a, xc - bw, xc + bw, zc - bh, zc + bh);
                    y_ref_a + t_a
                };
                let y_far = if relief_side == ReliefSide::Front {
                    let t_b = surface_thickness(tm_b, xc - bw, xc + bw, zc - bh, zc + bh);
                    y_ref_b - t_b
                } else {
                    y_ref_b
                };

                let ya = y_near - overlap;
                let yb = y_far + overlap;

                bridge_parts.push(hull_manifold(
                    &[
                        [xc - bw, ya, zc - bh],
                        [xc + bw, ya, zc - bh],
                        [xc - bw, ya, zc + bh],
                        [xc + bw, ya, zc + bh],
                        [xc - bw, yb, zc - bh],
                        [xc + bw, yb, zc - bh],
                        [xc - bw, yb, zc + bh],
                        [xc + bw, yb, zc + bh],
                    ],
                    "inter-panel bridge",
                )?);
            }
        }
    }

    let fin_manifold = union_manifolds(&fin_parts, "multi outer fins")?;
    let bridge_manifold = union_manifolds(&bridge_parts, "multi bridges")?;
    let combined_sup = union_manifolds(&[fin_manifold.clone(), bridge_manifold.clone()], "multi supports")?;

    let fin_mesh = manifold_to_mesh(&fin_manifold);
    let bridge_mesh = manifold_to_mesh(&bridge_manifold);

    let bridge_info = BridgeReportInfo {
        stations_x_mm: bridge_stations.iter().map(|&x| round2(x)).collect(),
        bridges_per_station: m_cfg.bridge_count,
        total_bridges,
    };

    Ok((combined_sup, fin_mesh, bridge_mesh, plan, bridge_info))
}
