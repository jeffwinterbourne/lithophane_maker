import {
  Copy,
  FilePlus2,
  FolderOpen,
  Pencil,
  Save,
  Search,
  Trash2,
  X,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import type { ProjectSummary } from '../types';

interface ProjectsModalProps {
  open: boolean;
  initialShowSaveForm?: boolean;
  onClose: () => void;
  projects: ProjectSummary[];
  activeProjectId: string | null;
  hasActiveImages: boolean;
  onNewProject: () => void;
  onSaveAsNew: (name: string, notes: string) => Promise<void>;
  onOpenProject: (id: string) => Promise<void>;
  onOpenProjectFile: () => Promise<void>;
  onDuplicateProject: (id: string, newName: string) => Promise<void>;
  onRenameProject: (id: string, newName: string, notes: string) => Promise<void>;
  onDeleteProject: (id: string) => Promise<void>;
}

function formatUpdated(ms: number): string {
  if (!ms) return '';
  try {
    return new Date(ms).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return '';
  }
}

export function ProjectsModal({
  open,
  initialShowSaveForm = false,
  onClose,
  projects,
  activeProjectId,
  hasActiveImages,
  onNewProject,
  onSaveAsNew,
  onOpenProject,
  onOpenProjectFile,
  onDuplicateProject,
  onRenameProject,
  onDeleteProject,
}: ProjectsModalProps) {
  const [query, setQuery] = useState('');
  const [showSaveForm, setShowSaveForm] = useState(false);
  const [saveName, setSaveName] = useState('');
  const [saveNotes, setSaveNotes] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [editNotes, setEditNotes] = useState('');
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open && initialShowSaveForm && hasActiveImages) {
      setShowSaveForm(true);
    }
  }, [open, initialShowSaveForm, hasActiveImages]);

  if (!open) return null;

  const filtered = projects.filter((p) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      p.notes.toLowerCase().includes(q) ||
      p.style.toLowerCase().includes(q)
    );
  });

  const handleCreateSubmit = async () => {
    if (!saveName.trim() || busy) return;
    setBusy(true);
    try {
      await onSaveAsNew(saveName.trim(), saveNotes.trim());
      setSaveName('');
      setSaveNotes('');
      setShowSaveForm(false);
    } finally {
      setBusy(false);
    }
  };

  const handleRenameSubmit = async (id: string) => {
    if (!editName.trim() || busy) return;
    setBusy(true);
    try {
      await onRenameProject(id, editName.trim(), editNotes.trim());
      setEditingId(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-4">
      <div className="w-full max-w-4xl max-h-[88vh] flex flex-col rounded-3xl bg-[#131822] border border-white/10 shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-white/10">
          <div>
            <h2 className="text-sm font-semibold text-slate-100">Projects</h2>
            <p className="text-[11px] text-slate-400">
              Self-contained folders with copied source images, settings, and thumbnails
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                onNewProject();
                onClose();
              }}
              className="h-8 px-3 rounded-lg text-xs font-medium bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1.5 transition"
            >
              <FilePlus2 className="w-3.5 h-3.5 text-amber-400" />
              New Empty
            </button>
            <button
              onClick={() => void onOpenProjectFile()}
              className="h-8 px-3 rounded-lg text-xs font-medium bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 flex items-center gap-1.5 transition"
            >
              <FolderOpen className="w-3.5 h-3.5" />
              Open File…
            </button>
            <button
              onClick={() => setShowSaveForm((s) => !s)}
              disabled={!hasActiveImages}
              className="h-8 px-3 rounded-lg text-xs font-semibold bg-gradient-to-r from-amber-400 to-orange-500 text-slate-950 disabled:opacity-40 disabled:pointer-events-none flex items-center gap-1.5 transition"
            >
              <Save className="w-3.5 h-3.5" />
              Save Current As…
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Save As New Form */}
        {showSaveForm && (
          <div className="px-5 py-3.5 bg-amber-500/10 border-b border-amber-500/20 flex flex-col sm:flex-row items-stretch sm:items-end gap-2.5 text-xs">
            <label className="flex-1 space-y-1">
              <span className="text-[11px] font-medium text-amber-200">Project name</span>
              <input
                type="text"
                autoFocus
                placeholder="e.g. Sunset Curved Lamp"
                value={saveName}
                onChange={(e) => setSaveName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleCreateSubmit();
                }}
                className="w-full h-8 px-2.5 rounded-lg bg-black/45 border border-white/15 text-slate-100 focus:outline-none focus:border-amber-400"
              />
            </label>
            <label className="flex-1 space-y-1">
              <span className="text-[11px] font-medium text-slate-300">Notes (optional)</span>
              <input
                type="text"
                placeholder="e.g. Jade White PLA, 110 mm radius"
                value={saveNotes}
                onChange={(e) => setSaveNotes(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleCreateSubmit();
                }}
                className="w-full h-8 px-2.5 rounded-lg bg-black/45 border border-white/15 text-slate-100 focus:outline-none focus:border-amber-400"
              />
            </label>
            <div className="flex items-center gap-1.5 shrink-0">
              <button
                onClick={() => void handleCreateSubmit()}
                disabled={!saveName.trim() || busy}
                className="h-8 px-3.5 rounded-lg font-semibold bg-amber-500 text-slate-950 disabled:opacity-40"
              >
                {busy ? 'Saving…' : 'Create Project'}
              </button>
              <button
                onClick={() => setShowSaveForm(false)}
                className="h-8 px-2.5 rounded-lg bg-white/5 text-slate-300 hover:bg-white/10"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Search bar */}
        <div className="px-5 py-2.5 border-b border-white/5 flex items-center gap-2 bg-black/20">
          <Search className="w-3.5 h-3.5 text-slate-500" />
          <input
            type="text"
            placeholder="Search saved projects by name, style, or notes…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="flex-1 bg-transparent text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none"
          />
          <span className="text-[11px] text-slate-500">{filtered.length} saved</span>
        </div>

        {/* Grid */}
        <div className="flex-1 overflow-y-auto p-5">
          {filtered.length === 0 ? (
            <div className="py-16 text-center text-xs text-slate-400 space-y-2">
              <p className="text-sm text-slate-300 font-medium">
                {projects.length === 0 ? 'No saved projects yet' : 'No projects match your search'}
              </p>
              <p>
                {hasActiveImages
                  ? 'Click "Save Current As…" above to save your current workspace and source photos.'
                  : 'Load an image on the canvas, then save it as a project to restore anytime.'}
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
              {filtered.map((proj) => {
                const isActive = proj.id === activeProjectId;
                const isEditing = editingId === proj.id;
                const confirmingDel = confirmDeleteId === proj.id;
                return (
                  <div
                    key={proj.id}
                    className={`rounded-2xl border overflow-hidden flex flex-col transition ${
                      isActive
                        ? 'bg-amber-500/10 border-amber-500/50 shadow-lg shadow-amber-500/5'
                        : 'bg-black/30 border-white/10 hover:border-white/20'
                    }`}
                  >
                    {/* Thumbnail */}
                    <button
                      onClick={() => void onOpenProject(proj.id)}
                      className="h-36 bg-black/55 relative flex items-center justify-center overflow-hidden group"
                    >
                      {proj.thumbnail_b64 ? (
                        <img
                          src={`data:image/png;base64,${proj.thumbnail_b64}`}
                          alt={proj.name}
                          className="w-full h-full object-contain group-hover:scale-[1.03] transition"
                        />
                      ) : (
                        <span className="text-xs text-slate-500">No preview</span>
                      )}
                      <span className="absolute top-2 left-2 px-2 py-0.5 rounded-md bg-black/75 border border-white/10 text-[10px] font-medium uppercase tracking-wider text-amber-300">
                        {proj.style === 'multi_flat' ? 'Multi' : proj.style}
                      </span>
                      {isActive && (
                        <span className="absolute top-2 right-2 px-2 py-0.5 rounded-md bg-amber-500 text-slate-950 text-[10px] font-semibold">
                          Active
                        </span>
                      )}
                    </button>

                    {/* Body */}
                    <div className="p-3 flex-1 flex flex-col justify-between gap-2 text-xs">
                      {isEditing ? (
                        <div className="space-y-1.5">
                          <input
                            type="text"
                            value={editName}
                            onChange={(e) => setEditName(e.target.value)}
                            className="w-full h-7 px-2 rounded bg-black/45 border border-white/15 text-slate-100 text-xs"
                          />
                          <input
                            type="text"
                            value={editNotes}
                            placeholder="Notes"
                            onChange={(e) => setEditNotes(e.target.value)}
                            className="w-full h-7 px-2 rounded bg-black/45 border border-white/15 text-slate-200 text-[11px]"
                          />
                          <div className="flex gap-1.5">
                            <button
                              onClick={() => void handleRenameSubmit(proj.id)}
                              className="flex-1 h-7 rounded bg-amber-500 text-slate-950 font-semibold text-[11px]"
                            >
                              Save
                            </button>
                            <button
                              onClick={() => setEditingId(null)}
                              className="px-2.5 h-7 rounded bg-white/5 text-slate-300 text-[11px]"
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      ) : (
                        <div>
                          <div className="font-semibold text-slate-100 truncate">{proj.name}</div>
                          {proj.notes && (
                            <div className="text-[11px] text-slate-400 line-clamp-2 mt-0.5">
                              {proj.notes}
                            </div>
                          )}
                          <div className="text-[10px] text-slate-500 mt-1.5 flex items-center justify-between">
                            <span>
                              {proj.image_count} image{proj.image_count === 1 ? '' : 's'}
                            </span>
                            <span>{formatUpdated(proj.updated_at_ms)}</span>
                          </div>
                        </div>
                      )}

                      {!isEditing && (
                        <div className="flex items-center justify-between pt-2 border-t border-white/5">
                          <button
                            onClick={() => void onOpenProject(proj.id)}
                            className="px-2.5 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 text-amber-200 font-medium text-[11px] transition"
                          >
                            Open
                          </button>

                          <div className="flex items-center gap-1">
                            <button
                              onClick={() => {
                                setEditingId(proj.id);
                                setEditName(proj.name);
                                setEditNotes(proj.notes);
                              }}
                              className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10"
                              title="Rename / edit notes"
                            >
                              <Pencil className="w-3.5 h-3.5" />
                            </button>
                            <button
                              onClick={() =>
                                void onDuplicateProject(proj.id, `${proj.name} Copy`)
                              }
                              className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10"
                              title="Duplicate project"
                            >
                              <Copy className="w-3.5 h-3.5" />
                            </button>
                            {confirmingDel ? (
                              <button
                                onClick={() => {
                                  setConfirmDeleteId(null);
                                  void onDeleteProject(proj.id);
                                }}
                                className="px-2 py-1 rounded-lg bg-rose-500/25 text-rose-200 font-semibold text-[10px]"
                              >
                                Confirm
                              </button>
                            ) : (
                              <button
                                onClick={() => setConfirmDeleteId(proj.id)}
                                className="p-1.5 rounded-lg text-slate-400 hover:text-rose-400 hover:bg-white/10"
                                title="Delete project"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                              </button>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
