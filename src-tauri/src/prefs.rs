//! Persistent application-level preferences stored in `<app_config_dir>/preferences.json`.

use std::path::{Path, PathBuf};

use litho_core::settings::ExportFormat;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Application-wide settings persisted across restarts (distinct from per-lithophane print settings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppPreferences {
    /// When false (default), 3D preview uses the exact same `pixel_size_mm` as final export.
    pub low_res_preview_enabled: bool,
    /// Sampling pitch (mm) used only when `low_res_preview_enabled` is true.
    pub low_res_preview_pixel_size_mm: f64,
    /// Root folder where restorable project folders are stored.
    pub projects_root_dir: String,
    /// Optional default export directory; when empty, uses project exports folder or OS dialog default.
    pub default_export_dir: Option<String>,
    /// Automatically reopen the most recently active project when launching the studio.
    pub auto_open_last_project: bool,
    /// ID of the most recently opened or saved project.
    pub last_project_id: Option<String>,
    /// Automatically regenerate the 3D preview when parameters change.
    pub auto_update_preview: bool,
    /// Debounce delay in milliseconds before running an automatic preview update.
    pub preview_debounce_ms: u32,
    /// Default 3D viewport shading mode (`"backlit"`, `"clay"`, or `"wireframe"`).
    pub default_shading_mode: String,
    /// Display print-bed reference grid at `Z = 0`.
    pub show_print_bed_grid: bool,
    /// Print bed width (`X`, mm).
    pub bed_size_x_mm: f64,
    /// Print bed depth (`Y`, mm).
    pub bed_size_y_mm: f64,
    /// Default file format when starting new projects.
    pub default_export_format: ExportFormat,
    /// Default PLA filament density (g/cm³) for mass estimates.
    pub default_pla_density_g_cm3: f64,
    /// Show a non-blocking caution badge when preview/export exceeds this triangle count.
    pub warn_triangle_threshold: usize,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            low_res_preview_enabled: false,
            low_res_preview_pixel_size_mm: 0.45,
            projects_root_dir: String::new(),
            default_export_dir: None,
            auto_open_last_project: true,
            last_project_id: None,
            auto_update_preview: true,
            preview_debounce_ms: 180,
            default_shading_mode: "backlit".to_string(),
            show_print_bed_grid: true,
            bed_size_x_mm: 256.0,
            bed_size_y_mm: 256.0,
            default_export_format: ExportFormat::Stl,
            default_pla_density_g_cm3: 1.24,
            warn_triangle_threshold: 2_000_000,
        }
    }
}

impl AppPreferences {
    /// Clamp numeric preferences to sane operational bounds.
    pub fn normalized(mut self, default_projects_dir: &Path) -> Self {
        self.low_res_preview_pixel_size_mm = self.low_res_preview_pixel_size_mm.clamp(0.05, 5.0);
        self.preview_debounce_ms = self.preview_debounce_ms.clamp(50, 2000);
        self.bed_size_x_mm = self.bed_size_x_mm.clamp(60.0, 1000.0);
        self.bed_size_y_mm = self.bed_size_y_mm.clamp(60.0, 1000.0);
        self.default_pla_density_g_cm3 = self.default_pla_density_g_cm3.clamp(0.5, 3.0);
        self.warn_triangle_threshold = self.warn_triangle_threshold.max(50_000);
        if !matches!(
            self.default_shading_mode.as_str(),
            "backlit" | "clay" | "wireframe"
        ) {
            self.default_shading_mode = "backlit".to_string();
        }
        if self.projects_root_dir.trim().is_empty() {
            self.projects_root_dir = default_projects_dir.display().to_string();
        }
        if let Some(ref d) = self.default_export_dir {
            if d.trim().is_empty() {
                self.default_export_dir = None;
            }
        }
        self
    }
}

/// Resolve the default folder where user projects are stored.
pub fn default_projects_dir(app: &AppHandle) -> PathBuf {
    if let Ok(docs) = app.path().document_dir() {
        return docs.join("Lithophane Studio").join("Projects");
    }
    if let Ok(data) = app.path().app_data_dir() {
        return data.join("projects");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("projects")
}

fn preferences_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not resolve app config directory: {e}"))?;
    Ok(dir.join("preferences.json"))
}

/// Load preferences from disk, falling back to sensible defaults if missing/corrupt.
pub fn load_preferences(app: &AppHandle) -> AppPreferences {
    let fallback_dir = default_projects_dir(app);
    let Ok(path) = preferences_file_path(app) else {
        return AppPreferences::default().normalized(&fallback_dir);
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return AppPreferences::default().normalized(&fallback_dir);
    };
    serde_json::from_str::<AppPreferences>(&raw)
        .unwrap_or_default()
        .normalized(&fallback_dir)
}

/// Persist normalized application preferences to `<app_config_dir>/preferences.json`.
pub fn persist_preferences(
    app: &AppHandle,
    prefs: AppPreferences,
) -> Result<AppPreferences, String> {
    let fallback_dir = default_projects_dir(app);
    let clean = prefs.normalized(&fallback_dir);
    let path = preferences_file_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create config directory: {e}"))?;
    }
    let json = serde_json::to_string_pretty(&clean)
        .map_err(|e| format!("failed to serialize preferences: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write preferences: {e}"))?;
    Ok(clean)
}
