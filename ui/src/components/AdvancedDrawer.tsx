import { FolderInput, RotateCcw, Save, X } from 'lucide-react';
import React from 'react';
import type {
  AppPreferences,
  BackgroundColor,
  CurvatureDirection,
  EqualizeMode,
  ExportFormat,
  FitMode,
  GreyscaleMode,
  LithophaneSettings,
  OrientationMode,
  ReliefSide,
  SizeMode,
  SupportOutput,
  SupportSides,
  ThicknessCurve,
} from '../types';
import { BACKLIGHT_PRESETS, FILAMENT_PRESETS } from './Viewport3D';

export const DRAWER_TABS = [
  { id: 'size', label: 'Size' },
  { id: 'image', label: 'Image' },
  { id: 'thickness', label: 'Thickness' },
  { id: 'lighting', label: 'Lighting' },
  { id: 'frame', label: 'Frame & Base' },
  { id: 'curved', label: 'Curved' },
  { id: 'supports', label: 'Supports' },
  { id: 'multi', label: 'Multi' },
  { id: 'export', label: 'Export' },
] as const;

export type DrawerTabId = (typeof DRAWER_TABS)[number]['id'];

interface AdvancedDrawerProps {
  open: boolean;
  activeTab: DrawerTabId;
  onTabChange: (tab: DrawerTabId) => void;
  onClose: () => void;
  settings: LithophaneSettings;
  appPreferences: AppPreferences;
  onOpenAppSettings: () => void;
  onUpdateSettings: (mutator: (draft: LithophaneSettings) => void) => void;
  onResetDefaults: () => void;
  onImportPresetJson: () => void;
  onSavePresetJson: () => void;
}

