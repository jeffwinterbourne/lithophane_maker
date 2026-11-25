import {
  Download,
  Eye,
  FolderKanban,
  Layers,
  RefreshCw,
  Save,
  Settings,
  SlidersHorizontal,
} from 'lucide-react';
import type { BundledPreset, StyleDescriptor, StyleId } from '../types';

interface TopBarProps {
  styles: StyleDescriptor[];
  activeStyle: StyleId;
  onStyleChange: (id: StyleId) => void;
  presets: BundledPreset[];
  activePreset: string;
  onPresetSelect: (name: string) => void;
  activeProjectName: string | null;
  isDirty: boolean;
  onOpenProjects: () => void;
  onSaveProject: () => void;
  hasImages: boolean;
  autoUpdatePreview: boolean;
  onManualPreview: () => void;
  onOpen2DModal: () => void;
  drawerOpen: boolean;
  onToggleDrawer: () => void;
  advancedTweakCount: number;
  onOpenAppSettings: () => void;
  onExport: () => void;
  isExporting: boolean;
}

export function TopBar({
  styles,
  activeStyle,
  onStyleChange,
  presets,
  activePreset,
  onPresetSelect,
  activeProjectName,
  isDirty,
  onOpenProjects,
  onSaveProject,
  hasImages,
  autoUpdatePreview,
  onManualPreview,
  onOpen2DModal,
  drawerOpen,
  onToggleDrawer,
  advancedTweakCount,
  onOpenAppSettings,
  onExport,
  isExporting,
}: TopBarProps) {
  return (
    <header className="absolute top-3 left-3 right-3 z-20 flex items-center justify-between gap-2 px-3.5 h-12 rounded-2xl bg-[#131822]/85 backdrop-blur-xl border border-white/10 shadow-2xl">
      {/* Left: Brand + Project Pill + Style Switcher */}
      <div className="flex items-center gap-2.5 min-w-0">
        <div className="flex items-center gap-2 shrink-0">
          <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-amber-400 to-orange-600 flex items-center justify-center shadow-md shadow-orange-500/20">
            <Layers className="w-4 h-4 text-slate-950" />
          </div>
          <span className="text-sm font-semibold tracking-tight text-slate-100 hidden lg:inline">
            Lithophane Studio
          </span>
        </div>

        <div className="h-5 w-px bg-white/10 hidden sm:block" />

        {/* Project Pill + Save */}
        <div className="flex items-center gap-1 shrink-0">
          <button
            onClick={onOpenProjects}
            className="h-8 px-2.5 rounded-lg bg-black/35 hover:bg-black/50 border border-white/10 text-xs text-slate-200 flex items-center gap-1.5 max-w-[160px] transition"
            title="Manage & restore saved projects"
          >
            <FolderKanban className="w-3.5 h-3.5 text-amber-400 shrink-0" />
            <span className="truncate">{activeProjectName ?? 'Untitled'}</span>
            {isDirty && (
              <span
                className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0"
                title="Unsaved changes"
              />
            )}
          </button>
          <button
            onClick={onSaveProject}
            disabled={!hasImages}
            className="h-8 w-8 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 disabled:opacity-35 disabled:pointer-events-none flex items-center justify-center transition"
            title="Save project (Ctrl+S)"
          >
            <Save className="w-3.5 h-3.5" />
          </button>
        </div>

        <div className="h-5 w-px bg-white/10 hidden md:block" />

        <nav className="flex items-center rounded-xl bg-black/30 p-0.5 border border-white/5">
          {styles.map((st) => {
            const active = st.id === activeStyle;
            return (
              <button
                key={st.id}
                onClick={() => onStyleChange(st.id)}
                title={st.tagline}
                className={`px-2.5 py-1 rounded-lg text-xs font-medium transition ${
                  active
                    ? 'bg-amber-500 text-slate-950 shadow-sm'
                    : 'text-slate-300 hover:text-white hover:bg-white/5'
                }`}
              >
                {st.name}
              </button>
            );
          })}
        </nav>
      </div>

      {/* Right: Preset picker, Manual refresh (if auto off), 2D & Report, All Settings, App Settings, Export */}
      <div className="flex items-center gap-1.5 shrink-0">
        <select
          value={activePreset}
          onChange={(e) => onPresetSelect(e.target.value)}
          className="h-8 rounded-lg bg-black/35 border border-white/10 px-2 text-xs text-slate-200 focus:outline-none focus:border-amber-500/60 max-w-[140px]"
          title="Load a bundled preset"
        >
          <option value="custom" disabled>
            Custom settings
          </option>
          {presets.map((p) => (
            <option key={p.name} value={p.name} className="bg-[#131822]">
              {p.label}
            </option>
          ))}
        </select>

        {!autoUpdatePreview && (
          <button
            onClick={onManualPreview}
            disabled={!hasImages}
            className="h-8 px-2.5 rounded-lg text-xs font-medium flex items-center gap-1 border border-amber-500/40 bg-amber-500/15 text-amber-200 hover:bg-amber-500/25 disabled:opacity-40 transition"
            title="Update 3D preview manually"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span className="hidden xl:inline">Update</span>
          </button>
        )}

        <button
          onClick={onOpen2DModal}
          disabled={!hasImages}
          className="h-8 px-2.5 rounded-lg text-xs font-medium flex items-center gap-1.5 border border-white/10 bg-white/5 text-slate-200 hover:bg-white/10 disabled:opacity-40 disabled:pointer-events-none transition"
          title="View 2D previews & print report"
        >
          <Eye className="w-3.5 h-3.5 text-amber-400" />
          <span className="hidden xl:inline">2D & Report</span>
        </button>

        <button
          onClick={onToggleDrawer}
          className={`h-8 px-2.5 rounded-lg text-xs font-medium flex items-center gap-1.5 border transition ${
            drawerOpen
              ? 'border-amber-500/60 bg-amber-500/15 text-amber-200'
              : 'border-white/10 bg-white/5 text-slate-200 hover:bg-white/10'
          }`}
          title="Open all lithophane settings & JSON presets"
        >
          <SlidersHorizontal className="w-3.5 h-3.5" />
          <span className="hidden md:inline">All Settings</span>
          {advancedTweakCount > 0 && (
            <span className="px-1.5 py-0.2 rounded-full bg-amber-500/25 text-amber-300 text-[10px] font-semibold">
              {advancedTweakCount}
            </span>
          )}
        </button>

        <button
          onClick={onOpenAppSettings}
          className="h-8 w-8 rounded-lg flex items-center justify-center border border-white/10 bg-white/5 text-slate-200 hover:bg-white/10 transition"
          title="Application settings (preview resolution, folders, bed size)"
        >
          <Settings className="w-3.5 h-3.5" />
        </button>

        <button
          onClick={onExport}
          disabled={!hasImages || isExporting}
          className="h-8 px-3.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 bg-gradient-to-r from-amber-400 to-orange-500 text-slate-950 shadow-lg shadow-orange-500/20 hover:brightness-105 disabled:opacity-40 disabled:pointer-events-none transition"
        >
          <Download className="w-3.5 h-3.5" />
          <span>{isExporting ? 'Exporting…' : 'Export…'}</span>
        </button>
      </div>
    </header>
  );
}
