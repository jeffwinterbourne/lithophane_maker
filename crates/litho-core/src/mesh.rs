//! Triangle mesh data structure, `manifold-rust` CSG bridge, validation, and STL/3MF export.

use std::fmt::Write as FmtWrite;
use std::io::{Seek, Write};

use manifold_rust::linalg::Vec3;
use manifold_rust::manifold::Manifold;
use manifold_rust::types::{Error as ManifoldError, MeshGL64, OpType};
use rayon::prelude::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::LithoError;
use crate::scene::{PartRole, ScenePart};

/// Indexed triangle mesh in millimetres (Z-up: X right along bed, Y depth from viewer `-Y`, Z up).
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f64; 3]>,
    pub indices: Vec<[u32; 3]>,
    /// Optional per-vertex optical thickness (mm) used by the backlit 3D preview shader.
    pub thickness: Option<Vec<f32>>,
}

impl MeshData {
    /// Axis-aligned bounding box `(min, max)` in millimetres.
    pub fn bounds(&self) -> ([f64; 3], [f64; 3]) {
        if self.positions.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                if p[k] < min[k] {
                    min[k] = p[k];
                }
                if p[k] > max[k] {
                    max[k] = p[k];
                }
            }
        }
        (min, max)
    }

    /// Translate all vertices along `+Y` in place.
    pub fn translate_y(&mut self, dy: f64) {
        for p in &mut self.positions {
            p[1] += dy;
        }
    }

    /// Translate all vertices along `(dx, dy, dz)` in place.
    pub fn translate(&mut self, dx: f64, dy: f64, dz: f64) {
        for p in &mut self.positions {
            p[0] += dx;
            p[1] += dy;
            p[2] += dz;
        }
    }

    /// Signed enclosed volume (`> 0` for outward-oriented closed manifold).
    pub fn signed_volume(&self) -> f64 {
        let sum: f64 = self
            .indices
            .par_iter()
            .map(|&[i0, i1, i2]| {
                let a = self.positions[i0 as usize];
                let b = self.positions[i1 as usize];
                let c = self.positions[i2 as usize];
                a[0] * (b[1] * c[2] - b[2] * c[1])
                    - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0])
            })
            .sum();
        sum / 6.0
    }

    /// Concatenate multiple meshes without boolean union (used for `separate_bodies` or preview).
    pub fn concatenate(meshes: &[&MeshData]) -> Self {
        let total_v: usize = meshes.iter().map(|m| m.positions.len()).sum();
        let total_i: usize = meshes.iter().map(|m| m.indices.len()).sum();
        let all_have_thick =
            !meshes.is_empty() && meshes.iter().all(|m| m.thickness.as_ref().is_some_and(|t| t.len() == m.positions.len()));

        let mut positions = Vec::with_capacity(total_v);
        let mut indices = Vec::with_capacity(total_i);
        let mut thickness = if all_have_thick {
            Some(Vec::with_capacity(total_v))
        } else {
            None
        };

        for m in meshes {
            let base = positions.len() as u32;
            positions.extend_from_slice(&m.positions);
            if let (Some(dst), Some(src)) = (&mut thickness, &m.thickness) {
                dst.extend_from_slice(src);
            }
            for &[a, b, c] in &m.indices {
                indices.push([base + a, base + b, base + c]);
            }
        }

        Self {
            positions,
            indices,
            thickness,
        }
    }

    /// Compute area-weighted smooth per-vertex normals.
    pub fn compute_vertex_normals(&self) -> Vec<[f32; 3]> {
        let mut accum = vec![[0.0f64; 3]; self.positions.len()];
        for &[i0, i1, i2] in &self.indices {
            let p0 = self.positions[i0 as usize];
            let p1 = self.positions[i1 as usize];
            let p2 = self.positions[i2 as usize];
            let ux = p1[0] - p0[0];
            let uy = p1[1] - p0[1];
            let uz = p1[2] - p0[2];
            let vx = p2[0] - p0[0];
            let vy = p2[1] - p0[1];
            let vz = p2[2] - p0[2];
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            for &idx in &[i0, i1, i2] {
                let slot = &mut accum[idx as usize];
                slot[0] += nx;
                slot[1] += ny;
                slot[2] += nz;
            }
        }

        accum
            .into_iter()
            .map(|n| {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if len > 1e-12 {
                    [(n[0] / len) as f32, (n[1] / len) as f32, (n[2] / len) as f32]
                } else {
                    [0.0, -1.0, 0.0]
                }
            })
            .collect()
    }
}

