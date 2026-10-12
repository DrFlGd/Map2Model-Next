//! Explicit, native-only, private experimental *candidate* terrain sampling.
//!
//! NOT a TerrainSourceManifestV1, NOT an ElevationGridV1, never a registered
//! trusted source, and never source-rights or known-vertical-datum evidence.
//! The opt-in and exact SHA protect against accidental production-path reuse.
use crate::terrarium::{decode_terrarium_webp, tile_coordinate, TerrariumTile};
use crate::{
    decompress, digest, err, inspect_archive, section, tile_id, tile_id_in_zoom, valid_webp,
    MAX_ARCHIVE_BYTES, MAX_COVERAGE_QUERY_TILES, MAX_METADATA_BYTES, MAX_TILE_BYTES,
};
use map2model_contracts::{validation, SelectionV1};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Read,
    path::Path,
};

/// Require an exact intentional user/developer opt-in for each native invocation.
/// The integrated Lead-owned UI will instead require a checked local-consent step.
pub const EXPLICIT_LOCAL_ONLY_ACK: &str =
    "I-ACCEPT-UNVERIFIED-TERRAIN-FOR-PRIVATE-LOCAL-EXPERIMENTATION";

const MAX_PREVIEW_SAMPLES: usize = 100_000;
const MAX_CACHED_TILES: usize = 64;

