//! Deterministic, offline Mapterhorn Terrarium RGB/WebP -> frozen ElevationGridV1.
//! Pixel values are decoded, never guessed from source zoom, bounding-box or
//! headers. Transparent source samples fail the G1 no-data-policy=fail contract.
use crate::ImportedSource;
use image::{codecs::webp::WebPDecoder, ColorType, ImageDecoder};
use map2model_contracts::{canonical, validation, ElevationGridV1, SelectionV1};
use std::collections::BTreeMap;
use std::io::Cursor;

pub const TILE_SIZE: usize = 512;
const MAX_CACHED_TILES: usize = 64;

fn error(code: &str, why: &str) -> String {
    format!("{code}: {why}")
}
/// Decoded WebP in north-up, row-major image order. Alpha<255 is no-data.
#[derive(Clone)]
pub struct TerrariumTile {
    heights: Vec<f64>,
    valid: Vec<bool>,
}
impl TerrariumTile {
    pub fn height(&self, column: usize, row: usize) -> Option<f64> {
        if column >= TILE_SIZE || row >= TILE_SIZE {
            return None;
        }
        let index = row * TILE_SIZE + column;
        self.valid[index].then_some(self.heights[index])
    }
    pub fn number_of_missing_pixels(&self) -> usize {
        self.valid.iter().filter(|valid| !**valid).count()
    }
}
/// Explicitly refuses resized, non-WebP or unknown color model data.
/// Terrarium formula: meters = R * 256 + G + B / 256 - 32768.
/// The 1/256-meter quantum, RGB8 and negative elevations are preserved as f64.
pub fn decode_terrarium_webp(bytes: &[u8]) -> Result<TerrariumTile, String> {
    if bytes.len() < 20
        || bytes.len() > 2 * 1024 * 1024
        || bytes.get(0..4) != Some(b"RIFF")
        || bytes.get(8..12) != Some(b"WEBP")
    {
        return Err(error(
            "INVALID_SOURCE",
            "WebP tile envelope invalid or too large",
        ));
    }
    let decoder = WebPDecoder::new(Cursor::new(bytes))
        .map_err(|_| error("INVALID_SOURCE", "WebP decoder rejected source payload"))?;
    if decoder.dimensions() != (512, 512) {
        return Err(error(
            "INVALID_SOURCE",
            "Terrarium WebP must be exactly 512x512 pixels",
        ));
    }
    let stride = match decoder.color_type() {
        ColorType::Rgb8 => 3,
        ColorType::Rgba8 => 4,
        _ => {
            return Err(error(
                "INVALID_SOURCE",
                "Terrarium requires RGB8 or RGBA8 pixels",
            ))
        }
    };
    let expected = TILE_SIZE * TILE_SIZE * stride;
    if decoder.total_bytes() != expected as u64 {
        return Err(error("INVALID_SOURCE", "invalid decoded WebP pixel budget"));
    }
    let mut pixels = vec![0_u8; expected];
    decoder
        .read_image(&mut pixels)
        .map_err(|_| error("INVALID_SOURCE", "WebP decompression failed"))?;
    let mut heights = Vec::with_capacity(TILE_SIZE * TILE_SIZE);
    let mut valid = Vec::with_capacity(TILE_SIZE * TILE_SIZE);
    for p in pixels.chunks_exact(stride) {
        // Terrarium sample is precisely representable in f64 as multiples of 1/256m.
        heights.push(
            (u32::from(p[0]) * 256 + u32::from(p[1])) as f64 + f64::from(p[2]) / 256. - 32768.,
        );
        valid.push(stride == 3 || p[3] == 255);
    }
    Ok(TerrariumTile { heights, valid })
}
fn tile_coordinate(lon: f64, lat: f64, zoom: u8) -> Result<(u32, u32, usize, usize), String> {
    if !lon.is_finite()
        || !lat.is_finite()
        || !(-180.0..=180.0).contains(&lon)
        || !(-60.0..=60.0).contains(&lat)
        || zoom > 22
    {
        return Err(error("INVALID_CRS", "unsupported WGS84 raster coordinate"));
    }
    let n = (1_u64 << zoom) as f64;
    let merc_x = ((lon + 180.) / 360. * n).clamp(0., n);
    let phi = lat.to_radians();
    let merc_y =
        ((1. - (phi.tan() + 1. / phi.cos()).ln() / std::f64::consts::PI) / 2. * n).clamp(0., n);
    fn sample_axis(value: f64, n: f64) -> (u32, usize) {
        // Closed-boundary preflight requires both touching tiles, while raster
        // node ownership deterministically selects the positive/right/down tile.
        let round = value.round();
        let exact = if (value - round).abs() <= 1e-9 {
            round
        } else {
            value
        };
        let tile = exact.floor().min(n - 1.) as u32;
        let local = if exact >= n { 1. } else { exact - tile as f64 };
        let pixel = (local * 512.).floor().clamp(0., 511.) as usize;
        (tile, pixel)
    }
    let (x, px) = sample_axis(merc_x, n);
    let (y, py) = sample_axis(merc_y, n);
    Ok((x, y, px, py))
}
/// Decodes all requested grid nodes, north-up row-major, without implicit
/// interpolation or filling: a missing tile/transparent pixel is DATA_MISSING.
/// Requests are bound to the imported source's verified source+index identity.
pub fn decode_selection_grid(
    source: &ImportedSource,
    selection: &SelectionV1,
    zoom: u8,
) -> Result<ElevationGridV1, String> {
    validation::validate_selection(selection)?;
    validation::validate_manifest(&source.manifest)?;
    if source.manifest.vertical_datum_status != "known"
        || source.manifest.vertical_datum_id.is_empty()
    {
        return Err(error(
            "UNSUPPORTED_DATUM",
            "unknown terrain source vertical reference",
        ));
    }
    let coverage_tiles = source.verify_selection_coverage(selection, zoom)?;
    if coverage_tiles > MAX_CACHED_TILES {
        return Err(error(
            "OUT_OF_MEMORY",
            "requested tile set exceeds bounded WebP decode cache",
        ));
    }
    let [west, south, east, north] = selection.bbox;
    let [east_m, north_m] = validation::project_wgs84([west, south], [east, north])?;
    let requested = selection.terrain_profile.target_ground_sample_meters;
    let width = (east_m / requested).ceil() as usize + 1;
    let height = (north_m / requested).ceil() as usize + 1;
    let count = width
        .checked_mul(height)
        .ok_or("OUT_OF_MEMORY: grid node overflow")?;
    if width < 2
        || height < 2
        || width > 4096
        || height > 4096
        || count > selection.terrain_profile.max_samples
        || count > 1_000_000
    {
        return Err(error(
            "OUT_OF_MEMORY",
            "frozen V1 elevation grid budget exceeded",
        ));
    }
    let dx = east_m / (width - 1) as f64;
    let dy = north_m / (height - 1) as f64;
    let mut cache: BTreeMap<(u32, u32), TerrariumTile> = BTreeMap::new();
    let mut samples = Vec::with_capacity(count);
    for row in 0..height {
        let lat = north - row as f64 / (height - 1) as f64 * (north - south);
        for col in 0..width {
            let lon = west + col as f64 / (width - 1) as f64 * (east - west);
            let (tx, ty, px, py) = tile_coordinate(lon, lat, zoom)?;
            if !cache.contains_key(&(tx, ty)) {
                if cache.len() >= MAX_CACHED_TILES {
                    return Err(error(
                        "OUT_OF_MEMORY",
                        "raster source spans too many decoded tiles",
                    ));
                }
                let bytes = source.read_registered_tile(zoom, tx, ty)?;
                cache.insert((tx, ty), decode_terrarium_webp(&bytes)?);
            }
            let tile = cache
                .get(&(tx, ty))
                .ok_or("INTERNAL_ERROR: tile lookup missing")?;
            samples.push(tile.height(px,py).ok_or_else(|| {
                format!("DATA_MISSING: source alpha indicates no-data at row={row}, col={col}, z={zoom}, x={tx}, y={ty}")
            })?);
        }
    }
    let mut grid = ElevationGridV1 {
        schema_version: "1.0.0".into(),
        kind: "ElevationGridV1".into(),
        grid_id: String::new(),
        selection_id: selection.selection_id.clone(),
        source_manifest_sha256: source.manifest.manifest_sha256.clone(),
        width,
        height,
        samples_meters: Some(samples.into_iter().map(Some).collect()),
        valid_mask: Some(vec![true; count]),
        storage_mode: "inline-fixture".into(),
        grid_data_token: None,
        grid_data_sha256: None,
        min_east_meters: 0.,
        max_north_meters: north_m,
        dx_meters: dx,
        dy_meters: dy,
        anchor_wgs84: [west, south],
        horizontal_transform_id: "WGS84-ENU-LINEARIZED-V1".into(),
        vertical_datum_id: source.manifest.vertical_datum_id.clone(),
        storage_convention: "north-up-f64-row-major".into(),
        grid_sha256: String::new(),
    };
    grid.grid_sha256 = canonical::grid_digest(&grid)?;
    grid.grid_id = format!("grid-{}", &grid.grid_sha256[..32]);
    validation::validate_grid(&grid)?;
    Ok(grid)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_local_pmtiles, SourceDeclaration};
    use image::{codecs::webp::WebPEncoder, ExtendedColorType};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn webp_alpha_fixture(alpha: bool) -> Vec<u8> {
        let stride = if alpha { 4 } else { 3 };
        let mut pixels = vec![0u8; TILE_SIZE * TILE_SIZE * stride];
        for row in 0..TILE_SIZE {
            for col in 0..TILE_SIZE {
                let h = if row < 260 {
                    if col < 127 {
                        [127, 132, 128]
                    } else {
                        [128, 100, 64]
                    }
                } else {
                    [127, 226, 0]
                };
                let p = (row * TILE_SIZE + col) * stride;
                pixels[p..p + 3].copy_from_slice(&h);
                if alpha {
                    pixels[p + 3] = if (row == 255 && col == 124) || row == 0 {
                        0
                    } else {
                        255
                    };
                }
            }
        }
        let mut bytes = Vec::new();
        WebPEncoder::new_lossless(&mut bytes)
            .encode(
                &pixels,
                512,
                512,
                if alpha {
                    ExtendedColorType::Rgba8
                } else {
                    ExtendedColorType::Rgb8
                },
            )
            .unwrap();
        bytes
    }
    fn put_u64(b: &mut [u8], p: usize, x: u64) {
        b[p..p + 8].copy_from_slice(&x.to_le_bytes())
    }
    fn put_i32(b: &mut [u8], p: usize, x: i32) {
        b[p..p + 4].copy_from_slice(&x.to_le_bytes())
    }
    fn varint(mut n: u64, out: &mut Vec<u8>) {
        while n >= 128 {
            out.push(n as u8 | 128);
            n >>= 7;
        }
        out.push(n as u8);
    }
    fn fixture_pmtiles(webp: &[u8]) -> Vec<u8> {
        let z = 12_u8;
        // One real XYZ tile: z12 x1091 y1554 at southwest Ohio synthetic coordinate.
        // Hilbert ID calculated independently from the PMTiles reference algorithm.
        let id = crate::tile_id(z, 1091, 1554).unwrap();
        let mut root = vec![1]; // one entry
        varint(id, &mut root);
        varint(1, &mut root);
        varint(webp.len() as u64, &mut root);
        varint(1, &mut root);
        let meta = br#"{"encoding":"terrarium","attribution":"Synthetic raster"}"#;
        let tile_start = 127 + root.len() + meta.len();
        let mut out = vec![0u8; 127];
        out[..7].copy_from_slice(b"PMTiles");
        out[7] = 3;
        put_u64(&mut out, 8, 127);
        put_u64(&mut out, 16, root.len() as u64);
        put_u64(&mut out, 24, (127 + root.len()) as u64);
        put_u64(&mut out, 32, meta.len() as u64);
        put_u64(&mut out, 40, tile_start as u64);
        put_u64(&mut out, 48, 0);
        put_u64(&mut out, 56, tile_start as u64);
        put_u64(&mut out, 64, webp.len() as u64);
        put_u64(&mut out, 72, 1);
        put_u64(&mut out, 80, 1);
        put_u64(&mut out, 88, 1);
        out[96] = 1;
        out[97] = 1;
        out[98] = 1;
        out[99] = 4;
        out[100] = z;
        out[101] = z;
        put_i32(&mut out, 102, -840900000);
        put_i32(&mut out, 106, 397400000);
        put_i32(&mut out, 110, -840800000);
        put_i32(&mut out, 114, 397500000);
        out.extend(root);
        out.extend(meta);
        out.extend(webp);
        out
    }
    fn policy() -> SourceDeclaration {
        SourceDeclaration {
            source_name: "Artificial 512px Terrarium-format DEM".into(),
            release_id: "synthetic-decoder-1".into(),
            archive_id: "ohio-synthetic-512px".into(),
            expected_archive_sha256: None,
            vertical_datum_id: "SYNTHETIC-RELATIVE-METERS".into(),
            vertical_datum_status: "known".into(),
            license_identifier: "CC0-1.0".into(),
            attribution: "Entirely fabricated RGB test heights, not Mapterhorn data".into(),
            nominal_ground_sample_meters: 12.,
            native_resolution_source: "artificial test raster".into(),
            uncertainty_meters: Some(0.),
            decoder_version: "map2model-data-terrarium-v1".into(),
        }
    }
    fn temp() -> PathBuf {
        std::env::temp_dir().join(format!(
            "m2m-decode-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    fn selection() -> SelectionV1 {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"
        ))
        .unwrap()
    }
    #[test]
    fn original_512px_lossless_webp_preserves_rgb_negative_heights_alpha_and_row_order() {
        let bytes = webp_alpha_fixture(false);
        let tile = decode_terrarium_webp(&bytes).unwrap();
        assert_eq!(tile.height(0, 0), Some(-123.5));
        assert_eq!(tile.height(400, 0), Some(100.25));
        assert_eq!(tile.height(400, 511), Some(-30.));
        assert_eq!(tile.number_of_missing_pixels(), 0);
        let alpha = decode_terrarium_webp(&webp_alpha_fixture(true)).unwrap();
        assert_eq!(alpha.height(124, 255), None);
        assert_eq!(alpha.height(124, 256), Some(-123.5));
        assert!(alpha.number_of_missing_pixels() > 0);
        assert!(decode_terrarium_webp(b"wrong").is_err());
        // A framing-valid 1x1 WebP is not an elevation tile.
        let mut single = Vec::new();
        WebPEncoder::new_lossless(&mut single)
            .encode(&[127, 132, 128], 1, 1, ExtendedColorType::Rgb8)
            .unwrap();
        assert!(decode_terrarium_webp(&single)
            .unwrap_err()
            .starts_with("INVALID_SOURCE"));
    }
    #[test]
    fn builds_verified_deterministic_north_up_grid_from_full_512_webp_pmtiles() {
        let root = temp();
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> Result<(), String> {
            let path = root.join("synthetic.pmtiles");
            fs::write(&path, fixture_pmtiles(&webp_alpha_fixture(false))).unwrap();
            let source = import_local_pmtiles(&path, &root.join("native-registry"), &policy())?;
            let selection = selection();
            assert_eq!(source.verify_selection_coverage(&selection, 12)?, 1);
            let grid = decode_selection_grid(&source, &selection, 12)?;
            assert_eq!(grid.kind, "ElevationGridV1");
            assert_eq!(grid.source_manifest_sha256, source.manifest.manifest_sha256);
            assert_eq!(grid.selection_id, selection.selection_id);
            assert_eq!(grid.anchor_wgs84, [selection.bbox[0], selection.bbox[1]]);
            assert_eq!(grid.storage_convention, "north-up-f64-row-major");
            assert_eq!(
                grid.samples_meters.as_ref().unwrap().len(),
                grid.width * grid.height
            );
            assert!(grid
                .samples_meters
                .as_ref()
                .unwrap()
                .iter()
                .flatten()
                .any(|n| *n < 0.));
            assert!(grid
                .samples_meters
                .as_ref()
                .unwrap()
                .iter()
                .flatten()
                .any(|n| *n > 0.));
            assert_eq!(grid.grid_sha256, canonical::grid_digest(&grid)?);
            let again = decode_selection_grid(&source, &selection, 12)?;
            assert_eq!(grid.grid_sha256, again.grid_sha256);
            assert_eq!(grid.samples_meters, again.samples_meters);
            let top = grid.samples_meters.as_ref().unwrap()[0].unwrap();
            let bottom =
                grid.samples_meters.as_ref().unwrap()[(grid.height - 1) * grid.width].unwrap();
            assert_eq!(top, -123.5);
            assert_eq!(bottom, -30.);
            let mut budget = selection.clone();
            budget.terrain_profile.max_samples = 4;
            budget.selection_id = format!("sel-{}", &canonical::selection_digest(&budget)?[..32]);
            assert!(decode_selection_grid(&source, &budget, 12)
                .unwrap_err()
                .starts_with("OUT_OF_MEMORY"));
            Ok(())
        })();
        let _ = fs::remove_dir_all(root);
        result.unwrap();
    }
    #[test]
    fn alpha_nodata_fails_closed_and_wrong_size_is_not_elevation() {
        let root = temp();
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> Result<(), String> {
            let path = root.join("rgba.pmtiles");
            fs::write(&path, fixture_pmtiles(&webp_alpha_fixture(true))).unwrap();
            let source = import_local_pmtiles(&path, &root.join("native-registry"), &policy())?;
            let tile = decode_terrarium_webp(&source.read_registered_tile(12, 1091, 1554)?)?;
            assert!(tile.number_of_missing_pixels() > 0);
            // Broad synthetic alpha no-data row 0 is outside the selected subregion,
            // so direct decoding exposes it without falsely filling those pixels.
            let mut masked = vec![0_u8; 512 * 512 * 4];
            for p in masked.chunks_exact_mut(4) {
                p.copy_from_slice(&[127, 132, 128, 0]);
            }
            let mut missing_bytes = Vec::new();
            WebPEncoder::new_lossless(&mut missing_bytes)
                .encode(&masked, 512, 512, ExtendedColorType::Rgba8)
                .unwrap();
            let path2 = root.join("nodata.pmtiles");
            fs::write(&path2, fixture_pmtiles(&missing_bytes)).unwrap();
            let other = import_local_pmtiles(&path2, &root.join("registry-other"), &policy())?;
            assert!(decode_selection_grid(&other, &selection(), 12)
                .unwrap_err()
                .starts_with("DATA_MISSING"));
            Ok(())
        })();
        let _ = fs::remove_dir_all(root);
        result.unwrap();
    }
}
