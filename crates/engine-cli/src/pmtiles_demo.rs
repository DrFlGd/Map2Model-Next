//! Lead-owned original synthetic PMTiles -> native decoded grid -> mesh -> real 3MF.
//! This is an explicit test-fixture path, NEVER real-source generation readiness.
use map2model_contracts::{validation, SelectionV1, TerrainBuildRequestV1};
use map2model_data::{import_local_pmtiles, terrarium::decode_selection_grid, SourceDeclaration};
use map2model_export::package::{export_3mf, write_3mf_atomic};
use map2model_mesh::build_mesh;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const SOURCE_SHA: &str = "489ec694f29c3d4c7bc5ed3f2dced736736e5c2a5d09cc1cef1ff3a0a8ede651";
const TEST_DATUM: &str = "SYNTHETIC-RELATIVE-METERS";

fn read_small(path: &Path) -> Result<Vec<u8>, String> {
    let info = fs::symlink_metadata(path).map_err(|_| "DATA_MISSING: fixture file missing")?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > 16_384 {
        return Err("INVALID_SOURCE: fixture JSON must be an ordinary bounded file".into());
    }
    let data = fs::read(path).map_err(|_| "DATA_MISSING: fixture JSON unreadable")?;
    if data.len() as u64 != info.len() {
        return Err("HASH_MISMATCH: fixture input changed".into());
    }
    Ok(data)
}

pub fn export_fixture(source_dir: &Path, destination: &Path) -> Result<Value, String> {
    if !source_dir.is_absolute()
        || !source_dir.is_dir()
        || !destination.is_absolute()
        || destination.extension().and_then(|x| x.to_str()) != Some("3mf")
        || destination.exists()
    {
        return Err("INVALID_REQUEST: fixture and fresh .3mf destination must be absolute".into());
    }
    let fixture = source_dir.join("complete.pmtiles");
    let declaration: SourceDeclaration =
        serde_json::from_slice(&read_small(&source_dir.join("complete.source.json"))?)
            .map_err(|_| "INVALID_SOURCE: synthetic declaration not valid JSON")?;
    let selection: SelectionV1 =
        serde_json::from_slice(&read_small(&source_dir.join("selection.json"))?)
            .map_err(|_| "INVALID_SELECTION: fixture selection is invalid")?;
    validation::validate_selection(&selection)?;
    // The bundled, reproducible and internally controlled CC0 mathematical
    // fixture is the only accepted source. Prevent substituting user-supplied
    // real PMTiles, arbitrary declarations or forged verified metadata.
    if declaration.expected_archive_sha256.as_deref() != Some(SOURCE_SHA)
        || declaration.source_name != "Map2Model original synthetic z12 raster; NOT Mapterhorn"
        || declaration.vertical_datum_id != TEST_DATUM
        || declaration.vertical_datum_status != "known"
        || declaration.license_identifier != "CC0-1.0"
        || declaration.release_id != "original-generated-z12-512px-v1"
        || selection.bbox != [-84.024, 39.7067, -84.023, 39.7077]
    {
        return Err("INVALID_SOURCE: only the original audited synthetic source is allowed".into());
    }
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| "INTERNAL_ERROR: OS randomness unavailable")?;
    let token = nonce.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let registry = std::env::temp_dir().join(format!(
        "m2m-original-pmtiles-{}-{token}",
        std::process::id()
    ));
    fs::create_dir(&registry).map_err(|_| "EXPORT_FAILED: temporary registry unavailable")?;
    let result = (|| -> Result<Value, String> {
        let imported = import_local_pmtiles(&fixture, &registry, &declaration)?;
        if imported.manifest.source_archive_sha256 != SOURCE_SHA
            || imported.manifest.vertical_datum_id != TEST_DATUM
        {
            return Err("HASH_MISMATCH: source identity differs from original fixture".into());
        }
        let grid = decode_selection_grid(&imported, &selection, 12)?;
        validation::validate_grid(&grid)?;
        let width_m = grid.dx_meters * (grid.width - 1) as f64;
        let height_m = grid.dy_meters * (grid.height - 1) as f64;
        let request = TerrainBuildRequestV1 {
            schema_version: "1.0.0".into(),
            kind: "TerrainBuildRequestV1".into(),
            job_id: "job-original-pmtiles-preview".into(),
            correlation_id: "corr-original-pmtiles-preview".into(),
            selection_id: grid.selection_id.clone(),
            grid_sha256: grid.grid_sha256.clone(),
            clip_rect_meters: [0., 0., width_m, height_m],
            model_width_mm: 120.,
            model_height_mm: 120. * height_m / width_m,
            base_thickness_mm: 4.,
            minimum_feature_thickness_mm: 1.,
            vertical_exaggeration: 1.,
            elevation_zero_meters: -150.,
            engine_version: "map2model-mesh/0.0.1".into(),
            export_token: "m2m-export:original-synthetic-four-tiles".into(),
        };
        let mesh = build_mesh(&grid, &request).map_err(|e| format!("{}: {}", e.code, e.message))?;
        let archive = export_3mf(&mesh, &grid, &request, &imported.manifest)
            .map_err(|e| format!("{}: {}", e.job_reason_code(), e.message))?;
        write_3mf_atomic(destination, &archive, || false)
            .map_err(|e| format!("{}: {}", e.job_reason_code(), e.message))?;
        let published =
            fs::read(destination).map_err(|_| "EXPORT_FAILED: cannot verify published file")?;
        use sha2::{Digest, Sha256};
        if format!("{:x}", Sha256::digest(&published)) != archive.package_sha256 {
            return Err("HASH_MISMATCH: published 3MF bytes differ from verified package".into());
        }
        Ok(json!({
            "synthetic": true,
            "originalPmtilesDecoded": true,
            "productionGenerationReady": false,
            "threeMfExported": true,
            "sourceArchiveSha256": imported.manifest.source_archive_sha256,
            "sourceManifestSha256": imported.manifest.manifest_sha256,
            "gridSha256": grid.grid_sha256,
            "gridWidth": grid.width,
            "gridHeight": grid.height,
            "selectionId": grid.selection_id,
            "canonicalGeometrySha256": archive.canonical_geometry_sha256,
            "provenanceSha256": archive.provenance_sha256,
            "packageSha256": archive.package_sha256,
            "bytes": published.len(),
            "vertices": archive.vertices,
            "triangles": archive.triangles,
            "aabbMm": archive.aabb_mm,
            "fileName": destination.file_name().and_then(|s| s.to_str()),
            "networkRequests": 0
        }))
    })();
    let _ = fs::remove_dir_all(&registry);
    result
}
