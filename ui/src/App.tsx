import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { AlertCircle, CheckCircle2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  deleteProject,
  duplicateProject,
  exportModel,
  generatePreview,
  getBootstrap,
  getPreviewGeometry,
  inspectImages,
  listProjects,
  loadPresetCmd,
  loadProject,
  openProjectFile,
  readSettingsJson,
  renameProject,
  rerenderBacklit2d,
  saveAppPreferences,
  saveProject,
  saveSettingsJson,
} from './api';
import { AdvancedDrawer, type DrawerTabId } from './components/AdvancedDrawer';
import { AppSettingsModal } from './components/AppSettingsModal';
import { DropZoneOverlay } from './components/DropZoneOverlay';
import { FloatingQuickBar } from './components/FloatingQuickBar';
import { Preview2DModal } from './components/Preview2DModal';
import { ProjectsModal } from './components/ProjectsModal';
import { TopBar } from './components/TopBar';
import { Viewport3D, type ViewportHandle } from './components/Viewport3D';
import type {
  AppPreferences,
  BinaryScenePart,
  BundledPreset,
  GenerationReport,
  ImagePreviewB64,
  LightingSettings,
  LithophaneSettings,
  LoadedImage,
  LoadedProject,
  MatchDimensionMode,
  ProjectSummary,
  ShadingMode,
  StyleDescriptor,
  StyleId,
} from './types';

const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff', 'gif']);

function isImagePath(p: string): boolean {
  const ext = p.split('.').pop()?.toLowerCase() ?? '';
  return IMAGE_EXTENSIONS.has(ext);
}

function cloneSettings(s: LithophaneSettings): LithophaneSettings {
  return JSON.parse(JSON.stringify(s)) as LithophaneSettings;
}

function snapshotKey(
  style: StyleId,
  settings: LithophaneSettings,
  images: LoadedImage[],
): string {
  return JSON.stringify({
    style,
    settings,
    inputs: images.map((im) => ({ path: im.path, mapping_mode: im.mapping_mode })),
  });
}

function joinDirAndFile(dir: string, file: string): string {
  const sep = dir.includes('\\') ? '\\' : '/';
  const cleaned = dir.replace(/[\\/]+$/, '');
  return `${cleaned}${sep}${file}`;
}

