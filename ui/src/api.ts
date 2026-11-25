import { invoke } from '@tauri-apps/api/core';
import type {
  AppPreferences,
  BinaryScenePart,
  BootstrapInfo,
  BundledPreset,
  ExportResponse,
  ImageMeta,
  LithophaneSettings,
  LoadedProject,
  MatchDimensionMode,
  PartRole,
  PreviewMetaResponse,
  ProjectSummary,
  StyleId,
} from './types';

export interface ImageRequestItem {
  path: string;
  mapping_mode: MatchDimensionMode | null;
}

export interface SaveProjectRequest {
  id: string | null;
  name: string;
  notes: string;
  style: StyleId;
  settings: LithophaneSettings;
  inputs: ImageRequestItem[];
  thumbnail_b64: string | null;
}

export async function getBootstrap(): Promise<BootstrapInfo> {
  return invoke<BootstrapInfo>('get_bootstrap');
}

export async function getAppPreferences(): Promise<AppPreferences> {
  return invoke<AppPreferences>('get_app_preferences');
}

export async function saveAppPreferences(preferences: AppPreferences): Promise<AppPreferences> {
  return invoke<AppPreferences>('save_app_preferences', { preferences });
}

export async function inspectImages(paths: string[]): Promise<ImageMeta[]> {
  return invoke<ImageMeta[]>('inspect_images', { paths });
}

export async function generatePreview(params: {
  style: StyleId;
  inputs: ImageRequestItem[];
  settings: LithophaneSettings;
  preview_pixel_size_mm: number | null;
}): Promise<PreviewMetaResponse> {
  return invoke<PreviewMetaResponse>('generate_preview', { request: params });
}

export async function rerenderBacklit2d(settings: LithophaneSettings): Promise<string[]> {
  return invoke<string[]>('rerender_backlit_2d', { settings });
}

export async function getPreviewGeometry(sceneId: number): Promise<BinaryScenePart[]> {
  const buf = await invoke<ArrayBuffer>('get_preview_geometry', { sceneId });
  return parseBinaryScene(buf);
}

export async function exportModel(params: {
  style: StyleId;
  inputs: ImageRequestItem[];
  settings: LithophaneSettings;
  target_path: string;
}): Promise<ExportResponse> {
  return invoke<ExportResponse>('export_model', { request: params });
}

export async function listProjects(): Promise<ProjectSummary[]> {
  return invoke<ProjectSummary[]>('list_projects');
}

export async function saveProject(request: SaveProjectRequest): Promise<LoadedProject> {
  return invoke<LoadedProject>('save_project', { request });
}

export async function loadProject(id: string): Promise<LoadedProject> {
  return invoke<LoadedProject>('load_project', { id });
}

export async function openProjectFile(path: string): Promise<LoadedProject> {
  return invoke<LoadedProject>('open_project_file', { path });
}

export async function duplicateProject(id: string, newName: string): Promise<LoadedProject> {
  return invoke<LoadedProject>('duplicate_project', { id, newName });
}

export async function renameProject(
  id: string,
  newName: string,
  notes: string,
): Promise<ProjectSummary> {
  return invoke<ProjectSummary>('rename_project', { id, newName, notes });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke<void>('delete_project', { id });
}

export async function loadPresetCmd(name: string): Promise<BundledPreset> {
  return invoke<BundledPreset>('load_preset_cmd', { name });
}

export async function saveSettingsJson(path: string, settings: LithophaneSettings): Promise<void> {
  return invoke<void>('save_settings_json', { path, settings });
}

export async function readSettingsJson(path: string): Promise<LithophaneSettings> {
  return invoke<LithophaneSettings>('read_settings_json', { path });
}

const ROLE_MAP: Record<number, PartRole> = {
  0: 'panel',
  1: 'base',
  2: 'support',
  3: 'bridge',
  4: 'color_layer',
  5: 'structural',
};

function parseBinaryScene(buffer: ArrayBuffer): BinaryScenePart[] {
  const dv = new DataView(buffer);
  const decoder = new TextDecoder();
  let off = 0;

  const partCount = dv.getUint32(off, true);
  off += 4;
  const parts: BinaryScenePart[] = [];

  for (let p = 0; p < partCount; p++) {
    const idLen = dv.getUint32(off, true);
    off += 4;
    const id = decoder.decode(new Uint8Array(buffer, off, idLen));
    off += idLen;

    const roleCode = dv.getUint8(off);
    off += 1;
    const role: PartRole = ROLE_MAP[roleCode] ?? 'panel';

    const rgba: [number, number, number, number] = [
      dv.getUint8(off),
      dv.getUint8(off + 1),
      dv.getUint8(off + 2),
      dv.getUint8(off + 3),
    ];
    off += 4;

    const vCount = dv.getUint32(off, true);
    off += 4;
    const iCount = dv.getUint32(off, true);
    off += 4;
    const hasThickness = dv.getUint8(off) === 1;
    off += 1;

    const posByteLen = vCount * 3 * 4;
    const positions = new Float32Array(vCount * 3);
    new Uint8Array(positions.buffer).set(new Uint8Array(buffer, off, posByteLen));
    off += posByteLen;

    const normals = new Float32Array(vCount * 3);
    new Uint8Array(normals.buffer).set(new Uint8Array(buffer, off, posByteLen));
    off += posByteLen;

    let thickness: Float32Array | null = null;
    if (hasThickness) {
      const tByteLen = vCount * 4;
      thickness = new Float32Array(vCount);
      new Uint8Array(thickness.buffer).set(new Uint8Array(buffer, off, tByteLen));
      off += tByteLen;
    }

    const idxByteLen = iCount * 4;
    const indices = new Uint32Array(iCount);
    new Uint8Array(indices.buffer).set(new Uint8Array(buffer, off, idxByteLen));
    off += idxByteLen;

    parts.push({ id, role, rgba, positions, normals, thickness, indices });
  }

  return parts;
}
