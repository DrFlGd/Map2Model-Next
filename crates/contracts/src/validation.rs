//! Native reusable G0 ingress validators; type deserialization alone is insufficient.
use crate::{
    canonical, ElevationGridV1, JobStateV1, MeshAnd3mfResultV1, SelectionV1, TerrainBuildRequestV1,
    TerrainSourceManifestV1,
};
const MAX_CELLS: usize = 1_000_000;
const MAX_LATITUDE: f64 = 60.0;
const MAX_EXTENT_METERS: f64 = 2_000.0;

fn finite(x: f64) -> bool {
    x.is_finite()
}
pub fn project_wgs84(anchor: [f64; 2], target: [f64; 2]) -> Result<[f64; 2], String> {
    if !anchor.iter().chain(target.iter()).all(|v| finite(*v))
        || anchor[1].abs() > MAX_LATITUDE
        || target[1].abs() > MAX_LATITUDE
        || anchor[0].abs() > 180.0
        || target[0].abs() > 180.0
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
        && b[1] >= -MAX_LATITUDE
        && b[3] <= MAX_LATITUDE
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
    if east > MAX_EXTENT_METERS || north > MAX_EXTENT_METERS {
        return Err("INVALID_SELECTION: region exceeds 2km G1 extent".into());
    }
    if v.terrain_profile.no_data_policy != "fail"
        || !finite(v.terrain_profile.target_ground_sample_meters)
        || !(0.1..=2000.0).contains(&v.terrain_profile.target_ground_sample_meters)
    {
        return Err("INVALID_SELECTION: unsupported no-data policy or sample spacing".into());
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
pub fn validate_manifest(v: &TerrainSourceManifestV1) -> Result<(), String> {
    // These wire discriminants are excluded from canonical manifest hashing.
    // serde parsing alone does not enforce the V1 JSON-Schema const values.
    if v.schema_version != "1.0.0" || v.kind != "TerrainSourceManifestV1" {
        return Err("INVALID_SOURCE: unsupported manifest version or kind".into());
    }
    for value in [
        &v.source_name,
        &v.release_id,
        &v.archive_id,
        &v.horizontal_crs,
        &v.vertical_datum_id,
        &v.native_resolution_source,
        &v.license_identifier,
        &v.attribution,
        &v.decoder_version,
    ] {
        if value.is_empty() {
            return Err("INVALID_SOURCE: required source or rights metadata missing".into());
        }
    }
    if v.source_bytes < 1 || v.source_bytes > 9_007_199_254_740_991 {
        return Err("INVALID_SOURCE: archive size outside positive JS-safe range".into());
    }
    if !finite(v.nominal_ground_sample_meters) || v.nominal_ground_sample_meters <= 0.0 {
        return Err("INVALID_SOURCE: nominal spacing must be finite and positive".into());
    }
    if let Some(uncertainty) = v.uncertainty_meters {
        if !finite(uncertainty) || uncertainty < 0.0 {
            return Err("INVALID_SOURCE: uncertainty must be nonnegative and finite".into());
        }
    }
    let valid_sha = |hash: &str| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if !valid_sha(&v.source_archive_sha256) || !valid_sha(&v.manifest_sha256) {
        return Err("INVALID_SOURCE: invalid source or manifest SHA-256".into());
    }
    if v.coverage_status != "verified-tile-index" && v.coverage_status != "envelope-only" {
        return Err("INVALID_SOURCE: unsupported coverage status".into());
    }
    if v.coverage_status == "verified-tile-index" {
        if v.coverage_index_sha256
            .as_deref()
            .is_none_or(|hash| !valid_sha(hash))
        {
            return Err("DATA_MISSING: declared tile index SHA-256 missing or malformed".into());
        }
    } else if v.coverage_index_sha256.is_some() {
        return Err("INVALID_SOURCE: envelope cannot claim a verified index".into());
    }
    if v.vertical_datum_status != "known"
        && v.vertical_datum_status != "unknown"
        && v.vertical_datum_status != "unsupported"
    {
        return Err("INVALID_SOURCE: unsupported vertical datum status".into());
    }
    if !valid_bbox(v.coverage_bbox)
        || v.local_asset_token != format!("m2m-asset:{}", v.source_archive_sha256)
    {
        return Err("HASH_MISMATCH: archive token or coverage invalid".into());
    }
    if v.vertical_datum_status != "known" || v.vertical_datum_id.is_empty() {
        return Err("UNSUPPORTED_DATUM: source datum not verified".into());
    }
    if canonical::manifest_digest(v)? != v.manifest_sha256 {
        return Err("HASH_MISMATCH: manifest SHA256 mismatch".into());
    }
    // A verified coverage index must STILL be queried for every required Z/X/Y tile.
    Ok(()) // An index declaration does NOT prove any requested tile is present.
}
pub fn validate_grid(v: &ElevationGridV1) -> Result<(), String> {
    if v.schema_version != "1.0.0"
        || v.kind != "ElevationGridV1"
        || v.storage_convention != "north-up-f64-row-major"
    {
        return Err("INVALID_SOURCE: unsupported grid schema or layout".into());
    }
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
    if req.selection_id != grid.selection_id {
        return Err("HASH_MISMATCH: request/grid selectionId mismatch".into());
    }
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
/// G0 output acceptance must bind output to its actual build request.
/// Runtime artifact bytes and model conformance are verified by A3-02.
pub fn validate_result_against_request(
    result: &MeshAnd3mfResultV1,
    request: &TerrainBuildRequestV1,
    grid: Option<&ElevationGridV1>,
) -> Result<(), String> {
    validate_result(result)?;
    if result.job_id != request.job_id || result.artifact.export_token != request.export_token {
        return Err("HASH_MISMATCH: export job/token differs from build".into());
    }
    let box_mm = result.geometry.measured_aabb_mm;
    let tolerance = 0.05_f64;
    if box_mm[0].abs() > tolerance
        || box_mm[1].abs() > tolerance
        || box_mm[2].abs() > tolerance
        || ((box_mm[3] - box_mm[0]) - request.model_width_mm).abs() > tolerance
        || ((box_mm[4] - box_mm[1]) - request.model_height_mm).abs() > tolerance
    {
        return Err("INVALID_DIMENSIONS: measured XY bounds differ from build".into());
    }
    if !finite(result.geometry.minimum_measured_thickness_mm)
        || result.geometry.minimum_measured_thickness_mm + tolerance
            < request
                .base_thickness_mm
                .max(request.minimum_feature_thickness_mm)
    {
        return Err("INVALID_GEOMETRY: insufficient measured thickness".into());
    }
    if let Some(g) = grid {
        validate_build(request, g)?;
        let [x0, y0, x1, y1] = request.clip_rect_meters;
        let scale = request.model_width_mm / (x1 - x0);
        let mut max_z = f64::NEG_INFINITY;
        let values = g
            .samples_meters
            .as_ref()
            .ok_or("DATA_MISSING: grid unresolved")?;
        for (i, sample) in values.iter().enumerate() {
            let gx = g.min_east_meters + (i % g.width) as f64 * g.dx_meters;
            let gy = g.max_north_meters - (i / g.width) as f64 * g.dy_meters;
            if gx < x0 - 1e-9 || gx > x1 + 1e-9 || gy < y0 - 1e-9 || gy > y1 + 1e-9 {
                continue;
            }
            let z = sample.ok_or("DATA_MISSING: clipped sample absent")?;
            max_z = max_z.max(
                request.base_thickness_mm
                    + (z - request.elevation_zero_meters) * scale * request.vertical_exaggeration,
            );
        }
        if !finite(max_z) || (box_mm[5] - max_z).abs() > tolerance {
            return Err("INVALID_DIMENSIONS: measured Z maximum differs from terrain".into());
        }
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
        assert!(validate_manifest(&m).is_ok());
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

#[cfg(test)]
mod additional_contract_tests {
    use super::*;
    #[test]
    fn reject_cross_selection_and_unlinked_export_result() {
        let grid: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
        ))
        .unwrap();
        let req: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
        ))
        .unwrap();
        let result: MeshAnd3mfResultV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/MeshAnd3mfResultV1.valid.json"
        ))
        .unwrap();
        assert!(validate_result_against_request(&result, &req, Some(&grid)).is_ok());
        let mut wrong = req.clone();
        wrong.selection_id = "sel-another-selection".into();
        assert!(validate_build(&wrong, &grid)
            .unwrap_err()
            .starts_with("HASH_MISMATCH"));
        let mut result_bad = result.clone();
        result_bad.artifact.export_token = "m2m-export:other".into();
        assert!(validate_result_against_request(&result_bad, &req, None).is_err());
        let mut thickness_bad = result.clone();
        thickness_bad.geometry.minimum_measured_thickness_mm = 1.0;
        assert!(validate_result_against_request(&thickness_bad, &req, None).is_err());
        let mut xy_bad = result;
        xy_bad.geometry.measured_aabb_mm = [0.0, 0.0, 0.0, 10.0, 20.0, 30.0];
        assert!(validate_result_against_request(&xy_bad, &req, None).is_err());
    }
    #[test]
    fn native_ingress_does_not_allow_canonical_hash_to_bypass_no_data_policy() {
        let mut sel: SelectionV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"
        ))
        .unwrap();
        sel.terrain_profile.no_data_policy = "fill".into();
        sel.selection_id = format!("sel-{}", &canonical::selection_digest(&sel).unwrap()[..32]);
        assert!(validate_selection(&sel).is_err());
        sel.terrain_profile.no_data_policy = "fail".into();
        sel.terrain_profile.target_ground_sample_meters = f64::INFINITY;
        assert!(validate_selection(&sel).is_err());
        let mut grid: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
        ))
        .unwrap();
        grid.anchor_wgs84 = [181.0, 0.0];
        assert!(validate_grid(&grid).unwrap_err().starts_with("INVALID_CRS"));
    }
}

