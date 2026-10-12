//! Original-generated PMTiles visual coverage checks: no real Mapterhorn data.
//! The classification is intentionally INDEX-ONLY; alpha/nodata is decoded at
//! generation time, and rights/datum remain unverified.
use getrandom::fill;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn temp_root() -> PathBuf {
    let mut nonce = [0_u8; 16];
    fill(&mut nonce).unwrap();
    std::env::temp_dir().join(format!(
        "m2m-local-coverage-{}-{}",
        std::process::id(),
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
fn p(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn subprocess(bin: &str, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap()
}
fn setup() -> PathBuf {
    let dir = temp_root();
    fs::create_dir(&dir).unwrap();
    let gen = subprocess(
        env!("CARGO_BIN_EXE_terrain_synthetic_fixture"),
        &[p(&dir.join("generated"))],
    );
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    dir
}
fn cover(file: &Path, sha: Option<&str>, bbox: Option<[&str; 4]>, zoom: Option<&str>) -> Output {
    let mut args = vec!["coverage", p(file)];
    if let (Some(digest), Some(b), Some(z)) = (sha, bbox, zoom) {
        args.push(digest);
        args.extend(b);
        args.push(z);
    }
    subprocess(env!("CARGO_BIN_EXE_terrain_offline"), &args)
}
fn successful(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn rejected(out: Output, error: &str) {
    assert!(!out.status.success(), "unexpected coverage success");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with(error), "expected {error}, got {err}");
}
const SEAM_BBOX: [&str; 4] = ["-84.024", "39.7067", "-84.023", "39.7077"];
const EDGE_PARTIAL: [&str; 4] = ["-83.936", "39.7068", "-83.9352", "39.7072"];
const OUTSIDE: [&str; 4] = ["-83.934", "39.7068", "-83.9335", "39.7072"];
const SE_TILE: [&str; 4] = ["-84", "39.68", "-83.99", "39.69"];

#[test]
fn local_file_picker_yields_maplibre_geojson_bbox_and_unverified_source_identity() {
    let root = setup();
    let file = root.join("generated/complete.pmtiles");
    let source_sha = sha256(&fs::read(&file).unwrap());
    let v = successful(cover(&file, None, None, None));
    assert_eq!(v["kind"], "LocalPMTilesCoveragePreviewV1");
    assert_eq!(v["sourceArchiveSha256"], source_sha);
    assert_eq!(
        v["declaredCoverageBboxWgs84"],
        serde_json::json!([-84.11132, 39.63954, -83.93555, 39.77476])
    );
    assert_eq!(v["coverageGeoJson"]["type"], "Feature");
    assert_eq!(v["coverageGeoJson"]["geometry"]["type"], "Polygon");
    let points = v["coverageGeoJson"]["geometry"]["coordinates"][0]
        .as_array()
        .unwrap();
    assert_eq!(points.len(), 5);
    assert_eq!(points.first(), points.last());
    assert_eq!(v["coverageGeoJson"]["properties"]["unverified"], true);
    assert_eq!(v["headerMinZoom"], 12);
    assert_eq!(v["headerMaxZoom"], 12);
    assert_eq!(v["indexedTileCount"], 4);
    assert_eq!(v["indexedTilesByZoom"]["12"], 4);
    assert_eq!(v["selection"], Value::Null);
    assert_eq!(v["sourceLicenseVerified"], false);
    assert_eq!(v["verticalDatumStatus"], "unknown");
    assert_eq!(v["nativeResolutionVerified"], false);
    assert_eq!(v["productionGenerationReady"], false);
    assert_eq!(v["networkRequests"], 0);
    assert!(v.get("candidateTerrariumHeightsMeters").is_none());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn index_and_header_distinguish_inside_partial_outside_for_dragged_rectangle() {
    let root = setup();
    let dir = root.join("generated");
    let file = dir.join("complete.pmtiles");
    let digest = sha256(&fs::read(&file).unwrap());
    let v = successful(cover(&file, Some(&digest), Some(SEAM_BBOX), Some("12")));
    let s = &v["selection"];
    assert_eq!(s["status"], "inside");
    assert_eq!(s["requiredClosedBoundaryTiles"], 4);
    assert_eq!(s["indexedRequiredTiles"], 4);
    assert_eq!(s["missingRequiredTiles"], 0);
    assert_eq!(s["allRequiredTilesIndexed"], true);
    assert_eq!(s["insideDeclaredBbox"], true);
    assert_eq!(s["zoomAvailable"], true);
    assert_eq!(s["requestedZoom"], 12);

    // Some geometrical overlap with the archive envelope, but half the
    // selected XYZ tiles are not indexed. Never report "inside".
    let partial = successful(cover(&file, Some(&digest), Some(EDGE_PARTIAL), Some("12")));
    assert_eq!(partial["selection"]["status"], "partial");
    assert_eq!(partial["selection"]["intersectsDeclaredBbox"], true);
    assert_eq!(partial["selection"]["insideDeclaredBbox"], false);
    assert_eq!(partial["selection"]["indexedRequiredTiles"], 2);
    assert_eq!(partial["selection"]["missingRequiredTiles"], 2);

    let outside = successful(cover(&file, Some(&digest), Some(OUTSIDE), Some("12")));
    assert_eq!(outside["selection"]["status"], "outside");
    assert_eq!(outside["selection"]["intersectsDeclaredBbox"], false);
    assert_eq!(outside["selection"]["indexedRequiredTiles"], 0);
    assert_eq!(
        outside["selection"]["intersectionWithDeclaredBboxWgs84"],
        Value::Null
    );

    // A header bbox alone would falsely classify this as inside. The real
    // indexed tile set has an absent southeast source tile.
    let missing = dir.join("missing-southeast.pmtiles");
    let missing_sha = sha256(&fs::read(&missing).unwrap());
    let partly = successful(cover(
        &missing,
        Some(&missing_sha),
        Some(SEAM_BBOX),
        Some("12"),
    ));
    assert_eq!(partly["selection"]["status"], "partial");
    assert_eq!(partly["selection"]["insideDeclaredBbox"], true);
    assert_eq!(partly["selection"]["requiredClosedBoundaryTiles"], 4);
    assert_eq!(partly["selection"]["indexedRequiredTiles"], 3);

    // A bbox strictly inside an absent tile is OUTSIDE indexed coverage.
    let absent_only = successful(cover(
        &missing,
        Some(&missing_sha),
        Some(SE_TILE),
        Some("12"),
    ));
    assert_eq!(absent_only["selection"]["status"], "outside");
    assert_eq!(absent_only["selection"]["intersectsDeclaredBbox"], true);
    assert_eq!(absent_only["selection"]["indexedRequiredTiles"], 0);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn index_only_inside_is_not_promise_of_pixel_or_export_readiness() {
    let root = setup();
    let file = root.join("generated/masked-southeast.pmtiles");
    let digest = sha256(&fs::read(&file).unwrap());
    let v = successful(cover(&file, Some(&digest), Some(SEAM_BBOX), Some("12")));
    assert_eq!(v["selection"]["status"], "inside");
    assert_eq!(
        v["selection"]["classificationScope"],
        "declared-header-envelope-and-real-index-only; alpha/nodata/height samples NOT verified"
    );
    assert_eq!(v["productionGenerationReady"], false);
    assert_eq!(v["sourceLicenseVerified"], false);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn coverage_query_rejects_mutated_source_nonfinite_invalid_bbox_oversize_and_bad_zoom() {
    let root = setup();
    let file = root.join("generated/complete.pmtiles");
    let digest = sha256(&fs::read(&file).unwrap());
    rejected(
        cover(&file, Some(&"0".repeat(64)), Some(SEAM_BBOX), Some("12")),
        "HASH_MISMATCH",
    );
    rejected(
        cover(
            &file,
            Some(&digest),
            Some(["NaN", "39", "-84", "40"]),
            Some("12"),
        ),
        "INVALID_SELECTION",
    );
    rejected(
        cover(
            &file,
            Some(&digest),
            Some(["-83", "39.70", "-84", "39.71"]),
            Some("12"),
        ),
        "INVALID_SELECTION",
    );
    rejected(
        cover(&file, Some(&digest), Some(SEAM_BBOX), Some("23")),
        "INVALID_SELECTION",
    );
    // Valid WGS84 but enormous z22 scan is not silently labelled partial.
    rejected(
        cover(&file, Some(&digest), Some(SEAM_BBOX), Some("22")),
        "OUT_OF_MEMORY",
    );

    let wrong_zoom = successful(cover(&file, Some(&digest), Some(SEAM_BBOX), Some("13")));
    assert_eq!(wrong_zoom["selection"]["zoomAvailable"], false);
    assert_eq!(wrong_zoom["selection"]["status"], "outside");
    assert_eq!(wrong_zoom["selection"]["indexedRequiredTiles"], 0);
    assert_eq!(wrong_zoom["selection"]["missingRequiredTiles"], 4);

    let mut bad = fs::read(&file).unwrap();
    bad[0] = 0;
    let corrupt = root.join("invalid.pmtiles");
    fs::write(&corrupt, bad).unwrap();
    rejected(cover(&corrupt, None, None, None), "INVALID_SOURCE");
    fs::remove_dir_all(root).unwrap();
}
