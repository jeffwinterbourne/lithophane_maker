export type StyleId = 'flat' | 'curved' | 'multi_flat';
export type SizeMode = 'max_dimension' | 'width' | 'height' | 'absolute';
export type FitMode = 'crop' | 'pad' | 'stretch';
export type GreyscaleMode = 'rec601' | 'rec709' | 'average' | 'red' | 'green' | 'blue';
export type BackgroundColor = 'white' | 'black';
export type EqualizeMode = 'none' | 'hist' | 'clahe';
export type ThicknessCurve = 'linear' | 'gamma' | 'transmission' | 'custom';
export type ReliefSide = 'front' | 'back';
export type OrientationMode = 'auto_short_up' | 'long_up' | 'image_up';
export type SupportSides = 'both' | 'front' | 'back';
export type SupportOutput = 'merged' | 'separate_bodies' | 'separate_files';
export type CurvatureDirection = 'concave' | 'convex';
export type MatchDimensionMode = 'long_side' | 'short_side' | 'width' | 'height';
export type ExportFormat = 'stl' | '3mf';
export type PartRole = 'panel' | 'base' | 'support' | 'bridge' | 'color_layer' | 'structural';

export interface SizeSettings {
  mode: SizeMode;
  max_dimension_mm: number;
  width_mm: number;
  height_mm: number;
  fit: FitMode;
  pixel_size_mm: number;
}

export interface ImageSettings {
  greyscale: GreyscaleMode;
  background: BackgroundColor;
  crop: [number, number, number, number] | null;
  rotate: 0 | 90 | 180 | 270;
  mirror: boolean;
  flip: boolean;
  auto_levels_percent: number;
  equalize: EqualizeMode;
  clahe_clip: number;
  brightness: number;
  contrast: number;
  gamma: number;
  blur_sigma_mm: number;
  sharpen_amount: number;
  sharpen_radius_mm: number;
  posterize_levels: number | null;
  invert: boolean;
}

export interface ThicknessSettings {
  min_thickness_mm: number;
  max_thickness_mm: number;
  curve: ThicknessCurve;
  curve_gamma: number;
  attenuation_per_mm: number;
  custom_points: [number, number][];
  relief_side: ReliefSide;
}

export interface FrameSettings {
  enabled: boolean;
  width_mm: number;
  thickness_mm: number | null;
}

export interface BottomExtensionSettings {
  enabled: boolean;
  height_mm: number;
  thickness_mm: number;
}

export interface OrientationSettings {
  mode: OrientationMode;
}

export interface SupportSettings {
  enabled: boolean;
  sides: SupportSides;
  output: SupportOutput;
  fin_count: number;
  fin_height_mm: number | null;
  fin_height_fraction: number;
  fin_base_length_mm: number | null;
  fin_thickness_mm: number;
  fin_inset_mm: number | null;
  interface_gap_mm: number;
  tab_count: number;
  tab_height_mm: number;
  tab_neck_height_mm: number;
  tab_width_mm: number | null;
  tab_overlap_mm: number;
  tab_margin_mm: number;
  pad_enabled: boolean;
  pad_thickness_mm: number;
  pad_side_margin_mm: number;
  pad_end_margin_mm: number;
}

export interface CurvedBaseSettings {
  enabled: boolean;
  thickness_mm: number;
  front_projection_mm: number;
  back_projection_mm: number;
  corner_fillet_mm: number;
}

export interface CurvedSettings {
  radius_mm: number;
  direction: CurvatureDirection;
  orientation: OrientationMode;
  supports_enabled: boolean;
  base: CurvedBaseSettings;
}

export interface MultiSettings {
  common_dimension_mm: number;
  panel_spacing_mm: number;
  item_modes: MatchDimensionMode[];
  bridge_count: number;
  bridge_width_mm: number;
  bridge_height_mm: number;
  bridge_overlap_mm: number;
  bridge_inset_mm: number | null;
  margin_z_mm: number;
  add_center_bridge: boolean;
}

export interface LightingSettings {
  filament_color: string;
  transmission_distance_mm: number;
  backlight_color: string;
  backlight_intensity: number;
  ambient_front_light: number;
}

export interface ExportSettings {
  format: ExportFormat;
  binary: boolean;
  filename_template: string;
  pla_density_g_cm3: number;
}

