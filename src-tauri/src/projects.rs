//! Self-contained restorable project folders (`project.json` + copied `assets/` + `thumbnail.png`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, ImageEncoder};
use litho_core::image_proc::load_image_rgb;
use litho_core::scene::ImageSource;
use litho_core::settings::{BackgroundColor, LithophaneSettings, MatchDimensionMode, StyleId};
use serde::{Deserialize, Serialize};

use crate::{ImageMeta, ImageRequestItem};

const MANIFEST_FILENAME: &str = "project.json";
const THUMBNAIL_FILENAME: &str = "thumbnail.png";
const ASSETS_DIR: &str = "assets";
const EXPORTS_DIR: &str = "exports";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInputRef {
    pub relative_path: String,
    pub original_stem: String,
    pub mapping_mode: MatchDimensionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub style: StyleId,
    pub settings: LithophaneSettings,
    pub inputs: Vec<ProjectInputRef>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub style: StyleId,
    pub image_count: usize,
    pub manifest_path: String,
    pub project_dir: String,
    pub exports_dir: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub thumbnail_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedProjectImage {
    #[serde(flatten)]
    pub meta: ImageMeta,
    pub mapping_mode: MatchDimensionMode,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedProject {
    pub summary: ProjectSummary,
    pub settings: LithophaneSettings,
    pub images: Vec<LoadedProjectImage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveProjectRequest {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub style: StyleId,
    pub settings: LithophaneSettings,
    pub inputs: Vec<ImageRequestItem>,
    pub thumbnail_b64: Option<String>,
}

/// Compute metadata and base64 PNG thumbnail for an image file on disk.
pub fn inspect_image_file(path: &Path) -> Result<ImageMeta, String> {
    let src = ImageSource::Path(path.to_path_buf());
    let stem = src.stem();
    let rgb = load_image_rgb(&src, BackgroundColor::White).map_err(|e| e.to_string())?;
    let (w, h) = rgb.dimensions();
    let aspect = w as f64 / h.max(1) as f64;
    let thumb = image::DynamicImage::ImageRgb8(rgb).resize(160, 160, FilterType::Triangle);
    let thumb_rgb = thumb.to_rgb8();
    let (tw, th) = thumb_rgb.dimensions();
    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(thumb_rgb.as_raw(), tw, th, ColorType::Rgb8.into())
        .map_err(|e| format!("thumbnail encode failed: {e}"))?;
    Ok(ImageMeta {
        path: path.display().to_string(),
        stem,
        width: w,
        height: h,
        aspect,
        thumbnail_b64: B64.encode(png_bytes),
    })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn sanitize_slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_sep = false;
        } else if !last_sep && !out.is_empty() {
            out.push('_');
            last_sep = true;
        }
        if out.len() >= 48 {
            break;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "project".to_string()
    } else {
        out
    }
}

fn validate_project_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
    {
        return Err(format!("invalid project id: {id:?}"));
    }
    Ok(())
}

fn allocate_unique_id(root: &Path, name: &str) -> String {
    let base = sanitize_slug(name);
    if !root.join(&base).exists() {
        return base;
    }
    for n in 2..=9999 {
        let candidate = format!("{base}_{n}");
        if !root.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base}_{}", now_ms())
}

fn build_summary(project_dir: &Path, manifest: &ProjectManifest) -> ProjectSummary {
    let thumb_path = project_dir.join(THUMBNAIL_FILENAME);
    let thumbnail_b64 = fs::read(&thumb_path).ok().map(|b| B64.encode(b));
    ProjectSummary {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        notes: manifest.notes.clone(),
        style: manifest.style,
        image_count: manifest.inputs.len(),
        manifest_path: project_dir.join(MANIFEST_FILENAME).display().to_string(),
        project_dir: project_dir.display().to_string(),
        exports_dir: project_dir.join(EXPORTS_DIR).display().to_string(),
        created_at_ms: manifest.created_at_ms,
        updated_at_ms: manifest.updated_at_ms,
        thumbnail_b64,
    }
}

/// List all valid projects in `root`, sorted newest-updated first.
pub fn list_projects_in(root: &Path) -> Result<Vec<ProjectSummary>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let entries =
        fs::read_dir(root).map_err(|e| format!("failed to read projects directory: {e}"))?;
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let manifest_path = dir.join(MANIFEST_FILENAME);
        if !manifest_path.is_file() {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<ProjectManifest>(&raw) {
                items.push(build_summary(&dir, &manifest));
            }
        }
    }
    items.sort_by_key(|a| std::cmp::Reverse(a.updated_at_ms));
    Ok(items)
}

