import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { FolderOpen, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { AppPreferences, ExportFormat, ShadingMode } from '../types';

interface AppSettingsModalProps {
  open: boolean;
  onClose: () => void;
  preferences: AppPreferences;
  onSavePreferences: (prefs: AppPreferences) => Promise<void>;
}

const BED_PRESETS = [
  { label: 'Bambu 256×256', x: 256, y: 256 },
  { label: 'Prusa 250×210', x: 250, y: 210 },
  { label: 'Ender 220×220', x: 220, y: 220 },
  { label: 'Voron 300×300', x: 300, y: 300 },
] as const;

export function AppSettingsModal({
  open,
  onClose,
  preferences,
  onSavePreferences,
}: AppSettingsModalProps) {
  const [draft, setDraft] = useState<AppPreferences>(preferences);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      setDraft(preferences);
    }
  }, [open, preferences]);

  if (!open) return null;

  const pickDirectory = async (
    current: string | null,
    onPicked: (dir: string) => void,
  ) => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: current ?? undefined,
    });
    if (selected && !Array.isArray(selected)) {
      onPicked(selected);
    }
  };

  const handleApply = async () => {
    setSaving(true);
    try {
      await onSavePreferences(draft);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-4">
      <div className="w-full max-w-2xl max-h-[88vh] flex flex-col rounded-3xl bg-[#131822] border border-white/10 shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-white/10">
          <div>
            <h2 className="text-sm font-semibold text-slate-100">Application Settings</h2>
            <p className="text-[11px] text-slate-400">
              Preview performance, project storage, print bed grid, and export defaults
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-5 text-xs">
          {/* 1. Preview Resolution & Performance */}
          <section className="space-y-2.5">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-amber-400">
              Preview Resolution & Performance
            </h3>
            <div className="rounded-2xl bg-black/30 border border-white/10 p-3.5 space-y-3">
              <label className="flex items-start justify-between gap-3 cursor-pointer">
                <div>
                  <div className="font-medium text-slate-100">Low-resolution preview mode</div>
                  <div className="text-[11px] text-slate-400 mt-0.5">
                    When disabled (default), the 3D preview uses your exact final export pixel size.
                    Enable this for faster scrubbing on huge or fine-detail lithophanes.
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={draft.low_res_preview_enabled}
                  onChange={(e) =>
                    setDraft((d) => ({ ...d, low_res_preview_enabled: e.target.checked }))
                  }
                  className="accent-amber-500 w-4 h-4 mt-0.5 shrink-0"
                />
              </label>

              {draft.low_res_preview_enabled && (
                <div className="grid grid-cols-2 gap-3 pt-2 border-t border-white/5">
                  <label className="space-y-1">
                    <span className="text-[11px] text-slate-400">Low-res preview pitch (mm)</span>
                    <input
                      type="number"
                      min={0.1}
                      max={2.0}
                      step={0.05}
                      value={draft.low_res_preview_pixel_size_mm}
                      onChange={(e) => {
                        const v = Number(e.target.value);
                        if (v >= 0.05 && v <= 5.0) {
                          setDraft((d) => ({ ...d, low_res_preview_pixel_size_mm: v }));
                        }
                      }}
                      className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                    />
                  </label>
                </div>
              )}

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2 border-t border-white/5">
                <label className="flex items-center justify-between gap-2 px-3 h-8 rounded-lg bg-black/25 border border-white/5 cursor-pointer self-end">
                  <span className="text-[11px] text-slate-200">Auto-update preview</span>
                  <input
                    type="checkbox"
                    checked={draft.auto_update_preview}
                    onChange={(e) =>
                      setDraft((d) => ({ ...d, auto_update_preview: e.target.checked }))
                    }
                    className="accent-amber-500 w-3.5 h-3.5"
                  />
                </label>
                <label className="space-y-1">
                  <span className="text-[11px] text-slate-400">Preview debounce (ms)</span>
                  <input
                    type="number"
                    min={50}
                    max={2000}
                    step={20}
                    value={draft.preview_debounce_ms}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (v >= 50 && v <= 2000) {
                        setDraft((d) => ({ ...d, preview_debounce_ms: Math.round(v) }));
                      }
                    }}
                    className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                  />
                </label>
                <label className="space-y-1">
                  <span className="text-[11px] text-slate-400">Heavy mesh warning (tris)</span>
                  <input
                    type="number"
                    min={100000}
                    max={10000000}
                    step={100000}
                    value={draft.warn_triangle_threshold}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (v >= 50000) {
                        setDraft((d) => ({ ...d, warn_triangle_threshold: Math.round(v) }));
                      }
                    }}
                    className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                  />
                </label>
              </div>
            </div>
          </section>

          {/* 2. Projects & Folders */}
          <section className="space-y-2.5">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-amber-400">
              Projects & Storage
            </h3>
            <div className="rounded-2xl bg-black/30 border border-white/10 p-3.5 space-y-3">
              <div className="space-y-1">
                <span className="text-[11px] text-slate-400">Projects folder</span>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={draft.projects_root_dir}
                    onChange={(e) =>
                      setDraft((d) => ({ ...d, projects_root_dir: e.target.value }))
                    }
                    className="flex-1 h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100 text-[11px]"
                  />
                  <button
                    onClick={() =>
                      void pickDirectory(draft.projects_root_dir, (dir) =>
                        setDraft((d) => ({ ...d, projects_root_dir: dir })),
                      )
                    }
                    className="h-8 px-2.5 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    Browse…
                  </button>
                </div>
              </div>

              <div className="space-y-1">
                <span className="text-[11px] text-slate-400">
                  Default export folder (optional)
                </span>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="Ask / use project folder"
                    value={draft.default_export_dir ?? ''}
                    onChange={(e) =>
                      setDraft((d) => ({
                        ...d,
                        default_export_dir: e.target.value.trim() || null,
                      }))
                    }
                    className="flex-1 h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100 placeholder:text-slate-500 text-[11px]"
                  />
                  <button
                    onClick={() =>
                      void pickDirectory(draft.default_export_dir, (dir) =>
                        setDraft((d) => ({ ...d, default_export_dir: dir })),
                      )
                    }
                    className="h-8 px-2.5 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    Browse…
                  </button>
                  {draft.default_export_dir && (
                    <button
                      onClick={() => setDraft((d) => ({ ...d, default_export_dir: null }))}
                      className="h-8 px-2.5 rounded-lg bg-white/5 hover:bg-white/10 text-slate-400"
                    >
                      Clear
                    </button>
                  )}
                </div>
              </div>

              <label className="flex items-center justify-between gap-2 pt-1 cursor-pointer">
                <span className="text-[11px] text-slate-200">
                  Automatically reopen last active project on launch
                </span>
                <input
                  type="checkbox"
                  checked={draft.auto_open_last_project}
                  onChange={(e) =>
                    setDraft((d) => ({ ...d, auto_open_last_project: e.target.checked }))
                  }
                  className="accent-amber-500 w-3.5 h-3.5"
                />
              </label>
            </div>
          </section>

          {/* 3. 3D Viewport & Print Bed */}
          <section className="space-y-2.5">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-amber-400">
              3D Viewport & Print Bed
            </h3>
            <div className="rounded-2xl bg-black/30 border border-white/10 p-3.5 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <label className="space-y-1">
                  <span className="text-[11px] text-slate-400">Default shading mode</span>
                  <select
                    value={draft.default_shading_mode}
                    onChange={(e) =>
                      setDraft((d) => ({
                        ...d,
                        default_shading_mode: e.target.value as ShadingMode,
                      }))
                    }
                    className="w-full h-8 px-2 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                  >
                    <option value="backlit" className="bg-[#131822]">
                      Backlit
                    </option>
                    <option value="clay" className="bg-[#131822]">
                      Clay
                    </option>
                    <option value="wireframe" className="bg-[#131822]">
                      Wireframe
                    </option>
                  </select>
                </label>

                <label className="flex items-center justify-between gap-2 px-3 h-8 rounded-lg bg-black/25 border border-white/5 cursor-pointer self-end">
                  <span className="text-[11px] text-slate-200">Show print bed grid</span>
                  <input
                    type="checkbox"
                    checked={draft.show_print_bed_grid}
                    onChange={(e) =>
                      setDraft((d) => ({ ...d, show_print_bed_grid: e.target.checked }))
                    }
                    className="accent-amber-500 w-3.5 h-3.5"
                  />
                </label>
              </div>

              <div className="flex flex-wrap gap-1.5">
                {BED_PRESETS.map((bp) => (
                  <button
                    key={bp.label}
                    onClick={() =>
                      setDraft((d) => ({ ...d, bed_size_x_mm: bp.x, bed_size_y_mm: bp.y }))
                    }
                    className="px-2 py-1 rounded-lg text-[10px] bg-white/5 hover:bg-white/10 border border-white/10 text-slate-300"
                  >
                    {bp.label}
                  </button>
                ))}
              </div>

              <div className="grid grid-cols-2 gap-3">
                <label className="space-y-1">
                  <span className="text-[11px] text-slate-400">Bed width X (mm)</span>
                  <input
                    type="number"
                    min={60}
                    max={1000}
                    step={5}
                    value={draft.bed_size_x_mm}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (v >= 60 && v <= 1000) setDraft((d) => ({ ...d, bed_size_x_mm: v }));
                    }}
                    className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                  />
                </label>
                <label className="space-y-1">
                  <span className="text-[11px] text-slate-400">Bed depth Y (mm)</span>
                  <input
                    type="number"
                    min={60}
                    max={1000}
                    step={5}
                    value={draft.bed_size_y_mm}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (v >= 60 && v <= 1000) setDraft((d) => ({ ...d, bed_size_y_mm: v }));
                    }}
                    className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                  />
                </label>
              </div>
            </div>
          </section>

          {/* 4. Export Defaults */}
          <section className="space-y-2.5">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-amber-400">
              Export Defaults
            </h3>
            <div className="rounded-2xl bg-black/30 border border-white/10 p-3.5 grid grid-cols-2 gap-3">
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Default export format</span>
                <select
                  value={draft.default_export_format}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      default_export_format: e.target.value as ExportFormat,
                    }))
                  }
                  className="w-full h-8 px-2 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                >
                  <option value="stl" className="bg-[#131822]">
                    STL
                  </option>
                  <option value="3mf" className="bg-[#131822]">
                    3MF
                  </option>
                </select>
              </label>
              <label className="space-y-1">
                <span className="text-[11px] text-slate-400">Default PLA density (g/cm³)</span>
                <input
                  type="number"
                  min={0.5}
                  max={3.0}
                  step={0.02}
                  value={draft.default_pla_density_g_cm3}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    if (v >= 0.5 && v <= 3.0) {
                      setDraft((d) => ({ ...d, default_pla_density_g_cm3: v }));
                    }
                  }}
                  className="w-full h-8 px-2.5 rounded-lg bg-black/40 border border-white/10 text-slate-100"
                />
              </label>
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-5 py-3.5 border-t border-white/10 bg-black/25">
          <button
            onClick={onClose}
            className="h-8 px-3.5 rounded-lg text-xs font-medium bg-white/5 hover:bg-white/10 text-slate-300"
          >
            Cancel
          </button>
          <button
            onClick={() => void handleApply()}
            disabled={saving}
            className="h-8 px-4 rounded-lg text-xs font-semibold bg-gradient-to-r from-amber-400 to-orange-500 text-slate-950 disabled:opacity-40"
          >
            {saving ? 'Saving…' : 'Save App Settings'}
          </button>
        </div>
      </div>
    </div>
  );
}
