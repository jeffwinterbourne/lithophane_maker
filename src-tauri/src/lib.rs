//! Tauri v2 desktop backend for Lithophane Studio.

mod prefs;
mod projects;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use litho_core::generate;
use litho_core::mesh::pack_preview_binary;
use litho_core::preview::backlit_png;
use litho_core::scene::{
    GenerationOptions, GenerationReport, ImageInput, ImageSource, MaterialSpec, PartRole,
};
use litho_core::settings::{
    bundled_presets, load_preset_with_style, BundledPreset, LithophaneSettings,
    MatchDimensionMode, StyleId,
};
use litho_core::styles::{StyleDescriptor, StyleRegistry};
use prefs::{load_preferences, persist_preferences, AppPreferences};
use projects::{
    delete_project_in, duplicate_project_in, inspect_image_file, list_projects_in,
    load_project_from_dir, rename_project_in, save_project_in, LoadedProject, ProjectSummary,
    SaveProjectRequest,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Cached image-frame thickness grid so lighting adjustments can refresh 2D backlit PNGs without remeshing.
struct CachedThicknessGrid {
    thickness: Vec<f32>,
    rows: usize,
    cols: usize,
}

/// Shared application state caching recent packed binary meshes for Three.js transfer.
pub struct AppState {
    next_id: AtomicU64,
    preview_cache: Mutex<HashMap<u64, Vec<u8>>>,
    thickness_cache: Mutex<Vec<CachedThicknessGrid>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            preview_cache: Mutex::new(HashMap::new()),
            thickness_cache: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapInfo {
    pub styles: Vec<StyleDescriptor>,
    pub presets: Vec<BundledPreset>,
    pub default_settings: LithophaneSettings,
    pub preferences: AppPreferences,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageMeta {
    pub path: String,
    pub stem: String,
    pub width: u32,
    pub height: u32,
    pub aspect: f64,
    pub thumbnail_b64: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageRequestItem {
    pub path: String,
    pub mapping_mode: Option<MatchDimensionMode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewRequest {
    pub style: StyleId,
    pub inputs: Vec<ImageRequestItem>,
    pub settings: LithophaneSettings,
    /// When `None`, preview uses the exact same `pixel_size_mm` as final export.
    pub preview_pixel_size_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewPartMeta {
    pub id: String,
    pub name: String,
    pub role: PartRole,
    pub material: MaterialSpec,
    pub triangles: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagePreviewB64 {
    pub stem: String,
    pub luminance_b64: String,
    pub heightmap_b64: String,
    pub backlit_b64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewMetaResponse {
    pub scene_id: u64,
    pub stem: String,
    pub parts: Vec<PreviewPartMeta>,
    pub previews: Vec<ImagePreviewB64>,
    pub report: GenerationReport,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub style: StyleId,
    pub inputs: Vec<ImageRequestItem>,
    pub settings: LithophaneSettings,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportFileEntry {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResponse {
    pub files: Vec<ExportFileEntry>,
    pub report: GenerationReport,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ExportProgressEvent {
    message: String,
    fraction: f32,
}

#[tauri::command]
fn get_bootstrap(app: AppHandle) -> BootstrapInfo {
    let reg = StyleRegistry::new();
    let preferences = load_preferences(&app);
    BootstrapInfo {
        styles: reg.descriptors(),
        presets: bundled_presets(),
        default_settings: LithophaneSettings::default(),
        preferences,
    }
}

#[tauri::command]
fn get_app_preferences(app: AppHandle) -> AppPreferences {
    load_preferences(&app)
}

#[tauri::command]
fn save_app_preferences(
    app: AppHandle,
    preferences: AppPreferences,
) -> Result<AppPreferences, String> {
    persist_preferences(&app, preferences)
}

#[tauri::command]
async fn inspect_images(paths: Vec<String>) -> Result<Vec<ImageMeta>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| inspect_image_file(&PathBuf::from(p)))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| format!("worker join error: {e}"))?
}

fn to_inputs(items: Vec<ImageRequestItem>) -> Vec<ImageInput> {
    items
        .into_iter()
        .map(|it| ImageInput {
            source: ImageSource::Path(PathBuf::from(it.path)),
            mapping_mode: it.mapping_mode,
        })
        .collect()
}

#[tauri::command]
async fn generate_preview(
    state: State<'_, AppState>,
    request: PreviewRequest,
) -> Result<PreviewMetaResponse, String> {
    let started = Instant::now();
    let inputs = to_inputs(request.inputs);
    let settings = request.settings;
    let style = request.style;
    let px = request.preview_pixel_size_mm.map(|v| v.max(0.05));

    let (scene, binary_buf) = tauri::async_runtime::spawn_blocking(move || {
        let opts = GenerationOptions {
            preview_pixel_size_mm: px,
            skip_final_boolean_merge: true,
        };
        let mut noop = |_: &str, _: f32| {};
        let scene = generate(style, &inputs, &settings, &opts, &mut noop)
            .map_err(|e| e.to_string())?;
        let binary_buf = pack_preview_binary(&scene.parts);
        Ok::<_, String>((scene, binary_buf))
    })
    .await
    .map_err(|e| format!("worker join error: {e}"))??;

    let scene_id = state.next_id.fetch_add(1, Ordering::Relaxed);
    {
        let mut cache = state.preview_cache.lock().map_err(|e| e.to_string())?;
        if cache.len() >= 4 {
            if let Some(&oldest_key) = cache.keys().min() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(scene_id, binary_buf);
    }

    {
        let mut tcache = state.thickness_cache.lock().map_err(|e| e.to_string())?;
        tcache.clear();
        for pv in &scene.previews {
            tcache.push(CachedThicknessGrid {
                thickness: pv.thickness_grid.clone(),
                rows: pv.grid_rows,
                cols: pv.grid_cols,
            });
        }
    }

    let parts = scene
        .parts
        .iter()
        .map(|p| PreviewPartMeta {
            id: p.id.clone(),
            name: p.name.clone(),
            role: p.role,
            material: p.material.clone(),
            triangles: p.mesh.indices.len(),
        })
        .collect();

    let previews = scene
        .previews
        .iter()
        .map(|pv| ImagePreviewB64 {
            stem: pv.stem.clone(),
            luminance_b64: B64.encode(&pv.luminance_png),
            heightmap_b64: B64.encode(&pv.heightmap_png),
            backlit_b64: B64.encode(&pv.backlit_png),
        })
        .collect();

    Ok(PreviewMetaResponse {
        scene_id,
        stem: scene.stem(),
        parts,
        previews,
        report: scene.report,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
fn rerender_backlit_2d(
    state: State<'_, AppState>,
    settings: LithophaneSettings,
) -> Result<Vec<String>, String> {
    settings.validate().map_err(|e| e.to_string())?;
    let tcache = state.thickness_cache.lock().map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(tcache.len());
    for grid in tcache.iter() {
        let png = backlit_png(&grid.thickness, grid.rows, grid.cols, &settings)
            .map_err(|e| e.to_string())?;
        out.push(B64.encode(png));
    }
    Ok(out)
}

#[tauri::command]
fn get_preview_geometry(
    state: State<'_, AppState>,
    scene_id: u64,
) -> Result<tauri::ipc::Response, String> {
    let cache = state.preview_cache.lock().map_err(|e| e.to_string())?;
    let bytes = cache
        .get(&scene_id)
        .ok_or_else(|| format!("preview scene {scene_id} is no longer cached"))?
        .clone();
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
async fn export_model(app: AppHandle, request: ExportRequest) -> Result<ExportResponse, String> {
    let started = Instant::now();
    let inputs = to_inputs(request.inputs);
    let settings = request.settings;
    let style = request.style;
    let target = PathBuf::from(&request.target_path);

    let (entries, report) = tauri::async_runtime::spawn_blocking(move || {
        let mut emit_progress = |msg: &str, frac: f32| {
            let _ = app.emit(
                "export-progress",
                ExportProgressEvent {
                    message: msg.to_string(),
                    fraction: frac,
                },
            );
        };
        let scene = generate(
            style,
            &inputs,
            &settings,
            &GenerationOptions::default(),
            &mut emit_progress,
        )
        .map_err(|e| e.to_string())?;

        emit_progress("Writing file(s)", 0.94);
        let paths = scene.save_to_path(&target).map_err(|e| e.to_string())?;
        let mut entries = Vec::with_capacity(paths.len());
        for p in paths {
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            entries.push(ExportFileEntry {
                path: p.display().to_string(),
                bytes: size,
            });
        }
        emit_progress("Export complete", 1.0);
        Ok::<_, String>((entries, scene.report))
    })
    .await
    .map_err(|e| format!("worker join error: {e}"))??;

    Ok(ExportResponse {
        files: entries,
        report,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
async fn list_projects(app: AppHandle) -> Result<Vec<ProjectSummary>, String> {
    let prefs = load_preferences(&app);
    let root = PathBuf::from(prefs.projects_root_dir);
    tauri::async_runtime::spawn_blocking(move || list_projects_in(&root))
        .await
        .map_err(|e| format!("worker join error: {e}"))?
}

#[tauri::command]
async fn save_project(
    app: AppHandle,
    request: SaveProjectRequest,
) -> Result<LoadedProject, String> {
    let mut prefs = load_preferences(&app);
    let root = PathBuf::from(&prefs.projects_root_dir);
    let loaded = tauri::async_runtime::spawn_blocking(move || save_project_in(&root, request))
        .await
        .map_err(|e| format!("worker join error: {e}"))??;
    prefs.last_project_id = Some(loaded.summary.id.clone());
    let _ = persist_preferences(&app, prefs);
    Ok(loaded)
}

#[tauri::command]
async fn load_project(app: AppHandle, id: String) -> Result<LoadedProject, String> {
    let mut prefs = load_preferences(&app);
    let dir = PathBuf::from(&prefs.projects_root_dir).join(&id);
    let loaded = tauri::async_runtime::spawn_blocking(move || load_project_from_dir(&dir))
        .await
        .map_err(|e| format!("worker join error: {e}"))??;
    prefs.last_project_id = Some(loaded.summary.id.clone());
    let _ = persist_preferences(&app, prefs);
    Ok(loaded)
}

#[tauri::command]
async fn open_project_file(app: AppHandle, path: String) -> Result<LoadedProject, String> {
    let pb = PathBuf::from(path);
    let loaded = tauri::async_runtime::spawn_blocking(move || load_project_from_dir(&pb))
        .await
        .map_err(|e| format!("worker join error: {e}"))??;
    let mut prefs = load_preferences(&app);
    if PathBuf::from(&loaded.summary.project_dir).parent()
        == Some(PathBuf::from(&prefs.projects_root_dir).as_path())
    {
        prefs.last_project_id = Some(loaded.summary.id.clone());
        let _ = persist_preferences(&app, prefs);
    }
    Ok(loaded)
}

#[tauri::command]
async fn duplicate_project(
    app: AppHandle,
    id: String,
    new_name: String,
) -> Result<LoadedProject, String> {
    let mut prefs = load_preferences(&app);
    let root = PathBuf::from(&prefs.projects_root_dir);
    let loaded =
        tauri::async_runtime::spawn_blocking(move || duplicate_project_in(&root, &id, &new_name))
            .await
            .map_err(|e| format!("worker join error: {e}"))??;
    prefs.last_project_id = Some(loaded.summary.id.clone());
    let _ = persist_preferences(&app, prefs);
    Ok(loaded)
}

#[tauri::command]
async fn rename_project(
    app: AppHandle,
    id: String,
    new_name: String,
    notes: String,
) -> Result<ProjectSummary, String> {
    let prefs = load_preferences(&app);
    let root = PathBuf::from(prefs.projects_root_dir);
    tauri::async_runtime::spawn_blocking(move || rename_project_in(&root, &id, &new_name, &notes))
        .await
        .map_err(|e| format!("worker join error: {e}"))?
}

#[tauri::command]
async fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let mut prefs = load_preferences(&app);
    let root = PathBuf::from(&prefs.projects_root_dir);
    let id_copy = id.clone();
    tauri::async_runtime::spawn_blocking(move || delete_project_in(&root, &id_copy))
        .await
        .map_err(|e| format!("worker join error: {e}"))??;
    if prefs.last_project_id.as_deref() == Some(id.as_str()) {
        prefs.last_project_id = None;
        let _ = persist_preferences(&app, prefs);
    }
    Ok(())
}

#[tauri::command]
fn load_preset_cmd(name: String) -> Result<BundledPreset, String> {
    load_preset_with_style(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_settings_json(json: String) -> Result<LithophaneSettings, String> {
    let s: LithophaneSettings =
        serde_json::from_str(&json).map_err(|e| format!("Invalid JSON preset: {e}"))?;
    s.validate().map_err(|e| e.to_string())?;
    Ok(s)
}

#[tauri::command]
fn save_settings_json(path: String, settings: LithophaneSettings) -> Result<(), String> {
    settings.validate().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_settings_json(path: String) -> Result<LithophaneSettings, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_settings_json(text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_bootstrap,
            get_app_preferences,
            save_app_preferences,
            inspect_images,
            generate_preview,
            rerender_backlit_2d,
            get_preview_geometry,
            export_model,
            list_projects,
            save_project,
            load_project,
            open_project_file,
            duplicate_project,
            rename_project,
            delete_project,
            load_preset_cmd,
            parse_settings_json,
            save_settings_json,
            read_settings_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lithophane Studio");
}