#[cfg(test)]
mod manifest_ingress_parity_tests {
    use super::*;

    fn fixture() -> TerrainSourceManifestV1 {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
        ))
        .unwrap()
    }

    #[test]
    fn altered_version_or_kind_cannot_bypass_digest_validation() {
        let original = fixture();
        let mut version = original.clone();
        version.schema_version = "2.0.0".into();
        assert_eq!(
            canonical::manifest_digest(&version).unwrap(),
            original.manifest_sha256
        );
        assert!(validate_manifest(&version).unwrap_err().starts_with("INVALID_SOURCE"));
        let mut kind = original.clone();
        kind.kind = "Other".into();
        assert_eq!(
            canonical::manifest_digest(&kind).unwrap(),
            original.manifest_sha256
        );
        assert!(validate_manifest(&kind).unwrap_err().starts_with("INVALID_SOURCE"));
    }

    #[test]
    fn rehashed_unrecognized_coverage_never_claims_readiness() {
        let mut forged = fixture();
        forged.coverage_status = "another-kind-of-coverage".into();
        forged.manifest_sha256 = canonical::manifest_digest(&forged).unwrap();
        assert!(validate_manifest(&forged).unwrap_err().starts_with("INVALID_SOURCE"));
        let mut declared = fixture();
        declared.coverage_status = "verified-tile-index".into();
        declared.coverage_index_sha256 = Some("f".repeat(64));
        declared.manifest_sha256 = canonical::manifest_digest(&declared).unwrap();
        assert!(validate_manifest(&declared).is_ok());
        // This returns validation only; existence and Z/X/Y presence require the native registry.
    }

    #[test]
    fn required_source_rights_size_and_resolution_fields_fail_closed() {
        let mut invalid = fixture();
        invalid.source_name.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.release_id.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.archive_id.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.horizontal_crs.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.vertical_datum_id.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.native_resolution_source.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.license_identifier.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.attribution.clear();
        assert!(validate_manifest(&invalid).is_err());
        let mut invalid = fixture();
        invalid.decoder_version.clear();
        assert!(validate_manifest(&invalid).is_err());

        for size in [0, 9_007_199_254_740_992, u64::MAX] {
            let mut invalid = fixture();
            invalid.source_bytes = size;
            assert!(validate_manifest(&invalid).is_err(), "size: {size}");
        }
        for nominal in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut invalid = fixture();
            invalid.nominal_ground_sample_meters = nominal;
            assert!(validate_manifest(&invalid).is_err());
        }
        for uncertainty in [-1.0, f64::NAN, f64::INFINITY] {
            let mut invalid = fixture();
            invalid.uncertainty_meters = Some(uncertainty);
            assert!(validate_manifest(&invalid).is_err());
        }
        let mut invalid = fixture();
        invalid.source_archive_sha256 = "NOT_A_HASH".into();
        assert!(validate_manifest(&invalid).is_err());
    }
}