/// Convert a [`MeshData`] to a verified positive-volume [`Manifold`].
pub fn mesh_to_manifold(mesh: &MeshData, label: &str) -> Result<Manifold, LithoError> {
    if mesh.positions.is_empty() || mesh.indices.is_empty() {
        return Err(LithoError::NonManifold(format!(
            "{label} mesh is empty (vertices={}, triangles={})",
            mesh.positions.len(),
            mesh.indices.len()
        )));
    }

    let mut vert_properties = Vec::with_capacity(mesh.positions.len() * 3);
    for p in &mesh.positions {
        vert_properties.push(p[0]);
        vert_properties.push(p[1]);
        vert_properties.push(p[2]);
    }
    let mut tri_verts = Vec::with_capacity(mesh.indices.len() * 3);
    for &[a, b, c] in &mesh.indices {
        tri_verts.push(a as u64);
        tri_verts.push(b as u64);
        tri_verts.push(c as u64);
    }

    let gl = MeshGL64 {
        num_prop: 3,
        vert_properties,
        tri_verts,
        ..Default::default()
    };

    let m = Manifold::from_mesh_gl64(&gl);
    validate_manifold(&m, label)?;
    Ok(m)
}

/// Verify a [`Manifold`] has no geometric error and positive enclosed volume.
pub fn validate_manifold(m: &Manifold, label: &str) -> Result<(), LithoError> {
    let status = m.status();
    if status != ManifoldError::NoError {
        return Err(LithoError::NonManifold(format!(
            "{label} manifold check failed: {}",
            status.to_str()
        )));
    }
    let vol = m.volume();
    if !vol.is_finite() || vol <= 0.0 {
        return Err(LithoError::NonManifold(format!(
            "{label} has non-positive volume ({vol:.6} mm^3)"
        )));
    }
    Ok(())
}

/// Extract a [`MeshData`] back from a [`Manifold`] using exact `f64` vertex coordinates.
pub fn manifold_to_mesh(m: &Manifold) -> MeshData {
    let gl = m.get_mesh_glp::<f64, u32>(-1);
    let stride = (gl.num_prop as usize).max(3);
    let num_v = gl.vert_properties.len() / stride;
    let mut positions = Vec::with_capacity(num_v);
    for i in 0..num_v {
        let off = i * stride;
        positions.push([
            gl.vert_properties[off],
            gl.vert_properties[off + 1],
            gl.vert_properties[off + 2],
        ]);
    }
    let num_t = gl.tri_verts.len() / 3;
    let mut indices = Vec::with_capacity(num_t);
    for i in 0..num_t {
        let off = i * 3;
        indices.push([
            gl.tri_verts[off],
            gl.tri_verts[off + 1],
            gl.tri_verts[off + 2],
        ]);
    }
    MeshData {
        positions,
        indices,
        thickness: None,
    }
}

/// Build a convex-hull [`Manifold`] from 3D points and verify it has positive volume.
pub fn hull_manifold(points: &[[f64; 3]], label: &str) -> Result<Manifold, LithoError> {
    let vec3s: Vec<Vec3> = points.iter().map(|&[x, y, z]| Vec3::new(x, y, z)).collect();
    let m = Manifold::hull(&vec3s);
    validate_manifold(&m, label)?;
    Ok(m)
}

/// Perform a manifold boolean union across one or more solids and verify the result.
pub fn union_manifolds(parts: &[Manifold], label: &str) -> Result<Manifold, LithoError> {
    if parts.is_empty() {
        return Err(LithoError::Geometry(format!(
            "{label}: cannot union empty list of solids"
        )));
    }
    if parts.len() == 1 {
        validate_manifold(&parts[0], label)?;
        return Ok(parts[0].clone());
    }
    let merged = Manifold::batch_boolean(parts, OpType::Add);
    validate_manifold(&merged, label)?;
    Ok(merged)
}

/// Compose disjoint solids into one multi-body [`Manifold`] without boolean overhead.
pub fn compose_manifolds(parts: &[Manifold]) -> Manifold {
    Manifold::compose(parts)
}

