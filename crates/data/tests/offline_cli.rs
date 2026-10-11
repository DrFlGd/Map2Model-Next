//! Native executable smoke: no Electron, no downloads or third-party source data.
use image::{codecs::webp::WebPEncoder, ExtendedColorType};
use map2model_contracts::{validation, ElevationGridV1, SelectionV1, TerrainBuildRequestV1};
use map2model_mesh::build_mesh;
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn u64le(header: &mut [u8], offset: usize, value: u64) {
    header[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn i32le(header: &mut [u8], offset: usize, value: i32) {
    header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn varint(mut n: u64, out: &mut Vec<u8>) {
    while n >= 128 {
        out.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
fn synthetic_fixture() -> Vec<u8> {
    // Completely original, all-zero-meter Terrarium pixel source, RGB(128,0,0).
    let mut pixels = vec![0_u8; 512 * 512 * 3];
    for rgb in pixels.chunks_exact_mut(3) {
        rgb.copy_from_slice(&[128, 0, 0]);
    }
    let mut webp = Vec::new();
    WebPEncoder::new_lossless(&mut webp)
        .encode(&pixels, 512, 512, ExtendedColorType::Rgb8)
        .unwrap();
    // z0 tile 0: root directory with one actual tile ID, run=1, tile length and offset=0.
    let mut dir = vec![1, 0, 1];
    varint(webp.len() as u64, &mut dir);
    dir.push(1);
    let meta = br#"{"encoding":"terrarium","name":"Original artificial binary test data"}"#;
    let mut archive = vec![0_u8; 127];
    archive[..7].copy_from_slice(b"PMTiles");
    archive[7] = 3;
    u64le(&mut archive, 8, 127);
    u64le(&mut archive, 16, dir.len() as u64);
    u64le(&mut archive, 24, (127 + dir.len()) as u64);
    u64le(&mut archive, 32, meta.len() as u64);
    u64le(&mut archive, 40, (127 + dir.len() + meta.len()) as u64);
    u64le(&mut archive, 48, 0);
    u64le(&mut archive, 56, (127 + dir.len() + meta.len()) as u64);
    u64le(&mut archive, 64, webp.len() as u64);
    u64le(&mut archive, 72, 1);
    u64le(&mut archive, 80, 1);
    u64le(&mut archive, 88, 1);
    archive[96] = 1;
    archive[97] = 1;
    archive[98] = 1;
    archive[99] = 4;
    archive[100] = 0;
    archive[101] = 0;
    i32le(&mut archive, 102, -840900000);
    i32le(&mut archive, 106, 397400000);
    i32le(&mut archive, 110, -840800000);
    i32le(&mut archive, 114, 397500000);
    archive.extend(dir);
    archive.extend(meta);
    archive.extend(webp);
    archive
}
fn exec(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_terrain_offline"))
        .args(args)
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap()
}
fn file_str(path: &Path) -> &str {
    path.to_str().expect("test path UTF-8")
}
fn private_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "m2m-offline-binary-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn binary_offline_source_inspection_and_grid_to_frozen_mesh_consumer() {
    let root = private_root();
    fs::create_dir_all(&root).unwrap();
    let result = (|| -> Result<(), String> {
        let archive = root.join("artificial.pmtiles");
        fs::write(&archive, synthetic_fixture()).unwrap();
        let output = exec(&["inspect", file_str(&archive), "0", "0", "0"]);
        assert!(
            output.status.success(),
            "inspect: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["productionGenerationReady"], false);
        assert_eq!(report["verticalDatumId"], serde_json::Value::Null);
        assert_eq!(report["sourceLicenseVerified"], false);
        assert_eq!(report["tileSample"]["webpWidth"], 512);
        assert_eq!(report["tileSample"]["webpHeight"], 512);
        assert_eq!(report["tileSample"]["elevationCenterMeters"], 0.0);
        assert_eq!(report["tileSample"]["maskedPixelCount"], 0);
        assert_eq!(report["indexedTileCount"], 1);
        assert_eq!(report["archiveSha256"].as_str().unwrap().len(), 64);

        // This declaration is TRUE ONLY for the original artificial pixels.
        // An authentic unknown Mapterhorn datum must never use this synthetic ID.
        let declaration = root.join("synthetic-provenance.json");
        fs::write(
            &declaration,
            serde_json::to_vec_pretty(&serde_json::json!({
                "sourceName":"Original artificial Terrarium pixel test",
                "releaseId":"artificial-zero-meters-v1",
                "archiveId":"synthetic-z0-only",
                "expectedArchiveSha256":report["archiveSha256"],
                "verticalDatumId":"SYNTHETIC-RELATIVE-METERS",
                "verticalDatumStatus":"known",
                "licenseIdentifier":"CC0-1.0",
                "attribution":"Original artificial data, not Mapterhorn terrain",
                "nominalGroundSampleMeters":12.0,
                "nativeResolutionSource":"Generated pixels for tests",
                "uncertaintyMeters":0.0,
                "decoderVersion":"map2model-data-headless/0.0.1"
            }))
            .unwrap(),
        )
        .unwrap();
        let selection = root.join("selection.json");
        fs::write(
            &selection,
            include_bytes!("../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"),
        )
        .unwrap();
        let grid_file = root.join("decoded-grid.json");
        let registry = root.join("native-private-registry");
        let args = [
            "decode",
            file_str(&archive),
            file_str(&registry),
            file_str(&declaration),
            file_str(&selection),
            "0",
            file_str(&grid_file),
        ];
        let output = exec(&args);
        assert!(
            output.status.success(),
            "decode: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(summary["productionGenerationReady"], false);
        assert_eq!(summary["licenseAndDatumIndependentlyVerified"], false);
        assert_eq!(summary["networkRequests"], 0);
        assert_eq!(summary["sourceArchiveSha256"], report["archiveSha256"]);
        let grid: ElevationGridV1 = serde_json::from_slice(&fs::read(&grid_file).unwrap()).unwrap();
        let selected: SelectionV1 = serde_json::from_slice(&fs::read(&selection).unwrap()).unwrap();
        validation::validate_grid(&grid)?;
        assert_eq!(grid.selection_id, selected.selection_id);
        assert_eq!(summary["gridSha256"], grid.grid_sha256);
        assert_eq!(grid.anchor_wgs84, [selected.bbox[0], selected.bbox[1]]);
        assert!(grid
            .samples_meters
            .as_ref()
            .unwrap()
            .iter()
            .all(|v| *v == Some(0.)));
        assert!(grid.valid_mask.as_ref().unwrap().iter().all(|v| *v));
        assert_eq!(grid.storage_convention, "north-up-f64-row-major");

        // Generation safety: explicit call from the user must never clobber an existing grid.
        assert!(!exec(&args).status.success());
        let pinned_declaration_bytes = fs::read(&declaration).unwrap();
        let mut unpinned: serde_json::Value =
            serde_json::from_slice(&pinned_declaration_bytes).unwrap();
        unpinned.as_object_mut().unwrap().remove("expectedArchiveSha256");
        fs::write(&declaration, serde_json::to_vec(&unpinned).unwrap()).unwrap();
        let unpinned_output = root.join("unpinned-grid.json");
        assert!(!exec(&[
            "decode", file_str(&archive), file_str(&registry),
            file_str(&declaration), file_str(&selection), "0", file_str(&unpinned_output)
        ]).status.success());
        assert!(!unpinned_output.exists());
        let mut unknown =
            serde_json::from_slice::<serde_json::Value>(&pinned_declaration_bytes).unwrap();
        unknown["verticalDatumStatus"] = serde_json::json!("unknown");
        fs::write(&declaration, serde_json::to_vec(&unknown).unwrap()).unwrap();
        let reject = root.join("blocked-grid.json");
        assert!(!exec(&[
            "decode",
            file_str(&archive),
            file_str(&registry),
            file_str(&declaration),
            file_str(&selection),
            "0",
            file_str(&reject)
        ])
        .status
        .success());
        assert!(!reject.exists());

        let x1 = grid.min_east_meters + (grid.width as f64 - 1.) * grid.dx_meters;
        let y0 = grid.max_north_meters - (grid.height as f64 - 1.) * grid.dy_meters;
        let y1 = grid.max_north_meters;
        let print_width = 80.0;
        let request = TerrainBuildRequestV1 {
            schema_version: "1.0.0".into(),
            kind: "TerrainBuildRequestV1".into(),
            job_id: "job-artificial-offline-cli".into(),
            correlation_id: "corr-artificial-offline-cli".into(),
            selection_id: grid.selection_id.clone(),
            grid_sha256: grid.grid_sha256.clone(),
            clip_rect_meters: [0., y0, x1, y1],
            model_width_mm: print_width,
            model_height_mm: print_width * (y1 - y0) / x1,
            base_thickness_mm: 4.,
            minimum_feature_thickness_mm: 1.,
            vertical_exaggeration: 1.,
            elevation_zero_meters: -100.,
            engine_version: "map2model-mesh/0.0.1".into(),
            export_token: "m2m-export:original-artificial".into(),
        };
        let mesh = build_mesh(&grid, &request).map_err(|e| e.to_string())?;
        assert!(mesh.geometry.closed && mesh.geometry.oriented);
        assert!(mesh.geometry.triangles > 0);
        assert_eq!(mesh.canonical_geometry_sha256.len(), 64);
        Ok(())
    })();
    let _ = fs::remove_dir_all(root);
    result.unwrap();
}