/// Create or update a self-contained project directory under `root`.
pub fn save_project_in(root: &Path, req: SaveProjectRequest) -> Result<LoadedProject, String> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err("project name cannot be empty".to_string());
    }
    req.settings.validate().map_err(|e| e.to_string())?;
    fs::create_dir_all(root).map_err(|e| format!("could not create projects root: {e}"))?;

    let id = match req.id.as_deref() {
        Some(existing) if !existing.trim().is_empty() => {
            validate_project_id(existing)?;
            existing.to_string()
        }
        _ => allocate_unique_id(root, name),
    };

    let project_dir = root.join(&id);
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("could not create project folder: {e}"))?;

    let existing_created = fs::read_to_string(project_dir.join(MANIFEST_FILENAME))
        .ok()
        .and_then(|s| serde_json::from_str::<ProjectManifest>(&s).ok())
        .map(|m| m.created_at_ms);

    let now = now_ms();
    let staging_dir = project_dir.join(".assets_staging");
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("could not create asset staging directory: {e}"))?;

    let mut refs = Vec::with_capacity(req.inputs.len());
    for (idx, item) in req.inputs.iter().enumerate() {
        let src = PathBuf::from(&item.path);
        if !src.is_file() {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!("source image not found: {}", src.display()));
        }
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("image")
            .to_string();
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty())
            .unwrap_or("png");
        let safe_stem = sanitize_slug(&stem);
        let filename = format!("{idx:02}_{safe_stem}.{ext}");
        let dst = staging_dir.join(&filename);
        fs::copy(&src, &dst).map_err(|e| {
            let _ = fs::remove_dir_all(&staging_dir);
            format!("failed to copy asset {}: {e}", src.display())
        })?;
        refs.push(ProjectInputRef {
            relative_path: format!("{ASSETS_DIR}/{filename}"),
            original_stem: stem,
            mapping_mode: item.mapping_mode.unwrap_or(MatchDimensionMode::LongSide),
        });
    }

    let assets_dir = project_dir.join(ASSETS_DIR);
    if assets_dir.exists() {
        fs::remove_dir_all(&assets_dir)
            .map_err(|e| format!("failed to replace previous assets directory: {e}"))?;
    }
    fs::rename(&staging_dir, &assets_dir)
        .map_err(|e| format!("failed to finalize project assets: {e}"))?;

    // Save thumbnail from preview or fallback to first asset
    let thumb_path = project_dir.join(THUMBNAIL_FILENAME);
    if let Some(ref b64) = req.thumbnail_b64 {
        if let Ok(bytes) = B64.decode(b64.trim()) {
            let _ = fs::write(&thumb_path, bytes);
        }
    } else if !thumb_path.exists() {
        if let Some(first_ref) = refs.first() {
            let first_path = project_dir.join(&first_ref.relative_path);
            if let Ok(meta) = inspect_image_file(&first_path) {
                if let Ok(bytes) = B64.decode(meta.thumbnail_b64) {
                    let _ = fs::write(&thumb_path, bytes);
                }
            }
        }
    }

    let manifest = ProjectManifest {
        version: 1,
        id,
        name: name.to_string(),
        notes: req.notes,
        style: req.style,
        settings: req.settings,
        inputs: refs,
        created_at_ms: existing_created.unwrap_or(now),
        updated_at_ms: now,
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize project manifest: {e}"))?;
    fs::write(project_dir.join(MANIFEST_FILENAME), json)
        .map_err(|e| format!("failed to write project manifest: {e}"))?;

    load_project_from_dir(&project_dir)
}

/// Load a project from its folder or directly from a `project.json` path.
pub fn load_project_from_dir(path: &Path) -> Result<LoadedProject, String> {
    let project_dir = if path.is_file() {
        path.parent()
            .ok_or_else(|| "invalid project manifest path".to_string())?
            .to_path_buf()
    } else {
        path.to_path_buf()
    };
    let manifest_path = project_dir.join(MANIFEST_FILENAME);
    let raw = fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "could not read project manifest at {}: {e}",
            manifest_path.display()
        )
    })?;
    let mut manifest: ProjectManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("corrupt project manifest {}: {e}", manifest_path.display()))?;
    manifest.settings.validate().map_err(|e| e.to_string())?;

    let mut images = Vec::with_capacity(manifest.inputs.len());
    for item in &manifest.inputs {
        let asset_path = project_dir.join(&item.relative_path);
        if !asset_path.is_file() {
            return Err(format!(
                "project asset missing: {}",
                asset_path.display()
            ));
        }
        let mut meta = inspect_image_file(&asset_path)?;
        if !item.original_stem.is_empty() {
            meta.stem = item.original_stem.clone();
        }
        images.push(LoadedProjectImage {
            meta,
            mapping_mode: item.mapping_mode,
        });
    }

    // Ensure manifest ID matches directory if folder was moved/copied manually
    if let Some(dir_name) = project_dir.file_name().and_then(|n| n.to_str()) {
        if manifest.id.is_empty() {
            manifest.id = dir_name.to_string();
        }
    }

    Ok(LoadedProject {
        summary: build_summary(&project_dir, &manifest),
        settings: manifest.settings,
        images,
    })
}

/// Duplicate an existing project under a new name (copying assets and thumbnail, omitting exports).
pub fn duplicate_project_in(
    root: &Path,
    project_id: &str,
    new_name: &str,
) -> Result<LoadedProject, String> {
    validate_project_id(project_id)?;
    let src_dir = root.join(project_id);
    let loaded = load_project_from_dir(&src_dir)?;
    let req = SaveProjectRequest {
        id: None,
        name: if new_name.trim().is_empty() {
            format!("{} Copy", loaded.summary.name)
        } else {
            new_name.trim().to_string()
        },
        notes: loaded.summary.notes.clone(),
        style: loaded.summary.style,
        settings: loaded.settings,
        inputs: loaded
            .images
            .into_iter()
            .map(|im| ImageRequestItem {
                path: im.meta.path,
                mapping_mode: Some(im.mapping_mode),
            })
            .collect(),
        thumbnail_b64: loaded.summary.thumbnail_b64,
    };
    save_project_in(root, req)
}