pub(crate) fn read_private_source(source: &Path) -> Result<Vec<u8>, String> {
    let info = fs::symlink_metadata(source)
        .map_err(|_| err("DATA_MISSING", "local experimental PMTiles absent"))?;
    if !info.is_file()
        || info.file_type().is_symlink()
        || info.len() < 127
        || info.len() > MAX_ARCHIVE_BYTES
    {
        return Err(err(
            "INVALID_SOURCE",
            "experimental input must be a bounded ordinary local PMTiles file",
        ));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .open(source)
        .map_err(|_| err("DATA_MISSING", "cannot open local experimental source"))?;
    let opened = file
        .metadata()
        .map_err(|_| err("INVALID_SOURCE", "cannot stat opened experimental source"))?;
    if !opened.is_file() || opened.len() != info.len() {
        return Err(err(
            "INVALID_SOURCE",
            "experimental source changed before read",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_ARCHIVE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| err("INVALID_SOURCE", "bounded experimental read failed"))?;
    if bytes.len() as u64 != info.len() || bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(err(
            "INVALID_SOURCE",
            "experimental source changed during read",
        ));
    }
    let after = fs::symlink_metadata(source)
        .map_err(|_| err("DATA_MISSING", "experimental source disappeared"))?;
    if !after.is_file() || after.file_type().is_symlink() || after.len() != info.len() {
        return Err(err("INVALID_SOURCE", "experimental source path changed"));
    }
    Ok(bytes)
}

/// Produces only an experimental preview DTO, intentionally incompatible with
/// the trusted/registered V1 grid and production-generation API.
pub fn decode_experimental_local_pmtiles(
    file: &Path,
    expected_sha256: &str,
    selection: &SelectionV1,
    zoom: u8,
    acknowledgement: &str,
) -> Result<serde_json::Value, String> {
    if acknowledgement != EXPLICIT_LOCAL_ONLY_ACK {
        return Err(err(
            "EXPERIMENTAL_CONSENT_REQUIRED",
            "local-only, unknown-datum preview requires explicit acknowledgement",
        ));
    }
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(err(
            "INVALID_SOURCE",
            "expected source SHA-256 must be lowercase hex",
        ));
    }
    validation::validate_selection(selection)?;
    let bytes = read_private_source(file)?;
    let source_sha = digest(&bytes);
    if source_sha != expected_sha256 {
        return Err(err(
            "HASH_MISMATCH",
            "experimental file differs from chosen exact SHA",
        ));
    }
    // The format inspector allows attribution-only metadata, but still rejects
    // any *contradictory* explicit encoding, invalid directory and non-WebP types.
    let parsed = inspect_archive(&bytes, false)?;
    let h = &parsed.header;
    if zoom < h.min_zoom || zoom > h.max_zoom {
        return Err(err(
            "DATA_MISSING",
            "requested zoom absent from experimental archive",
        ));
    }
    let indexed_min = parsed
        .tiles
        .keys()
        .next()
        .and_then(|id| (h.min_zoom..=h.max_zoom).find(|z| tile_id_in_zoom(*id, *z, *z)));
    let indexed_max = parsed
        .tiles
        .keys()
        .next_back()
        .and_then(|id| (h.min_zoom..=h.max_zoom).find(|z| tile_id_in_zoom(*id, *z, *z)));
    if indexed_min != Some(h.min_zoom) || indexed_max != Some(h.max_zoom) {
        return Err(err("INVALID_SOURCE", "PMTiles header/index zoom mismatch"));
    }
    let [west, south, east, north] = selection.bbox;
    let envelope = h.bbox;
    if west < envelope[0] || south < envelope[1] || east > envelope[2] || north > envelope[3] {
        return Err(err(
            "DATA_MISSING",
            "selection outside local source envelope",
        ));
    }
    let metadata = decompress(
        section(&bytes, h.metadata.0, h.metadata.1, "metadata")?,
        h.internal_compression,
        MAX_METADATA_BYTES,
    )?;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata)
        .map_err(|_| err("INVALID_SOURCE", "invalid metadata JSON"))?;
    let attribution = metadata
        .get("attribution")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 2048);

    // Closed-bbox coverage: BOTH neighbors must exist on an exact x/y seam,
    // even though north-up samples deterministically belong to right/down.
    let n = (1_u64 << zoom) as f64;
    let mx = |lon: f64| ((lon + 180.) / 360. * n).clamp(0., n);
    let my = |lat: f64| {
        let phi = lat.to_radians();
        ((1. - (phi.tan() + 1. / phi.cos()).ln() / std::f64::consts::PI) / 2. * n).clamp(0., n)
    };
    let low = |f: f64| {
        let seam = f.round();
        if (f - seam).abs() <= 1e-9 && seam > 0. && seam < n {
            seam as u32 - 1
        } else {
            f.floor().min(n - 1.) as u32
        }
    };
    let high = |f: f64| {
        let seam = f.round();
        if (f - seam).abs() <= 1e-9 {
            seam.min(n - 1.) as u32
        } else {
            f.floor().min(n - 1.) as u32
        }
    };
    let (x0, x1) = (low(mx(west)), high(mx(east)));
    let (y0, y1) = (low(my(north)), high(my(south)));
    if x1 < x0 || y1 < y0 {
        return Err(err("INVALID_SELECTION", "empty experimental XYZ range"));
    }
    let tile_count = (u64::from(x1) - u64::from(x0) + 1)
        .checked_mul(u64::from(y1) - u64::from(y0) + 1)
        .ok_or("OUT_OF_MEMORY: experimental tile count overflow")?;
    if tile_count > MAX_COVERAGE_QUERY_TILES as u64 || tile_count > MAX_CACHED_TILES as u64 {
        return Err(err(
            "OUT_OF_MEMORY",
            "experimental tile count exceeds WebP cache budget",
        ));
    }
    for x in x0..=x1 {
        for y in y0..=y1 {
            let id = tile_id(zoom, x, y)?;
            if !parsed.tiles.contains_key(&id) {
                return Err(err(
                    "DATA_MISSING",
                    "experimental selected source tile absent",
                ));
            }
        }
    }

    let [east_m, north_m] = validation::project_wgs84([west, south], [east, north])?;
    let target = selection.terrain_profile.target_ground_sample_meters;
    let width = (east_m / target).ceil() as usize + 1;
    let height = (north_m / target).ceil() as usize + 1;
    let count = width
        .checked_mul(height)
        .ok_or("OUT_OF_MEMORY: experimental grid shape overflow")?;
    if width < 2
        || height < 2
        || width > 4096
        || height > 4096
        || count > MAX_PREVIEW_SAMPLES
        || count > selection.terrain_profile.max_samples
    {
        return Err(err(
            "OUT_OF_MEMORY",
            "experimental candidate sample budget exceeded",
        ));
    }
    let mut cache = BTreeMap::<(u32, u32), TerrariumTile>::new();
    let mut heights = Vec::with_capacity(count);
    let mut heights_sha = Sha256::new();
    heights_sha.update(b"M2M-EXPERIMENTAL-UNVERIFIED-HEIGHTS-1\0");
    heights_sha.update(source_sha.as_bytes());
    heights_sha.update(selection.selection_id.as_bytes());
    for row in 0..height {
        let lat = north - row as f64 / (height - 1) as f64 * (north - south);
        for col in 0..width {
            let lon = west + col as f64 / (width - 1) as f64 * (east - west);
            let (x, y, px, py) = tile_coordinate(lon, lat, zoom)?;
            if !cache.contains_key(&(x, y)) {
                if cache.len() >= MAX_CACHED_TILES {
                    return Err(err("OUT_OF_MEMORY", "experimental source cache full"));
                }
                let id = tile_id(zoom, x, y)?;
                let record = parsed.tiles.get(&id).ok_or_else(|| {
                    err("DATA_MISSING", "experimental sampled source tile absent")
                })?;
                let webp = decompress(
                    section(&bytes, record.offset, u64::from(record.length), "tile")?,
                    h.tile_compression,
                    MAX_TILE_BYTES,
                )?;
                valid_webp(&webp)?;
                cache.insert((x, y), decode_terrarium_webp(&webp)?);
            }
            let candidate = cache
                .get(&(x, y))
                .and_then(|tile| tile.height(px, py))
                .ok_or_else(|| {
                    err(
                        "DATA_MISSING",
                        "experimental alpha no-data in selected raster sample",
                    )
                })?;
            heights_sha.update(candidate.to_le_bytes());
            heights.push(candidate);
        }
    }
    Ok(serde_json::json!({
        "kind": "ExperimentalLocalTerrainPreviewV1",
        "experimentalOnly": true,
        "localPersonalUseOnly": true,
        "ownerAcknowledgedUnverified": true,
        "sourceArchiveSha256": source_sha,
        "sourceArchiveBytes": bytes.len(),
        "sourceAttributionFromArchive": attribution,
        "sourceMetadataDeclaresTerrarium": metadata.get("encoding").and_then(serde_json::Value::as_str) == Some("terrarium"),
        "terrariumInterpretation": "publisher-documented, NOT per-file authenticated",
        "encodingAuthenticated": false,
        "selectionId": selection.selection_id,
        "bboxWgs84": selection.bbox,
        "zoom": zoom,
        "closedBoundaryIndexedTileCount": tile_count,
        "width": width,
        "height": height,
        "anchorWgs84": [west, south],
        "dxMeters": east_m / (width - 1) as f64,
        "dyMeters": north_m / (height - 1) as f64,
        "storageConvention": "north-up-f64-row-major",
        "candidateTerrariumHeightsMeters": heights,
        "candidateHeightsSha256": format!("{:x}", heights_sha.finalize()),
        "verticalDatumStatus": "unknown",
        "sourceLicenseVerified": false,
        "nativeResolutionVerified": false,
        "redistributionApproved": false,
        "productionGenerationReady": false,
        "warnings": [
            "Experimental local-only, unverified real-source terrain values.",
            "Absolute elevation and final vertical datum are unknown.",
            "Keep attribution; external provider terms still apply.",
            "Do not redistribute experimental preview, derived model or source tiles without independent rights review."
        ],
        "networkRequests": 0
    }))
}
