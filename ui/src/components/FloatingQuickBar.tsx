import { ChevronDown, ChevronUp, FolderOpen, SlidersHorizontal } from 'lucide-react';
import { useState } from 'react';
import type {
  GenerationReport,
  LithophaneSettings,
  LoadedImage,
  MatchDimensionMode,
  StyleId,
} from '../types';
import { MultiPhotoStrip } from './MultiPhotoStrip';

interface FloatingQuickBarProps {
  activeStyle: StyleId;
  images: LoadedImage[];
  settings: LithophaneSettings;
  report: GenerationReport | null;
  onUpdateSettings: (mutator: (draft: LithophaneSettings) => void) => void;
  onSelectImages: () => void;
  onAddMoreMultiPhotos: () => void;
  onRemoveMultiPhoto: (index: number) => void;
  onMoveMultiPhoto: (index: number, delta: -1 | 1) => void;
  onMappingChange: (index: number, mode: MatchDimensionMode) => void;
  onOpenAdvanced: (tab?: string) => void;
  onOpen2D: () => void;
}

const QUALITY_STEPS = [
  { label: 'Draft', px: 0.4 },
  { label: 'Std', px: 0.25 },
  { label: 'Fine', px: 0.15 },
  { label: 'Ultra', px: 0.1 },
];

