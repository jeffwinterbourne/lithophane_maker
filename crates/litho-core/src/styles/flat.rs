//! Flat upright lithophane panel with removable breakaway triangular support fins.

use manifold_rust::manifold::Manifold;

use crate::error::LithoError;
use crate::heightmap::{build_thickness_map, ThicknessMap};
use crate::image_proc::process_image;
use crate::mesh::{
    compose_manifolds, manifold_to_mesh, mesh_to_manifold, union_manifolds, MeshData,
};
use crate::preview::image_frame_previews;
use crate::scene::{
    DimensionsMm, GeneratedScene, GenerationOptions, GenerationReport, GridReportInfo, ImageInput,
    MaterialSpec, PartRole, ScenePart, SupportReportInfo, ThicknessRangeReport,
};
use crate::settings::{LithophaneSettings, ReliefSide, StyleId, SupportOutput};
use crate::styles::{LithophaneStyle, StyleDescriptor};
use crate::supports::build_flat_supports;

/// Classic flat upright lithophane panel style.
pub struct FlatStyle;

impl LithophaneStyle for FlatStyle {
    fn descriptor(&self) -> StyleDescriptor {
        StyleDescriptor {
            id: StyleId::Flat,
            name: "Flat",
            tagline: "Upright panel with removable snap-off support fins",
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
                "flat style requires exactly 1 image, got {}",
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
        let tm = build_thickness_map(lum, plan, &eff_settings, eff_settings.orientation.mode);

        progress("Building panel mesh", 0.35);
        let panel_mesh = build_flat_panel_mesh(&tm, eff_settings.thickness.relief_side);
        let preview_2d = image_frame_previews(&stem, &tm, &eff_settings)?;

        let mut parts = vec![ScenePart {
            id: "panel".into(),
            name: format!("{stem} Panel"),
            role: PartRole::Panel,
            material: MaterialSpec::ivory_pla(),
            mesh: panel_mesh.clone(),
        }];

        let mut warnings = Vec::new();
        let mut sup_report = None;
        let mut sup_manifold: Option<Manifold> = None;

        if eff_settings.supports.enabled {
            progress("Building supports", 0.58);
            let (sm, sup_mesh, sup_plan) = build_flat_supports(&tm, &eff_settings)?;
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
                name: "Breakaway Supports".into(),
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
            progress("Validating & joining manifold", 0.78);
            let panel_manifold = mesh_to_manifold(&panel_mesh, "flat panel")?;
            let final_manifold = match (&sup_manifold, eff_settings.supports.output) {
                (Some(sm), SupportOutput::Merged) => {
                    union_manifolds(&[panel_manifold, sm.clone()], "merged flat model")?
                }
                (Some(sm), SupportOutput::SeparateBodies | SupportOutput::SeparateFiles) => {
                    compose_manifolds(&[panel_manifold, sm.clone()])
                }
                (None, _) => panel_manifold,
            };
            let vol = final_manifold.volume();
            let body_count = if final_manifold.num_tri() < 400_000 {
                let b = final_manifold.decompose().len();
                if eff_settings.supports.output == SupportOutput::Merged && b != 1 {
                    warnings.push(format!("Merged model has {b} bodies; expected 1."));
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
        let triangles: usize = if let Some(ref c) = combined_mesh {
            c.indices.len()
        } else {
            parts.iter().map(|p| p.mesh.indices.len()).sum()
        };
        if triangles > 5_000_000 {
            warnings.push(format!(
                "{triangles} triangles: very large file; consider a larger pixel size."
            ));
        }

        let filament_g = (volume_mm3 / 1000.0) * eff_settings.export.pla_density_g_cm3;
        let max_t = tm.max_thickness();
        let min_t = tm.min_thickness();

        progress("Done", 1.0);
        Ok(GeneratedScene {
            style: StyleId::Flat,
            settings: eff_settings,
            parts,
            combined: combined_mesh,
            previews: vec![preview_2d],
            report: GenerationReport {
                style: StyleId::Flat,
                dimensions_mm: DimensionsMm {
                    x: round3(tm.length_x()),
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
                curved_info: None,
                supports: sup_report,
                bridges: None,
                panels: Vec::new(),
                warnings,
            },
            source_stems: vec![stem],
        })
    }
}

/// Grid perimeter indices `(row, col)` ordered counter-clockwise as seen from `-Y`.
pub(crate) fn perimeter_indices(rows: usize, cols: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(2 * rows + 2 * cols - 4);
    for i in 0..rows {
        out.push((i, 0));
    }
    for j in 1..cols {
        out.push((rows - 1, j));
    }
    for i in (0..rows - 1).rev() {
        out.push((i, cols - 1));
    }
    for j in (1..cols - 1).rev() {
        out.push((0, j));
    }
    out
}

/// Build the watertight flat panel shell (`relief` grid + flat `y = 0` back fan + walls).
pub fn build_flat_panel_mesh(tm: &ThicknessMap, relief_side: ReliefSide) -> MeshData {
    let rows = tm.rows;
    let cols = tm.cols;
    let len_x = tm.length_x();
    let hz = tm.height_z();

    let perim = perimeter_indices(rows, cols);
    let n_per = perim.len();
    let base_back = rows * cols;
    let centre_idx = (base_back + n_per) as u32;

    let mut positions = Vec::with_capacity(base_back + n_per + 1);
    let mut thickness = Vec::with_capacity(base_back + n_per + 1);

    for i in 0..rows {
        let z = (rows - 1 - i) as f64 * tm.dz;
        for j in 0..cols {
            let x = j as f64 * tm.dx - len_x / 2.0;
            let t = tm.get(i, j);
            positions.push([x, -t, z]);
            thickness.push(t as f32);
        }
    }

    for &(i, j) in &perim {
        let x = j as f64 * tm.dx - len_x / 2.0;
        let z = (rows - 1 - i) as f64 * tm.dz;
        positions.push([x, 0.0, z]);
        thickness.push(tm.get(i, j) as f32);
    }

    positions.push([0.0, 0.0, hz / 2.0]);
    thickness.push(tm.frame_thickness_mm as f32);

    let mut indices = Vec::with_capacity(2 * (rows - 1) * (cols - 1) + 3 * n_per);

    let idx = |r: usize, c: usize| -> u32 { (r * cols + c) as u32 };
    for i in 0..rows - 1 {
        for j in 0..cols - 1 {
            let a = idx(i, j);
            let b = idx(i, j + 1);
            let c = idx(i + 1, j);
            let d = idx(i + 1, j + 1);
            indices.push([a, c, d]);
            indices.push([a, d, b]);
        }
    }

    for k in 0..n_per {
        let k1 = (k + 1) % n_per;
        let (r0, c0) = perim[k];
        let (r1, c1) = perim[k1];
        let f0 = idx(r0, c0);
        let f1 = idx(r1, c1);
        let b0 = (base_back + k) as u32;
        let b1 = (base_back + k1) as u32;

        indices.push([f1, f0, b0]);
        indices.push([f1, b0, b1]);
        indices.push([centre_idx, b1, b0]);
    }

    if relief_side == ReliefSide::Back {
        for p in &mut positions {
            p[1] = -p[1];
        }
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

pub(crate) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub(crate) fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

pub(crate) fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
