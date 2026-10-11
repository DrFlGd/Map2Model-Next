//! Synthetic decoder-shaped grid -> genuine Rust mesh -> actual 3MF test.
//! The original, artificial samples below are NOT read from PMTiles/WebP:
//! Agent 2 owns that still-independent integration fixture and native decoder.
use map2model_contracts::canonical::{grid_digest, manifest_digest};
use map2model_contracts::validation::{validate_build, validate_grid, validate_manifest};
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_export::package::{export_3mf, write_3mf_atomic};
use map2model_mesh::build_mesh;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::process::Command;
use zip::ZipArchive;

struct Case {
    width: usize,
    height: usize,
    exaggeration: f64,
    interior_clip: bool,
}

fn signed_synthetic_grid(
    case: &Case,
) -> (
    ElevationGridV1,
    TerrainBuildRequestV1,
    TerrainSourceManifestV1,
) {
    // Variable grid shapes and metric step lengths mimic the V1 inline
    // output shape from Agent 2's decode_selection_grid() implementation.
    let mut grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .unwrap();
    let mut request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .unwrap();
    let mut source: TerrainSourceManifestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
    ))
    .unwrap();
    let heights = (0..case.height)
        .flat_map(|row| {
            (0..case.width).map(move |col| {
                Some(90.0 + ((3 * col + 5 * row + (col * row) % 7) % 19) as f64 / 4.0)
            })
        })
        .collect::<Vec<_>>();
    // Bind the synthetic source archive ID/SHA to the actual original
    // generated sample payload, NOT to Mapterhorn or a purported PMTiles file.
    let original_payload = serde_json::to_vec(&heights).unwrap();
    source.source_name = "Original Agent 3 decoder-shaped artificial elevations".into();
    source.release_id = "export-variable-grid-test-v1".into();
    source.archive_id = format!("NOT-PMTILES-original-test-{}x{}", case.width, case.height);
    source.source_archive_sha256 = format!("{:x}", Sha256::digest(&original_payload));
    source.source_bytes = original_payload.len() as u64;
    source.local_asset_token = format!("m2m-asset:{}", source.source_archive_sha256);
    source.coverage_status = "envelope-only".into();
    source.coverage_index_sha256 = None;
    source.attribution =
        "Entirely artificial test elevations, NOT downloaded or decoded terrain".into();
    source.license_identifier = "CC0-1.0".into();
    source.vertical_datum_id = "SYNTHETIC-RELATIVE-METERS".into();
    source.vertical_datum_status = "known".into();
    source.decoder_version = "agent-3-synthetic-grid-adapter/1".into();
    source.manifest_sha256 = manifest_digest(&source).unwrap();
    grid.width = case.width;
    grid.height = case.height;
    grid.min_east_meters = 0.0;
    // Match the exact frozen SelectionV1 WGS84 bounds and Agent 2's
    // projected-metric EN rectangle, but intentionally vary node density.
    let selection: map2model_contracts::SelectionV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"
    ))
    .unwrap();
    let [east_m, north_m] = map2model_contracts::validation::project_wgs84(
        [selection.bbox[0], selection.bbox[1]],
        [selection.bbox[2], selection.bbox[3]],
    )
    .unwrap();
    grid.dx_meters = east_m / (case.width - 1) as f64;
    grid.dy_meters = north_m / (case.height - 1) as f64;
    grid.max_north_meters = north_m;
    grid.samples_meters = Some(heights);
    grid.valid_mask = Some(vec![true; case.width * case.height]);
    grid.source_manifest_sha256 = source.manifest_sha256.clone();
    grid.vertical_datum_id = source.vertical_datum_id.clone();
    grid.grid_sha256 = grid_digest(&grid).unwrap();
    grid.grid_id = format!("grid-{}", &grid.grid_sha256[..32]);
    request.grid_sha256 = grid.grid_sha256.clone();
    request.selection_id = grid.selection_id.clone();
    let (west, south, east, north) = if case.interior_clip {
        (
            grid.dx_meters,
            grid.dy_meters,
            grid.dx_meters * (case.width as f64 - 2.0),
            grid.max_north_meters - grid.dy_meters,
        )
    } else {
        (
            0.0,
            0.0,
            (case.width - 1) as f64 * grid.dx_meters,
            grid.max_north_meters,
        )
    };
    request.clip_rect_meters = [west, south, east, north];
    request.model_width_mm = (east - west) * 3.0;
    request.model_height_mm = (north - south) * 3.0;
    request.base_thickness_mm = 5.0;
    request.minimum_feature_thickness_mm = 2.0;
    request.elevation_zero_meters = 90.0;
    request.vertical_exaggeration = case.exaggeration;
    request.engine_version = "agent3-variable-grid-test/1".into();
    request.job_id = format!("job-{}-{}-{}", case.width, case.height, case.exaggeration);
    request.export_token = format!("m2m-export:variable-{}-{}", case.width, case.height);
    (grid, request, source)
}

