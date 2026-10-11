//! A1-04 local, synthetic-only contract-to-mesh integration smoke.
//! No renderer path, network, geodata entitlement, or 3MF file is involved.
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_export::package::{export_3mf, write_3mf_atomic};
use std::path::Path;
use map2model_mesh::build_mesh;
use serde_json::{json, Value};

pub fn run() -> Result<Value, String> {
    let grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen synthetic grid fixture cannot deserialize")?;
    let request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen synthetic build fixture cannot deserialize")?;
    let built = build_mesh(&grid, &request)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    Ok(json!({
        "fixture": "G0-synthetic-elevation-grid-v1",
        "synthetic": true,
        "meshVerified": built.geometry.closed &&
            built.geometry.oriented &&
            built.geometry.non_manifold_edges == 0 &&
            built.geometry.degenerate_triangles == 0,
        "vertices": built.geometry.vertices,
        "triangles": built.geometry.triangles,
        "aabbMm": built.geometry.measured_aabb_mm,
        "minimumThicknessMm": built.geometry.minimum_measured_thickness_mm,
        "canonicalGeometrySha256": built.canonical_geometry_sha256,
        "threeMfExported": false,
        "productionGenerationReady": false
    }))
}


pub fn export_to(destination: &Path) -> Result<Value, String> {
    if !destination.is_absolute() || destination.extension().and_then(|x| x.to_str()) != Some("3mf") {
        return Err("INVALID_REQUEST: native approved destination must be an absolute .3mf path".into());
    }
    let parent = destination.parent().ok_or("INVALID_REQUEST: export has no directory")?;
    if !parent.is_dir() || destination.exists() {
        return Err("EXPORT_FAILED: destination exists or directory unavailable".into());
    }
    let grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen grid fixture invalid")?;
    let request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen request fixture invalid")?;
    let manifest: TerrainSourceManifestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen source fixture invalid")?;
    let mesh = build_mesh(&grid, &request)
        .map_err(|e| format!("{}: {}", e.code, e.message))?;
    let artifact = export_3mf(&mesh, &grid, &request, &manifest)
        .map_err(|e| format!("{}: {}", e.job_reason_code(), e.message))?;
    write_3mf_atomic(destination, &artifact, || false)
        .map_err(|e| format!("{}: {}", e.job_reason_code(), e.message))?;
    let info = std::fs::metadata(destination)
        .map_err(|_| "EXPORT_FAILED: published artifact cannot be stat-ed")?;
    if info.len() != artifact.bytes.len() as u64 {
        return Err("HASH_MISMATCH: published file length differs".into());
    }
    Ok(json!({
        "synthetic": true,
        "sourceName": "synthetic",
        "productionGenerationReady": false,
        "fileName": destination.file_name().and_then(|s| s.to_str()),
        "packageSha256": artifact.package_sha256,
        "provenanceSha256": artifact.provenance_sha256,
        "canonicalGeometrySha256": artifact.canonical_geometry_sha256,
        "bytes": info.len(),
        "vertices": artifact.vertices,
        "triangles": artifact.triangles,
        "aabbMm": artifact.aabb_mm,
        "threeMfExported": true,
        "validation": "Rust ZIP/OPC XML readback and mesh hash; slicer acceptance pending"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_grid_reaches_actual_watertight_mesh() {
        let value = run().expect("frozen G0 fixture builds");
        assert_eq!(value["meshVerified"], true);
        assert_eq!(value["vertices"], 24);
        assert_eq!(value["triangles"], 44);
        assert_eq!(value["aabbMm"], json!([0., 0., 0., 180., 80., 204.]));
        assert_eq!(
            value["canonicalGeometrySha256"],
            "dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9"
        );
        assert_eq!(value["threeMfExported"], false);
        assert_eq!(value["productionGenerationReady"], false);
    }
}