#[inline]
fn tri_normal_f32(p0: [f64; 3], p1: [f64; 3], p2: [f64; 3]) -> [f32; 3] {
    let ux = p1[0] - p0[0];
    let uy = p1[1] - p0[1];
    let uz = p1[2] - p0[2];
    let vx = p2[0] - p0[0];
    let vy = p2[1] - p0[1];
    let vz = p2[2] - p0[2];
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-15 {
        [(nx / len) as f32, (ny / len) as f32, (nz / len) as f32]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Write a mesh as a standard little-endian binary STL.
pub fn write_stl_binary<W: Write>(mesh: &MeshData, mut writer: W) -> Result<(), LithoError> {
    let mut header = [0u8; 80];
    let banner = b"Lithophane Studio binary STL";
    header[..banner.len()].copy_from_slice(banner);
    writer.write_all(&header)?;

    let tri_count: u32 = mesh
        .indices
        .len()
        .try_into()
        .map_err(|_| LithoError::Export("triangle count exceeds u32 binary STL limit".into()))?;
    writer.write_all(&tri_count.to_le_bytes())?;

    let records: Vec<[u8; 50]> = mesh
        .indices
        .par_iter()
        .map(|&[i0, i1, i2]| {
            let p0 = mesh.positions[i0 as usize];
            let p1 = mesh.positions[i1 as usize];
            let p2 = mesh.positions[i2 as usize];
            let n = tri_normal_f32(p0, p1, p2);
            let mut rec = [0u8; 50];
            let mut off = 0usize;
            for f in [
                n[0],
                n[1],
                n[2],
                p0[0] as f32,
                p0[1] as f32,
                p0[2] as f32,
                p1[0] as f32,
                p1[1] as f32,
                p1[2] as f32,
                p2[0] as f32,
                p2[1] as f32,
                p2[2] as f32,
            ] {
                rec[off..off + 4].copy_from_slice(&f.to_le_bytes());
                off += 4;
            }
            rec
        })
        .collect();

    for rec in &records {
        writer.write_all(rec)?;
    }
    writer.flush()?;
    Ok(())
}

/// Write a mesh as an ASCII STL.
pub fn write_stl_ascii<W: Write>(mesh: &MeshData, mut writer: W) -> Result<(), LithoError> {
    writeln!(writer, "solid lithophane")?;
    for &[i0, i1, i2] in &mesh.indices {
        let p0 = mesh.positions[i0 as usize];
        let p1 = mesh.positions[i1 as usize];
        let p2 = mesh.positions[i2 as usize];
        let n = tri_normal_f32(p0, p1, p2);
        writeln!(writer, "  facet normal {:.6} {:.6} {:.6}", n[0], n[1], n[2])?;
        writeln!(writer, "    outer loop")?;
        writeln!(writer, "      vertex {:.6} {:.6} {:.6}", p0[0], p0[1], p0[2])?;
        writeln!(writer, "      vertex {:.6} {:.6} {:.6}", p1[0], p1[1], p1[2])?;
        writeln!(writer, "      vertex {:.6} {:.6} {:.6}", p2[0], p2[1], p2[2])?;
        writeln!(writer, "    endloop")?;
        writeln!(writer, "  endfacet")?;
    }
    writeln!(writer, "endsolid lithophane")?;
    writer.flush()?;
    Ok(())
}

/// Write one or more [`ScenePart`] objects into a valid 3MF archive (OPC ZIP, units = millimetre).
///
/// Multi-part scenes are grouped under a single assembly `<object><components>` so slicers
/// preserve exact relative Z heights (for example, a curved panel resting atop a 1.6 mm base)
/// while still allowing per-part filament assignment.
pub fn write_3mf<W: Write + Seek>(parts: &[ScenePart], writer: W) -> Result<(), LithoError> {
    if parts.is_empty() {
        return Err(LithoError::Export("cannot write empty 3MF scene".into()));
    }

    let mut zip = ZipWriter::new(writer);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", opts)
        .map_err(|e| LithoError::Export(format!("zip error: {e}")))?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml" />
</Types>
"#,
    )?;

    zip.start_file("_rels/.rels", opts)
        .map_err(|e| LithoError::Export(format!("zip error: {e}")))?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" />
</Relationships>
"#,
    )?;

    let mut xml = String::with_capacity(64 * 1024);
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <metadata name="Application">Lithophane Studio</metadata>
  <resources>
    <basematerials id="1">
"#,
    );

    for part in parts {
        let [r, g, b, a] = part.material.rgba;
        let _ = writeln!(
            xml,
            "      <base name=\"{}\" displaycolor=\"#{:02X}{:02X}{:02X}{:02X}\" />",
            xml_escape(&part.material.name),
            r,
            g,
            b,
            a
        );
    }
    xml.push_str("    </basematerials>\n");

    for (idx, part) in parts.iter().enumerate() {
        let obj_id = idx + 2;
        let _ = writeln!(
            xml,
            "    <object id=\"{obj_id}\" type=\"model\" name=\"{}\" pid=\"1\" pindex=\"{idx}\">",
            xml_escape(&part.name)
        );
        xml.push_str("      <mesh>\n        <vertices>\n");
        for p in &part.mesh.positions {
            let _ = writeln!(
                xml,
                "          <vertex x=\"{:.5}\" y=\"{:.5}\" z=\"{:.5}\" />",
                p[0], p[1], p[2]
            );
        }
        xml.push_str("        </vertices>\n        <triangles>\n");
        for &[v1, v2, v3] in &part.mesh.indices {
            let _ = writeln!(
                xml,
                "          <triangle v1=\"{v1}\" v2=\"{v2}\" v3=\"{v3}\" />"
            );
        }
        xml.push_str("        </triangles>\n      </mesh>\n    </object>\n");
    }

    let build_id = if parts.len() > 1 {
        let asm_id = parts.len() + 2;
        let _ = writeln!(xml, "    <object id=\"{asm_id}\" type=\"model\" name=\"Lithophane Assembly\">\n      <components>");
        for idx in 0..parts.len() {
            let _ = writeln!(xml, "        <component objectid=\"{}\" />", idx + 2);
        }
        xml.push_str("      </components>\n    </object>\n");
        asm_id
    } else {
        2
    };

    xml.push_str("  </resources>\n  <build>\n");
    let _ = writeln!(xml, "    <item objectid=\"{build_id}\" />");
    xml.push_str("  </build>\n</model>\n");

    zip.start_file("3D/3dmodel.model", opts)
        .map_err(|e| LithoError::Export(format!("zip error: {e}")))?;
    zip.write_all(xml.as_bytes())?;
    zip.finish()
        .map_err(|e| LithoError::Export(format!("zip finish error: {e}")))?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Pack scene parts into a compact binary buffer for zero-JSON Three.js IPC transfer.