fn read_provenance(actual_zip: &[u8]) -> Value {
    let mut z = ZipArchive::new(Cursor::new(actual_zip)).unwrap();
    let mut bytes = Vec::new();
    z.by_name("Metadata/map2model.provenance.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn python_stdlib_readback(bytes: &[u8], output: &map2model_mesh::BuildOutput) -> Value {
    let directory = std::env::temp_dir().join(format!(
        "m2m-variable-3mf-{}-{}",
        std::process::id(),
        &output.canonical_geometry_sha256[..14]
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let zip_path = directory.join("variable.3mf");
    let reference = directory.join("reference.json");
    std::fs::write(&zip_path, bytes).unwrap();
    std::fs::write(
        &reference,
        serde_json::to_vec(&json!({
            "vertices":output.geometry.vertices,
            "triangles":output.geometry.triangles,
            "canonicalGeometrySha256":output.canonical_geometry_sha256,
            "measuredAabbMm":output.geometry.measured_aabb_mm,
            "volumeMm3":output.geometry.volume_mm3,
        }))
        .unwrap(),
    )
    .unwrap();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/independent_3mf.py");
    let mut result = None;
    for executable in ["python", "python3"] {
        match Command::new(executable)
            .arg(script)
            .arg(&zip_path)
            .arg(&reference)
            .output()
        {
            Ok(out) => {
                result = Some(out);
                break;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => panic!("Python external reader launch: {err}"),
        }
    }
    let result = result.expect("Python 3 required by existing independent 3MF test suite");
    let _ = std::fs::remove_dir_all(directory);
    assert!(
        result.status.success(),
        "independent 3MF readback failure: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn variable_original_synthetic_grids_export_deterministic_signed_provenance_3mfs() {
    let cases = [
        Case {
            width: 5,
            height: 4,
            exaggeration: 0.75,
            interior_clip: false,
        },
        Case {
            width: 8,
            height: 6,
            exaggeration: 2.25,
            interior_clip: true,
        },
        Case {
            width: 13,
            height: 9,
            exaggeration: 3.5,
            interior_clip: false,
        },
    ];
    let mut package_hashes = std::collections::HashSet::new();
    let mut mesh_hashes = std::collections::HashSet::new();
    for case in cases {
        let (grid, request, source) = signed_synthetic_grid(&case);
        validate_grid(&grid).unwrap();
        validate_manifest(&source).unwrap();
        validate_build(&request, &grid).unwrap();
        let output = build_mesh(&grid, &request).unwrap();
        let exported = export_3mf(&output, &grid, &request, &source).unwrap();
        let repeat = export_3mf(&output, &grid, &request, &source).unwrap();
        assert_eq!(exported.bytes, repeat.bytes);
        assert_eq!(exported.package_sha256, repeat.package_sha256);
        assert_eq!(
            exported.canonical_geometry_sha256,
            output.canonical_geometry_sha256
        );
        assert_eq!(exported.vertices, output.geometry.vertices);
        assert_eq!(exported.triangles, output.geometry.triangles);
        assert_eq!(exported.aabb_mm, output.geometry.measured_aabb_mm);
        assert!((exported.aabb_mm[3] - request.model_width_mm).abs() < 0.05);
        assert!((exported.aabb_mm[4] - request.model_height_mm).abs() < 0.05);
        assert!(output.geometry.minimum_measured_thickness_mm >= request.base_thickness_mm);
        assert!(output.geometry.volume_mm3 > 0.0);
        assert!(package_hashes.insert(exported.package_sha256.clone()));
        assert!(mesh_hashes.insert(exported.canonical_geometry_sha256.clone()));
        assert_ne!(
            exported.provenance_sha256,
            exported.canonical_geometry_sha256
        );
        assert_ne!(exported.package_sha256, exported.provenance_sha256);
        let provenance = read_provenance(&exported.bytes);
        assert_eq!(
            provenance["sourceArchiveSha256"],
            source.source_archive_sha256
        );
        assert_eq!(provenance["sourceManifestSha256"], source.manifest_sha256);
        assert_eq!(provenance["gridSha256"], grid.grid_sha256);
        assert_eq!(provenance["selectionId"], request.selection_id);
        assert_eq!(provenance["sourceLicenseIdentifier"], "CC0-1.0");
        assert_eq!(
            provenance["sourceVerticalDatumId"],
            "SYNTHETIC-RELATIVE-METERS"
        );
        assert_eq!(provenance["verticalExaggeration"], case.exaggeration);
        assert_eq!(
            provenance["canonicalGeometrySha256"],
            output.canonical_geometry_sha256
        );
        let report = python_stdlib_readback(&exported.bytes, &output);
        assert_eq!(report["status"], "passed");
        assert_eq!(report["packageSha256"], exported.package_sha256);
        assert_eq!(report["provenanceSha256"], exported.provenance_sha256);
        assert_eq!(
            report["canonicalGeometrySha256"],
            exported.canonical_geometry_sha256
        );
        println!(
            "variable={}x{} exaggeration={} zip={} mesh={} provenance={} volume={}",
            case.width,
            case.height,
            case.exaggeration,
            exported.package_sha256,
            exported.canonical_geometry_sha256,
            exported.provenance_sha256,
            output.geometry.volume_mm3
        );
    }
    assert_eq!(package_hashes.len(), 3);
    assert_eq!(mesh_hashes.len(), 3);
}

#[test]
fn varied_grid_export_blocks_unverified_rights_datum_missing_clipped_nodes_or_mismatched_hash() {
    let case = Case {
        width: 8,
        height: 6,
        exaggeration: 2.25,
        interior_clip: true,
    };
    let (grid, request, source) = signed_synthetic_grid(&case);
    let output = build_mesh(&grid, &request).unwrap();
    let mut wrong_source = source.clone();
    wrong_source.vertical_datum_status = "unknown".into();
    wrong_source.manifest_sha256 = manifest_digest(&wrong_source).unwrap();
    let err = export_3mf(&output, &grid, &request, &wrong_source).unwrap_err();
    assert_eq!(err.job_reason_code(), "UNSUPPORTED_DATUM");

    let mut wrong_source = source.clone();
    wrong_source.license_identifier.clear();
    wrong_source.manifest_sha256 = manifest_digest(&wrong_source).unwrap();
    assert_eq!(
        export_3mf(&output, &grid, &request, &wrong_source)
            .unwrap_err()
            .job_reason_code(),
        "INVALID_SOURCE"
    );

    let mut wrong_grid = grid.clone();
    wrong_grid.samples_meters.as_mut().unwrap()[wrong_grid.width + 2] = None;
    wrong_grid.valid_mask.as_mut().unwrap()[wrong_grid.width + 2] = false;
    wrong_grid.grid_sha256 = grid_digest(&wrong_grid).unwrap();
    let mut wrong_request = request.clone();
    wrong_request.grid_sha256 = wrong_grid.grid_sha256.clone();
    let failure = build_mesh(&wrong_grid, &wrong_request).unwrap_err();
    assert_eq!(failure.code, "DATA_MISSING");

    let mut wrong_grid = grid.clone();
    wrong_grid.grid_sha256 = "f".repeat(64);
    assert_eq!(
        export_3mf(&output, &wrong_grid, &request, &source)
            .unwrap_err()
            .job_reason_code(),
        "INVALID_GRID_HASH"
    );

    let mut wrong_request = request.clone();
    wrong_request.selection_id = "sel-wrong-but-well-formed".into();
    assert_eq!(
        export_3mf(&output, &grid, &wrong_request, &source)
            .unwrap_err()
            .job_reason_code(),
        "HASH_MISMATCH"
    );

    let mut wrong_request = request.clone();
    wrong_request.clip_rect_meters[0] += 1.0;
    assert_eq!(
        export_3mf(&output, &grid, &wrong_request, &source)
            .unwrap_err()
            .job_reason_code(),
        "INVALID_DIMENSIONS"
    );

    let exported = export_3mf(&output, &grid, &request, &source).unwrap();
    let parent = std::env::temp_dir().join(format!(
        "m2m-variable-cancel-{}",
        &exported.package_sha256[..20]
    ));
    std::fs::create_dir_all(&parent).unwrap();
    let dest = parent.join("cancel.3mf");
    let mut polls = 0;
    assert_eq!(
        write_3mf_atomic(&dest, &exported, || {
            polls += 1;
            polls == 2
        })
        .unwrap_err()
        .job_reason_code(),
        "CANCELLED"
    );
    assert!(!dest.exists());
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    std::fs::remove_dir_all(parent).unwrap();
}