export interface LithophaneSettings {
  size: SizeSettings;
  image: ImageSettings;
  thickness: ThicknessSettings;
  frame: FrameSettings;
  bottom_extension: BottomExtensionSettings;
  orientation: OrientationSettings;
  supports: SupportSettings;
  curved: CurvedSettings;
  multi: MultiSettings;
  lighting: LightingSettings;
  export: ExportSettings;
}

export type ShadingMode = 'backlit' | 'clay' | 'wireframe';

export interface AppPreferences {
  low_res_preview_enabled: boolean;
  low_res_preview_pixel_size_mm: number;
  projects_root_dir: string;
  default_export_dir: string | null;
  auto_open_last_project: boolean;
  last_project_id: string | null;
  auto_update_preview: boolean;
  preview_debounce_ms: number;
  default_shading_mode: ShadingMode;
  show_print_bed_grid: boolean;
  bed_size_x_mm: number;
  bed_size_y_mm: number;
  default_export_format: ExportFormat;
  default_pla_density_g_cm3: number;
  warn_triangle_threshold: number;
}

export interface StyleDescriptor {
  id: StyleId;
  name: string;
  tagline: string;
  min_images: number;
  max_images: number | null;
}

export interface BundledPreset {
  name: string;
  label: string;
  description: string;
  recommended_style: StyleId;
  settings: LithophaneSettings;
}

export interface BootstrapInfo {
  styles: StyleDescriptor[];
  presets: BundledPreset[];
  default_settings: LithophaneSettings;
  preferences: AppPreferences;
}

export interface ImageMeta {
  path: string;
  stem: string;
  width: number;
  height: number;
  aspect: number;
  thumbnail_b64: string;
}

export interface LoadedImage extends ImageMeta {
  mapping_mode: MatchDimensionMode;
}

export interface ProjectSummary {
  id: string;
  name: string;
  notes: string;
  style: StyleId;
  image_count: number;
  manifest_path: string;
  project_dir: string;
  exports_dir: string;
  created_at_ms: number;
  updated_at_ms: number;
  thumbnail_b64: string | null;
}

export interface LoadedProject {
  summary: ProjectSummary;
  settings: LithophaneSettings;
  images: LoadedImage[];
}

export interface DimensionsMm {
  x: number;
  y: number;
  z: number;
}

export interface CurvedReportInfo {
  radius_mm: number;
  arc_angle_deg: number;
  chord_width_mm: number;
  arc_depth_mm: number;
  direction: CurvatureDirection;
  base_enabled: boolean;
  base_thickness_mm: number | null;
  base_effective_fillet_in_mm: number | null;
  base_effective_fillet_out_mm: number | null;
}

export interface GenerationReport {
  style: StyleId;
  dimensions_mm: DimensionsMm;
  overall_dimensions_mm: DimensionsMm;
  grid: {
    columns: number;
    rows: number;
    dx_mm: number;
    dz_mm: number;
  } | null;
  rotated_for_print: boolean;
  thickness_mm: {
    min: number;
    max: number;
  };
  triangles: number;
  volume_mm3: number;
  filament_g_estimate: number;
  bodies: number | null;
  curved_info: CurvedReportInfo | null;
  supports: {
    fin_height_mm: number;
    fin_base_length_mm: number;
    fin_centres_x_mm: number[];
    sides: string;
  } | null;
  bridges: {
    stations_x_mm: number[];
    bridges_per_station: number;
    total_bridges: number;
  } | null;
  panels: {
    index: number;
    source_stem: string;
    mode: MatchDimensionMode;
    dimensions_mm: DimensionsMm;
    y_ref_mm: number;
    triangles: number;
  }[];
  warnings: string[];
}

export interface ImagePreviewB64 {
  stem: string;
  luminance_b64: string;
  heightmap_b64: string;
  backlit_b64: string;
}

export interface PreviewPartMeta {
  id: string;
  name: string;
  role: PartRole;
  material: { name: string; rgba: [number, number, number, number] };
  triangles: number;
}

export interface PreviewMetaResponse {
  scene_id: number;
  stem: string;
  parts: PreviewPartMeta[];
  previews: ImagePreviewB64[];
  report: GenerationReport;
  elapsed_ms: number;
}

export interface ExportResponse {
  files: { path: string; bytes: number }[];
  report: GenerationReport;
  elapsed_ms: number;
}

export interface BinaryScenePart {
  id: string;
  role: PartRole;
  rgba: [number, number, number, number];
  positions: Float32Array;
  normals: Float32Array;
  thickness: Float32Array | null;
  indices: Uint32Array;
}