///
/// Layout (little-endian):
/// - `u32`: `part_count`
/// - Repeated `part_count` times:
///   - `u32`: `id_byte_len`, followed by UTF-8 `id` bytes
///   - `u8`: `role` (`0=Panel, 1=Base, 2=Support, 3=Bridge, 4=ColorLayer, 5=Structural`)
///   - `[u8; 4]`: `material.rgba`
///   - `u32`: `vertex_count`
///   - `u32`: `index_count` (`triangles * 3`)
///   - `u8`: `has_thickness` (`1` or `0`)
///   - `[f32; vertex_count * 3]`: positions `(x, y, z)` in mm
///   - `[f32; vertex_count * 3]`: vertex normals
///   - (if `has_thickness == 1`) `[f32; vertex_count]`: per-vertex thickness in mm
///   - `[u32; index_count]`: triangle indices
pub fn pack_preview_binary(parts: &[ScenePart]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1024 * 64);
    buf.extend_from_slice(&(parts.len() as u32).to_le_bytes());

    for part in parts {
        let id_bytes = part.id.as_bytes();
        buf.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(id_bytes);

        let role_byte: u8 = match part.role {
            PartRole::Panel => 0,
            PartRole::Base => 1,
            PartRole::Support => 2,
            PartRole::Bridge => 3,
            PartRole::ColorLayer => 4,
            PartRole::Structural => 5,
        };
        buf.push(role_byte);
        buf.extend_from_slice(&part.material.rgba);

        let v_count = part.mesh.positions.len();
        let i_count = part.mesh.indices.len() * 3;
        let thick_slice = part
            .mesh
            .thickness
            .as_deref()
            .filter(|t| t.len() == v_count);
        let has_thick = thick_slice.is_some();

        buf.extend_from_slice(&(v_count as u32).to_le_bytes());
        buf.extend_from_slice(&(i_count as u32).to_le_bytes());
        buf.push(if has_thick { 1 } else { 0 });

        for p in &part.mesh.positions {
            buf.extend_from_slice(&(p[0] as f32).to_le_bytes());
            buf.extend_from_slice(&(p[1] as f32).to_le_bytes());
            buf.extend_from_slice(&(p[2] as f32).to_le_bytes());
        }

        let normals = part.mesh.compute_vertex_normals();
        for n in &normals {
            buf.extend_from_slice(&n[0].to_le_bytes());
            buf.extend_from_slice(&n[1].to_le_bytes());
            buf.extend_from_slice(&n[2].to_le_bytes());
        }

        if let Some(thick) = thick_slice {
            for &t in thick {
                buf.extend_from_slice(&t.to_le_bytes());
            }
        }

        for &[a, b, c] in &part.mesh.indices {
            buf.extend_from_slice(&a.to_le_bytes());
            buf.extend_from_slice(&b.to_le_bytes());
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }

    buf
}