export function FloatingQuickBar({
  activeStyle,
  images,
  settings,
  report,
  onUpdateSettings,
  onSelectImages,
  onAddMoreMultiPhotos,
  onRemoveMultiPhoto,
  onMoveMultiPhoto,
  onMappingChange,
  onOpenAdvanced,
  onOpen2D,
}: FloatingQuickBarProps) {
  const [collapsed, setCollapsed] = useState(false);

  const primary = images[0];
  const arcDeg = report?.curved_info?.arc_angle_deg;

  return (
    <aside className="absolute bottom-4 left-4 z-20 w-[320px] max-w-[calc(100vw-2rem)] rounded-2xl bg-[#131822]/88 backdrop-blur-xl border border-white/10 shadow-2xl overflow-hidden">
      {/* Header strip */}
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-white/5">
        {activeStyle === 'multi_flat' ? (
          <div className="flex items-center gap-2 min-w-0">
            <span className="text-xs font-semibold text-slate-100">
              Multi-Panel ({images.length})
            </span>
          </div>
        ) : primary ? (
          <div className="flex items-center gap-2.5 min-w-0 flex-1">
            <img
              src={`data:image/png;base64,${primary.thumbnail_b64}`}
              alt={primary.stem}
              className="w-8 h-8 rounded-lg object-cover border border-white/10 shrink-0"
            />
            <div className="min-w-0 flex-1">
              <div className="text-xs font-medium text-slate-100 truncate">{primary.stem}</div>
              <div className="text-[10px] text-slate-400">
                {primary.width} × {primary.height} px
              </div>
            </div>
            <button
              onClick={onSelectImages}
              className="p-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-slate-300 hover:text-white transition"
              title="Change image"
            >
              <FolderOpen className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <button
            onClick={onSelectImages}
            className="flex items-center gap-2 text-xs font-medium text-amber-300 hover:text-amber-200 transition"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            Load image…
          </button>
        )}

        <button
          onClick={() => setCollapsed((c) => !c)}
          className="ml-2 p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/5 transition"
          title={collapsed ? 'Expand quick controls' : 'Collapse quick controls'}
        >
          {collapsed ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
        </button>
      </div>

      {!collapsed && (
        <div className="p-3 space-y-3 text-xs">
          {/* Multi photo strip */}
          {activeStyle === 'multi_flat' && (
            <MultiPhotoStrip
              images={images}
              commonDimMm={settings.multi.common_dimension_mm}
              onAddPhotos={onAddMoreMultiPhotos}
              onRemovePhoto={onRemoveMultiPhoto}
              onMovePhoto={onMoveMultiPhoto}
              onMappingChange={onMappingChange}
            />
          )}

          {/* Size control */}
          {activeStyle === 'multi_flat' ? (
            <div className="grid grid-cols-2 gap-2">
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Bed width (mm)</span>
                <input
                  type="number"
                  min={20}
                  max={400}
                  step={5}
                  value={settings.multi.common_dimension_mm}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    if (v > 5) {
                      onUpdateSettings((d) => {
                        d.multi.common_dimension_mm = v;
                      });
                    }
                  }}
                  className="w-full h-8 px-2.5 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
                />
              </label>
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Gap Y (mm)</span>
                <input
                  type="number"
                  min={2}
                  max={100}
                  step={1}
                  value={settings.multi.panel_spacing_mm}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    if (v >= 2) {
                      onUpdateSettings((d) => {
                        d.multi.panel_spacing_mm = v;
                      });
                    }
                  }}
                  className="w-full h-8 px-2.5 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
                />
              </label>
            </div>
          ) : (
            <div className="space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-slate-400">
                  {activeStyle === 'curved' ? 'Arc size (max mm)' : 'Max size (mm)'}
                </span>
                <span className="text-[11px] font-semibold text-slate-200">
                  {settings.size.max_dimension_mm.toFixed(0)} mm
                </span>
              </div>
              <input
                type="range"
                min={40}
                max={260}
                step={5}
                value={settings.size.max_dimension_mm}
                onChange={(e) => {
                  const v = Number(e.target.value);
                  onUpdateSettings((d) => {
                    d.size.mode = 'max_dimension';
                    d.size.max_dimension_mm = v;
                  });
                }}
                className="w-full studio-range"
              />
            </div>
          )}

          {/* Curved-only sleek controls */}
          {activeStyle === 'curved' && (
            <div className="space-y-2.5 pt-1 border-t border-white/5">
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-slate-400">Curve radius</span>
                  <div className="flex items-center gap-1.5">
                    {arcDeg !== undefined && (
                      <span className="px-1.5 py-0.2 rounded bg-amber-500/15 text-amber-300 text-[10px] font-medium">
                        {arcDeg.toFixed(0)}°
                      </span>
                    )}
                    <span className="text-[11px] font-semibold text-slate-200">
                      {settings.curved.radius_mm.toFixed(0)} mm
                    </span>
                  </div>
                </div>
                <input
                  type="range"
                  min={30}
                  max={300}
                  step={2}
                  value={settings.curved.radius_mm}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    onUpdateSettings((d) => {
                      d.curved.radius_mm = v;
                    });
                  }}
                  className="w-full studio-range"
                />
              </div>

              <div className="grid grid-cols-2 rounded-xl bg-black/35 p-0.5 border border-white/5">
                {(['concave', 'convex'] as const).map((dir) => (
                  <button
                    key={dir}
                    onClick={() =>
                      onUpdateSettings((d) => {
                        d.curved.direction = dir;
                      })
                    }
                    className={`py-1 rounded-lg text-[11px] font-medium capitalize transition ${
                      settings.curved.direction === dir
                        ? 'bg-amber-500/20 text-amber-200 border border-amber-500/40'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    {dir}
                  </button>
                ))}
              </div>

              {/* Built-in Base toggle + compact parameters */}
              <div className="rounded-xl bg-black/25 border border-white/5 p-2 space-y-2">
                <label className="flex items-center justify-between cursor-pointer">
                  <span className="text-[11px] font-medium text-slate-200">
                    Built-in filleted base
                  </span>
                  <input
                    type="checkbox"
                    checked={settings.curved.base.enabled}
                    onChange={(e) =>
                      onUpdateSettings((d) => {
                        d.curved.base.enabled = e.target.checked;
                      })
                    }
                    className="accent-amber-500 w-3.5 h-3.5"
                  />
                </label>

                {settings.curved.base.enabled && (
                  <div className="grid grid-cols-3 gap-1.5">
                    <label className="space-y-0.5">
                      <span className="text-[10px] text-slate-400 block">Thick</span>
                      <input
                        type="number"
                        min={0.4}
                        max={10}
                        step={0.2}
                        value={settings.curved.base.thickness_mm}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          if (v >= 0.2) {
                            onUpdateSettings((d) => {
                              d.curved.base.thickness_mm = v;
                            });
                          }
                        }}
                        className="w-full h-7 px-1.5 rounded bg-black/40 border border-white/10 text-[11px] text-slate-100"
                      />
                    </label>
                    <label className="space-y-0.5">
                      <span className="text-[10px] text-slate-400 block">Proj F/B</span>
                      <input
                        type="number"
                        min={0}
                        max={40}
                        step={1}
                        value={settings.curved.base.front_projection_mm}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          if (v >= 0) {
                            onUpdateSettings((d) => {
                              d.curved.base.front_projection_mm = v;
                              d.curved.base.back_projection_mm = v;
                            });
                          }
                        }}
                        className="w-full h-7 px-1.5 rounded bg-black/40 border border-white/10 text-[11px] text-slate-100"
                      />
                    </label>
                    <label className="space-y-0.5">
                      <span className="text-[10px] text-slate-400 block">Fillet</span>
                      <input
                        type="number"
                        min={0}
                        max={20}
                        step={0.5}
                        value={settings.curved.base.corner_fillet_mm}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          if (v >= 0) {
                            onUpdateSettings((d) => {
                              d.curved.base.corner_fillet_mm = v;
                            });
                          }
                        }}
                        className="w-full h-7 px-1.5 rounded bg-black/40 border border-white/10 text-[11px] text-slate-100"
                      />
                    </label>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Thickness range */}
          {activeStyle !== 'multi_flat' && (
            <div className="grid grid-cols-2 gap-2">
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Min thick (mm)</span>
                <input
                  type="number"
                  min={0.2}
                  max={5}
                  step={0.1}
                  value={settings.thickness.min_thickness_mm}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    if (v > 0.05 && v < settings.thickness.max_thickness_mm) {
                      onUpdateSettings((d) => {
                        d.thickness.min_thickness_mm = v;
                      });
                    }
                  }}
                  className="w-full h-7 px-2 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
                />
              </label>
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Max thick (mm)</span>
                <input
                  type="number"
                  min={0.5}
                  max={10}
                  step={0.1}
                  value={settings.thickness.max_thickness_mm}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    if (v > settings.thickness.min_thickness_mm) {
                      onUpdateSettings((d) => {
                        d.thickness.max_thickness_mm = v;
                      });
                    }
                  }}
                  className="w-full h-7 px-2 rounded-lg bg-black/35 border border-white/10 text-slate-100 focus:outline-none focus:border-amber-500/60"
                />
              </label>
            </div>
          )}

          {/* Export resolution quick pills */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-slate-400">Export resolution</span>
              <span className="text-[10px] text-slate-500">
                {settings.size.pixel_size_mm.toFixed(2)} mm/px
              </span>
            </div>
            <div className="grid grid-cols-4 gap-1 rounded-xl bg-black/35 p-0.5 border border-white/5">
              {QUALITY_STEPS.map((q) => {
                const active = Math.abs(settings.size.pixel_size_mm - q.px) < 0.02;
                return (
                  <button
                    key={q.label}
                    onClick={() =>
                      onUpdateSettings((d) => {
                        d.size.pixel_size_mm = q.px;
                      })
                    }
                    className={`py-1 rounded-lg text-[11px] font-medium transition ${
                      active
                        ? 'bg-amber-500 text-slate-950 shadow-sm'
                        : 'text-slate-400 hover:text-slate-100'
                    }`}
                  >
                    {q.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Supports toggle */}
          <label className="flex items-center justify-between px-2 py-1.5 rounded-xl bg-black/25 border border-white/5 cursor-pointer">
            <span className="text-[11px] text-slate-300">
              {activeStyle === 'curved'
                ? 'Radial support fins'
                : activeStyle === 'multi_flat'
                  ? 'Outer fins & breakaway bridges'
                  : 'Breakaway support fins'}
            </span>
            <input
              type="checkbox"
              checked={
                activeStyle === 'curved'
                  ? settings.curved.supports_enabled
                  : settings.supports.enabled
              }
              onChange={(e) => {
                const checked = e.target.checked;
                onUpdateSettings((d) => {
                  if (activeStyle === 'curved') {
                    d.curved.supports_enabled = checked;
                  } else {
                    d.supports.enabled = checked;
                  }
                });
              }}
              className="accent-amber-500 w-3.5 h-3.5"
            />
          </label>

          {/* Footer links */}
          <div className="flex items-center justify-between pt-1 border-t border-white/5 text-[11px]">
            <button
              onClick={() => onOpenAdvanced()}
              className="flex items-center gap-1 text-slate-400 hover:text-amber-300 transition"
            >
              <SlidersHorizontal className="w-3 h-3" />
              All Settings…
            </button>
            <button
              onClick={onOpen2D}
              disabled={images.length === 0}
              className="text-slate-400 hover:text-amber-300 disabled:opacity-40 transition"
            >
              2D Previews & Report
            </button>
          </div>
        </div>
      )}
    </aside>
  );
}
