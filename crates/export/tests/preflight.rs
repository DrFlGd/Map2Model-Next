use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_export::{prepare_export, CORE_VERSION, MODEL_PART, PROVENANCE_PART};
use map2model_mesh::build_mesh;
use sha2::{Digest, Sha256};

fn fixtures() -> (
    ElevationGridV1,
    TerrainBuildRequestV1,
    TerrainSourceManifestV1,
) {
    let grid = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .unwrap();
    let request = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .unwrap();
    let source = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
    ))
    .unwrap();
    (grid, request, source)
}

#[test]
fn source_licensed_geometry_preflight_is_repeatable_and_contains_full_provenance() {
    let (grid, request, source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    let first = prepare_export(&mesh, &grid, &request, &source).unwrap();
    let second = prepare_export(&mesh, &grid, &request, &source).unwrap();
    assert_eq!(first.provenance_bytes, second.provenance_bytes);
    assert_eq!(first.provenance_sha256, second.provenance_sha256);
    assert_eq!(
        first.canonical_geometry_sha256,
        mesh.canonical_geometry_sha256
    );
    assert_eq!(
        first.canonical_geometry_sha256,
        "dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9"
    );
    assert_eq!(
        first.provenance_sha256,
        format!("{:x}", Sha256::digest(&first.provenance_bytes))
    );
    assert_eq!(first.model_part, MODEL_PART);
    assert_eq!(first.provenance_part, PROVENANCE_PART);
    assert_eq!(first.core_spec_version, CORE_VERSION);
    let value: serde_json::Value = serde_json::from_slice(&first.provenance_bytes).unwrap();
    assert_eq!(value["formatVersion"], "M2M-PROVENANCE-1");
    assert_eq!(
        value["sourceAttribution"],
        "Artificial elevation samples, original Map2Model Next test fixture"
    );
    assert_eq!(value["sourceLicenseIdentifier"], "CC0-1.0");
    assert_eq!(value["sourceManifestSha256"], grid.source_manifest_sha256);
    assert_eq!(value["engineVersion"], request.engine_version);
    assert_eq!(
        value["canonicalGeometrySha256"],
        first.canonical_geometry_sha256
    );
    assert_eq!(
        value["measuredAabbMm"],
        serde_json::json!([0, 0, 0, 180, 80, 204])
    );
    assert!(value.get("provenanceSha256").is_none());
    assert_eq!(value["coverageStatus"], "envelope-only"); // Never imply live tile coverage.
}
#[test]
fn refuse_wrong_source_or_unlicensed_export_without_artifact() {
    let (grid, request, mut source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    source.manifest_sha256 = "f".repeat(64);
    assert_eq!(
        prepare_export(&mesh, &grid, &request, &source)
            .unwrap_err()
            .code,
        "HASH_MISMATCH"
    );
    let (_, _, mut source) = fixtures();
    source.license_identifier.clear();
    assert_eq!(
        prepare_export(&mesh, &grid, &request, &source)
            .unwrap_err()
            .code,
        "INVALID_SOURCE"
    );
    let (_, _, mut source) = fixtures();
    source.vertical_datum_id = "OTHER-DATUM".to_owned();
    assert!(prepare_export(&mesh, &grid, &request, &source).is_err());
}
#[test]
fn refuse_tampered_mesh_hash_geometry_qa_or_request_before_writer() {
    let (grid, request, source) = fixtures();
    let actual = build_mesh(&grid, &request).unwrap();
    let mut wrong = actual.clone();
    wrong.canonical_geometry_sha256 = "0".repeat(64);
    assert_eq!(
        prepare_export(&wrong, &grid, &request, &source)
            .unwrap_err()
            .code,
        "HASH_MISMATCH"
    );
    let mut wrong = actual.clone();
    wrong.geometry.volume_mm3 += 2.0;
    assert_eq!(
        prepare_export(&wrong, &grid, &request, &source)
            .unwrap_err()
            .code,
        "INVALID_GEOMETRY"
    );
    let mut wrong = actual.clone();
    wrong.mesh.triangles[0].swap(0, 1);
    assert_eq!(
        prepare_export(&wrong, &grid, &request, &source)
            .unwrap_err()
            .code,
        "INVALID_GEOMETRY"
    );
    let mut wrong_request = request.clone();
    wrong_request.model_height_mm = 100.0;
    assert_eq!(
        prepare_export(&actual, &grid, &wrong_request, &source)
            .unwrap_err()
            .code,
        "INVALID_DIMENSIONS"
    );
}
