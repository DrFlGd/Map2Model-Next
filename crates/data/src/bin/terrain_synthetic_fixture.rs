//! Fully original, reproducible, no-network 512px WebP PMTiles source generator.
//! Build/test handoff ONLY: synthetic data is never an actual terrain datum.
//! Usage: cargo run -p map2model-data --bin terrain_synthetic_fixture -- <new-dir>
use image::{codecs::webp::WebPEncoder, ExtendedColorType};
use map2model_contracts::{canonical, validation, SelectionV1, TerrainProfileV1};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const Z: u8 = 12;
const WIDTH: usize = 512;
// Original, known purely test-space terrain function in signed meters:
// height(global_col,global_row) = -96 + global_col/8 + global_row/16.
const SOURCE: &str = "Map2Model original synthetic z12 raster; NOT Mapterhorn";
const KNOWN_SYNTHETIC_DATUM: &str = "SYNTHETIC-RELATIVE-METERS";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn u64le(out: &mut [u8], offset: usize, n: u64) {
    out[offset..offset + 8].copy_from_slice(&n.to_le_bytes());
}
fn i32le(out: &mut [u8], offset: usize, n: i32) {
    out[offset..offset + 4].copy_from_slice(&n.to_le_bytes());
}
fn varint(mut n: u64, out: &mut Vec<u8>) {
    while n >= 128 {
        out.push((n as u8 & 0x7f) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}
fn hilbert_id(z: u8, x: u32, y: u32) -> u64 {
    let mut x = x as i64;
    let mut y = y as i64;
    let mut s = (1_i64 << z) / 2;
    let mut d = 0_u64;
    while s > 0 {
        let rx = u64::from(x & s != 0);
        let ry = u64::from(y & s != 0);
        d += (s as u64) * (s as u64) * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = s - 1 - x;
                y = s - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    ((1_u64 << (2 * z)) - 1) / 3 + d
}
fn encoded_webp(x: u32, y: u32, masked: bool) -> Result<Vec<u8>, String> {
    let stride = if masked { 4 } else { 3 };
    let mut pixels = vec![0_u8; WIDTH * WIDTH * stride];
    for row in 0..WIDTH {
        for col in 0..WIDTH {
            let global_col = (x - 1091) as usize * WIDTH + col;
            let global_row = (y - 1554) as usize * WIDTH + row;
            let meters = -96. + global_col as f64 / 8. + global_row as f64 / 16.;
            let shifted = meters + 32768.;
            let integral = shifted.floor() as u32;
            let idx = (row * WIDTH + col) * stride;
            pixels[idx] = (integral / 256) as u8;
            pixels[idx + 1] = (integral % 256) as u8;
            pixels[idx + 2] = ((shifted.fract() * 256.) as u32) as u8;
            if masked {
                // Entire selected southeast tile is no-data in this negative fixture.
                pixels[idx + 3] = 0;
            }
        }
    }
    let mut webp = Vec::new();
    WebPEncoder::new_lossless(&mut webp)
        .encode(
            &pixels,
            WIDTH as u32,
            WIDTH as u32,
            if masked {
                ExtendedColorType::Rgba8
            } else {
                ExtendedColorType::Rgb8
            },
        )
        .map_err(|_| "INVALID_SOURCE: cannot generate original test WebP")?;
    Ok(webp)
}
fn original_archive(missing_southeast: bool, mask_southeast: bool) -> Result<Vec<u8>, String> {
    let mut tiles = Vec::new();
    for (x, y) in [(1091, 1554), (1091, 1555), (1092, 1554), (1092, 1555)] {
        if missing_southeast && (x, y) == (1092, 1555) {
            continue;
        }
        let webp = encoded_webp(x, y, mask_southeast && (x, y) == (1092, 1555))?;
        tiles.push((hilbert_id(Z, x, y), webp));
    }
    tiles.sort_by_key(|(id, _)| *id);
    let mut directory = Vec::new();
    varint(tiles.len() as u64, &mut directory);
    let mut before = 0_u64;
    for (id, _) in &tiles {
        varint(id - before, &mut directory);
        before = *id;
    }
    for _ in &tiles {
        varint(1, &mut directory); // one tile per indexed entry
    }
    for (_, webp) in &tiles {
        varint(webp.len() as u64, &mut directory);
    }
    for i in 0..tiles.len() {
        varint(if i == 0 { 1 } else { 0 }, &mut directory);
    }
    let metadata = br#"{"encoding":"terrarium","name":"Original Map2Model generated test raster","attribution":"Original mathematical height function; no Mapterhorn source data"}"#;
    let tile_offset = 127 + directory.len() + metadata.len();
    let tile_bytes = tiles.iter().map(|(_, image)| image.len()).sum::<usize>();
    let mut archive = vec![0_u8; 127];
    archive[..7].copy_from_slice(b"PMTiles");
    archive[7] = 3;
    u64le(&mut archive, 8, 127);
    u64le(&mut archive, 16, directory.len() as u64);
    u64le(&mut archive, 24, (127 + directory.len()) as u64);
    u64le(&mut archive, 32, metadata.len() as u64);
    u64le(&mut archive, 40, tile_offset as u64);
    u64le(&mut archive, 48, 0); // no leaf directories
    u64le(&mut archive, 56, tile_offset as u64);
    u64le(&mut archive, 64, tile_bytes as u64);
    for at in [72, 80, 88] {
        u64le(&mut archive, at, tiles.len() as u64);
    }
    archive[96] = 1; // clustered
    archive[97] = 1; // internal compression: none
    archive[98] = 1; // tile compression: none
    archive[99] = 4; // WebP
    archive[100] = Z;
    archive[101] = Z;
    // Intentional envelope spanning the full four-tile group in WGS84,
    // compatible with the G1 max-1-degree bounded imported source.
    i32le(&mut archive, 102, -841113200);
    i32le(&mut archive, 106, 396395400);
    i32le(&mut archive, 110, -839355500);
    i32le(&mut archive, 114, 397747600);
    archive.extend(directory);
    archive.extend(metadata);
    for (_, webp) in tiles {
        archive.extend(webp);
    }
    Ok(archive)
}
fn selection() -> Result<SelectionV1, String> {
    // The center crosses BOTH the x=1092 and y=1555 WebMercator tile seams.
    let mut selection = SelectionV1 {
        schema_version: "1.0.0".into(),
        kind: "SelectionV1".into(),
        selection_id: String::new(),
        planet: "earth".into(),
        horizontal_crs: "EPSG:4326".into(),
        bbox: [-84.024, 39.7067, -84.023, 39.7077],
        terrain_profile: TerrainProfileV1 {
            target_ground_sample_meters: 18.,
            no_data_policy: "fail".into(),
            max_samples: 10_000,
        },
    };
    selection.selection_id = format!(
        "sel-{}",
        &canonical::selection_digest(&selection)?[..32]
    );
    validation::validate_selection(&selection)?;
    Ok(selection)
}
fn declaration(sha: &str, variant: &str) -> Value {
    json!({
        "sourceName": SOURCE,
        "releaseId": "original-generated-z12-512px-v1",
        "archiveId": format!("artificial-4-tiles-{variant}"),
        "expectedArchiveSha256": sha,
        "verticalDatumId": KNOWN_SYNTHETIC_DATUM,
        "verticalDatumStatus": "known",
        "licenseIdentifier": "CC0-1.0",
        "attribution": "Mathematically generated synthetic heights; no real elevation raster was copied",
        "nominalGroundSampleMeters": 18.,
        "nativeResolutionSource": "Synthetic 512px mathematical raster, zoom12",
        "uncertaintyMeters": 0.,
        "decoderVersion": "map2model-data/synthetic-fixture-1"
    })
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|_| "INVALID_SOURCE: fixture output cannot overwrite an existing file")?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|_| "INVALID_SOURCE: cannot write original fixture")?;
    Ok(())
}
fn generate(output: &Path) -> Result<Value, String> {
    // Fail closed: never overwrite an existing output directory or real data.
    fs::create_dir(output)
        .map_err(|_| "INVALID_SOURCE: supply a NEW empty output directory")?;
    let selected = selection()?;
    write_new(
        &output.join("selection.json"),
        &serde_json::to_vec_pretty(&selected).map_err(|_| "INVALID_SOURCE: selection JSON")?,
    )?;
    let mut reports = Vec::new();
    for (variant, missing, masked) in [
        ("complete", false, false),
        ("missing-southeast", true, false),
        ("masked-southeast", false, true),
    ] {
        let data = original_archive(missing, masked)?;
        let sha = hash(&data);
        let archive_name = format!("{variant}.pmtiles");
        let source_name = format!("{variant}.source.json");
        write_new(&output.join(&archive_name), &data)?;
        write_new(
            &output.join(&source_name),
            &serde_json::to_vec_pretty(&declaration(&sha, variant))
                .map_err(|_| "INVALID_SOURCE: fixture declaration JSON")?,
        )?;
        reports.push(json!({
            "variant": variant,
            "archive": archive_name,
            "sourceDeclaration": source_name,
            "archiveSha256": sha,
            "archiveBytes": data.len(),
            "indexedTiles": if missing { 3 } else { 4 },
            "expectedDecode": if missing || masked { "DATA_MISSING" } else { "OK" }
        }));
    }
    let report = json!({
        "kind": "Map2ModelOriginalSyntheticTerrainFixtureV1",
        "format": "PMTiles-v3 lossless WebP 512x512 original generated mathematical data",
        "licenseScope": "CC0-1.0 SYNTHETIC TEST ASSET ONLY; NOT real-source authorization",
        "verticalDatumId": KNOWN_SYNTHETIC_DATUM,
        "sampleZoom": Z,
        "selectionId": selected.selection_id,
        "selectionBboxWgs84": selected.bbox,
        "selectionFile": "selection.json",
        "variants": reports,
        "networkRequests": 0,
        "productionRealSourceReady": false
    });
    write_new(
        &output.join("fixture-report.json"),
        &serde_json::to_vec_pretty(&report).map_err(|_| "INVALID_SOURCE: report JSON")?,
    )?;
    Ok(report)
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [dir] = args.as_slice() else {
        eprintln!("INVALID_REQUEST: terrain_synthetic_fixture <new-output-directory>");
        std::process::exit(2);
    };
    match generate(Path::new(dir)) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_hilbert_ids_match_pmtiles_known_reference() {
        assert_eq!(hilbert_id(12, 1091, 1554), 8_646_750);
        assert_eq!(hilbert_id(12, 1091, 1555), 8_646_751);
        assert_eq!(hilbert_id(12, 1092, 1555), 8_646_794);
        assert_eq!(hilbert_id(12, 1092, 1554), 8_646_795);
    }
    #[test]
    fn generating_original_512px_source_is_deterministic_and_sha_distinguishes_variants() {
        let a = original_archive(false, false).unwrap();
        let b = original_archive(false, false).unwrap();
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));
        assert_ne!(hash(&a), hash(&original_archive(true, false).unwrap()));
        assert_ne!(hash(&a), hash(&original_archive(false, true).unwrap()));
        assert_eq!(a[99], 4);
        assert_eq!(a[100], 12);
        assert_eq!(a[101], 12);
    }
}