//! Opt-in test for the *privately* acquired real Interlaken PMTiles sample.
//! The source archive is NEVER present in repository fixtures or GitHub CI.
//! CI reports SKIPPED unless a local operator passes the real file path.
//! This is native FORMAT inspection only, not a source license/datum assertion.
use std::path::Path;
use std::process::Command;

const SHA: &str = "75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0";

fn inspect(file: &Path, tile: Option<(u8, u32, u32)>) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_terrain_offline"));
    cmd.arg("inspect").arg(file);
    if let Some((z, x, y)) = tile {
        cmd.args([z.to_string(), x.to_string(), y.to_string()]);
    }
    // The Rust executable is inherently local-only; additionally configure
    // failing HTTP proxies as a defense in depth, not as network-proof evidence.
    let output = cmd
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .map_err(|err| format!("native Rust binary cannot launch: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "native Rust inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("native inspector output is not JSON: {err}"))
}

#[test]
fn real_private_interlaken_offline_rust_format_and_512px_numeric_golden() {
    let Some(file) = std::env::var_os("MAP2MODEL_PRIVATE_INTERLAKEN_PM_TILES") else {
        eprintln!(
            "SKIPPED private real-source Rust smoke: set MAP2MODEL_PRIVATE_INTERLAKEN_PM_TILES to the private corrected Interlaken extract; GitHub CI is synthetic-only"
        );
        return;
    };
    let file = Path::new(&file);
    let report = inspect(file, None).expect("private archive must pass actual Rust inspection");
    // Exact content-addressed pin prevents silently using the earlier malformed
    // maxZoom=18 archive (which has a different SHA despite identical DEM tiles).
    assert_eq!(report["archiveSha256"], SHA);
    assert_eq!(report["archiveBytes"], 3_127_295);
    assert_eq!(report["headerMinZoom"], 13);
    assert_eq!(report["headerMaxZoom"], 17);
    assert_eq!(report["actualIndexedMinZoom"], 13);
    assert_eq!(report["actualIndexedMaxZoom"], 17);
    assert_eq!(report["headerZoomRangeConsistent"], true);
    assert_eq!(report["indexedTileCount"], 40);
    assert_eq!(report["indexedTilesByZoom"]["13"], 1);
    assert_eq!(report["indexedTilesByZoom"]["14"], 2);
    assert_eq!(report["indexedTilesByZoom"]["15"], 4);
    assert_eq!(report["indexedTilesByZoom"]["16"], 9);
    assert_eq!(report["indexedTilesByZoom"]["17"], 24);
    assert_eq!(report["internalCompression"], "gzip");
    assert_eq!(report["tileCompression"], "none");
    assert_eq!(report["metadataExplicitlyDeclaresTerrarium"], false);
    assert!(report["encodingDeclaredInMetadata"].is_null());
    assert_eq!(report["sourceLicenseVerified"], false);
    assert_eq!(report["verticalDatumVerified"], false);
    assert_eq!(report["nativeResolutionVerified"], false);
    assert!(report["verticalDatumId"].is_null());
    assert_eq!(report["productionGenerationReady"], false);

    // Independent reference: the privately analyzed archive's raw WebP RGB
    // bytes were separately decoded outside Rust. These exact binary fractions
    // are Terrarium *candidate* values; no absolute vertical datum is claimed.
    // (z, x, y, north-west, center, south-east) for one actual tile per zoom.
    let golden = [
        (13_u8, 4275_u32, 2891_u32, 1495., 563.75, 829.25),
        (14, 8551, 5783, 563.75, 578.875, 831.),
        (15, 17102, 11566, 563.75, 565.25, 578.375),
        (16, 34204, 23133, 565.4375, 568.03125, 567.21875),
        (17, 68408, 46266, 565.453125, 568.453125, 568.),
    ];
    for (z, x, y, nw, center, se) in golden {
        let tile = inspect(file, Some((z, x, y))).expect("actual directory tile must decode");
        assert_eq!(tile["archiveSha256"], SHA);
        assert_eq!(tile["tileSample"]["verifiedDirectoryEntry"], true);
        assert_eq!(tile["tileSample"]["webpWidth"], 512);
        assert_eq!(tile["tileSample"]["webpHeight"], 512);
        assert_eq!(tile["tileSample"]["maskedPixelCount"], 0);
        assert_eq!(tile["tileSample"]["elevationNorthWestMeters"], nw);
        assert_eq!(tile["tileSample"]["elevationCenterMeters"], center);
        assert_eq!(tile["tileSample"]["elevationSouthEastMeters"], se);
        assert_eq!(tile["metadataExplicitlyDeclaresTerrarium"], false);
        assert_eq!(tile["productionGenerationReady"], false);
    }

    // A real 13..17 extract MUST NOT silently fetch or synthesize z18 data.
    let missing = Command::new(env!("CARGO_BIN_EXE_terrain_offline"))
        .args(["inspect"])
        .arg(file)
        .args(["18", "136816", "92532"])
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .expect("private CLI");
    assert!(!missing.status.success(), "missing z18 must fail closed");
    assert!(String::from_utf8_lossy(&missing.stderr).contains("DATA_MISSING"));
    eprintln!(
        "REAL PRIVATE NATIVE RUST FORMAT PASS: sourceSha256={SHA}, tiles=40, verified 512px z13..17 RGB/Terrarium candidate pixels, z18 DATA_MISSING; datum/license/production still NOT verified"
    );
}
