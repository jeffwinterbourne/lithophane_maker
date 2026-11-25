import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react';
import type { LoadedImage, MatchDimensionMode } from '../types';

interface MultiPhotoStripProps {
  images: LoadedImage[];
  commonDimMm: number;
  onAddPhotos: () => void;
  onRemovePhoto: (index: number) => void;
  onMovePhoto: (index: number, delta: -1 | 1) => void;
  onMappingChange: (index: number, mode: MatchDimensionMode) => void;
}

function estimateDims(aspect: number, commonDim: number, mode: MatchDimensionMode): [number, number] {
  switch (mode) {
    case 'long_side':
      return aspect >= 1 ? [commonDim, commonDim / aspect] : [commonDim, commonDim * aspect];
    case 'short_side':
      return aspect >= 1 ? [commonDim, commonDim * aspect] : [commonDim, commonDim / aspect];
    case 'width':
      return [commonDim, commonDim / aspect];
    case 'height':
      return [commonDim, commonDim * aspect];
  }
}

export function MultiPhotoStrip({
  images,
  commonDimMm,
  onAddPhotos,
  onRemovePhoto,
  onMovePhoto,
  onMappingChange,
}: MultiPhotoStripProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[11px] font-medium uppercase tracking-wider text-slate-400">
          Photos ({images.length})
        </span>
        <button
          onClick={onAddPhotos}
          className="flex items-center gap-1 text-[11px] font-medium text-amber-400 hover:text-amber-300 transition"
        >
          <Plus className="w-3.5 h-3.5" />
          Add photos
        </button>
      </div>

      <div className="max-h-40 overflow-y-auto space-y-1.5 pr-1">
        {images.map((img, i) => {
          const [xMm, zMm] = estimateDims(img.aspect, commonDimMm, img.mapping_mode);
          const role =
            i === 0 ? 'Front' : i === images.length - 1 ? 'Back' : `Mid ${i}`;
          return (
            <div
              key={`${img.path}-${i}`}
              className="flex items-center gap-2 p-1.5 rounded-lg bg-black/35 border border-white/5"
            >
              <img
                src={`data:image/png;base64,${img.thumbnail_b64}`}
                alt={img.stem}
                className="w-8 h-8 rounded object-cover shrink-0 border border-white/10"
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-1">
                  <span className="text-xs font-medium text-slate-200 truncate">
                    {img.stem}
                  </span>
                  <span className="text-[10px] px-1.5 py-0.2 rounded bg-white/10 text-slate-300 shrink-0">
                    {role}
                  </span>
                </div>
                <div className="flex items-center justify-between gap-1.5 mt-0.5">
                  <select
                    value={img.mapping_mode}
                    onChange={(e) => onMappingChange(i, e.target.value as MatchDimensionMode)}
                    className="bg-transparent text-[11px] text-amber-300 focus:outline-none"
                  >
                    <option value="long_side" className="bg-[#131822]">
                      Long side
                    </option>
                    <option value="short_side" className="bg-[#131822]">
                      Short side
                    </option>
                    <option value="width" className="bg-[#131822]">
                      Width
                    </option>
                    <option value="height" className="bg-[#131822]">
                      Height
                    </option>
                  </select>
                  <span className="text-[10px] text-slate-400">
                    {xMm.toFixed(0)} × {zMm.toFixed(0)} mm
                  </span>
                </div>
              </div>
              <div className="flex flex-col gap-0.5">
                <button
                  onClick={() => onMovePhoto(i, -1)}
                  disabled={i === 0}
                  className="text-slate-400 hover:text-white disabled:opacity-25"
                  title="Move forward"
                >
                  <ArrowUp className="w-3 h-3" />
                </button>
                <button
                  onClick={() => onMovePhoto(i, 1)}
                  disabled={i === images.length - 1}
                  className="text-slate-400 hover:text-white disabled:opacity-25"
                  title="Move back"
                >
                  <ArrowDown className="w-3 h-3" />
                </button>
              </div>
              <button
                onClick={() => onRemovePhoto(i)}
                className="text-slate-500 hover:text-rose-400 transition p-0.5"
                title="Remove photo"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