/// Update project display name and notes in place.
pub fn rename_project_in(
    root: &Path,
    project_id: &str,
    new_name: &str,
    notes: &str,
) -> Result<ProjectSummary, String> {
    validate_project_id(project_id)?;
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("project name cannot be empty".to_string());
    }
    let dir = root.join(project_id);
    let manifest_path = dir.join(MANIFEST_FILENAME);
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("could not read project manifest: {e}"))?;
    let mut manifest: ProjectManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid project manifest: {e}"))?;
    manifest.name = trimmed.to_string();
    manifest.notes = notes.to_string();
    manifest.updated_at_ms = now_ms();
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize project manifest: {e}"))?;
    fs::write(&manifest_path, json).map_err(|e| format!("failed to write manifest: {e}"))?;
    Ok(build_summary(&dir, &manifest))
}

/// Permanently delete a project folder under `root`.
pub fn delete_project_in(root: &Path, project_id: &str) -> Result<(), String> {
    validate_project_id(project_id)?;
    let dir = root.join(project_id);
    if !dir.is_dir() {
        return Err(format!("project {project_id:?} not found"));
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("failed to delete project: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn make_temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("litho_proj_test_{tag}_{}", now_ms()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_sample_png(path: &Path, rgb: [u8; 3]) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(32, 24, Rgb(rgb));
        img.save(path).unwrap();
    }

    #[test]
    fn save_load_reorder_duplicate_rename_and_delete_project() {
        let root = make_temp_root("roundtrip");
        let src_dir = make_temp_root("src_imgs");
        let img_a = src_dir.join("alpha.png");
        let img_b = src_dir.join("beta.png");
        write_sample_png(&img_a, [240, 200, 180]);
        write_sample_png(&img_b, [40, 80, 120]);

        let mut settings = LithophaneSettings::default();
        settings.curved.radius_mm = 95.0;
        settings.lighting.transmission_distance_mm = 6.2;

        let saved = save_project_in(
            &root,
            SaveProjectRequest {
                id: None,
                name: "Living Room Lamp".to_string(),
                notes: "First test".to_string(),
                style: StyleId::Curved,
                settings: settings.clone(),
                inputs: vec![
                    ImageRequestItem {
                        path: img_a.display().to_string(),
                        mapping_mode: Some(MatchDimensionMode::LongSide),
                    },
                    ImageRequestItem {
                        path: img_b.display().to_string(),
                        mapping_mode: Some(MatchDimensionMode::Width),
                    },
                ],
                thumbnail_b64: None,
            },
        )
        .expect("save should succeed");

        assert_eq!(saved.summary.id, "living_room_lamp");
        assert_eq!(saved.images.len(), 2);
        assert!(saved.summary.thumbnail_b64.is_some());

        // Delete original source files to prove self-contained restorability
        fs::remove_dir_all(&src_dir).unwrap();

        let listed = list_projects_in(&root).unwrap();
        assert_eq!(listed.len(), 1);

        // Re-save existing project with reversed image order from its own assets/ directory!
        let reordered = save_project_in(
            &root,
            SaveProjectRequest {
                id: Some(saved.summary.id.clone()),
                name: "Living Room Lamp".to_string(),
                notes: "Reordered".to_string(),
                style: StyleId::MultiFlat,
                settings: saved.settings.clone(),
                inputs: vec![
                    ImageRequestItem {
                        path: saved.images[1].meta.path.clone(),
                        mapping_mode: Some(MatchDimensionMode::Height),
                    },
                    ImageRequestItem {
                        path: saved.images[0].meta.path.clone(),
                        mapping_mode: Some(MatchDimensionMode::LongSide),
                    },
                ],
                thumbnail_b64: None,
            },
        )
        .expect("reordering self-contained assets must not clobber files");

        assert_eq!(reordered.images[0].meta.stem, "01_beta");
        assert_eq!(reordered.images[1].meta.stem, "00_alpha");

        let renamed =
            rename_project_in(&root, &reordered.summary.id, "Hallway Frame", "Updated note")
                .unwrap();
        assert_eq!(renamed.name, "Hallway Frame");
        assert_eq!(renamed.notes, "Updated note");

        let dup = duplicate_project_in(&root, &reordered.summary.id, "Hallway Copy").unwrap();
        assert_ne!(dup.summary.id, reordered.summary.id);
        assert_eq!(list_projects_in(&root).unwrap().len(), 2);

        delete_project_in(&root, &dup.summary.id).unwrap();
        assert_eq!(list_projects_in(&root).unwrap().len(), 1);
        assert!(delete_project_in(&root, "../escape").is_err());

        let _ = fs::remove_dir_all(&root);
    }
}
