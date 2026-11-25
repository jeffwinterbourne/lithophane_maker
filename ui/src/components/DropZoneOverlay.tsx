import { ImagePlus, Sparkles } from 'lucide-react';
import type { StyleId } from '../types';

interface DropZoneOverlayProps {
  isDragging: boolean;
  hasImages: boolean;
  activeStyle: StyleId;
  onSelectImages: () => void;
  onSwitchStyle: (id: StyleId) => void;
}

export function DropZoneOverlay({
  isDragging,
  hasImages,
  activeStyle,
  onSelectImages,
  onSwitchStyle,
}: DropZoneOverlayProps) {
  if (!isDragging && hasImages) return null;

  return (
    <div
      className={`absolute inset-0 z-10 flex items-center justify-center transition ${
        isDragging ? 'bg-amber-500/10 backdrop-blur-sm z-40' : 'pointer-events-none'
      }`}
    >
      <div
        className={`pointer-events-auto max-w-md w-full mx-4 p-7 rounded-3xl border text-center shadow-2xl transition ${
          isDragging
            ? 'bg-[#131822]/95 border-amber-400 scale-[1.02]'
            : 'bg-[#131822]/85 backdrop-blur-xl border-white/10'
        }`}
      >
        <div className="mx-auto w-12 h-12 rounded-2xl bg-gradient-to-br from-amber-400/25 to-orange-500/20 border border-amber-400/30 flex items-center justify-center mb-4">
          <ImagePlus className="w-6 h-6 text-amber-400" />
        </div>
        <h2 className="text-lg font-semibold text-slate-100">
          {isDragging ? 'Drop images to load' : 'Drop a photo to craft a lithophane'}
        </h2>
        <p className="text-xs text-slate-400 mt-1.5 leading-relaxed">
          {activeStyle === 'multi_flat'
            ? 'Select 2 or more photos to build linked parallel panels with outer breakaway fins.'
            : 'Drag & drop PNG, JPEG, WebP, or TIFF anywhere — or drop 2+ images to launch Multi mode.'}
        </p>

        {!isDragging && (
          <div className="mt-5 flex flex-col gap-2.5">
            <button
              onClick={onSelectImages}
              className="w-full py-2.5 px-4 rounded-xl text-xs font-semibold bg-gradient-to-r from-amber-400 to-orange-500 text-slate-950 shadow-lg shadow-orange-500/20 hover:brightness-105 transition"
            >
              Choose Image{activeStyle === 'multi_flat' ? 's' : ''}…
            </button>
            {activeStyle !== 'curved' && (
              <button
                onClick={() => onSwitchStyle('curved')}
                className="w-full py-2 px-3 rounded-xl text-xs font-medium bg-white/5 hover:bg-white/10 border border-white/10 text-slate-300 flex items-center justify-center gap-1.5 transition"
              >
                <Sparkles className="w-3.5 h-3.5 text-amber-400" />
                Try Curved with Filleted Base
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
