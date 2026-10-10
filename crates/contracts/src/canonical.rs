//! Deterministic V1 canonical bytes. See ADR-004; no JSON numeric formatting in hashes.
use crate::{ElevationGridV1, SelectionV1, TerrainSourceManifestV1};
use sha2::{Digest, Sha256};

struct Writer(Vec<u8>);
impl Writer {
    fn new(prefix: &[u8]) -> Self {
        Self(prefix.to_vec())
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) -> Result<(), String> {
        if !v.is_finite() {
            return Err("nonfinite f64".into());
        }
        self.0
            .extend_from_slice(&(if v == 0.0 { 0.0 } else { v }).to_le_bytes());
        Ok(())
    }
    fn string(&mut self, v: &str) -> Result<(), String> {
        let bytes = v.as_bytes();
        let len: u32 = bytes.len().try_into().map_err(|_| "oversize string")?;
        self.u32(len);
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    fn hash_bytes(&mut self, h: &str) -> Result<(), String> {
        if h.len() != 64
            || !h
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("invalid lowercase SHA-256".into());
        }
        for i in (0..64).step_by(2) {
            self.u8(u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| "malformed SHA256")?);
        }
        Ok(())
    }
    fn digest(self) -> String {
        format!("{:x}", Sha256::digest(&self.0))
    }
}
pub fn selection_digest(v: &SelectionV1) -> Result<String, String> {
    let mut w = Writer::new(b"M2M-SELECTION-1\0");
    for q in &v.bbox {
        w.f64(*q)?;
    }
    w.f64(v.terrain_profile.target_ground_sample_meters)?;
    w.string(&v.terrain_profile.no_data_policy)?;
    w.u32(
        v.terrain_profile
            .max_samples
            .try_into()
            .map_err(|_| "maxSamples exceeds u32")?,
    );
    Ok(w.digest())
}
pub fn manifest_digest(v: &TerrainSourceManifestV1) -> Result<String, String> {
    let mut w = Writer::new(b"M2M-MANIFEST-1\0");
    w.string(&v.source_name)?;
    w.string(&v.release_id)?;
    w.string(&v.archive_id)?;
    w.hash_bytes(&v.source_archive_sha256)?;
    for q in &v.coverage_bbox {
        w.f64(*q)?;
    }
    w.string(&v.horizontal_crs)?;
    w.string(&v.vertical_datum_status)?;
    w.string(&v.vertical_datum_id)?;
    w.f64(v.nominal_ground_sample_meters)?;
    w.string(&v.native_resolution_source)?;
    match v.uncertainty_meters {
        Some(n) => {
            w.u8(1);
            w.f64(n)?;
        }
        None => w.u8(0),
    }
    w.string(&v.coverage_status)?;
    match &v.coverage_index_sha256 {
        Some(n) => {
            w.u8(1);
            w.hash_bytes(n)?;
        }
        None => w.u8(0),
    }
    w.u64(v.source_bytes);
    w.string(&v.license_identifier)?;
    w.string(&v.attribution)?;
    w.string(&v.decoder_version)?;
    Ok(w.digest())
}
pub fn grid_digest(v: &ElevationGridV1) -> Result<String, String> {
    let samples = v
        .samples_meters
        .as_ref()
        .ok_or("grid bytes missing; resolve verified token first")?;
    let mask = v.valid_mask.as_ref().ok_or("grid validity mask missing")?;
    let total = v.width.checked_mul(v.height).ok_or("grid shape overflow")?;
    if samples.len() != total || mask.len() != total || total > 1_000_000 {
        return Err("invalid grid shape".into());
    }
    let mut w = Writer::new(b"M2M-GRID-1\0");
    w.u32(v.width.try_into().map_err(|_| "grid width overflow")?);
    w.u32(v.height.try_into().map_err(|_| "grid height overflow")?);
    w.string(&v.selection_id)?;
    w.hash_bytes(&v.source_manifest_sha256)?;
    w.string(&v.horizontal_transform_id)?;
    w.string(&v.vertical_datum_id)?;
    for q in &v.anchor_wgs84 {
        w.f64(*q)?;
    }
    for q in [
        v.min_east_meters,
        v.max_north_meters,
        v.dx_meters,
        v.dy_meters,
    ] {
        w.f64(q)?;
    }
    for (valid, sample) in mask.iter().zip(samples.iter()) {
        match (*valid, sample) {
            (true, Some(q)) => {
                w.u8(1);
                w.f64(*q)?;
            }
            (false, None) => w.u8(0),
            _ => return Err("grid mask/sample mismatch".into()),
        }
    }
    Ok(w.digest())
}
#[derive(Debug, Clone)]
pub struct MeshTriangle {
    pub part_id: String,
    pub indices: [usize; 3],
}
fn quantize(x: f64) -> Result<i64, String> {
    if !x.is_finite() {
        return Err("nonfinite mesh vertex".into());
    }
    let q = (x.abs() * 1_000_000.0 + 0.5).floor().copysign(x);
    if !q.is_finite() || q.abs() > ((1u64 << 53) as f64) {
        return Err("mesh integer overflow".into());
    }
    Ok(q as i64)
}
pub fn canonical_mesh_digest(
    vertices: &[[f64; 3]],
    triangles: &[MeshTriangle],
) -> Result<String, String> {
    let mut sorted: Vec<([i64; 3], usize)> = Vec::with_capacity(vertices.len());
    for (i, v) in vertices.iter().enumerate() {
        sorted.push(([quantize(v[0])?, quantize(v[1])?, quantize(v[2])?], i));
    }
    sorted.sort_by_key(|v| v.0);
    for adjacent in sorted.windows(2) {
        if adjacent[0].0 == adjacent[1].0 {
            return Err("duplicate quantized vertex".into());
        }
    }
    let mut remapped = vec![0_usize; vertices.len()];
    for (index, original) in sorted.iter().enumerate() {
        remapped[original.1] = index;
    }
    let mut tris = Vec::new();
    for t in triangles {
        if t.part_id.is_empty() || t.indices.iter().any(|i| *i >= vertices.len()) {
            return Err("invalid mesh triangle".into());
        }
        let x = [
            remapped[t.indices[0]],
            remapped[t.indices[1]],
            remapped[t.indices[2]],
        ];
        if x[0] == x[1] || x[1] == x[2] || x[0] == x[2] {
            return Err("degenerate triangle".into());
        }
        let variants = [x, [x[1], x[2], x[0]], [x[2], x[0], x[1]]];
        let canonical = *variants.iter().min().ok_or("no triangle")?;
        tris.push((t.part_id.as_bytes().to_vec(), canonical));
    }
    tris.sort();
    let mut w = Writer::new(b"M2M-MESH-1\0");
    w.u32(
        sorted
            .len()
            .try_into()
            .map_err(|_| "vertex count overflow")?,
    );
    for (v, _) in sorted {
        for q in v {
            w.i64(q);
        }
    }
    w.u32(
        tris.len()
            .try_into()
            .map_err(|_| "triangle count overflow")?,
    );
    for (id, v) in tris {
        w.string(std::str::from_utf8(&id).map_err(|_| "invalid part id")?)?;
        for i in v {
            w.u32(i.try_into().map_err(|_| "triangle index overflow")?);
        }
    }
    Ok(w.digest())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independently_computed_selection_manifest_and_grid_vectors() {
        let s: SelectionV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"
        ))
        .unwrap();
        let m: TerrainSourceManifestV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
        ))
        .unwrap();
        let g: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
        ))
        .unwrap();
        assert_eq!(
            selection_digest(&s).unwrap(),
            "9c13f5eaa44b01562cc6c26796669d44e25bd9ed593420861ef353065c9ae274"
        );
        assert_eq!(
            manifest_digest(&m).unwrap(),
            "08f6fe9aa91e6415b162fc7b7f42616c302da72bf5fb946a03620c6627e121f3"
        );
        assert_eq!(
            grid_digest(&g).unwrap(),
            "62ba45025ea9f6f4de60edcfa05136de7097c0a87ce77541322cb1a8bb83b7dd"
        );
        let invalid: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.missing.json"
        ))
        .unwrap();
        assert_eq!(
            grid_digest(&invalid).unwrap(),
            "8f9b1b045fa0f5c1d31da6f927a1da48c4e1e70d565763ecb37db5771aaff833"
        );
    }
    #[test]
    fn canonical_3mf_geometry_vector_is_zip_independent() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/MeshCanonicalV1.box.json"
        ))
        .unwrap();
        let vertices: Vec<[f64; 3]> = serde_json::from_value(fixture["vertices"].clone()).unwrap();
        let triangles = fixture["triangles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| MeshTriangle {
                part_id: t["partId"].as_str().unwrap().into(),
                indices: serde_json::from_value(t["indices"].clone()).unwrap(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            canonical_mesh_digest(&vertices, &triangles).unwrap(),
            "19755deb6cdd855f7a5facb8067fbbdec6a7667378f22a9acda85c1854cfa9ac"
        );
    }
}
