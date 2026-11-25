import { AlertTriangle, Sun } from 'lucide-react';
import React, { useEffect, useImperativeHandle, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { BinaryScenePart, GenerationReport, LightingSettings, ShadingMode } from '../types';

export type CameraPreset = 'iso' | 'front' | 'back' | 'top';

export interface ViewportHandle {
  setCameraPreset: (preset: CameraPreset) => void;
}

interface Viewport3DProps {
  parts: BinaryScenePart[];
  report: GenerationReport | null;
  shading: ShadingMode;
  onShadingChange: (mode: ShadingMode) => void;
  lighting: LightingSettings;
  onLightingChange: (mutator: (lit: LightingSettings) => void) => void;
  showBedGrid: boolean;
  bedSizeXMm: number;
  bedSizeYMm: number;
  lowResActive: boolean;
  warnTriangleThreshold: number;
  isLoading: boolean;
  elapsedMs: number | null;
}

export const FILAMENT_PRESETS = [
  { label: 'White PLA', color: '#f6f6f2', td: 5.0 },
  { label: 'Matte White', color: '#f2f0ec', td: 2.8 },
  { label: 'Warm Ivory', color: '#f5ead6', td: 4.5 },
  { label: 'Cool White', color: '#eef3f8', td: 6.0 },
  { label: 'Natural PLA', color: '#ebe4d2', td: 9.5 },
] as const;

export const BACKLIGHT_PRESETS = [
  { label: 'Daylight 6500K', color: '#ffffff' },
  { label: 'Neutral 4500K', color: '#fff4e4' },
  { label: 'Warm LED 3000K', color: '#ffd9ad' },
  { label: 'Candle 2200K', color: '#ffbb76' },
  { label: 'Cool Sky 8000K', color: '#e6f0ff' },
] as const;

const BACKLIT_VERT = /* glsl */ `
  attribute float aThickness;
  varying float vThickness;
  varying vec3 vNormalWorld;
  varying vec3 vViewDir;

  void main() {
    vThickness = aThickness;
    vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
    vViewDir = normalize(-mvPos.xyz);
    vNormalWorld = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * mvPos;
  }
`;

const BACKLIT_FRAG = /* glsl */ `
  uniform vec3 uFilamentColor;
  uniform vec3 uBacklightColor;
  uniform float uTransmissionDist;
  uniform float uBacklightIntensity;
  uniform float uAmbientFront;
  varying float vThickness;
  varying vec3 vNormalWorld;
  varying vec3 vViewDir;

  void main() {
    float k = 4.605170186 / max(uTransmissionDist, 0.1);
    vec3 fil = clamp(uFilamentColor, vec3(0.02), vec3(1.0));
    vec3 transmit = exp(-k * vThickness) * pow(fil, vec3(vThickness / 1.5));
    vec3 transmitted = uBacklightIntensity * uBacklightColor * transmit;

    vec3 N = normalize(vNormalWorld);
    vec3 V = normalize(vViewDir);
    vec3 L = normalize(vec3(0.32, 0.52, 0.79));
    float diff = abs(dot(N, L));
    float rim = pow(1.0 - clamp(abs(dot(N, V)), 0.0, 1.0), 2.6);
    vec3 ambient = uAmbientFront * fil * (0.45 + 0.55 * diff) + uAmbientFront * 0.65 * fil * rim;

    gl_FragColor = vec4(transmitted + ambient, 1.0);
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
  }
`;

function buildBedGrid(widthX: number, depthY: number): THREE.Group {
  const bedGroup = new THREE.Group();
  const hx = widthX * 0.5;
  const hz = depthY * 0.5;
  const step = 20;

  const gridVerts: number[] = [];
  for (let x = -hx; x <= hx + 0.01; x += step) {
    gridVerts.push(x, -0.05, -hz, x, -0.05, hz);
  }
  for (let z = -hz; z <= hz + 0.01; z += step) {
    gridVerts.push(-hx, -0.05, z, hx, -0.05, z);
  }

  const gridGeo = new THREE.BufferGeometry();
  gridGeo.setAttribute('position', new THREE.Float32BufferAttribute(gridVerts, 3));
  const gridMat = new THREE.LineBasicMaterial({ color: 0x1c2432 });
  bedGroup.add(new THREE.LineSegments(gridGeo, gridMat));

  // Print bed outer perimeter
  const borderVerts = [
    -hx, -0.03, -hz, hx, -0.03, -hz,
    hx, -0.03, -hz, hx, -0.03, hz,
    hx, -0.03, hz, -hx, -0.03, hz,
    -hx, -0.03, hz, -hx, -0.03, -hz,
  ];
  const borderGeo = new THREE.BufferGeometry();
  borderGeo.setAttribute('position', new THREE.Float32BufferAttribute(borderVerts, 3));
  const borderMat = new THREE.LineBasicMaterial({ color: 0x334155 });
  bedGroup.add(new THREE.LineSegments(borderGeo, borderMat));

  // Center axes
  const axisVerts = [-hx, -0.02, 0, hx, -0.02, 0, 0, -0.02, -hz, 0, -0.02, hz];
  const axisGeo = new THREE.BufferGeometry();
  axisGeo.setAttribute('position', new THREE.Float32BufferAttribute(axisVerts, 3));
  const axisMat = new THREE.LineBasicMaterial({ color: 0x2b3648 });
  bedGroup.add(new THREE.LineSegments(axisGeo, axisMat));

  return bedGroup;
}

export const Viewport3D = React.forwardRef<ViewportHandle, Viewport3DProps>(function Viewport3D(
  {
    parts,
    report,
    shading,
    onShadingChange,
    lighting,
    onLightingChange,
    showBedGrid,
    bedSizeXMm,
    bedSizeYMm,
    lowResActive,
    warnTriangleThreshold,
    isLoading,
    elapsedMs,
  },
  ref,
) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const controlsRef = useRef<OrbitControls | null>(null);
  const groupRef = useRef<THREE.Group | null>(null);
  const bedRef = useRef<THREE.Group | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const hasFittedOnceRef = useRef(false);
  const [lightingOpen, setLightingOpen] = useState(false);

  const shaderUniformsRef = useRef({
    uFilamentColor: { value: new THREE.Color(lighting.filament_color) },
    uBacklightColor: { value: new THREE.Color(lighting.backlight_color) },
    uTransmissionDist: { value: lighting.transmission_distance_mm },
    uBacklightIntensity: { value: lighting.backlight_intensity },
    uAmbientFront: { value: lighting.ambient_front_light },
  });

  const fitCamera = (preset: CameraPreset) => {
    const group = groupRef.current;
    const camera = cameraRef.current;
    const controls = controlsRef.current;
    if (!group || !camera || !controls) return;

    const box = new THREE.Box3().setFromObject(group);
    if (box.isEmpty()) return;
    const center = new THREE.Vector3();
    const size = new THREE.Vector3();
    box.getCenter(center);
    box.getSize(size);
    const maxDim = Math.max(size.x, size.y, size.z, 20);
    const dist = maxDim * 1.45;

    const dir = new THREE.Vector3();
    switch (preset) {
      case 'front':
        dir.set(0, 0.1, 1);
        break;
      case 'back':
        dir.set(0, 0.1, -1);
        break;
      case 'top':
        dir.set(0, 1, 0.05);
        break;
      case 'iso':
      default:
        dir.set(0.52, 0.36, 0.88);
        break;
    }
    dir.normalize();
    controls.target.copy(center);
    camera.position.copy(center).addScaledVector(dir, dist);
    camera.near = Math.max(0.2, dist / 500);
    camera.far = dist * 20;
    camera.updateProjectionMatrix();
    controls.update();
  };

  useImperativeHandle(ref, () => ({
    setCameraPreset: (preset: CameraPreset) => fitCamera(preset),
  }));

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x0b0e14);

    const camera = new THREE.PerspectiveCamera(
      40,
      container.clientWidth / Math.max(1, container.clientHeight),
      0.5,
      5000,
    );
    camera.position.set(120, 110, 210);

    const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.0;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    container.appendChild(renderer.domElement);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 15;
    controls.maxDistance = 1800;

    // 3-point studio lighting for Clay / Base / Supports
    const hemi = new THREE.HemisphereLight(0xfff5e8, 0x1e293b, 0.75);
    scene.add(hemi);

    const key = new THREE.DirectionalLight(0xfff1dc, 1.45);
    key.position.set(140, 220, 240);
    scene.add(key);

    const fill = new THREE.DirectionalLight(0x93c5fd, 0.55);
    fill.position.set(-180, 110, 120);
    scene.add(fill);

    const rim = new THREE.DirectionalLight(0xfbbf24, 0.65);
    rim.position.set(0, 90, -240);
    scene.add(rim);

    const bedHolder = new THREE.Group();
    scene.add(bedHolder);

    const group = new THREE.Group();
    scene.add(group);

    sceneRef.current = scene;
    cameraRef.current = camera;
    controlsRef.current = controls;
    bedRef.current = bedHolder;
    groupRef.current = group;
    rendererRef.current = renderer;

    const ro = new ResizeObserver(() => {
      if (!container || !rendererRef.current || !cameraRef.current) return;
      const w = container.clientWidth;
      const h = Math.max(1, container.clientHeight);
      cameraRef.current.aspect = w / h;
      cameraRef.current.updateProjectionMatrix();
      rendererRef.current.setSize(w, h);
    });
    ro.observe(container);

    let rafId = 0;
    const animate = () => {
      rafId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };
    animate();

    return () => {
      cancelAnimationFrame(rafId);
      ro.disconnect();
      controls.dispose();
      renderer.dispose();
      if (renderer.domElement.parentNode === container) {
        container.removeChild(renderer.domElement);
      }
    };
  }, []);

  // Update dynamic bed grid whenever dimensions or visibility change
  useEffect(() => {
    const holder = bedRef.current;
    if (!holder) return;
    while (holder.children.length > 0) {
      const ch = holder.children[0] as THREE.LineSegments;
      holder.remove(ch);
      ch.geometry.dispose();
      (ch.material as THREE.Material).dispose();
    }
    if (showBedGrid) {
      const built = buildBedGrid(bedSizeXMm, bedSizeYMm);
      while (built.children.length > 0) {
        holder.add(built.children[0]);
      }
    }
  }, [showBedGrid, bedSizeXMm, bedSizeYMm]);

  // Instant 60fps shader uniform update when lighting changes (no geometry rebuild!)
  useEffect(() => {
    const u = shaderUniformsRef.current;
    u.uFilamentColor.value.set(lighting.filament_color);
    u.uBacklightColor.value.set(lighting.backlight_color);
    u.uTransmissionDist.value = lighting.transmission_distance_mm;
    u.uBacklightIntensity.value = lighting.backlight_intensity;
    u.uAmbientFront.value = lighting.ambient_front_light;

    const group = groupRef.current;
    if (group && shading !== 'wireframe') {
      const filCol = new THREE.Color(lighting.filament_color);
      for (const child of group.children) {
        const mesh = child as THREE.Mesh;
        if (mesh.material instanceof THREE.MeshStandardMaterial) {
          if (mesh.userData.role === 'panel') {
            mesh.material.color.copy(filCol);
          } else if (mesh.userData.role === 'base') {
            mesh.material.color.copy(filCol).multiplyScalar(0.86);
          }
        }
      }
    }
  }, [lighting, shading]);

  useEffect(() => {
    const group = groupRef.current;
    if (!group) return;

    while (group.children.length > 0) {
      const child = group.children[0] as THREE.Mesh;
      group.remove(child);
      child.geometry.dispose();
      if (Array.isArray(child.material)) {
        child.material.forEach((m) => m.dispose());
      } else {
        child.material.dispose();
      }
    }

    if (parts.length === 0) {
      hasFittedOnceRef.current = false;
      return;
    }

    const isWire = shading === 'wireframe';
    const filCol = new THREE.Color(lighting.filament_color);

    for (const part of parts) {
      const vCount = part.positions.length / 3;
      const threePos = new Float32Array(part.positions.length);
      const threeNorm = new Float32Array(part.normals.length);

      // Right-handed rotation Z-up -> Three.js Y-up: (x, y, z) -> (x, z, -y)
      for (let i = 0; i < vCount; i++) {
        const i3 = i * 3;
        threePos[i3] = part.positions[i3];
        threePos[i3 + 1] = part.positions[i3 + 2];
        threePos[i3 + 2] = -part.positions[i3 + 1];

        threeNorm[i3] = part.normals[i3];
        threeNorm[i3 + 1] = part.normals[i3 + 2];
        threeNorm[i3 + 2] = -part.normals[i3 + 1];
      }

      const geom = new THREE.BufferGeometry();
      geom.setAttribute('position', new THREE.BufferAttribute(threePos, 3));
      geom.setAttribute('normal', new THREE.BufferAttribute(threeNorm, 3));
      if (part.thickness) {
        geom.setAttribute('aThickness', new THREE.BufferAttribute(part.thickness, 1));
      }
      geom.setIndex(new THREE.BufferAttribute(part.indices, 1));

      let material: THREE.Material;
      if (isWire) {
        const col =
          part.role === 'panel'
            ? 0xfbbf24
            : part.role === 'base'
              ? 0xe2e8f0
              : 0xf97316;
        material = new THREE.MeshBasicMaterial({ color: col, wireframe: true });
      } else if (shading === 'backlit' && part.role === 'panel' && part.thickness) {
        material = new THREE.ShaderMaterial({
          uniforms: shaderUniformsRef.current,
          vertexShader: BACKLIT_VERT,
          fragmentShader: BACKLIT_FRAG,
        });
      } else if (part.role === 'panel') {
        material = new THREE.MeshStandardMaterial({
          color: filCol.clone(),
          roughness: 0.62,
          metalness: 0.02,
        });
      } else if (part.role === 'base') {
        material = new THREE.MeshStandardMaterial({
          color: filCol.clone().multiplyScalar(0.86),
          roughness: 0.55,
          metalness: 0.05,
        });
      } else {
        material = new THREE.MeshStandardMaterial({
          color: 0xd97706,
          roughness: 0.5,
          metalness: 0.05,
          transparent: shading === 'backlit',
          opacity: shading === 'backlit' ? 0.78 : 1.0,
        });
      }

      const mesh = new THREE.Mesh(geom, material);
      mesh.name = part.id;
      mesh.userData.role = part.role;
      group.add(mesh);
    }

    if (!hasFittedOnceRef.current) {
      hasFittedOnceRef.current = true;
      fitCamera('iso');
    }
    // Note: lighting colour updates are handled live in the uniform effect above
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [parts, shading]);

  const curvedInfo = report?.curved_info;
  const heavyMesh = (report?.triangles ?? 0) > warnTriangleThreshold;

  return (
    <div className="relative w-full h-full">
      <div ref={containerRef} className="w-full h-full cursor-grab active:cursor-grabbing" />

      {/* Top-right Shading, Lighting Popover & Camera Controls */}
      {parts.length > 0 && (
        <div className="absolute top-16 right-4 z-10 flex flex-col items-end gap-2">
          <div className="flex items-center gap-2">
            <div className="flex items-center rounded-xl bg-[#131822]/85 backdrop-blur-md border border-white/10 p-1 shadow-xl">
              {(['backlit', 'clay', 'wireframe'] as ShadingMode[]).map((mode) => (
                <button
                  key={mode}
                  onClick={() => onShadingChange(mode)}
                  className={`px-2.5 py-1 rounded-lg text-xs font-medium capitalize transition ${
                    shading === mode
                      ? 'bg-amber-500 text-slate-950 shadow-sm'
                      : 'text-slate-300 hover:text-white hover:bg-white/5'
                  }`}
                >
                  {mode}
                </button>
              ))}
            </div>

            <button
              onClick={() => setLightingOpen((o) => !o)}
              className={`h-8 px-2.5 rounded-xl flex items-center gap-1.5 text-xs font-medium backdrop-blur-md border shadow-xl transition ${
                lightingOpen
                  ? 'bg-amber-500/20 border-amber-500/50 text-amber-200'
                  : 'bg-[#131822]/85 border-white/10 text-slate-200 hover:bg-[#1a2230]'
              }`}
              title="Filament colour, Transmission Distance (TD), and backlight"
            >
              <Sun className="w-3.5 h-3.5 text-amber-400" />
              <span>Light</span>
              <span className="text-[10px] text-slate-400 hidden sm:inline">
                TD {lighting.transmission_distance_mm.toFixed(1)}
              </span>
            </button>

            <div className="flex items-center rounded-xl bg-[#131822]/85 backdrop-blur-md border border-white/10 p-1 shadow-xl">
              {(['front', 'back', 'top', 'iso'] as CameraPreset[]).map((preset) => (
                <button
                  key={preset}
                  onClick={() => fitCamera(preset)}
                  className="px-2 py-1 rounded-lg text-xs font-medium uppercase tracking-wide text-slate-300 hover:text-white hover:bg-white/5 transition"
                  title={`View from ${preset}`}
                >
                  {preset === 'iso' ? '3/4' : preset}
                </button>
              ))}
            </div>
          </div>

          {lightingOpen && (
            <div className="w-80 rounded-2xl bg-[#131822]/94 backdrop-blur-xl border border-white/10 p-3.5 shadow-2xl space-y-3 text-xs">
              {/* Filament section */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                    Filament & Transmission Distance
                  </span>
                  <span className="text-[11px] font-semibold text-amber-300">
                    TD {lighting.transmission_distance_mm.toFixed(1)} mm
                  </span>
                </div>
                <div className="flex flex-wrap gap-1">
                  {FILAMENT_PRESETS.map((fp) => {
                    const active =
                      lighting.filament_color.toLowerCase() === fp.color &&
                      Math.abs(lighting.transmission_distance_mm - fp.td) < 0.05;
                    return (
                      <button
                        key={fp.label}
                        onClick={() =>
                          onLightingChange((lit) => {
                            lit.filament_color = fp.color;
                            lit.transmission_distance_mm = fp.td;
                          })
                        }
                        className={`px-2 py-1 rounded-lg text-[10px] font-medium flex items-center gap-1.5 border transition ${
                          active
                            ? 'bg-amber-500/20 border-amber-500/50 text-amber-200'
                            : 'bg-black/30 border-white/10 text-slate-300 hover:bg-white/5'
                        }`}
                      >
                        <span
                          className="w-2.5 h-2.5 rounded-full border border-white/30"
                          style={{ backgroundColor: fp.color }}
                        />
                        {fp.label}
                      </button>
                    );
                  })}
                </div>

                <div className="flex items-center gap-2 pt-0.5">
                  <input
                    type="color"
                    value={lighting.filament_color}
                    onChange={(e) =>
                      onLightingChange((lit) => {
                        lit.filament_color = e.target.value;
                      })
                    }
                    className="w-8 h-7 rounded cursor-pointer bg-transparent border border-white/15"
                    title="Filament colour"
                  />
                  <input
                    type="range"
                    min={0.5}
                    max={20}
                    step={0.1}
                    value={lighting.transmission_distance_mm}
                    onChange={(e) =>
                      onLightingChange((lit) => {
                        lit.transmission_distance_mm = Number(e.target.value);
                      })
                    }
                    className="flex-1 studio-range"
                    title="Transmission distance (mm)"
                  />
                  <input
                    type="number"
                    min={0.2}
                    max={30}
                    step={0.1}
                    value={lighting.transmission_distance_mm}
                    onChange={(e) => {
                      const v = Number(e.target.value);
                      if (v >= 0.2 && v <= 30) {
                        onLightingChange((lit) => {
                          lit.transmission_distance_mm = v;
                        });
                      }
                    }}
                    className="w-14 h-7 px-1.5 rounded bg-black/40 border border-white/10 text-[11px] text-slate-100"
                  />
                </div>
              </div>

              {/* Backlight section */}
              <div className="space-y-1.5 pt-2 border-t border-white/10">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                    Backlight
                  </span>
                  <span className="text-[11px] text-slate-300">
                    {lighting.backlight_intensity.toFixed(2)}×
                  </span>
                </div>
                <div className="flex flex-wrap gap-1">
                  {BACKLIGHT_PRESETS.map((bp) => {
                    const active = lighting.backlight_color.toLowerCase() === bp.color;
                    return (
                      <button
                        key={bp.label}
                        onClick={() =>
                          onLightingChange((lit) => {
                            lit.backlight_color = bp.color;
                          })
                        }
                        className={`px-2 py-1 rounded-lg text-[10px] font-medium flex items-center gap-1.5 border transition ${
                          active
                            ? 'bg-amber-500/20 border-amber-500/50 text-amber-200'
                            : 'bg-black/30 border-white/10 text-slate-300 hover:bg-white/5'
                        }`}
                      >
                        <span
                          className="w-2.5 h-2.5 rounded-full border border-white/30"
                          style={{ backgroundColor: bp.color }}
                        />
                        {bp.label}
                      </button>
                    );
                  })}
                </div>

                <div className="flex items-center gap-2 pt-0.5">
                  <input
                    type="color"
                    value={lighting.backlight_color}
                    onChange={(e) =>
                      onLightingChange((lit) => {
                        lit.backlight_color = e.target.value;
                      })
                    }
                    className="w-8 h-7 rounded cursor-pointer bg-transparent border border-white/15"
                    title="Backlight colour"
                  />
                  <input
                    type="range"
                    min={0.2}
                    max={4.0}
                    step={0.05}
                    value={lighting.backlight_intensity}
                    onChange={(e) =>
                      onLightingChange((lit) => {
                        lit.backlight_intensity = Number(e.target.value);
                      })
                    }
                    className="flex-1 studio-range"
                    title="Backlight brightness"
                  />
                </div>

                <div className="flex items-center justify-between gap-2 pt-1">
                  <span className="text-[11px] text-slate-400 shrink-0">Front ambient</span>
                  <input
                    type="range"
                    min={0}
                    max={0.25}
                    step={0.005}
                    value={lighting.ambient_front_light}
                    onChange={(e) =>
                      onLightingChange((lit) => {
                        lit.ambient_front_light = Number(e.target.value);
                      })
                    }
                    className="flex-1 studio-range"
                  />
                  <span className="text-[10px] text-slate-400 w-9 text-right">
                    {(lighting.ambient_front_light * 100).toFixed(0)}%
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Subtle computing pill */}
      {isLoading && (
        <div className="absolute top-16 left-1/2 -translate-x-1/2 z-20 flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-[#131822]/90 backdrop-blur-md border border-amber-500/30 text-xs text-amber-200 shadow-lg">
          <span className="h-2 w-2 rounded-full bg-amber-400 animate-ping" />
          Updating mesh…
        </div>
      )}

      {/* Bottom-right telemetry pill */}
      {report && (
        <div className="absolute bottom-4 right-4 z-10 flex flex-wrap items-center gap-x-2.5 gap-y-1 px-3.5 py-2 rounded-xl bg-[#131822]/85 backdrop-blur-md border border-white/10 text-xs text-slate-300 shadow-xl">
          {lowResActive && (
            <span className="px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 text-[10px] font-semibold">
              Low-Res Preview
            </span>
          )}
          <span className="font-semibold text-slate-100">
            {report.overall_dimensions_mm.x.toFixed(1)} × {report.overall_dimensions_mm.y.toFixed(1)} ×{' '}
            {report.overall_dimensions_mm.z.toFixed(1)} mm
          </span>
          {curvedInfo && (
            <>
              <span className="text-slate-600">•</span>
              <span className="text-amber-300">
                R{curvedInfo.radius_mm.toFixed(0)} · {curvedInfo.arc_angle_deg.toFixed(0)}° arc
              </span>
            </>
          )}
          <span className="text-slate-600">•</span>
          <span className={`flex items-center gap-1 ${heavyMesh ? 'text-amber-300' : ''}`}>
            {heavyMesh && <AlertTriangle className="w-3 h-3 text-amber-400" />}
            {report.triangles.toLocaleString()} tris
          </span>
          <span className="text-slate-600">•</span>
          <span>~{report.filament_g_estimate.toFixed(1)} g PLA</span>
          {elapsedMs !== null && (
            <>
              <span className="text-slate-600">•</span>
              <span className="text-emerald-400">{elapsedMs} ms</span>
            </>
          )}
        </div>
      )}
    </div>
  );
});