export default function App() {
  const [styles, setStyles] = useState<StyleDescriptor[]>([]);
  const [presets, setPresets] = useState<BundledPreset[]>([]);
  const [defaultSettings, setDefaultSettings] = useState<LithophaneSettings | null>(null);
  const [settings, setSettings] = useState<LithophaneSettings | null>(null);
  const [appPrefs, setAppPrefs] = useState<AppPreferences | null>(null);
  const [activeStyle, setActiveStyle] = useState<StyleId>('flat');
  const [activePreset, setActivePreset] = useState<string>('default');

  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [activeProject, setActiveProject] = useState<ProjectSummary | null>(null);
  const [savedSnapshot, setSavedSnapshot] = useState<string>('');

  const [images, setImages] = useState<LoadedImage[]>([]);
  const [shading, setShading] = useState<ShadingMode>('backlit');

  const [sceneParts, setSceneParts] = useState<BinaryScenePart[]>([]);
  const [previews2D, setPreviews2D] = useState<ImagePreviewB64[]>([]);
  const [report, setReport] = useState<GenerationReport | null>(null);
  const [lastStem, setLastStem] = useState<string>('lithophane');
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);

  const [isPreviewLoading, setIsPreviewLoading] = useState(false);
  const [manualTrigger, setManualTrigger] = useState(0);
  const [isExporting, setIsExporting] = useState(false);
  const [exportProgress, setExportProgress] = useState<{ message: string; fraction: number } | null>(
    null,
  );
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerTab, setDrawerTab] = useState<DrawerTabId>('curved');
  const [modal2DOpen, setModal2DOpen] = useState(false);
  const [projectsModalOpen, setProjectsModalOpen] = useState(false);
  const [projectsSavePrompt, setProjectsSavePrompt] = useState(false);
  const [appSettingsOpen, setAppSettingsOpen] = useState(false);
  const [isDragging, setIsDragging] = useState(false);

  const viewportRef = useRef<ViewportHandle | null>(null);
  const requestSeqRef = useRef(0);
  const settingsRef = useRef<LithophaneSettings | null>(null);
  settingsRef.current = settings;

  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    window.setTimeout(() => setToastMsg((cur) => (cur === msg ? null : cur)), 4500);
  }, []);

  const applyLoadedProject = useCallback((loaded: LoadedProject) => {
    setActiveProject(loaded.summary);
    setActiveStyle(loaded.summary.style);
    setSettings(cloneSettings(loaded.settings));
    setImages(loaded.images);
    setActivePreset('custom');
    setSavedSnapshot(snapshotKey(loaded.summary.style, loaded.settings, loaded.images));
    setManualTrigger((n) => n + 1);
  }, []);

  const refreshProjectsList = useCallback(async () => {
    try {
      const list = await listProjects();
      setProjects(list);
    } catch {
      // Non-fatal if projects folder doesn't exist yet
    }
  }, []);

  // Initial bootstrap from Rust core + optional last-project auto-restore
  useEffect(() => {
    getBootstrap()
      .then(async (info) => {
        setStyles(info.styles);
        setPresets(info.presets);

        const base = cloneSettings(info.default_settings);
        base.export.format = info.preferences.default_export_format;
        base.export.pla_density_g_cm3 = info.preferences.default_pla_density_g_cm3;
        setDefaultSettings(cloneSettings(base));
        setSettings(cloneSettings(base));
        setAppPrefs(info.preferences);
        setShading(info.preferences.default_shading_mode);

        await refreshProjectsList();

        if (info.preferences.auto_open_last_project && info.preferences.last_project_id) {
          try {
            const restored = await loadProject(info.preferences.last_project_id);
            applyLoadedProject(restored);
            showToast(`Restored project "${restored.summary.name}"`);
          } catch {
            // Last project missing on disk — fall back cleanly to fresh studio
          }
        }
      })
      .catch((err) => setErrorMsg(String(err)));
  }, [applyLoadedProject, refreshProjectsList, showToast]);

  // Listen to export progress events from Rust worker
  useEffect(() => {
    const unlistenPromise = listen<{ message: string; fraction: number }>('export-progress', (ev) => {
      setExportProgress(ev.payload);
    });
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  const addOrReplaceImages = useCallback(async (paths: string[], appendForMulti: boolean) => {
    const validPaths = paths.filter(isImagePath);
    if (validPaths.length === 0) return;

    try {
      setErrorMsg(null);
      const metas = await inspectImages(validPaths);
      const loaded: LoadedImage[] = metas.map((m) => ({ ...m, mapping_mode: 'long_side' }));

      if (appendForMulti) {
        setImages((prev) => [...prev, ...loaded]);
      } else {
        setImages(loaded);
        if (loaded.length >= 2) {
          setActiveStyle('multi_flat');
        }
      }
      setManualTrigger((n) => n + 1);
    } catch (err) {
      setErrorMsg(`Failed to load image(s): ${String(err)}`);
    }
  }, []);

  // Native window file drag & drop
  useEffect(() => {
    const webview = getCurrentWebview();
    const unlistenPromise = webview.onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        setIsDragging(true);
      } else if (event.payload.type === 'leave') {
        setIsDragging(false);
      } else if (event.payload.type === 'drop') {
        setIsDragging(false);
        const dropped = event.payload.paths;
        if (dropped.length > 0) {
          void addOrReplaceImages(dropped, activeStyle === 'multi_flat' && images.length > 0);
        }
      }
    });
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, [activeStyle, images.length, addOrReplaceImages]);

  const handleSelectImages = async (appendForMulti = false) => {
    const multiple = activeStyle === 'multi_flat' || appendForMulti;
    const selected = await openDialog({
      multiple,
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff', 'gif'],
        },
      ],
    });
    if (!selected) return;
    const list = Array.isArray(selected) ? selected : [selected];
    await addOrReplaceImages(list, appendForMulti);
  };

  const updateSettings = useCallback((mutator: (draft: LithophaneSettings) => void) => {
    setSettings((prev) => {
      if (!prev) return prev;
      const next = cloneSettings(prev);
      mutator(next);
      return next;
    });
    setActivePreset('custom');
  }, []);

  const updateLighting = useCallback((mutator: (lit: LightingSettings) => void) => {
    setSettings((prev) => {
      if (!prev) return prev;
      const next = cloneSettings(prev);
      mutator(next.lighting);
      return next;
    });
  }, []);

  // Mesh-only settings hash so live 60 FPS lighting changes never trigger a mesh rebuild
  const meshSettingsKey = useMemo(() => {
    if (!settings) return '';
    const { lighting: _lit, ...rest } = settings;
    return JSON.stringify(rest);
  }, [settings]);

  const runPreviewNow = useCallback(async () => {
    const curSettings = settingsRef.current;
    if (!curSettings || !appPrefs) return;
    const minNeeded = activeStyle === 'multi_flat' ? 2 : 1;
    if (images.length < minNeeded) {
      setSceneParts([]);
      setReport(null);
      setPreviews2D([]);
      return;
    }

    const reqId = ++requestSeqRef.current;
    setIsPreviewLoading(true);
    setErrorMsg(null);
    try {
      const inputs =
        activeStyle === 'multi_flat'
          ? images.map((im) => ({ path: im.path, mapping_mode: im.mapping_mode }))
          : [{ path: images[0].path, mapping_mode: null }];

      const previewPx = appPrefs.low_res_preview_enabled
        ? appPrefs.low_res_preview_pixel_size_mm
        : null;

      const meta = await generatePreview({
        style: activeStyle,
        inputs,
        settings: curSettings,
        preview_pixel_size_mm: previewPx,
      });
      if (reqId !== requestSeqRef.current) return;

      const binParts = await getPreviewGeometry(meta.scene_id);
      if (reqId !== requestSeqRef.current) return;

      setSceneParts(binParts);
      setPreviews2D(meta.previews);
      setReport(meta.report);
      setLastStem(meta.stem);
      setElapsedMs(meta.elapsed_ms);
    } catch (err) {
      if (reqId === requestSeqRef.current) {
        setErrorMsg(String(err));
      }
    } finally {
      if (reqId === requestSeqRef.current) {
        setIsPreviewLoading(false);
      }
    }
  }, [activeStyle, images, appPrefs]);

  // Automatic debounced preview when auto_update_preview is enabled
  useEffect(() => {
    if (!appPrefs || !settingsRef.current) return;
    const minNeeded = activeStyle === 'multi_flat' ? 2 : 1;
    if (images.length < minNeeded) {
      setSceneParts([]);
      setReport(null);
      setPreviews2D([]);
      return;
    }
    if (!appPrefs.auto_update_preview) {
      return;
    }

    const timer = window.setTimeout(() => {
      void runPreviewNow();
    }, appPrefs.preview_debounce_ms);

    return () => window.clearTimeout(timer);
  }, [
    meshSettingsKey,
    activeStyle,
    images,
    appPrefs?.low_res_preview_enabled,
    appPrefs?.low_res_preview_pixel_size_mm,
    appPrefs?.auto_update_preview,
    appPrefs?.preview_debounce_ms,
    runPreviewNow,
    appPrefs,
  ]);

  // Trigger one preview automatically on explicit image load/project load even when auto-update is off
  useEffect(() => {
    if (manualTrigger === 0 || !appPrefs || appPrefs.auto_update_preview) return;
    void runPreviewNow();
  }, [manualTrigger, appPrefs, runPreviewNow]);

  const handleOpen2DModal = async () => {
    if (settings && previews2D.length > 0) {
      try {
        const updatedBacklits = await rerenderBacklit2d(settings);
        setPreviews2D((prev) =>
          prev.map((p, i) => ({
            ...p,
            backlit_b64: updatedBacklits[i] ?? p.backlit_b64,
          })),
        );
      } catch {
        // Keep existing cached PNG if re-render unavailable
      }
    }
    setModal2DOpen(true);
  };

  const handleStyleChange = (id: StyleId) => {
    setActiveStyle(id);
    if (drawerOpen) {
      if (id === 'curved') setDrawerTab('curved');
      if (id === 'multi_flat') setDrawerTab('multi');
      if (id === 'flat') setDrawerTab('size');
    }
  };

  const handlePresetSelect = async (name: string) => {
    try {
      const p = await loadPresetCmd(name);
      setSettings(p.settings);
      setActivePreset(p.name);
      if (activeStyle !== 'multi_flat') {
        setActiveStyle(p.recommended_style);
      }
      showToast(`Loaded preset "${p.label}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const buildRequestInputs = useCallback(
    () => images.map((im) => ({ path: im.path, mapping_mode: im.mapping_mode })),
    [images],
  );

  const handleQuickSaveProject = useCallback(async () => {
    if (!settings || images.length === 0) return;
    if (!activeProject) {
      setProjectsSavePrompt(true);
      setProjectsModalOpen(true);
      return;
    }
    try {
      const saved = await saveProject({
        id: activeProject.id,
        name: activeProject.name,
        notes: activeProject.notes,
        style: activeStyle,
        settings,
        inputs: buildRequestInputs(),
        thumbnail_b64: previews2D[0]?.backlit_b64 ?? null,
      });
      setActiveProject(saved.summary);
      setImages(saved.images);
      setSavedSnapshot(snapshotKey(saved.summary.style, saved.settings, saved.images));
      await refreshProjectsList();
      showToast(`Saved "${saved.summary.name}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  }, [
    settings,
    images,
    activeProject,
    activeStyle,
    buildRequestInputs,
    previews2D,
    refreshProjectsList,
    showToast,
  ]);

  // Ctrl+S keyboard shortcut for saving active project
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        void handleQuickSaveProject();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleQuickSaveProject]);

  const handleSaveAsNewProject = async (name: string, notes: string) => {
    if (!settings) return;
    const saved = await saveProject({
      id: null,
      name,
      notes,
      style: activeStyle,
      settings,
      inputs: buildRequestInputs(),
      thumbnail_b64: previews2D[0]?.backlit_b64 ?? null,
    });
    setActiveProject(saved.summary);
    setImages(saved.images);
    setSavedSnapshot(snapshotKey(saved.summary.style, saved.settings, saved.images));
    await refreshProjectsList();
    setProjectsSavePrompt(false);
    setProjectsModalOpen(false);
    showToast(`Saved project "${saved.summary.name}"`);
  };

  const handleNewEmptyProject = () => {
    if (!defaultSettings) return;
    setActiveProject(null);
    setSavedSnapshot('');
    setImages([]);
    setSceneParts([]);
    setReport(null);
    setPreviews2D([]);
    setActiveStyle('flat');
    setSettings(cloneSettings(defaultSettings));
    setActivePreset('default');
  };

  const handleOpenProjectById = async (id: string) => {
    try {
      const loaded = await loadProject(id);
      applyLoadedProject(loaded);
      setProjectsModalOpen(false);
      showToast(`Opened "${loaded.summary.name}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleOpenProjectFromFile = async () => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: 'Lithophane Project (project.json)', extensions: ['json'] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      const loaded = await openProjectFile(selected);
      applyLoadedProject(loaded);
      setProjectsModalOpen(false);
      showToast(`Opened "${loaded.summary.name}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleDuplicateProject = async (id: string, newName: string) => {
    try {
      const loaded = await duplicateProject(id, newName);
      await refreshProjectsList();
      applyLoadedProject(loaded);
      showToast(`Duplicated as "${loaded.summary.name}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleRenameProject = async (id: string, newName: string, notes: string) => {
    try {
      const updated = await renameProject(id, newName, notes);
      if (activeProject?.id === id) {
        setActiveProject(updated);
      }
      await refreshProjectsList();
      showToast(`Updated "${updated.name}"`);
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleDeleteProject = async (id: string) => {
    try {
      await deleteProject(id);
      if (activeProject?.id === id) {
        setActiveProject(null);
        setSavedSnapshot('');
      }
      await refreshProjectsList();
      showToast('Deleted project');
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleSaveAppPreferences = async (nextPrefs: AppPreferences) => {
    try {
      const saved = await saveAppPreferences(nextPrefs);
      setAppPrefs(saved);
      if (defaultSettings) {
        const updatedDef = cloneSettings(defaultSettings);
        updatedDef.export.format = saved.default_export_format;
        updatedDef.export.pla_density_g_cm3 = saved.default_pla_density_g_cm3;
        setDefaultSettings(updatedDef);
      }
      await refreshProjectsList();
      showToast('Saved application settings');
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleExport = async () => {
    if (!settings) return;
    const minNeeded = activeStyle === 'multi_flat' ? 2 : 1;
    if (images.length < minNeeded) {
      setErrorMsg(`Need at least ${minNeeded} image(s) for ${activeStyle} export.`);
      return;
    }

    const ext = settings.export.format === '3mf' ? '3mf' : 'stl';
    const fileName = `${lastStem}.${ext}`;
    const preferredDir = appPrefs?.default_export_dir ?? activeProject?.exports_dir ?? null;
    const defaultPath = preferredDir ? joinDirAndFile(preferredDir, fileName) : fileName;

    const chosen = await saveDialog({
      defaultPath,
      filters: [
        { name: 'STL Mesh (*.stl)', extensions: ['stl'] },
        { name: '3MF Project (*.3mf)', extensions: ['3mf'] },
      ],
    });
    if (!chosen) return;

    setIsExporting(true);
    setExportProgress({ message: 'Preparing high-resolution mesh…', fraction: 0.05 });
    setErrorMsg(null);
    try {
      const inputs =
        activeStyle === 'multi_flat'
          ? images.map((im) => ({ path: im.path, mapping_mode: im.mapping_mode }))
          : [{ path: images[0].path, mapping_mode: null }];
      const res = await exportModel({
        style: activeStyle,
        inputs,
        settings,
        target_path: chosen,
      });
      setReport(res.report);
      const totalMb = res.files.reduce((acc, f) => acc + f.bytes, 0) / 1e6;
      const summary =
        res.files.length === 1
          ? `Exported ${res.files[0].path.split(/[\\/]/).pop()} (${totalMb.toFixed(2)} MB in ${res.elapsed_ms} ms)`
          : `Exported ${res.files.length} files (${totalMb.toFixed(2)} MB total)`;
      showToast(summary);
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setIsExporting(false);
      setExportProgress(null);
    }
  };

  const handleImportJsonPreset = async () => {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: 'JSON Preset', extensions: ['json'] }],
    });
    if (!path || Array.isArray(path)) return;
    try {
      const loaded = await readSettingsJson(path);
      setSettings(loaded);
      setActivePreset('custom');
      showToast('Imported settings JSON');
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleSaveJsonPreset = async () => {
    if (!settings) return;
    const path = await saveDialog({
      defaultPath: 'lithophane_preset.json',
      filters: [{ name: 'JSON Preset', extensions: ['json'] }],
    });
    if (!path) return;
    try {
      await saveSettingsJson(path, settings);
      showToast('Saved settings JSON');
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const advancedTweakCount = useMemo(() => {
    if (!settings || !defaultSettings) return 0;
    let count = 0;
    if (JSON.stringify(settings.image) !== JSON.stringify(defaultSettings.image)) count++;
    if (JSON.stringify(settings.frame) !== JSON.stringify(defaultSettings.frame)) count++;
    if (
      JSON.stringify(settings.bottom_extension) !==
      JSON.stringify(defaultSettings.bottom_extension)
    )
      count++;
    if (settings.thickness.curve !== defaultSettings.thickness.curve) count++;
    if (settings.thickness.relief_side !== defaultSettings.thickness.relief_side) count++;
    if (JSON.stringify(settings.lighting) !== JSON.stringify(defaultSettings.lighting)) count++;
    return count;
  }, [settings, defaultSettings]);

  const isDirty = useMemo(() => {
    if (!settings) return false;
    if (!activeProject) return images.length > 0;
    return snapshotKey(activeStyle, settings, images) !== savedSnapshot;
  }, [activeProject, activeStyle, settings, images, savedSnapshot]);

  const hasEnoughImages =
    activeStyle === 'multi_flat' ? images.length >= 2 : images.length >= 1;

  if (!settings || !appPrefs) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-[#0b0e14] text-slate-400 text-xs">
        Initializing Lithophane Studio…
      </div>
    );
  }

  return (
    <div className="relative w-full h-full overflow-hidden bg-[#0b0e14]">
      <Viewport3D
        ref={viewportRef}
        parts={sceneParts}
        report={report}
        shading={shading}
        onShadingChange={setShading}
        lighting={settings.lighting}
        onLightingChange={updateLighting}
        showBedGrid={appPrefs.show_print_bed_grid}
        bedSizeXMm={appPrefs.bed_size_x_mm}
        bedSizeYMm={appPrefs.bed_size_y_mm}
        lowResActive={appPrefs.low_res_preview_enabled}
        warnTriangleThreshold={appPrefs.warn_triangle_threshold}
        isLoading={isPreviewLoading}
        elapsedMs={elapsedMs}
      />

      <TopBar
        styles={styles}
        activeStyle={activeStyle}
        onStyleChange={handleStyleChange}
        presets={presets}
        activePreset={activePreset}
        onPresetSelect={(name) => void handlePresetSelect(name)}
        activeProjectName={activeProject?.name ?? null}
        isDirty={isDirty}
        onOpenProjects={() => {
          setProjectsSavePrompt(false);
          void refreshProjectsList();
          setProjectsModalOpen(true);
        }}
        onSaveProject={() => void handleQuickSaveProject()}
        hasImages={hasEnoughImages}
        autoUpdatePreview={appPrefs.auto_update_preview}
        onManualPreview={() => void runPreviewNow()}
        onOpen2DModal={() => void handleOpen2DModal()}
        drawerOpen={drawerOpen}
        onToggleDrawer={() => setDrawerOpen((o) => !o)}
        advancedTweakCount={advancedTweakCount}
        onOpenAppSettings={() => setAppSettingsOpen(true)}
        onExport={() => void handleExport()}
        isExporting={isExporting}
      />

      <FloatingQuickBar
        activeStyle={activeStyle}
        images={images}
        settings={settings}
        report={report}
        onUpdateSettings={updateSettings}
        onSelectImages={() => void handleSelectImages(false)}
        onAddMoreMultiPhotos={() => void handleSelectImages(true)}
        onRemoveMultiPhoto={(idx) =>
          setImages((prev) => prev.filter((_, i) => i !== idx))
        }
        onMoveMultiPhoto={(idx, delta) =>
          setImages((prev) => {
            const next = [...prev];
            const swap = idx + delta;
            if (swap < 0 || swap >= next.length) return prev;
            [next[idx], next[swap]] = [next[swap], next[idx]];
            return next;
          })
        }
        onMappingChange={(idx: number, mode: MatchDimensionMode) =>
          setImages((prev) =>
            prev.map((item, i) => (i === idx ? { ...item, mapping_mode: mode } : item)),
          )
        }
        onOpenAdvanced={(tab) => {
          if (tab) setDrawerTab(tab as DrawerTabId);
          setDrawerOpen(true);
        }}
        onOpen2D={() => void handleOpen2DModal()}
      />

      <AdvancedDrawer
        open={drawerOpen}
        activeTab={drawerTab}
        onTabChange={setDrawerTab}
        onClose={() => setDrawerOpen(false)}
        settings={settings}
        appPreferences={appPrefs}
        onOpenAppSettings={() => setAppSettingsOpen(true)}
        onUpdateSettings={updateSettings}
        onResetDefaults={() => {
          if (defaultSettings) {
            setSettings(cloneSettings(defaultSettings));
            setActivePreset('default');
            showToast('Reset settings to defaults');
          }
        }}
        onImportPresetJson={() => void handleImportJsonPreset()}
        onSavePresetJson={() => void handleSaveJsonPreset()}
      />

      <ProjectsModal
        open={projectsModalOpen}
        initialShowSaveForm={projectsSavePrompt}
        onClose={() => {
          setProjectsModalOpen(false);
          setProjectsSavePrompt(false);
        }}
        projects={projects}
        activeProjectId={activeProject?.id ?? null}
        hasActiveImages={images.length > 0}
        onNewProject={handleNewEmptyProject}
        onSaveAsNew={handleSaveAsNewProject}
        onOpenProject={handleOpenProjectById}
        onOpenProjectFile={handleOpenProjectFromFile}
        onDuplicateProject={handleDuplicateProject}
        onRenameProject={handleRenameProject}
        onDeleteProject={handleDeleteProject}
      />

      <AppSettingsModal
        open={appSettingsOpen}
        onClose={() => setAppSettingsOpen(false)}
        preferences={appPrefs}
        onSavePreferences={handleSaveAppPreferences}
      />

      <Preview2DModal
        open={modal2DOpen}
        onClose={() => setModal2DOpen(false)}
        previews={previews2D}
        report={report}
      />

      <DropZoneOverlay
        isDragging={isDragging}
        hasImages={hasEnoughImages}
        activeStyle={activeStyle}
        onSelectImages={() => void handleSelectImages(activeStyle === 'multi_flat')}
        onSwitchStyle={handleStyleChange}
      />

      {/* Export progress bar overlay */}
      {isExporting && exportProgress && (
        <div className="absolute top-16 left-1/2 -translate-x-1/2 z-40 w-80 rounded-2xl bg-[#131822]/95 backdrop-blur-xl border border-amber-500/30 p-3 shadow-2xl space-y-2">
          <div className="flex items-center justify-between text-xs">
            <span className="font-medium text-amber-200">{exportProgress.message}</span>
            <span className="text-slate-400">{Math.round(exportProgress.fraction * 100)}%</span>
          </div>
          <div className="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-amber-400 to-orange-500 transition-all duration-150"
              style={{ width: `${Math.round(exportProgress.fraction * 100)}%` }}
            />
          </div>
        </div>
      )}

      {/* Error banner */}
      {errorMsg && (
        <div className="absolute top-16 left-1/2 -translate-x-1/2 z-40 max-w-lg flex items-center gap-2.5 px-4 py-2.5 rounded-xl bg-rose-950/90 border border-rose-500/40 text-xs text-rose-100 shadow-2xl">
          <AlertCircle className="w-4 h-4 text-rose-400 shrink-0" />
          <span className="flex-1">{errorMsg}</span>
          <button
            onClick={() => setErrorMsg(null)}
            className="text-rose-300 hover:text-white font-semibold"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Success toast */}
      {toastMsg && (
        <div className="absolute bottom-16 right-4 z-40 flex items-center gap-2 px-3.5 py-2 rounded-xl bg-emerald-950/90 border border-emerald-500/40 text-xs text-emerald-100 shadow-2xl">
          <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
          <span>{toastMsg}</span>
        </div>
      )}
    </div>
  );
}
