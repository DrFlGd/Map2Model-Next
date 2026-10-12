//! Genuine original-generated 512px PMTiles -> registered grid -> mesh,
//! with two-direction XYZ seam coverage and fail-closed source policies.
use getrandom::fill;
use map2model_contracts::{
    canonical, validation, ElevationGridV1, SelectionV1, TerrainBuildRequestV1,
};
use map2model_mesh::build_mesh;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    let mut salt = [0_u8; 16];
    fill(&mut salt).unwrap();
    std::env::temp_dir().join(format!(
        "m2m-original-pmtiles-{}-{}",
        std::process::id(),
        salt.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}
fn path(p: &Path) -> &str {
    p.to_str().expect("OS test temporary root supports UTF-8")
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn run(cmd: &str, args: &[&str]) -> Output {
    Command::new(cmd)
        .args(args)
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .expect("synthetic fixture subprocess starts")
}
fn fixture_command(args: &[&str]) -> Output {
    run(env!("CARGO_BIN_EXE_terrain_synthetic_fixture"), args)
}
fn offline(args: &[&str]) -> Output {
    run(env!("CARGO_BIN_EXE_terrain_offline"), args)
}
fn check_success(out: &Output, where_: &str) {
    assert!(
        out.status.success(),
        "{where_}: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn failure(out: &Output, code: &str) {
    assert!(!out.status.success(), "unexpected success: {code}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with(code), "expected {code}; got {stderr}");
}
fn setup() -> (PathBuf, Value) {
    let root = root();
    let generated = root.join("input");
    fs::create_dir_all(&root).unwrap();
    let result = fixture_command(&[path(&generated)]);
    check_success(&result, "fixture generation");
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["networkRequests"], 0);
    assert_eq!(report["productionRealSourceReady"], false);
    assert_eq!(report["sampleZoom"], 12);
    (root, report)
}
fn decode(
    root: &Path,
    variant: &str,
    input: &Path,
    declaration: &Path,
    selection: &Path,
) -> (Output, PathBuf) {
    let dest = root.join(format!("{variant}.grid.json"));
    let registry = root.join(format!("{variant}.registry"));
    let out = offline(&[
        "decode",
        path(input),
        path(&registry),
        path(declaration),
        path(selection),
        "12",
        path(&dest),
    ]);
    (out, dest)
}
fn variant_paths(root: &Path, kind: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = root.join("input");
    (
        base.join(format!("{kind}.pmtiles")),
        base.join(format!("{kind}.source.json")),
        base.join("selection.json"),
    )
}
#[test]
fn two_axis_seam_original_pmtiles_to_reproducible_grid_and_watertight_mesh() {
    let (base, report) = setup();
    let run = || -> Result<(), String> {
        let (archive, declaration, selection_file) = variant_paths(&base, "complete");
        let archive_bytes = fs::read(&archive).unwrap();
        let sha_actual = sha(&archive_bytes);
        assert_eq!(report["variants"][0]["archiveSha256"], sha_actual);
        let source: Value = serde_json::from_slice(&fs::read(&declaration).unwrap()).unwrap();
        assert_eq!(source["expectedArchiveSha256"], sha_actual);
        assert_eq!(source["verticalDatumId"], "SYNTHETIC-RELATIVE-METERS");
        assert_eq!(source["licenseIdentifier"], "CC0-1.0");
        assert_eq!(source["verticalDatumStatus"], "known");
        let s: SelectionV1 = serde_json::from_slice(&fs::read(&selection_file).unwrap()).unwrap();
        validation::validate_selection(&s)?;
        assert_eq!(s.bbox, [-84.024, 39.7067, -84.023, 39.7077]);
        assert_eq!(s.selection_id, report["selectionId"]);

        let inspect = offline(&["inspect", path(&archive), "12", "1091", "1554"]);
        check_success(&inspect, "original synthetic 512px PMTiles inspect");
        let parsed: Value = serde_json::from_slice(&inspect.stdout).unwrap();
        assert_eq!(parsed["archiveSha256"], sha_actual);
        assert_eq!(parsed["indexedTileCount"], 4);
        assert_eq!(parsed["indexedTilesByZoom"]["12"], 4);
        assert_eq!(parsed["metadataExplicitlyDeclaresTerrarium"], true);
        assert_eq!(parsed["tileSample"]["webpWidth"], 512);
        assert_eq!(parsed["tileSample"]["webpHeight"], 512);
        assert_eq!(parsed["productionGenerationReady"], false);

        let (result, grid_path) =
            decode(&base, "complete", &archive, &declaration, &selection_file);
        check_success(&result, "complete two-axis seam source decode");
        let summary: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(summary["networkRequests"], 0);
        assert_eq!(summary["sourceArchiveSha256"], sha_actual);
        assert_eq!(summary["productionGenerationReady"], false);
        let grid: ElevationGridV1 = serde_json::from_slice(&fs::read(&grid_path).unwrap()).unwrap();
        validation::validate_grid(&grid)?;
        assert_eq!(grid.kind, "ElevationGridV1");
        assert_eq!(grid.selection_id, s.selection_id);
        assert_eq!(grid.grid_sha256, summary["gridSha256"]);
        assert_eq!(grid.anchor_wgs84, [s.bbox[0], s.bbox[1]]);
        assert_eq!(grid.vertical_datum_id, "SYNTHETIC-RELATIVE-METERS");
        assert_eq!(grid.storage_convention, "north-up-f64-row-major");
        assert!(grid.width >= 4 && grid.height >= 4);
        assert!(grid.valid_mask.as_ref().unwrap().iter().all(|v| *v));
        assert!(grid
            .samples_meters
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
            .any(|v| *v < 0.));
        assert!(grid
            .samples_meters
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
            .any(|v| *v > 0.));
        let x_extent = grid.dx_meters * (grid.width - 1) as f64;
        let y_extent = grid.dy_meters * (grid.height - 1) as f64;
        let expected_metric =
            validation::project_wgs84([s.bbox[0], s.bbox[1]], [s.bbox[2], s.bbox[3]])?;
        assert!((x_extent - expected_metric[0]).abs() < 1e-8);
        assert!((y_extent - expected_metric[1]).abs() < 1e-8);
        assert_eq!(grid.grid_sha256, canonical::grid_digest(&grid)?);
        assert_eq!(grid.grid_id, format!("grid-{}", &grid.grid_sha256[..32]));

        let retry = offline(&[
            "decode",
            path(&archive),
            path(&base.join("again-registry")),
            path(&declaration),
            path(&selection_file),
            "12",
            path(&base.join("again.grid.json")),
        ]);
        check_success(&retry, "second independent decode");
        let again: ElevationGridV1 =
            serde_json::from_slice(&fs::read(base.join("again.grid.json")).unwrap()).unwrap();
        assert_eq!(grid.grid_sha256, again.grid_sha256);
        assert_eq!(grid.samples_meters, again.samples_meters);
        assert_eq!(grid.valid_mask, again.valid_mask);

        let request = TerrainBuildRequestV1 {
            schema_version: "1.0.0".into(),
            kind: "TerrainBuildRequestV1".into(),
            job_id: "job-original-synthetic-pmtiles".into(),
            correlation_id: "corr-original-synthetic-pmtiles".into(),
            selection_id: grid.selection_id.clone(),
            grid_sha256: grid.grid_sha256.clone(),
            clip_rect_meters: [0., 0., x_extent, y_extent],
            model_width_mm: 120.,
            model_height_mm: 120. * y_extent / x_extent,
            base_thickness_mm: 4.,
            minimum_feature_thickness_mm: 1.,
            vertical_exaggeration: 1.,
            elevation_zero_meters: -150.,
            engine_version: "map2model-mesh/0.0.1".into(),
            export_token: "m2m-export:artificial-four-tiles".into(),
        };
        let mesh = build_mesh(&grid, &request).map_err(|err| err.to_string())?;
        assert!(mesh.geometry.closed);
        assert!(mesh.geometry.oriented);
        assert!(mesh.geometry.triangles > 0);
        assert!(mesh.geometry.vertices > 0);
        assert_eq!(mesh.geometry.non_manifold_edges, 0);
        assert_eq!(mesh.geometry.component_count, 1);
        eprintln!(
            "SYNTHETIC_PM_TILES_HANDOFF sourceSha256={} manifestSha256={} gridSha256={} meshSha256={} selectionId={} grid={}x{} meshVertices={} meshTriangles={} networkRequests=0",
            sha_actual, grid.source_manifest_sha256, grid.grid_sha256, mesh.canonical_geometry_sha256,
            grid.selection_id, grid.width, grid.height, mesh.geometry.vertices, mesh.geometry.triangles
        );
        Ok(())
    };
    let result = run();
    let _ = fs::remove_dir_all(&base);
    result.unwrap();
}
#[test]
fn original_synthetic_missing_nodata_hash_rights_datum_budget_and_malformed_fail_closed() {
    let (base, _) = setup();
    let run = || -> Result<(), String> {
        for variant in ["missing-southeast", "masked-southeast"] {
            let (archive, declaration, selection) = variant_paths(&base, variant);
            let (out, grid_path) = decode(&base, variant, &archive, &declaration, &selection);
            failure(&out, "DATA_MISSING");
            assert!(!grid_path.exists());
        }
        let (archive, declaration, selection_file) = variant_paths(&base, "complete");
        let original: Value = serde_json::from_slice(&fs::read(&declaration).unwrap()).unwrap();
        for (label, key, replacement, wanted) in [
            (
                "bad-sha",
                "expectedArchiveSha256",
                json!("00".repeat(32)),
                "HASH_MISMATCH",
            ),
            (
                "unknown-datum",
                "verticalDatumStatus",
                json!("unknown"),
                "UNSUPPORTED_DATUM",
            ),
            (
                "absent-license",
                "licenseIdentifier",
                json!(""),
                "INVALID_SOURCE",
            ),
            (
                "no-resolution",
                "nativeResolutionSource",
                json!(""),
                "INVALID_SOURCE",
            ),
        ] {
            let mut altered = original.clone();
            altered[key] = replacement;
            let altered_source = base.join(format!("{label}.source.json"));
            fs::write(&altered_source, serde_json::to_vec(&altered).unwrap()).unwrap();
            let (out, dest) = decode(&base, label, &archive, &altered_source, &selection_file);
            failure(&out, wanted);
            assert!(!dest.exists());
        }
        let selection: SelectionV1 =
            serde_json::from_slice(&fs::read(&selection_file).unwrap()).unwrap();
        let mut outside = selection.clone();
        outside.bbox = [-84.03, 39.70, -84.029, 39.701]; // header envelope but tile beyond the 2x2 index?
        outside.selection_id = format!("sel-{}", &canonical::selection_digest(&outside)?[..32]);
        let outside_file = base.join("outside.selection.json");
        fs::write(&outside_file, serde_json::to_vec(&outside).unwrap()).unwrap();
        // Outside selection is inside the northwest x1091/y1555 tile, so
        // deliberately choose missing tile *to the west of x1091* instead.
        outside.bbox = [-84.112, 39.7067, -84.111, 39.7077];
        outside.selection_id = format!("sel-{}", &canonical::selection_digest(&outside)?[..32]);
        fs::write(&outside_file, serde_json::to_vec(&outside).unwrap()).unwrap();
        let (out, dest) = decode(
            &base,
            "outside-index",
            &archive,
            &declaration,
            &outside_file,
        );
        assert!(
            !out.status.success(),
            "unavailable geographic coverage cannot decode"
        );
        assert!(!dest.exists());

        let mut budget = selection.clone();
        budget.terrain_profile.max_samples = 4;
        budget.selection_id = format!("sel-{}", &canonical::selection_digest(&budget)?[..32]);
        let budget_file = base.join("budget.selection.json");
        fs::write(&budget_file, serde_json::to_vec(&budget).unwrap()).unwrap();
        let (out, dest) = decode(&base, "overbudget", &archive, &declaration, &budget_file);
        failure(&out, "OUT_OF_MEMORY");
        assert!(!dest.exists());

        let corrupt_path = base.join("damaged.pmtiles");
        let mut corrupt = fs::read(&archive).unwrap();
        corrupt[0] = 0x00;
        fs::write(&corrupt_path, corrupt).unwrap();
        let (out, dest) = decode(
            &base,
            "damaged",
            &corrupt_path,
            &declaration,
            &selection_file,
        );
        assert!(!out.status.success());
        assert!(!dest.exists());
        // Exact SHA must be checked before structurally interpreting forged input.
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("HASH_MISMATCH"));
        let inspected = offline(&["inspect", path(&corrupt_path)]);
        failure(&inspected, "INVALID_SOURCE");

        // The generator is explicit create-new; it must never overwrite private files.
        let no_overwrite = fixture_command(&[path(&base.join("input"))]);
        failure(&no_overwrite, "INVALID_SOURCE");
        Ok(())
    };
    let result = run();
    let _ = fs::remove_dir_all(&base);
    result.unwrap();
}