export function AdvancedDrawer({
  open,
  activeTab,
  onTabChange,
  onClose,
  settings,
  appPreferences,
  onOpenAppSettings,
  onUpdateSettings,
  onResetDefaults,
  onImportPresetJson,
  onSavePresetJson,
}: AdvancedDrawerProps) {
  if (!open) return null;

  return (
    <div className="absolute top-16 bottom-4 right-4 z-30 w-[390px] max-w-[calc(100vw-2rem)] flex flex-col rounded-2xl bg-[#131822]/94 backdrop-blur-2xl border border-white/10 shadow-2xl overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div>
          <h3 className="text-sm font-semibold text-slate-100">All Settings</h3>
          <p className="text-[11px] text-slate-400">Complete parameter & preset control</p>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={onImportPresetJson}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
            title="Import JSON preset"
          >
            <FolderInput className="w-4 h-4" />
          </button>
          <button
            onClick={onSavePresetJson}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
            title="Save JSON preset"
          >
            <Save className="w-4 h-4" />
          </button>
          <button
            onClick={onResetDefaults}
            className="p-1.5 rounded-lg text-slate-400 hover:text-amber-300 hover:bg-white/10 transition"
            title="Reset all settings to defaults"
          >
            <RotateCcw className="w-4 h-4" />
          </button>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Category tabs */}
      <div className="flex flex-wrap gap-1 px-3 py-2 border-b border-white/10 bg-black/25">
        {DRAWER_TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => onTabChange(t.id)}
            className={`px-2.5 py-1 rounded-lg text-[11px] font-medium transition ${
              activeTab === t.id
                ? 'bg-amber-500 text-slate-950'
                : 'text-slate-400 hover:text-white hover:bg-white/5'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3 text-xs">
        {activeTab === 'size' && (
          <>
            <SelectField
              label="Sizing mode"
              value={settings.size.mode}
              options={[
                ['max_dimension', 'Max dimension (longest outer side)'],
                ['width', 'Width (auto height)'],
                ['height', 'Height (auto width)'],
                ['absolute', 'Absolute (width × height)'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.size.mode = v as SizeMode; })}
            />
            {settings.size.mode === 'max_dimension' && (
              <NumField
                label="Max dimension (mm)"
                value={settings.size.max_dimension_mm}
                min={10}
                max={500}
                step={5}
                onChange={(v) => onUpdateSettings((d) => { d.size.max_dimension_mm = v; })}
              />
            )}
            {(settings.size.mode === 'width' || settings.size.mode === 'absolute') && (
              <NumField
                label="Outer width (mm)"
                value={settings.size.width_mm}
                min={10}
                max={500}
                step={5}
                onChange={(v) => onUpdateSettings((d) => { d.size.width_mm = v; })}
              />
            )}
            {(settings.size.mode === 'height' || settings.size.mode === 'absolute') && (
              <NumField
                label="Outer height (mm)"
                value={settings.size.height_mm}
                min={10}
                max={500}
                step={5}
                onChange={(v) => onUpdateSettings((d) => { d.size.height_mm = v; })}
              />
            )}
            {settings.size.mode === 'absolute' && (
              <SelectField
                label="Aspect fit"
                value={settings.size.fit}
                options={[
                  ['crop', 'Crop'],
                  ['pad', 'Pad'],
                  ['stretch', 'Stretch'],
                ]}
                onChange={(v) => onUpdateSettings((d) => { d.size.fit = v as FitMode; })}
              />
            )}
            <NumField
              label="Output pixel size (mm)"
              value={settings.size.pixel_size_mm}
              min={0.01}
              max={5}
              step={0.01}
              onChange={(v) => onUpdateSettings((d) => { d.size.pixel_size_mm = v; })}
            />
            <div className="flex items-center justify-between gap-2 px-2.5 py-2 rounded-xl bg-black/25 border border-white/5">
              <div>
                <div className="text-[11px] text-slate-300">3D Preview Resolution</div>
                <div className="text-[10px] text-slate-400">
                  {appPreferences.low_res_preview_enabled
                    ? `Low-res mode (${appPreferences.low_res_preview_pixel_size_mm.toFixed(2)} mm/px)`
                    : 'Matches final output resolution'}
                </div>
              </div>
              <button
                onClick={onOpenAppSettings}
                className="px-2 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-[10px] text-amber-300 border border-white/10"
              >
                App Settings
              </button>
            </div>
            <SelectField
              label="Flat print orientation"
              value={settings.orientation.mode}
              options={[
                ['auto_short_up', 'Auto short up (landscape prints upright)'],
                ['long_up', 'Long side up'],
                ['image_up', 'Image upright (no rotation)'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.orientation.mode = v as OrientationMode; })}
            />
          </>
        )}

        {activeTab === 'image' && (
          <>
            <div className="grid grid-cols-2 gap-2">
              <SelectField
                label="Greyscale"
                value={settings.image.greyscale}
                options={[
                  ['rec601', 'Rec. 601'],
                  ['rec709', 'Rec. 709'],
                  ['average', 'Average'],
                  ['red', 'Red'],
                  ['green', 'Green'],
                  ['blue', 'Blue'],
                ]}
                onChange={(v) => onUpdateSettings((d) => { d.image.greyscale = v as GreyscaleMode; })}
              />
              <SelectField
                label="Alpha background"
                value={settings.image.background}
                options={[
                  ['white', 'White'],
                  ['black', 'Black'],
                ]}
                onChange={(v) => onUpdateSettings((d) => { d.image.background = v as BackgroundColor; })}
              />
            </div>

            <div className="grid grid-cols-3 gap-2">
              <SelectField
                label="Rotate CW"
                value={String(settings.image.rotate)}
                options={[
                  ['0', '0°'],
                  ['90', '90°'],
                  ['180', '180°'],
                  ['270', '270°'],
                ]}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.image.rotate = Number(v) as 0 | 90 | 180 | 270;
                  })
                }
              />
              <BoolField
                label="Mirror L-R"
                checked={settings.image.mirror}
                onChange={(c) => onUpdateSettings((d) => { d.image.mirror = c; })}
              />
              <BoolField
                label="Flip T-B"
                checked={settings.image.flip}
                onChange={(c) => onUpdateSettings((d) => { d.image.flip = c; })}
              />
            </div>

            <CropEditor
              crop={settings.image.crop}
              onChange={(c) => onUpdateSettings((d) => { d.image.crop = c; })}
            />

            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Auto-levels (%)"
                value={settings.image.auto_levels_percent}
                min={0}
                max={20}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.image.auto_levels_percent = v; })}
              />
              <SelectField
                label="Equalize"
                value={settings.image.equalize}
                options={[
                  ['none', 'None'],
                  ['hist', 'Global Histogram'],
                  ['clahe', 'Adaptive CLAHE'],
                ]}
                onChange={(v) => onUpdateSettings((d) => { d.image.equalize = v as EqualizeMode; })}
              />
            </div>
            {settings.image.equalize === 'clahe' && (
              <NumField
                label="CLAHE clip limit"
                value={settings.image.clahe_clip}
                min={0.002}
                max={1}
                step={0.005}
                onChange={(v) => onUpdateSettings((d) => { d.image.clahe_clip = v; })}
              />
            )}

            <div className="grid grid-cols-3 gap-2">
              <NumField
                label="Brightness"
                value={settings.image.brightness}
                min={-1}
                max={1}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.image.brightness = v; })}
              />
              <NumField
                label="Contrast"
                value={settings.image.contrast}
                min={0.1}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.image.contrast = v; })}
              />
              <NumField
                label="Gamma"
                value={settings.image.gamma}
                min={0.1}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.image.gamma = v; })}
              />
            </div>

            <div className="grid grid-cols-3 gap-2">
              <NumField
                label="Blur (mm)"
                value={settings.image.blur_sigma_mm}
                min={0}
                max={10}
                step={0.02}
                onChange={(v) => onUpdateSettings((d) => { d.image.blur_sigma_mm = v; })}
              />
              <NumField
                label="Sharpen"
                value={settings.image.sharpen_amount}
                min={0}
                max={10}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.image.sharpen_amount = v; })}
              />
              <NumField
                label="Sharp radius"
                value={settings.image.sharpen_radius_mm}
                min={0.05}
                max={10}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.image.sharpen_radius_mm = v; })}
              />
            </div>

            <div className="grid grid-cols-2 gap-2">
              <OptNumField
                label="Posterize levels"
                value={settings.image.posterize_levels}
                min={2}
                max={256}
                step={1}
                placeholder="Off"
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.image.posterize_levels = v !== null ? Math.round(v) : null;
                  })
                }
              />
              <BoolField
                label="Invert negative"
                checked={settings.image.invert}
                onChange={(c) => onUpdateSettings((d) => { d.image.invert = c; })}
              />
            </div>
          </>
        )}

        {activeTab === 'thickness' && (
          <>
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Min thickness (white, mm)"
                value={settings.thickness.min_thickness_mm}
                min={0.2}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.thickness.min_thickness_mm = v; })}
              />
              <NumField
                label="Max thickness (black, mm)"
                value={settings.thickness.max_thickness_mm}
                min={0.5}
                max={12}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.thickness.max_thickness_mm = v; })}
              />
            </div>
            <SelectField
              label="Thickness curve"
              value={settings.thickness.curve}
              options={[
                ['linear', 'Linear in darkness'],
                ['transmission', 'Transmission (Beer-Lambert optical)'],
                ['gamma', 'Gamma power curve'],
                ['custom', 'Custom control points'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.thickness.curve = v as ThicknessCurve; })}
            />
            {settings.thickness.curve === 'gamma' && (
              <NumField
                label="Curve gamma"
                value={settings.thickness.curve_gamma}
                min={0.1}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.thickness.curve_gamma = v; })}
              />
            )}
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <NumField
                  label="Material attenuation (1/mm)"
                  value={settings.thickness.attenuation_per_mm}
                  min={0.05}
                  max={10}
                  step={0.05}
                  onChange={(v) => onUpdateSettings((d) => { d.thickness.attenuation_per_mm = v; })}
                />
              </div>
              <button
                onClick={() =>
                  onUpdateSettings((d) => {
                    d.thickness.attenuation_per_mm = Number(
                      (Math.log(100) / Math.max(0.2, d.lighting.transmission_distance_mm)).toFixed(2),
                    );
                  })
                }
                className="h-8 px-2.5 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-[11px] text-amber-300 shrink-0"
                title="Set attenuation k = ln(100) / TD to match filament Transmission Distance"
              >
                Match TD
              </button>
            </div>
            <SelectField
              label="Relief side"
              value={settings.thickness.relief_side}
              options={[
                ['front', 'Front (faces viewer)'],
                ['back', 'Back (smooth front)'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.thickness.relief_side = v as ReliefSide; })}
            />
          </>
        )}

        {activeTab === 'lighting' && (
          <>
            <SectionTitle title="Filament Optical Properties" />
            <div className="flex flex-wrap gap-1">
              {FILAMENT_PRESETS.map((fp) => (
                <button
                  key={fp.label}
                  onClick={() =>
                    onUpdateSettings((d) => {
                      d.lighting.filament_color = fp.color;
                      d.lighting.transmission_distance_mm = fp.td;
                    })
                  }
                  className="px-2 py-1 rounded-lg text-[10px] bg-black/35 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1.5"
                >
                  <span
                    className="w-2.5 h-2.5 rounded-full border border-white/25"
                    style={{ backgroundColor: fp.color }}
                  />
                  {fp.label} ({fp.td} mm)
                </button>
              ))}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <label className="space-y-1 block">
                <span className="text-[11px] text-slate-400 block">Filament colour</span>
                <div className="flex items-center gap-2 h-8 px-2 rounded-lg bg-black/35 border border-white/10">
                  <input
                    type="color"
                    value={settings.lighting.filament_color}
                    onChange={(e) =>
                      onUpdateSettings((d) => {
                        d.lighting.filament_color = e.target.value;
                      })
                    }
                    className="w-6 h-5 rounded bg-transparent border-0 cursor-pointer"
                  />
                  <span className="text-[11px] font-mono text-slate-200">
                    {settings.lighting.filament_color}
                  </span>
                </div>
              </label>
              <NumField
                label="Transmission dist (TD, mm)"
                value={settings.lighting.transmission_distance_mm}
                min={0.2}
                max={30}
                step={0.1}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.lighting.transmission_distance_mm = v;
                  })
                }
              />
            </div>

            <SectionTitle title="Backlight & Ambient Illumination" />
            <div className="flex flex-wrap gap-1">
              {BACKLIGHT_PRESETS.map((bp) => (
                <button
                  key={bp.label}
                  onClick={() =>
                    onUpdateSettings((d) => {
                      d.lighting.backlight_color = bp.color;
                    })
                  }
                  className="px-2 py-1 rounded-lg text-[10px] bg-black/35 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1.5"
                >
                  <span
                    className="w-2.5 h-2.5 rounded-full border border-white/25"
                    style={{ backgroundColor: bp.color }}
                  />
                  {bp.label}
                </button>
              ))}
            </div>
            <div className="grid grid-cols-3 gap-2">
              <label className="space-y-1 block">
                <span className="text-[11px] text-slate-400 block">Backlight</span>
                <div className="flex items-center gap-1.5 h-8 px-2 rounded-lg bg-black/35 border border-white/10">
                  <input
                    type="color"
                    value={settings.lighting.backlight_color}
                    onChange={(e) =>
                      onUpdateSettings((d) => {
                        d.lighting.backlight_color = e.target.value;
                      })
                    }
                    className="w-5 h-5 rounded bg-transparent border-0 cursor-pointer"
                  />
                  <span className="text-[10px] font-mono text-slate-200 truncate">
                    {settings.lighting.backlight_color}
                  </span>
                </div>
              </label>
              <NumField
                label="Intensity"
                value={settings.lighting.backlight_intensity}
                min={0.1}
                max={5.0}
                step={0.1}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.lighting.backlight_intensity = v;
                  })
                }
              />
              <NumField
                label="Front ambient"
                value={settings.lighting.ambient_front_light}
                min={0}
                max={0.5}
                step={0.01}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.lighting.ambient_front_light = v;
                  })
                }
              />
            </div>
          </>
        )}

        {activeTab === 'frame' && (
          <>
            <SectionTitle title="Panel Border Frame" />
            <BoolField
              label="Enable flat frame border"
              checked={settings.frame.enabled}
              onChange={(c) => onUpdateSettings((d) => { d.frame.enabled = c; })}
            />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Frame width (mm)"
                value={settings.frame.width_mm}
                min={0.5}
                max={50}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.frame.width_mm = v; })}
              />
              <OptNumField
                label="Frame thickness (mm)"
                value={settings.frame.thickness_mm}
                min={0.2}
                max={15}
                step={0.1}
                placeholder="Max thick"
                onChange={(v) => onUpdateSettings((d) => { d.frame.thickness_mm = v; })}
              />
            </div>

            <SectionTitle title="Bottom Holder Extension" />
            <BoolField
              label="Enable flat bottom slot extension"
              checked={settings.bottom_extension.enabled}
              onChange={(c) => onUpdateSettings((d) => { d.bottom_extension.enabled = c; })}
            />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Extension height (mm)"
                value={settings.bottom_extension.height_mm}
                min={0}
                max={100}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.bottom_extension.height_mm = v; })}
              />
              <NumField
                label="Extension thickness (mm)"
                value={settings.bottom_extension.thickness_mm}
                min={0.4}
                max={50}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.bottom_extension.thickness_mm = v; })}
              />
            </div>
          </>
        )}

        {activeTab === 'curved' && (
          <>
            <SectionTitle title="Cylindrical Curvature" />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Radius (mm)"
                value={settings.curved.radius_mm}
                min={10}
                max={1000}
                step={2}
                onChange={(v) => onUpdateSettings((d) => { d.curved.radius_mm = v; })}
              />
              <SelectField
                label="Curvature direction"
                value={settings.curved.direction}
                options={[
                  ['concave', 'Concave toward viewer'],
                  ['convex', 'Convex toward viewer'],
                ]}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.curved.direction = v as CurvatureDirection;
                  })
                }
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <SelectField
                label="Curved orientation"
                value={settings.curved.orientation}
                options={[
                  ['image_up', 'Upright (default)'],
                  ['auto_short_up', 'Auto short up'],
                  ['long_up', 'Long side up'],
                ]}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.curved.orientation = v as OrientationMode;
                  })
                }
              />
              <BoolField
                label="Radial supports"
                checked={settings.curved.supports_enabled}
                onChange={(c) => onUpdateSettings((d) => { d.curved.supports_enabled = c; })}
              />
            </div>

            <SectionTitle title="Curved Built-in Base" />
            <BoolField
              label="Enable curved stand base"
              checked={settings.curved.base.enabled}
              onChange={(c) => onUpdateSettings((d) => { d.curved.base.enabled = c; })}
            />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Base thickness (mm)"
                value={settings.curved.base.thickness_mm}
                min={0.2}
                max={20}
                step={0.2}
                onChange={(v) => onUpdateSettings((d) => { d.curved.base.thickness_mm = v; })}
              />
              <NumField
                label="Corner fillet (mm)"
                value={settings.curved.base.corner_fillet_mm}
                min={0}
                max={40}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.curved.base.corner_fillet_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Front projection (mm)"
                value={settings.curved.base.front_projection_mm}
                min={0}
                max={80}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.curved.base.front_projection_mm = v; })}
              />
              <NumField
                label="Back projection (mm)"
                value={settings.curved.base.back_projection_mm}
                min={0}
                max={80}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.curved.base.back_projection_mm = v; })}
              />
            </div>
          </>
        )}

        {activeTab === 'supports' && (
          <>
            <div className="grid grid-cols-2 gap-2">
              <BoolField
                label="Supports enabled"
                checked={settings.supports.enabled}
                onChange={(c) => onUpdateSettings((d) => { d.supports.enabled = c; })}
              />
              <SelectField
                label="Sides"
                value={settings.supports.sides}
                options={[
                  ['both', 'Both'],
                  ['front', 'Front (-Y)'],
                  ['back', 'Back (+Y)'],
                ]}
                onChange={(v) => onUpdateSettings((d) => { d.supports.sides = v as SupportSides; })}
              />
            </div>
            <SelectField
              label="Output packaging"
              value={settings.supports.output}
              options={[
                ['merged', 'Merged single watertight body'],
                ['separate_bodies', 'Separate shells in one file'],
                ['separate_files', 'Separate panel & support files'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.supports.output = v as SupportOutput; })}
            />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Fin count per side"
                value={settings.supports.fin_count}
                min={1}
                max={20}
                step={1}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.supports.fin_count = Math.round(v);
                  })
                }
              />
              <NumField
                label="Fin thickness (mm)"
                value={settings.supports.fin_thickness_mm}
                min={0.4}
                max={10}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.supports.fin_thickness_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <OptNumField
                label="Fin height (mm)"
                value={settings.supports.fin_height_mm}
                min={5}
                max={300}
                step={5}
                placeholder="Auto frac"
                onChange={(v) => onUpdateSettings((d) => { d.supports.fin_height_mm = v; })}
              />
              <NumField
                label="Height fraction"
                value={settings.supports.fin_height_fraction}
                min={0.1}
                max={1}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.supports.fin_height_fraction = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <OptNumField
                label="Fin base length (mm)"
                value={settings.supports.fin_base_length_mm}
                min={5}
                max={150}
                step={2}
                placeholder="Auto"
                onChange={(v) => onUpdateSettings((d) => { d.supports.fin_base_length_mm = v; })}
              />
              <OptNumField
                label="Fin inset (mm)"
                value={settings.supports.fin_inset_mm}
                min={0}
                max={100}
                step={0.5}
                placeholder="Frame ctr"
                onChange={(v) => onUpdateSettings((d) => { d.supports.fin_inset_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Interface gap (mm)"
                value={settings.supports.interface_gap_mm}
                min={0.05}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.supports.interface_gap_mm = v; })}
              />
              <NumField
                label="Tabs per fin"
                value={settings.supports.tab_count}
                min={1}
                max={20}
                step={1}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.supports.tab_count = Math.round(v);
                  })
                }
              />
            </div>
            <div className="grid grid-cols-3 gap-2">
              <NumField
                label="Tab height"
                value={settings.supports.tab_height_mm}
                min={0.2}
                max={10}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.supports.tab_height_mm = v; })}
              />
              <NumField
                label="Neck height"
                value={settings.supports.tab_neck_height_mm}
                min={0.1}
                max={5}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.supports.tab_neck_height_mm = v; })}
              />
              <NumField
                label="Tab overlap"
                value={settings.supports.tab_overlap_mm}
                min={0}
                max={2}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.supports.tab_overlap_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <OptNumField
                label="Tab width (mm)"
                value={settings.supports.tab_width_mm}
                min={0.2}
                max={10}
                step={0.1}
                placeholder="Fin thick"
                onChange={(v) => onUpdateSettings((d) => { d.supports.tab_width_mm = v; })}
              />
              <NumField
                label="Tab margin (mm)"
                value={settings.supports.tab_margin_mm}
                min={0}
                max={20}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.supports.tab_margin_mm = v; })}
              />
            </div>
            <SectionTitle title="Bed Adhesion Pads" />
            <div className="grid grid-cols-2 gap-2">
              <BoolField
                label="Enable pads"
                checked={settings.supports.pad_enabled}
                onChange={(c) => onUpdateSettings((d) => { d.supports.pad_enabled = c; })}
              />
              <NumField
                label="Pad thickness"
                value={settings.supports.pad_thickness_mm}
                min={0.2}
                max={5}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.supports.pad_thickness_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Pad side margin"
                value={settings.supports.pad_side_margin_mm}
                min={0}
                max={25}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.supports.pad_side_margin_mm = v; })}
              />
              <NumField
                label="Pad end margin"
                value={settings.supports.pad_end_margin_mm}
                min={0}
                max={25}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.supports.pad_end_margin_mm = v; })}
              />
            </div>
          </>
        )}

        {activeTab === 'multi' && (
          <>
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Common bed length (mm)"
                value={settings.multi.common_dimension_mm}
                min={10}
                max={450}
                step={5}
                onChange={(v) => onUpdateSettings((d) => { d.multi.common_dimension_mm = v; })}
              />
              <NumField
                label="Panel spacing Y (mm)"
                value={settings.multi.panel_spacing_mm}
                min={2}
                max={100}
                step={1}
                onChange={(v) => onUpdateSettings((d) => { d.multi.panel_spacing_mm = v; })}
              />
            </div>
            <SectionTitle title="Breakaway Bridges" />
            <div className="grid grid-cols-2 gap-2">
              <NumField
                label="Bridges per station"
                value={settings.multi.bridge_count}
                min={1}
                max={20}
                step={1}
                onChange={(v) =>
                  onUpdateSettings((d) => {
                    d.multi.bridge_count = Math.round(v);
                  })
                }
              />
              <BoolField
                label="Add center bridge"
                checked={settings.multi.add_center_bridge}
                onChange={(c) => onUpdateSettings((d) => { d.multi.add_center_bridge = c; })}
              />
            </div>
            <div className="grid grid-cols-3 gap-2">
              <NumField
                label="Bridge width"
                value={settings.multi.bridge_width_mm}
                min={0.5}
                max={20}
                step={0.2}
                onChange={(v) => onUpdateSettings((d) => { d.multi.bridge_width_mm = v; })}
              />
              <NumField
                label="Bridge height"
                value={settings.multi.bridge_height_mm}
                min={0.4}
                max={10}
                step={0.1}
                onChange={(v) => onUpdateSettings((d) => { d.multi.bridge_height_mm = v; })}
              />
              <NumField
                label="Anchor overlap"
                value={settings.multi.bridge_overlap_mm}
                min={0}
                max={2}
                step={0.05}
                onChange={(v) => onUpdateSettings((d) => { d.multi.bridge_overlap_mm = v; })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <OptNumField
                label="Bridge inset (mm)"
                value={settings.multi.bridge_inset_mm}
                min={0}
                max={100}
                step={0.5}
                placeholder="Match fins"
                onChange={(v) => onUpdateSettings((d) => { d.multi.bridge_inset_mm = v; })}
              />
              <NumField
                label="Z margin (mm)"
                value={settings.multi.margin_z_mm}
                min={0}
                max={30}
                step={0.5}
                onChange={(v) => onUpdateSettings((d) => { d.multi.margin_z_mm = v; })}
              />
            </div>
          </>
        )}

        {activeTab === 'export' && (
          <>
            <SelectField
              label="Default export format"
              value={settings.export.format}
              options={[
                ['stl', 'STL (universal slicer mesh)'],
                ['3mf', '3MF (compressed multi-part assembly + materials)'],
              ]}
              onChange={(v) => onUpdateSettings((d) => { d.export.format = v as ExportFormat; })}
            />
            <BoolField
              label="Binary STL (smaller, faster)"
              checked={settings.export.binary}
              onChange={(c) => onUpdateSettings((d) => { d.export.binary = c; })}
            />
            <label className="space-y-1 block">
              <span className="text-[11px] text-slate-400">Filename template</span>
              <input
                type="text"
                value={settings.export.filename_template}
                onChange={(e) =>
                  onUpdateSettings((d) => {
                    d.export.filename_template = e.target.value;
                  })
                }
                className="w-full h-8 px-2.5 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
              />
            </label>
            <NumField
              label="PLA density (g/cm³)"
              value={settings.export.pla_density_g_cm3}
              min={0.5}
              max={3.0}
              step={0.02}
              onChange={(v) => onUpdateSettings((d) => { d.export.pla_density_g_cm3 = v; })}
            />
          </>
        )}
      </div>
    </div>
  );
}

function SectionTitle({ title }: { title: string }) {
  return (
    <div className="pt-2 pb-0.5 text-[11px] font-semibold uppercase tracking-wider text-amber-400/90 border-b border-white/5">
      {title}
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: [string, string][];
  onChange: (val: string) => void;
}) {
  return (
    <label className="space-y-1 block">
      <span className="text-[11px] text-slate-400 block">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full h-8 px-2 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
      >
        {options.map(([val, text]) => (
          <option key={val} value={val} className="bg-[#131822]">
            {text}
          </option>
        ))}
      </select>
    </label>
  );
}

function NumField({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (val: number) => void;
}) {
  return (
    <label className="space-y-1 block">
      <span className="text-[11px] text-slate-400 block truncate">{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (!Number.isNaN(n) && n >= min && n <= max) {
            onChange(n);
          }
        }}
        className="w-full h-8 px-2 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
      />
    </label>
  );
}

function OptNumField({
  label,
  value,
  min,
  max,
  step,
  placeholder,
  onChange,
}: {
  label: string;
  value: number | null;
  min: number;
  max: number;
  step: number;
  placeholder: string;
  onChange: (val: number | null) => void;
}) {
  return (
    <label className="space-y-1 block">
      <span className="text-[11px] text-slate-400 block truncate">{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        step={step}
        placeholder={placeholder}
        value={value ?? ''}
        onChange={(e) => {
          if (e.target.value.trim() === '') {
            onChange(null);
            return;
          }
          const n = Number(e.target.value);
          if (!Number.isNaN(n) && n >= min && n <= max) {
            onChange(n);
          }
        }}
        className="w-full h-8 px-2 rounded-lg bg-black/35 border border-white/10 text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-amber-500/60"
      />
    </label>
  );
}

function BoolField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-2 px-2.5 h-8 rounded-lg bg-black/25 border border-white/5 cursor-pointer self-end">
      <span className="text-[11px] text-slate-200 truncate">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="accent-amber-500 w-3.5 h-3.5 shrink-0"
      />
    </label>
  );
}

function CropEditor({
  crop,
  onChange,
}: {
  crop: [number, number, number, number] | null;
  onChange: (c: [number, number, number, number] | null) => void;
}) {
  const val = crop ?? [0, 0, 1, 1];
  const updateIdx = (idx: number, raw: number) => {
    const next: [number, number, number, number] = [...val] as [number, number, number, number];
    next[idx] = Math.min(1, Math.max(0, raw));
    if (next[0] < next[2] && next[1] < next[3]) {
      onChange(next);
    }
  };
  return (
    <div className="rounded-xl bg-black/25 border border-white/5 p-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[11px] text-slate-300">Crop box (0..1 fractions)</span>
        <button
          onClick={() => onChange(crop ? null : [0.05, 0.05, 0.95, 0.95])}
          className="text-[10px] text-amber-400 hover:text-amber-300"
        >
          {crop ? 'Clear crop' : 'Enable crop'}
        </button>
      </div>
      {crop && (
        <div className="grid grid-cols-4 gap-1.5">
          {(['L', 'T', 'R', 'B'] as const).map((lbl, idx) => (
            <label key={lbl} className="space-y-0.5">
              <span className="text-[10px] text-slate-500">{lbl}</span>
              <input
                type="number"
                min={0}
                max={1}
                step={0.02}
                value={val[idx]}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  updateIdx(idx, Number(e.target.value))
                }
                className="w-full h-7 px-1.5 rounded bg-black/40 border border-white/10 text-[11px] text-slate-100"
              />
            </label>
          ))}
        </div>
      )}
    </div>
  );
}
