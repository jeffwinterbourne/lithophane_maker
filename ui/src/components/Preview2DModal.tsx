import { AlertTriangle, X } from 'lucide-react';
import { useState } from 'react';
import type { GenerationReport, ImagePreviewB64 } from '../types';

interface Preview2DModalProps {
  open: boolean;
  onClose: () => void;
  previews: ImagePreviewB64[];
  report: GenerationReport | null;
}

export function Preview2DModal({ open, onClose, previews, report }: Preview2DModalProps) {
  const [idx, setIdx] = useState(0);
  if (!open) return null;

  const active = previews[Math.min(idx, Math.max(0, previews.length - 1))];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-md p-4">
      <div className="w-full max-w-4xl max-h-[90vh] flex flex-col rounded-3xl bg-[#131822] border border-white/10 shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-5 py-3.5 border-b border-white/10">
          <div>
            <h2 className="text-sm font-semibold text-slate-100">2D Maps & Print Report</h2>
            <p className="text-[11px] text-slate-400">
              Backlit optical simulation, calibrated thickness heightmap, and geometry telemetry
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {previews.length > 1 && (
            <div className="flex flex-wrap gap-1.5">
              {previews.map((p, i) => (
                <button
                  key={`${p.stem}-${i}`}
                  onClick={() => setIdx(i)}
                  className={`px-3 py-1 rounded-lg text-xs font-medium transition ${
                    i === idx
                      ? 'bg-amber-500 text-slate-950'
                      : 'bg-white/5 text-slate-300 hover:bg-white/10'
                  }`}
                >
                  {i + 1}. {p.stem}
                </button>
              ))}
            </div>
          )}

          {active && (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <PreviewCard
                title="Simulated Backlit"
                subtitle="Filament colour, TD & backlight"
                b64={active.backlit_b64}
              />
              <PreviewCard
                title="Thickness Map"
                subtitle="Yellow = thin · Purple = thick"
                b64={active.heightmap_b64}
              />
              <PreviewCard
                title="Processed Greyscale"
                subtitle="After crop & tonal pipeline"
                b64={active.luminance_b64}
              />
            </div>
          )}

          {report && (
            <div className="space-y-3">
              {report.warnings.length > 0 && (
                <div className="rounded-2xl bg-amber-500/10 border border-amber-500/30 p-3 space-y-1">
                  {report.warnings.map((w, i) => (
                    <div key={i} className="flex items-start gap-2 text-xs text-amber-200">
                      <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
                      <span>{w}</span>
                    </div>
                  ))}
                </div>
              )}

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                <StatCard
                  label="Overall Size"
                  value={`${report.overall_dimensions_mm.x} × ${report.overall_dimensions_mm.y} × ${report.overall_dimensions_mm.z} mm`}
                />
                <StatCard
                  label="Thickness Range"
                  value={`${report.thickness_mm.min} – ${report.thickness_mm.max} mm`}
                />
                <StatCard
                  label="Triangles"
                  value={report.triangles.toLocaleString()}
                />
                <StatCard
                  label="Volume / PLA"
                  value={`${(report.volume_mm3 / 1000).toFixed(1)} cm³ (~${report.filament_g_estimate} g)`}
                />
              </div>

              {report.curved_info && (
                <div className="rounded-2xl bg-black/30 border border-white/5 p-3.5 text-xs grid grid-cols-2 sm:grid-cols-4 gap-3">
                  <div>
                    <div className="text-slate-400">Radius / Direction</div>
                    <div className="font-semibold text-slate-100 capitalize">
                      {report.curved_info.radius_mm} mm ({report.curved_info.direction})
                    </div>
                  </div>
                  <div>
                    <div className="text-slate-400">Arc Angle</div>
                    <div className="font-semibold text-amber-300">
                      {report.curved_info.arc_angle_deg}°
                    </div>
                  </div>
                  <div>
                    <div className="text-slate-400">Chord × Sagitta</div>
                    <div className="font-semibold text-slate-100">
                      {report.curved_info.chord_width_mm} × {report.curved_info.arc_depth_mm} mm
                    </div>
                  </div>
                  <div>
                    <div className="text-slate-400">Built-in Base</div>
                    <div className="font-semibold text-slate-100">
                      {report.curved_info.base_enabled
                        ? `${report.curved_info.base_thickness_mm} mm (fillet ${report.curved_info.base_effective_fillet_in_mm}/${report.curved_info.base_effective_fillet_out_mm} mm)`
                        : 'Off'}
                    </div>
                  </div>
                </div>
              )}

              {report.panels.length > 0 && (
                <div className="rounded-2xl bg-black/30 border border-white/5 overflow-hidden">
                  <table className="w-full text-left text-xs">
                    <thead className="bg-white/5 text-slate-400">
                      <tr>
                        <th className="px-3 py-2">#</th>
                        <th className="px-3 py-2">Image</th>
                        <th className="px-3 py-2">Mapping</th>
                        <th className="px-3 py-2">Size (X × Z)</th>
                        <th className="px-3 py-2">Y Offset</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-white/5 text-slate-200">
                      {report.panels.map((p) => (
                        <tr key={p.index}>
                          <td className="px-3 py-1.5">{p.index + 1}</td>
                          <td className="px-3 py-1.5 font-medium">{p.source_stem}</td>
                          <td className="px-3 py-1.5 text-amber-300">{p.mode}</td>
                          <td className="px-3 py-1.5">
                            {p.dimensions_mm.x} × {p.dimensions_mm.z} mm
                          </td>
                          <td className="px-3 py-1.5">{p.y_ref_mm} mm</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function PreviewCard({ title, subtitle, b64 }: { title: string; subtitle: string; b64: string }) {
  return (
    <div className="rounded-2xl bg-black/35 border border-white/10 p-3 space-y-2">
      <div className="aspect-auto max-h-64 flex items-center justify-center overflow-hidden rounded-xl bg-black/50">
        <img
          src={`data:image/png;base64,${b64}`}
          alt={title}
          className="max-h-60 object-contain"
        />
      </div>
      <div className="text-center">
        <div className="text-xs font-medium text-slate-100">{title}</div>
        <div className="text-[10px] text-slate-400">{subtitle}</div>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-black/30 border border-white/5 p-3">
      <div className="text-[10px] uppercase tracking-wider text-slate-400">{label}</div>
      <div className="text-xs font-semibold text-slate-100 mt-1">{value}</div>
    </div>
  );
}
