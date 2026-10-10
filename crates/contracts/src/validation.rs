//! Native reusable G0 ingress validators; type deserialization alone is insufficient.
use crate::{
    canonical, ElevationGridV1, JobStateV1, MeshAnd3mfResultV1, SelectionV1, TerrainBuildRequestV1,
    TerrainSourceManifestV1,
};
const MAX_CELLS: usize = 1_000_000;

fn finite(x: f64) -> bool {
    x.is_finite()
}
pub fn project_wgs84(anchor: [f64; 2], target: [f64; 2]) -> Result<[f64; 2], String> {
    if !anchor.iter().chain(target.iter()).all(|v| finite(*v))
        || anchor[1].abs() > 80.0
        || target[1].abs() > 80.0
        || (target[0] - anchor[0]).abs() > 1.0
    {
        return Err("INVALID_CRS: outside linearized EN domain".into());
    }
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    let e2 = F * (2.0 - F);
    let phi = anchor[1].to_radians();
    let q = 1.0 - e2 * phi.sin().powi(2);
    let meridian = A * (1.0 - e2) / q.powf(1.5);
    let prime_vertical = A / q.sqrt();
    Ok([
        (target[0] - anchor[0]).to_radians() * prime_vertical * phi.cos(),
        (target[1] - anchor[1]).to_radians() * meridian,
    ])
}
fn valid_bbox(b: [f64; 4]) -> bool {
    b.iter().all(|x| finite(*x))
        && b[0] >= -180.0
        && b[2] <= 180.0
        && b[1] >= -80.0
        && b[3] <= 80.0
        && b[0] < b[2]
        && b[1] < b[3]
        && (b[2] - b[0]) < 1.0
}
pub fn validate_selection(v: &SelectionV1) -> Result<(), String> {
    if v.schema_version != "1.0.0"
        || v.kind != "SelectionV1"
        || v.planet != "earth"
        || v.horizontal_crs != "EPSG:4326"
        || !valid_bbox(v.bbox)
    {
        return Err("INVALID_SELECTION: bbox, kind or reference invalid".into());
    }
    let [east, north] = project_wgs84([v.bbox[0], v.bbox[1]], [v.bbox[2], v.bbox[3]])?;
    if east > 10_000.0 || north > 10_000.0 {
        return Err("INVALID_SELECTION: region exceeds 10km G1 extent".into());
    }
    if v.terrain_profile.max_samples < 4 || v.terrain_profile.max_samples > MAX_CELLS {
        return Err("OUT_OF_MEMORY: sample budget outside G1 limit".into());
    }
    let digest = canonical::selection_digest(v)?;
    if v.selection_id != format!("sel-{}", &digest[..32]) {
        return Err("INVALID_SELECTION: selection hash mismatch".into());
    }
    Ok(())
}
pub fn validate_manifest(v: &TerrainSourceManifestV1) -> Result<bool, String> {
    if !valid_bbox(v.coverage_bbox)
        || v.local_asset_token != format!("m2m-asset:{}", v.source_archive_sha256)
    {
        return Err("HASH_MISMATCH: archive token or coverage invalid".into());
    }
    if v.coverage_status == "verified-tile-index" {
        if v.coverage_index_sha256.is_none() {
            return Err("DATA_MISSING: missing verified tile index".into());
        }
    } else if v.coverage_status != "envelope-only" || v.coverage_index_sha256.is_some() {
        return Err("INVALID_SOURCE: invalid coverage declaration".into());
    }
    if v.vertical_datum_status != "known" || v.vertical_datum_id.is_empty() {
        return Err("UNSUPPORTED_DATUM: source datum not verified".into());
    }
    if !finite(v.nominal_ground_sample_meters) || v.nominal_ground_sample_meters <= 0.0 {
        return Err("INVALID_SOURCE: nominal spacing invalid".into());
    }
    if canonical::manifest_digest(v)? != v.manifest_sha256 {
        return Err("HASH_MISMATCH: manifest SHA256 mismatch".into());
    }
    // A verified coverage index must STILL be queried for every required Z/X/Y tile.
    Ok(v.coverage_status == "verified-tile-index")
}
pub fn validate_grid(v: &ElevationGridV1) -> Result<(), String> {
    let total = v
        .width
        .checked_mul(v.height)
        .ok_or("OUT_OF_MEMORY: shape overflow")?;
    if v.width < 2 || v.height < 2 || v.width > 4096 || v.height > 4096 || total > MAX_CELLS {
        return Err("OUT_OF_MEMORY: grid sample budget exceeded".into());
    }
    if !finite(v.dx_meters)
        || !finite(v.dy_meters)
        || v.dx_meters <= 0.0
        || v.dy_meters <= 0.0
        || v.horizontal_transform_id != "WGS84-ENU-LINEARIZED-V1"
    {
        return Err("INVALID_CRS: transform or spacing invalid".into());
    }
    project_wgs84(v.anchor_wgs84, v.anchor_wgs84)?;
    if v.vertical_datum_id.is_empty() || v.vertical_datum_id == "unknown" {
        return Err("UNSUPPORTED_DATUM: unknown grid datum".into());
    }
    if v.storage_mode == "local-binary-token" {
        if v.samples_meters.is_some()
            || v.valid_mask.is_some()
            || v.grid_data_sha256.is_none()
            || v.grid_data_token != v.grid_data_sha256.as_ref().map(|s| format!("m2m-grid:{s}"))
        {
            return Err("INVALID_SOURCE: invalid grid bulk token".into());
        }
        return Err("DATA_MISSING: resolve and independently validate bulk grid first".into());
    }
    let samples = v
        .samples_meters
        .as_ref()
        .ok_or("INCOMPLETE_GRID: missing samples")?;
    let masks = v
        .valid_mask
        .as_ref()
        .ok_or("INCOMPLETE_GRID: missing masks")?;
    if samples.len() != total || masks.len() != total || v.storage_mode != "inline-fixture" {
        return Err("INCOMPLETE_GRID: sample/mask length mismatch".into());
    }
    for (value, valid) in samples.iter().zip(masks.iter()) {
        match (valid, value) {
            (true, Some(n)) if n.is_finite() => {}
            (false, None) => {}
            _ => return Err("INCOMPLETE_GRID: mask/null/finite invariant violation".into()),
        }
    }
    if canonical::grid_digest(v)? != v.grid_sha256 {
        return Err("INVALID_GRID_HASH: decoded grid does not match SHA".into());
    }
    Ok(())
}
pub fn validate_build(req: &TerrainBuildRequestV1, grid: &ElevationGridV1) -> Result<(), String> {
    validate_grid(grid)?;
    if req.grid_sha256 != grid.grid_sha256 {
        return Err("HASH_MISMATCH: build/grid digest mismatch".into());
    }
    let [x0, y0, x1, y1] = req.clip_rect_meters;
    let fields = [
        x0,
        y0,
        x1,
        y1,
        req.model_width_mm,
        req.model_height_mm,
        req.base_thickness_mm,
        req.minimum_feature_thickness_mm,
        req.vertical_exaggeration,
        req.elevation_zero_meters,
    ];
    if !fields.iter().all(|x| finite(*x))
        || x0 >= x1
        || y0 >= y1
        || req.model_width_mm <= 0.0
        || req.model_height_mm <= 0.0
        || req.base_thickness_mm <= 0.0
        || req.minimum_feature_thickness_mm <= 0.0
        || req.vertical_exaggeration <= 0.0
    {
        return Err("INVALID_DIMENSIONS: print parameters invalid".into());
    }
    let sx = req.model_width_mm / (x1 - x0);
    let sy = req.model_height_mm / (y1 - y0);
    if (sx - sy).abs() > 1e-9 * sx.max(sy) {
        return Err("INVALID_DIMENSIONS: XY aspect ratio differs".into());
    }
    let min_e = grid.min_east_meters;
    let min_n = grid.max_north_meters - (grid.height as f64 - 1.0) * grid.dy_meters;
    let max_e = min_e + (grid.width as f64 - 1.0) * grid.dx_meters;
    let max_n = grid.max_north_meters;
    if x0 < min_e - 1e-9 || x1 > max_e + 1e-9 || y0 < min_n - 1e-9 || y1 > max_n + 1e-9 {
        return Err("INVALID_DIMENSIONS: clip outside grid".into());
    }
    for (value, start, step) in [
        (x0, min_e, grid.dx_meters),
        (x1, min_e, grid.dx_meters),
        (y0, min_n, grid.dy_meters),
        (y1, min_n, grid.dy_meters),
    ] {
        let q = (value - start) / step;
        if (q - q.round()).abs() > 1e-8 {
            return Err("INVALID_DIMENSIONS: G1 clip must be node-aligned".into());
        }
    }
    let values = grid.samples_meters.as_ref().ok_or("INCOMPLETE_GRID")?;
    let mut lowest = f64::INFINITY;
    let mut included = 0_usize;
    for (i, value) in values.iter().enumerate() {
        let col = i % grid.width;
        let row = i / grid.width;
        let gx = grid.min_east_meters + col as f64 * grid.dx_meters;
        let gy = grid.max_north_meters - row as f64 * grid.dy_meters;
        if gx < x0 - 1e-9 || gx > x1 + 1e-9 || gy < y0 - 1e-9 || gy > y1 + 1e-9 {
            continue;
        }
        included += 1;
        let z = value.ok_or("DATA_MISSING: clipped sample absent")?;
        lowest = lowest.min(z);
    }
    if included < 4 {
        return Err("INVALID_DIMENSIONS: clip includes fewer than 2x2 grid nodes".into());
    }
    if req.elevation_zero_meters > lowest + 1e-9 {
        return Err("INVALID_DIMENSIONS: Z baseline above included terrain".into());
    }
    Ok(())
}
pub fn validate_result(v: &MeshAnd3mfResultV1) -> Result<(), String> {
    let g = &v.geometry;
    if !g.closed
        || !g.oriented
        || g.non_manifold_edges > 0
        || g.degenerate_triangles > 0
        || g.component_count != 1
        || g.self_intersection_status != "passed"
        || g.validation.status != "passed"
        || v.artifact.conformance != "passed"
    {
        return Err("INVALID_GEOMETRY: mesh not independently validated".into());
    }
    if !g.measured_aabb_mm.iter().all(|q| finite(*q))
        || g.measured_aabb_mm[0] >= g.measured_aabb_mm[3]
        || g.measured_aabb_mm[1] >= g.measured_aabb_mm[4]
        || g.measured_aabb_mm[2] >= g.measured_aabb_mm[5]
        || !finite(g.volume_mm3)
        || g.volume_mm3 <= 0.0
    {
        return Err("INVALID_GEOMETRY: AABB or volume invalid".into());
    }
    Ok(())
}
pub fn validate_job(v: &JobStateV1) -> Result<(), String> {
    if !finite(v.progress01) || !(0.0..=1.0).contains(&v.progress01) {
        return Err("INTERNAL_ERROR: job progress outside bounds".into());
    }
    if v.status == "succeeded"
        && (v.reason_code.is_some() || v.phase != "complete" || v.progress01 != 1.0)
    {
        return Err("INTERNAL_ERROR: false job success".into());
    }
    if v.status == "failed"
        && (v.reason_code.is_none() || v.reason_code.as_deref() == Some("CANCELLED"))
    {
        return Err("INTERNAL_ERROR: missing failure reason".into());
    }
    if v.status == "cancelled"
        && (v.phase != "cancel" || v.reason_code.as_deref() != Some("CANCELLED"))
    {
        return Err("INTERNAL_ERROR: missing cancellation code".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reusable_native_boundary_checks_fixture_and_false_success() {
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
        let b: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
        ))
        .unwrap();
        let j: JobStateV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/JobStateV1.valid.json"
        ))
        .unwrap();
        assert!(validate_selection(&s).is_ok());
        assert!(!validate_manifest(&m).unwrap());
        assert!(validate_grid(&g).is_ok());
        assert!(validate_build(&b, &g).is_ok());
        assert!(validate_job(&j).is_ok());
        let mut bad = j;
        bad.status = "failed".into();
        assert!(validate_job(&bad).is_err());
        let mut missing: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.missing.json"
        ))
        .unwrap();
        assert!(validate_grid(&missing).is_ok());
        missing.grid_sha256 = b.grid_sha256.clone();
        assert!(validate_grid(&missing).is_err());
    }
    #[test]
    fn projection_reference() {
        let [e, n] = project_wgs84([-84.09, 39.741], [-84.089, 39.742]).unwrap();
        assert!((e - 85.71560839369111).abs() < 1e-6);
        assert!((n - 111.02965952283863).abs() < 1e-6);
    }
}
