//! End-to-end private experimental preview behavior using ONLY generated
//! mathematical PMTiles, never the owner's real Interlaken bytes.
use getrandom::fill;
use map2model_contracts::ElevationGridV1;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
const ACK: &str = "I-ACCEPT-UNVERIFIED-TERRAIN-FOR-PRIVATE-LOCAL-EXPERIMENTATION";

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap()
}
fn s(p: &Path) -> &str { p.to_str().unwrap() }
fn sha(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn tmp() -> PathBuf {
    let mut nonce = [0_u8; 16];
    fill(&mut nonce).unwrap();
    std::env::temp_dir().join(format!(
        "m2m-experimental-{}-{}",
        std::process::id(),
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>(),
    ))
}
fn original(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let result = run(env!("CARGO_BIN_EXE_terrain_synthetic_fixture"), &[s(&root.join("source"))]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}
fn preview(root: &Path, variant: &str, sha: &str, consent: &str) -> (Output, PathBuf) {
    let file = root.join("source").join(format!("{variant}.pmtiles"));
    let sel = root.join("source").join("selection.json");
    let out = root.join(format!("{variant}-preview.json"));
    let response = run(
        env!("CARGO_BIN_EXE_terrain_offline"),
        &["experimental-local", s(&file), sha, s(&sel), "12", consent, s(&out)],
    );
    (response, out)
}
fn failed(out: &Output, prefix: &str) {
    assert!(!out.status.success(), "unexpected experimental mode success");
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with(prefix),
        "want {prefix}: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn experimental_local_candidate_raster_is_real_but_never_a_verified_v1_grid() {
    let root = tmp();
    original(&root);
    let result = (|| -> Result<(), String> {
        let archive = root.join("source").join("complete.pmtiles");
        let actual_sha = sha(&fs::read(&archive).unwrap());
        let (out, path) = preview(&root, "complete", &actual_sha, ACK);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let summary: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(summary["productionGenerationReady"], false);
        assert_eq!(summary["networkRequests"], 0);
        let v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["kind"], "ExperimentalLocalTerrainPreviewV1");
        assert_eq!(v["experimentalOnly"], true);
        assert_eq!(v["localPersonalUseOnly"], true);
        assert_eq!(v["sourceArchiveSha256"], actual_sha);
        assert_eq!(v["selectionId"], summary["selectionId"]);
        assert_eq!(v["sourceMetadataDeclaresTerrarium"], true);
        assert_eq!(v["width"], 6);
        assert_eq!(v["height"], 8);
        assert_eq!(v["closedBoundaryIndexedTileCount"], 4);
        assert_eq!(v["storageConvention"], "north-up-f64-row-major");
        assert_eq!(v["verticalDatumStatus"], "unknown");
        for name in [
            "encodingAuthenticated",
            "sourceLicenseVerified",
            "nativeResolutionVerified",
            "redistributionApproved",
            "productionGenerationReady",
        ] {
            assert_eq!(v[name], false);
        }
        let values = v["candidateTerrariumHeightsMeters"].as_array().unwrap();
        assert_eq!(values.len(), 48);
        assert!(values.iter().all(|n| n.is_f64()));
        assert_ne!(values.first(), values.last());
        assert!(serde_json::from_value::<ElevationGridV1>(v.clone()).is_err());
        assert_eq!(v["candidateHeightsSha256"], summary["candidateHeightsSha256"]);
        let (repeat, _) = preview(&root, "complete", &actual_sha, ACK);
        failed(&repeat, "INVALID_SOURCE"); // cannot overwrite output

        let (denied, denied_path) = preview(&root, "missing-southeast", &actual_sha, ACK);
        failed(&denied, "HASH_MISMATCH");
        assert!(!denied_path.exists());
        let (none, none_path) = preview(
            &root,
            "masked-southeast",
            &sha(&fs::read(root.join("source").join("masked-southeast.pmtiles")).unwrap()),
            "NO_CONSENT",
        );
        failed(&none, "EXPERIMENTAL_CONSENT_REQUIRED");
        assert!(!none_path.exists());
        Ok(())
    })();
    let _ = fs::remove_dir_all(&root);
    result.unwrap();
}

#[test]
fn experimental_missing_neighbor_and_alpha_no_data_stay_fail_closed() {
    let root = tmp();
    original(&root);
    let result = (|| -> Result<(), String> {
        for variant in ["missing-southeast", "masked-southeast"] {
            let file = root.join("source").join(format!("{variant}.pmtiles"));
            let (result, dest) = preview(&root, variant, &sha(&fs::read(&file).unwrap()), ACK);
            failed(&result, "DATA_MISSING");
            assert!(!dest.exists());
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&root);
    result.unwrap();
}

#[test]
fn attribution_only_metadata_is_allowed_experimentally_but_not_registered_or_verified() {
    let root = tmp();
    original(&root);
    let result = (|| -> Result<(), String> {
        let original = root.join("source").join("complete.pmtiles");
        let mut bytes = fs::read(&original).unwrap();
        let offset = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
        let length = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
        let replacement = br#"{"attribution":"Original synthetic only, unverified source metadata"}"#;
        bytes.splice(offset..offset + length, replacement.iter().copied());
        bytes[32..40].copy_from_slice(&(replacement.len() as u64).to_le_bytes());
        let tile_offset = offset + replacement.len();
        bytes[40..48].copy_from_slice(&(tile_offset as u64).to_le_bytes());
        bytes[56..64].copy_from_slice(&(tile_offset as u64).to_le_bytes());
        let test_path = root.join("source").join("attribution-only.pmtiles");
        fs::write(&test_path, &bytes).unwrap();
        let (result, dest) = preview(&root, "attribution-only", &sha(&bytes), ACK);
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let v: Value = serde_json::from_slice(&fs::read(&dest).unwrap()).unwrap();
        assert_eq!(v["sourceMetadataDeclaresTerrarium"], false);
        assert_eq!(
            v["sourceAttributionFromArchive"],
            "Original synthetic only, unverified source metadata"
        );
        assert_eq!(v["verticalDatumStatus"], "unknown");
        assert_eq!(v["encodingAuthenticated"], false);
        assert_eq!(v["productionGenerationReady"], false);
        assert_eq!(v["candidateTerrariumHeightsMeters"].as_array().unwrap().len(), 48);
        assert_eq!(v["networkRequests"], 0);

        // A conflicting explicit encoding must fail even with user opt-in.
        let mut bad_bytes = bytes;
        let bad_start = br#"{"attribution":"Original synthetic only, unverified source metadata"}"#;
        let at = bad_bytes.windows(bad_start.len()).position(|w| w == bad_start).unwrap();
        let bad = br#"{"encoding":"mapbox","attribution":"Synthetic"}"#;
        bad_bytes.splice(at..at + bad_start.len(), bad.iter().copied());
        bad_bytes[32..40].copy_from_slice(&(bad.len() as u64).to_le_bytes());
        let tile_start = at + bad.len();
        bad_bytes[40..48].copy_from_slice(&(tile_start as u64).to_le_bytes());
        bad_bytes[56..64].copy_from_slice(&(tile_start as u64).to_le_bytes());
        fs::write(root.join("source").join("conflicting.pmtiles"), &bad_bytes).unwrap();
        let (bad_output, bad_dest) = preview(&root, "conflicting", &sha(&bad_bytes), ACK);
        failed(&bad_output, "INVALID_SOURCE");
        assert!(!bad_dest.exists());
        Ok(())
    })();
    let _ = fs::remove_dir_all(&root);
    result.unwrap();
}
