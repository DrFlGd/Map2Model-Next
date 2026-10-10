//! Offline, bounded PMTiles v3 importer for Terrarium WebP archives.
//! Data never leaves the local native engine. All source policy is caller-supplied;
//! no zoom, header bbox, or metadata string is substituted for proven rights/datum.
use flate2::read::GzDecoder;
use map2model_contracts::{canonical, validation, SelectionV1, TerrainSourceManifestV1};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const HEADER_BYTES: usize = 127;
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_INDEX_TILES: usize = 200_000;
const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 128 * 1024;
const MAX_TILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ZOOM: u8 = 22;
const INDEX_MAGIC: &[u8] = b"M2M-PMT-INDEX-1\0";
const MAX_COVERAGE_QUERY_TILES: usize = 4096;

fn err(code: &str, detail: &str) -> String {
    format!("{code}: {detail}")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn section<'a>(bytes: &'a [u8], offset: u64, len: u64, label: &str) -> Result<&'a [u8], String> {
    let start = usize::try_from(offset).map_err(|_| err("INVALID_SOURCE", label))?;
    let count = usize::try_from(len).map_err(|_| err("INVALID_SOURCE", label))?;
    let end = start
        .checked_add(count)
        .ok_or_else(|| err("INVALID_SOURCE", label))?;
    bytes
        .get(start..end)
        .ok_or_else(|| err("INVALID_SOURCE", label))
}
fn u64_at(bytes: &[u8], p: usize) -> Result<u64, String> {
    let data: [u8; 8] = bytes
        .get(p..p + 8)
        .ok_or("INVALID_SOURCE: truncated header")?
        .try_into()
        .map_err(|_| "INVALID_SOURCE: truncated u64")?;
    Ok(u64::from_le_bytes(data))
}
fn i32_at(bytes: &[u8], p: usize) -> Result<i32, String> {
    let data: [u8; 4] = bytes
        .get(p..p + 4)
        .ok_or("INVALID_SOURCE: truncated header")?
        .try_into()
        .map_err(|_| "INVALID_SOURCE: truncated i32")?;
    Ok(i32::from_le_bytes(data))
}
#[derive(Debug, Clone)]
struct Header {
    root: (u64, u64),
    metadata: (u64, u64),
    leaf: (u64, u64),
    tiles: (u64, u64),
    addressed: u64,
    internal_compression: u8,
    tile_compression: u8,
    min_zoom: u8,
    max_zoom: u8,
    bbox: [f64; 4],
}
fn parse_header(bytes: &[u8]) -> Result<Header, String> {
    if bytes.len() < HEADER_BYTES || &bytes[0..7] != b"PMTiles" || bytes[7] != 3 {
        return Err(err("INVALID_SOURCE", "PMTiles v3 magic/header required"));
    }
    let root = (u64_at(bytes, 8)?, u64_at(bytes, 16)?);
    let metadata = (u64_at(bytes, 24)?, u64_at(bytes, 32)?);
    let leaf = (u64_at(bytes, 40)?, u64_at(bytes, 48)?);
    let tiles = (u64_at(bytes, 56)?, u64_at(bytes, 64)?);
    let addressed = u64_at(bytes, 72)?;
    let internal_compression = bytes[97];
    let tile_compression = bytes[98];
    let min_zoom = bytes[100];
    let max_zoom = bytes[101];
    if bytes[99] != 4
        || ![1, 2].contains(&internal_compression)
        || ![1, 2].contains(&tile_compression)
        || min_zoom > max_zoom
        || max_zoom > MAX_ZOOM
        || root.0 < HEADER_BYTES as u64
        || root.1 == 0
        || root.0.checked_add(root.1).is_none_or(|n| n > 16_384)
        || root.1 as usize > 16_384 - HEADER_BYTES
        || metadata.1 == 0
        || metadata.1 > MAX_METADATA_BYTES as u64
        || leaf.1 > MAX_ARCHIVE_BYTES
        || tiles.1 == 0
    {
        return Err(err(
            "INVALID_SOURCE",
            "unsupported compression, WebP/zoom, or directory layout",
        ));
    }
    for (a, b) in [root, metadata, leaf, tiles] {
        section(bytes, a, b, "PMTiles section outside archive")?;
    }
    // Sections are absolute offsets. Reject overlap with one another or header.
    let mut occupied = vec![(0_u64, HEADER_BYTES as u64)];
    for (offset, length) in [root, metadata, leaf, tiles] {
        if length == 0 {
            continue;
        }
        let end = offset + length; // section() above checked range and overflow
        if occupied.iter().any(|&(s, e)| offset < e && s < end) {
            return Err(err("INVALID_SOURCE", "overlapping PMTiles sections"));
        }
        occupied.push((offset, end));
    }
    let bbox = [
        i32_at(bytes, 102)? as f64 / 1e7,
        i32_at(bytes, 106)? as f64 / 1e7,
        i32_at(bytes, 110)? as f64 / 1e7,
        i32_at(bytes, 114)? as f64 / 1e7,
    ];
    if bbox[0] < -180.
        || bbox[2] > 180.
        || bbox[1] < -60.
        || bbox[3] > 60.
        || bbox[0] >= bbox[2]
        || bbox[1] >= bbox[3]
        || bbox[2] - bbox[0] >= 1.
    {
        return Err(err(
            "INVALID_CRS",
            "G1 archive coverage envelope outside selected CRS domain",
        ));
    }
    Ok(Header {
        root,
        metadata,
        leaf,
        tiles,
        addressed,
        internal_compression,
        tile_compression,
        min_zoom,
        max_zoom,
        bbox,
    })
}
fn decompress(bytes: &[u8], method: u8, cap: usize) -> Result<Vec<u8>, String> {
    if method == 1 {
        if bytes.len() > cap {
            return Err(err("OUT_OF_MEMORY", "uncompressed section too large"));
        }
        return Ok(bytes.to_vec());
    }
    if method != 2 {
        return Err(err("INVALID_SOURCE", "unknown compression"));
    }
    let mut decoded = Vec::new();
    GzDecoder::new(bytes)
        .take(cap as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| err("INVALID_SOURCE", "invalid gzip compression"))?;
    if decoded.len() > cap {
        return Err(err("OUT_OF_MEMORY", "gzip output exceeds budget"));
    }
    Ok(decoded)
}
fn varint(data: &[u8], p: &mut usize) -> Result<u64, String> {
    let mut value = 0_u64;
    for n in 0..10 {
        let b = *data
            .get(*p)
            .ok_or("INVALID_SOURCE: truncated directory varint")?;
        *p += 1;
        if n == 9 && b > 1 {
            return Err(err("INVALID_SOURCE", "varint overflow"));
        }
        value |= ((b & 127) as u64) << (7 * n);
        if b & 128 == 0 {
            return Ok(value);
        }
    }
    Err(err("INVALID_SOURCE", "unterminated directory varint"))
}
#[derive(Debug, Clone)]
struct DirEntry {
    id: u64,
    offset: u64,
    length: u64,
    run: u64,
}
fn parse_directory(bytes: &[u8]) -> Result<Vec<DirEntry>, String> {
    let mut p = 0;
    let count = usize::try_from(varint(bytes, &mut p)?)
        .map_err(|_| err("OUT_OF_MEMORY", "directory count"))?;
    if count == 0 || count > MAX_INDEX_TILES {
        return Err(err("OUT_OF_MEMORY", "empty or oversized PMTiles directory"));
    }
    let mut ids = Vec::with_capacity(count);
    let mut last = 0_u64;
    for i in 0..count {
        last = last
            .checked_add(varint(bytes, &mut p)?)
            .ok_or_else(|| err("INVALID_SOURCE", "tile ID overflow"))?;
        if i > 0 && last <= ids[i - 1] {
            return Err(err(
                "INVALID_SOURCE",
                "directory IDs must strictly increase",
            ));
        }
        ids.push(last);
    }
    let mut entries: Vec<DirEntry> = ids
        .into_iter()
        .map(|id| DirEntry {
            id,
            offset: 0,
            length: 0,
            run: 0,
        })
        .collect();
    for e in &mut entries {
        e.run = varint(bytes, &mut p)?;
    }
    for e in &mut entries {
        e.length = varint(bytes, &mut p)?;
        if e.length == 0 {
            return Err(err("INVALID_SOURCE", "zero-length directory entry"));
        }
    }
    let mut preceding_end: Option<u64> = None;
    for e in &mut entries {
        let v = varint(bytes, &mut p)?;
        e.offset = if v == 0 {
            preceding_end.ok_or_else(|| err("INVALID_SOURCE", "first directory offset zero"))?
        } else {
            v - 1
        };
        preceding_end = Some(
            e.offset
                .checked_add(e.length)
                .ok_or_else(|| err("INVALID_SOURCE", "directory offset overflow"))?,
        );
    }
    if p != bytes.len() {
        return Err(err("INVALID_SOURCE", "trailing directory data"));
    }
    Ok(entries)
}
fn tile_id(z: u8, x: u32, y: u32) -> Result<u64, String> {
    if z > MAX_ZOOM || x >= (1_u32 << z) || y >= (1_u32 << z) {
        return Err(err("INVALID_SELECTION", "invalid Z/X/Y coordinate"));
    }
    let mut tx = i64::from(x);
    let mut ty = i64::from(y);
    let mut d = 0_u64;
    let mut s = 1_i64 << z;
    s /= 2;
    while s > 0 {
        let rx = if tx & s != 0 { 1_u64 } else { 0 };
        let ry = if ty & s != 0 { 1_u64 } else { 0 };
        d += (s as u64) * (s as u64) * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                tx = s - 1 - tx;
                ty = s - 1 - ty;
            }
            std::mem::swap(&mut tx, &mut ty);
        }
        s >>= 1;
    }
    Ok(((1_u64 << (2 * z)) - 1) / 3 + d)
}
fn tile_id_in_zoom(id: u64, min_z: u8, max_z: u8) -> bool {
    (min_z..=max_z).any(|z| {
        let start = ((1_u64 << (2 * z)) - 1) / 3;
        id >= start && id < start + (1_u64 << (2 * z))
    })
}
fn valid_webp(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 20
        || bytes.len() > MAX_TILE_BYTES
        || bytes.get(0..4) != Some(b"RIFF")
        || bytes.get(8..12) != Some(b"WEBP")
    {
        return Err(err(
            "INVALID_SOURCE",
            "tile is not a bounded WebP container",
        ));
    }
    let declared = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if declared.checked_add(8) != Some(bytes.len()) || !bytes.len().is_multiple_of(2) {
        return Err(err("INVALID_SOURCE", "invalid WebP RIFF length"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct TileRecord {
    offset: u64,
    length: u32,
}
#[derive(Debug, Clone)]
struct InspectedArchive {
    header: Header,
    tiles: BTreeMap<u64, TileRecord>,
}
fn inspect(bytes: &[u8]) -> Result<InspectedArchive, String> {
    let h = parse_header(bytes)?;
    let meta = decompress(
        section(bytes, h.metadata.0, h.metadata.1, "metadata")?,
        h.internal_compression,
        MAX_METADATA_BYTES,
    )?;
    let object: serde_json::Value = serde_json::from_slice(&meta)
        .map_err(|_| err("INVALID_SOURCE", "PMTiles metadata must be JSON"))?;
    if !object.is_object() || object.get("encoding").and_then(|v| v.as_str()) != Some("terrarium") {
        return Err(err(
            "INVALID_SOURCE",
            "expected explicitly declared Terrarium metadata",
        ));
    }
    let root = decompress(
        section(bytes, h.root.0, h.root.1, "root directory")?,
        h.internal_compression,
        MAX_DIRECTORY_BYTES,
    )?;
    let root_entries = parse_directory(&root)?;
    let mut records = BTreeMap::new();
    let mut total = 0_usize;
    for (root_position, e) in root_entries.iter().enumerate() {
        let next_start = root_entries.get(root_position + 1).map(|next| next.id);
        // The parent is the exact starting key for a leaf, and the following
        // root key is an exclusive bound. Otherwise indexed tiles are unreachable
        // using a standards-conforming PMTiles point lookup.
        let entries = if e.run == 0 {
            let absolute = h
                .leaf
                .0
                .checked_add(e.offset)
                .ok_or_else(|| err("INVALID_SOURCE", "leaf directory offset overflow"))?;
            if e.offset.checked_add(e.length).is_none_or(|n| n > h.leaf.1) {
                return Err(err("INVALID_SOURCE", "leaf outside declared leaf section"));
            }
            let leaf = decompress(
                section(bytes, absolute, e.length, "leaf directory")?,
                h.internal_compression,
                MAX_DIRECTORY_BYTES,
            )?;
            let children = parse_directory(&leaf)?;
            if children.first().is_none_or(|child| child.id != e.id)
                || children.iter().any(|child| child.run == 0)
            {
                return Err(err("INVALID_SOURCE", "leaf must begin at parent ID and contain tile runs"));
            }
            children
        } else {
            vec![e.clone()]
        };
        for t in entries {
            let end =
                t.id.checked_add(t.run)
                    .ok_or_else(|| err("INVALID_SOURCE", "tile run overflow"))?;
            if next_start.is_some_and(|limit| end > limit) {
                return Err(err("INVALID_SOURCE", "tile run crosses the next root boundary"));
            }
            let offset = h
                .tiles
                .0
                .checked_add(t.offset)
                .ok_or_else(|| err("INVALID_SOURCE", "tile offset overflow"))?;
            if t.offset.checked_add(t.length).is_none_or(|n| n > h.tiles.1)
                || t.length > MAX_TILE_BYTES as u64
                || t.length > u32::MAX as u64
            {
                return Err(err("INVALID_SOURCE", "tile section bounds invalid"));
            }
            let webp = decompress(
                section(bytes, offset, t.length, "tile data")?,
                h.tile_compression,
                MAX_TILE_BYTES,
            )?;
            valid_webp(&webp)?;
            for id in t.id..end {
                total += 1;
                if total > MAX_INDEX_TILES {
                    return Err(err("OUT_OF_MEMORY", "tile index exceeds G1 budget"));
                }
                if !tile_id_in_zoom(id, h.min_zoom, h.max_zoom)
                    || records
                        .insert(
                            id,
                            TileRecord {
                                offset,
                                length: t.length as u32,
                            },
                        )
                        .is_some()
                {
                    return Err(err("INVALID_SOURCE", "overlapping or out-of-zoom tile IDs"));
                }
            }
        }
    }
    if records.is_empty() || (h.addressed > 0 && h.addressed != total as u64) {
        return Err(err(
            "INVALID_SOURCE",
            "addressed tile count disagrees with directory",
        ));
    }
    Ok(InspectedArchive {
        header: h,
        tiles: records,
    })
}
/// Release, region rights and datum MUST be established outside this parser.
/// These are never inferred from PMTiles zoom, JSON tile attribution or WGS84.
#[derive(Debug, Clone)]
pub struct SourceDeclaration {
    pub source_name: String,
    pub release_id: String,
    pub archive_id: String,
    pub expected_archive_sha256: Option<String>,
    pub vertical_datum_id: String,
    pub vertical_datum_status: String,
    pub license_identifier: String,
    pub attribution: String,
    pub nominal_ground_sample_meters: f64,
    pub native_resolution_source: String,
    pub uncertainty_meters: Option<f64>,
    pub decoder_version: String,
}
#[derive(Debug)]
pub struct ImportedSource {
    pub manifest: TerrainSourceManifestV1,
    pub coverage_index_token: String,
    pub tile_count: usize,
    registry_root: PathBuf,
    records: BTreeMap<u64, TileRecord>,
    tile_compression: u8,
    min_zoom: u8,
    max_zoom: u8,
}
impl ImportedSource {
    /// Re-authenticate the durable index before using any cached in-memory lookup.
    /// A header or digest declaration is NOT a coverage proof.
    fn verify_registered_index(&self) -> Result<(), String> {
        let sha = self.manifest.coverage_index_sha256.as_deref()
            .ok_or("DATA_MISSING: verified coverage index not declared")?;
        let stored = read_registry_entry(&self.registry_root.join("indices"), sha,
            "idx", INDEX_MAGIC.len() + 32 + 4 + MAX_INDEX_TILES * 20)?;
        let expected = index_bytes(&self.manifest.source_archive_sha256, &self.records)?;
        if stored != expected {
            return Err(err("HASH_MISMATCH", "registered index differs from canonical inspected archive"));
        }
        Ok(())
    }
    /// Uses the *actual checked directory*. Header bounds alone cannot satisfy this query.
    pub fn verify_required_tiles(&self, required: &[(u8, u32, u32)]) -> Result<(), String> {
        self.verify_registered_index()?;
        for &(z, x, y) in required {
            let id = tile_id(z, x, y)?;
            if !self.records.contains_key(&id) {
                return Err(format!(
                    "DATA_MISSING: required tile z={z} x={x} y={y} absent"
                ));
            }
        }
        Ok(())
    }
    /// Generate the complete conservative XYZ tile set touched by a frozen WGS84
    /// selection. Never mark Generation Ready from header bbox/zoom alone.
    pub fn verify_selection_coverage(&self, selection: &SelectionV1, zoom: u8)
        -> Result<usize, String>
    {
        validation::validate_selection(selection)?;
        if zoom < self.min_zoom || zoom > self.max_zoom {
            return Err(err("DATA_MISSING", "requested source zoom not present"));
        }
        let b = selection.bbox;
        let h = self.manifest.coverage_bbox;
        if b[0] < h[0] || b[1] < h[1] || b[2] > h[2] || b[3] > h[3] {
            return Err(err("DATA_MISSING", "selection outside source geographic envelope"));
        }
        let n = (1_u64 << zoom) as f64;
        let x = |lon: f64| ((lon + 180.) * n / 360.).clamp(0., n);
        let y = |lat: f64| {
            let p = lat.to_radians();
            ((1. - (p.tan() + 1. / p.cos()).ln() / std::f64::consts::PI) / 2. * n)
                .clamp(0., n)
        };
        let lower = |f: f64| f.floor().min(n - 1.) as u32;
        let upper = |f: f64| {
            if f == f.floor() && f > 0. { (f as u32 - 1).min(n as u32 - 1) }
            else { lower(f) }
        };
        let (x0, x1) = (lower(x(b[0])), upper(x(b[2])));
        let (y0, y1) = (lower(y(b[3])), upper(y(b[1])));
        if x1 < x0 || y1 < y0 {
            return Err(err("INVALID_SELECTION", "selection yields empty XYZ tile range"));
        }
        let count = (u64::from(x1) - u64::from(x0) + 1)
            .checked_mul(u64::from(y1) - u64::from(y0) + 1)
            .ok_or("OUT_OF_MEMORY: selection tile count overflow")?;
        if count > MAX_COVERAGE_QUERY_TILES as u64 {
            return Err(err("OUT_OF_MEMORY", "selection tile coverage exceeds query limit"));
        }
        let mut needed = Vec::with_capacity(count as usize);
        for x in x0..=x1 {
            for y in y0..=y1 {
                needed.push((zoom, x, y));
            }
        }
        self.verify_required_tiles(&needed)?;
        Ok(needed.len())
    }
    /// Native registry access; renderer passes no path. Rehashes immutable on-disk archive.
    pub fn read_registered_tile(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
        self.verify_required_tiles(&[(z, x, y)])?;
        let id = tile_id(z, x, y)?;
        let token = &self.manifest.source_archive_sha256;
        let bytes = read_registry_entry(&self.registry_root.join("assets"), token,
            "pmtiles", MAX_ARCHIVE_BYTES as usize)?;
        let record = &self.records[&id];
        let result = decompress(
            section(
                &bytes,
                record.offset,
                u64::from(record.length),
                "registered tile",
            )?,
            self.tile_compression,
            MAX_TILE_BYTES,
        )?;
        valid_webp(&result)?;
        Ok(result)
    }
}
/// Open registry bytes with a bounded handle read and recheck path identity.
 /// The registry directory is an authorized, private native root (never UI-supplied).
fn read_registry_entry(directory: &Path, sha: &str, ext: &str, cap: usize)
    -> Result<Vec<u8>, String>
{
    let d = fs::symlink_metadata(directory)
        .map_err(|_| err("DATA_MISSING", "native registry directory unavailable"))?;
    if !d.is_dir() || d.file_type().is_symlink() {
        return Err(err("INVALID_SOURCE", "registry path must be a trusted directory"));
    }
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(err("INVALID_SOURCE", "untrusted registry identity"));
    }
    let path = directory.join(format!("{sha}.{ext}"));
    let info = fs::symlink_metadata(&path)
        .map_err(|_| err("DATA_MISSING", "registered native token unavailable"))?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > cap as u64 {
        return Err(err("INVALID_SOURCE", "registry item must be a bounded regular file"));
    }
    let mut file = OpenOptions::new().read(true).open(&path)
        .map_err(|_| err("DATA_MISSING", "registered native item unreadable"))?;
    let handle = file.metadata().map_err(|_| err("INVALID_SOURCE", "registry handle invalid"))?;
    if !handle.is_file() || handle.len() != info.len() {
        return Err(err("INVALID_SOURCE", "registry handle metadata changed"));
    }
    let mut bytes = Vec::new();
    file.by_ref().take(cap as u64 + 1).read_to_end(&mut bytes)
        .map_err(|_| err("INVALID_SOURCE", "registered bounded read failed"))?;
    if bytes.len() > cap || bytes.len() as u64 != info.len() {
        return Err(err("INVALID_SOURCE", "registry asset changed or is too large"));
    }
    let latest = fs::symlink_metadata(&path)
        .map_err(|_| err("DATA_MISSING", "registered native item was removed"))?;
    if !latest.is_file() || latest.file_type().is_symlink() || latest.len() != info.len() {
        return Err(err("INVALID_SOURCE", "registered path mutated while reading"));
    }
    if digest(&bytes) != sha {
        return Err(err("HASH_MISMATCH", "registered native token bytes differ from SHA-256"));
    }
    Ok(bytes)
}

/// Native restart/re-hydration from an independently authenticated archive AND index.
/// A serialized manifest alone never proves coverage; the registry must be explicit.
pub fn reopen_registered_pmtiles(registry_root: &Path, manifest: TerrainSourceManifestV1)
    -> Result<ImportedSource, String>
{
    validation::validate_manifest(&manifest)?;
    if manifest.coverage_status != "verified-tile-index" {
        return Err(err("DATA_MISSING", "source has no declared immutable coverage index"));
    }
    let idx_sha = manifest.coverage_index_sha256.as_deref()
        .ok_or("DATA_MISSING: missing registered index identity")?;
    let archive_bytes = read_registry_entry(&registry_root.join("assets"),
        &manifest.source_archive_sha256, "pmtiles", MAX_ARCHIVE_BYTES as usize)?;
    if archive_bytes.len() as u64 != manifest.source_bytes {
        return Err(err("HASH_MISMATCH", "registered source length differs from manifest"));
    }
    let inspected = inspect(&archive_bytes)?;
    if inspected.header.bbox != manifest.coverage_bbox {
        return Err(err("HASH_MISMATCH", "registered source coverage differs from manifest"));
    }
    let stored_index = read_registry_entry(&registry_root.join("indices"), idx_sha,
        "idx", INDEX_MAGIC.len() + 32 + 4 + MAX_INDEX_TILES * 20)?;
    let expected = index_bytes(&manifest.source_archive_sha256, &inspected.tiles)?;
    if stored_index != expected {
        return Err(err("HASH_MISMATCH", "registry index differs from inspected source directory"));
    }
    let count = inspected.tiles.len();
    Ok(ImportedSource {
        coverage_index_token: format!("m2m-index:{idx_sha}"),
        tile_count: count,
        manifest,
        registry_root: registry_root.to_path_buf(),
        records: inspected.tiles,
        tile_compression: inspected.header.tile_compression,
        min_zoom: inspected.header.min_zoom,
        max_zoom: inspected.header.max_zoom,
    })
}

fn registry_put(directory: &Path, sha: &str, extension: &str, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(directory)
        .map_err(|_| err("INVALID_SOURCE", "registry directory creation failed"))?;
    let d = fs::symlink_metadata(directory)
        .map_err(|_| err("INVALID_SOURCE", "registry directory missing"))?;
    if !d.is_dir() || d.file_type().is_symlink() {
        return Err(err(
            "INVALID_SOURCE",
            "registry directory must not be a symlink",
        ));
    }
    let target = directory.join(format!("{sha}.{extension}"));
    if target.exists() {
        read_registry_entry(directory, sha, extension, bytes.len())?;
        return Ok(());
    }
    // OS-entropy, exclusive, unguessable staging name: safe across concurrent
    // threads/processes, and never reused after a crash leaves an orphan.
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random)
        .map_err(|_| err("INVALID_SOURCE", "no secure random source for staging"))?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let temp = directory.join(format!(".{sha}.{extension}.stage-{nonce}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| err("INVALID_SOURCE", "exclusive staging file unavailable"))?;
    let attempt = (|| -> Result<(), String> {
        file.write_all(bytes)
            .map_err(|_| err("INVALID_SOURCE", "staged write failed"))?;
        file.sync_all()
            .map_err(|_| err("INVALID_SOURCE", "staged sync failed"))?;
        // Hard-link creates a new immutable name atomically without replacing another importer.
        match fs::hard_link(&temp, &target) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                read_registry_entry(directory, sha, extension, bytes.len()).map(|_| ())
            }
            Err(_) => Err(err("INVALID_SOURCE", "atomic registry publication failed")),
        }
    })();
    drop(file);
    let _ = fs::remove_file(&temp);
    // On Windows the read-only bit prevents deleting the staging hard link.
    // Publish with a writable staging name, unlink it, then protect the final name.
    if attempt.is_ok() {
        let mut perms = fs::metadata(&target)
            .map_err(|_| err("INVALID_SOURCE", "registered asset metadata"))?
            .permissions();
        perms.set_readonly(true);
        fs::set_permissions(&target, perms)
            .map_err(|_| err("INVALID_SOURCE", "registered readonly flag failed"))?;
    }
    attempt
}
fn index_bytes(source_sha: &str, records: &BTreeMap<u64, TileRecord>) -> Result<Vec<u8>, String> {
    let mut out = INDEX_MAGIC.to_vec();
    let sha = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&source_sha[i..i + 2], 16).unwrap());
    out.extend(sha);
    out.extend(
        u32::try_from(records.len())
            .map_err(|_| err("OUT_OF_MEMORY", "index count"))?
            .to_le_bytes(),
    );
    for (&id, rec) in records {
        out.extend(id.to_le_bytes());
        out.extend(rec.offset.to_le_bytes());
        out.extend(rec.length.to_le_bytes());
    }
    Ok(out)
}
pub fn import_local_pmtiles(
    source_path: &Path,
    registry_root: &Path,
    declaration: &SourceDeclaration,
) -> Result<ImportedSource, String> {
    let info = fs::symlink_metadata(source_path)
        .map_err(|_| err("DATA_MISSING", "local PMTiles file unavailable"))?;
    if !info.is_file()
        || info.file_type().is_symlink()
        || info.len() < HEADER_BYTES as u64
        || info.len() > MAX_ARCHIVE_BYTES
    {
        return Err(err(
            "INVALID_SOURCE",
            "PMTiles input must be a bounded regular file",
        ));
    }
    if declaration.vertical_datum_status != "known"
        || declaration.vertical_datum_id.trim().is_empty()
    {
        return Err(err(
            "UNSUPPORTED_DATUM",
            "source vertical reference is not verified",
        ));
    }
    for text in [
        &declaration.source_name,
        &declaration.release_id,
        &declaration.archive_id,
        &declaration.license_identifier,
        &declaration.attribution,
        &declaration.native_resolution_source,
        &declaration.decoder_version,
    ] {
        if text.trim().is_empty() {
            return Err(err(
                "INVALID_SOURCE",
                "source, rights and resolution declarations are mandatory",
            ));
        }
    }
    if !declaration.nominal_ground_sample_meters.is_finite()
        || declaration.nominal_ground_sample_meters <= 0.
        || declaration
            .uncertainty_meters
            .is_some_and(|x| !x.is_finite() || x < 0.)
    {
        return Err(err(
            "INVALID_SOURCE",
            "nominal resolution or uncertainty invalid",
        ));
    }
    let bytes =
        fs::read(source_path).map_err(|_| err("DATA_MISSING", "PMTiles file cannot be read"))?;
    if bytes.len() as u64 != info.len() {
        return Err(err("INVALID_SOURCE", "archive changed during import"));
    }
    let source_sha = digest(&bytes);
    if declaration
        .expected_archive_sha256
        .as_ref()
        .is_some_and(|expected| expected != &source_sha)
    {
        return Err(err(
            "HASH_MISMATCH",
            "expected archive SHA-256 differs from source bytes",
        ));
    }
    let archive = inspect(&bytes)?;
    let index = index_bytes(&source_sha, &archive.tiles)?;
    let index_sha = digest(&index);
    let mut manifest = TerrainSourceManifestV1 {
        schema_version: "1.0.0".into(),
        kind: "TerrainSourceManifestV1".into(),
        source_name: declaration.source_name.clone(),
        release_id: declaration.release_id.clone(),
        archive_id: declaration.archive_id.clone(),
        local_asset_token: format!("m2m-asset:{source_sha}"),
        source_archive_sha256: source_sha,
        manifest_sha256: String::new(),
        coverage_status: "verified-tile-index".into(),
        coverage_index_sha256: Some(index_sha.clone()),
        vertical_datum_status: "known".into(),
        coverage_bbox: archive.header.bbox,
        horizontal_crs: "EPSG:4326".into(),
        vertical_datum_id: declaration.vertical_datum_id.clone(),
        nominal_ground_sample_meters: declaration.nominal_ground_sample_meters,
        native_resolution_source: declaration.native_resolution_source.clone(),
        uncertainty_meters: declaration.uncertainty_meters,
        source_bytes: bytes.len() as u64,
        license_identifier: declaration.license_identifier.clone(),
        attribution: declaration.attribution.clone(),
        decoder_version: declaration.decoder_version.clone(),
    };
    manifest.manifest_sha256 = canonical::manifest_digest(&manifest)?;
    validation::validate_manifest(&manifest)?;
    // Identity is fixed before registration; no renderer-controlled path/token is returned.
    registry_put(
        &registry_root.join("assets"),
        &manifest.source_archive_sha256,
        "pmtiles",
        &bytes,
    )?;
    registry_put(&registry_root.join("indices"), &index_sha, "idx", &index)?;
    Ok(ImportedSource {
        manifest,
        coverage_index_token: format!("m2m-index:{index_sha}"),
        tile_count: archive.tiles.len(),
        registry_root: registry_root.to_path_buf(),
        records: archive.tiles,
        tile_compression: archive.header.tile_compression,
        min_zoom: archive.header.min_zoom,
        max_zoom: archive.header.max_zoom,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const WEBP: &[u8] = &[
        0x52, 0x49, 0x46, 0x46, 0x1c, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50, 0x38,
        0x4c, 0x0f, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x07, 0x10, 0x05, 0x8c, 0xfe,
        0x07, 0x22, 0xa2, 0xff, 0x01, 0x00,
    ]; // Independently created 1x1 lossless WebP RGB fixture (Pillow), original synthetic pixels.
    fn put_u64(header: &mut [u8], at: usize, n: u64) {
        header[at..at + 8].copy_from_slice(&n.to_le_bytes());
    }
    fn put_i32(header: &mut [u8], at: usize, n: i32) {
        header[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        // Two actual z=1 WebP tiles: (0,0) ID1, (0,1) ID2.
        // Directory: n2, delta ids 1,1, runs 1,1, lengths 36,36, offsets 0/contiguous.
        let dir = [2_u8, 1, 1, 1, 1, 36, 36, 1, 0];
        let meta = br#"{"encoding":"terrarium","name":"Synthetic legal test raster","attribution":"Artificial data"}"#;
        let tile_start = (HEADER_BYTES + dir.len() + meta.len()) as u64;
        let mut out = vec![0_u8; HEADER_BYTES];
        out[..7].copy_from_slice(b"PMTiles");
        out[7] = 3;
        put_u64(&mut out, 8, HEADER_BYTES as u64);
        put_u64(&mut out, 16, dir.len() as u64);
        put_u64(&mut out, 24, (HEADER_BYTES + dir.len()) as u64);
        put_u64(&mut out, 32, meta.len() as u64);
        put_u64(&mut out, 40, tile_start);
        put_u64(&mut out, 48, 0);
        put_u64(&mut out, 56, tile_start);
        put_u64(&mut out, 64, (WEBP.len() * 2) as u64);
        put_u64(&mut out, 72, 2);
        put_u64(&mut out, 80, 2);
        put_u64(&mut out, 88, 2);
        out[96] = 1;
        out[97] = 1;
        out[98] = 1;
        out[99] = 4;
        out[100] = 1;
        out[101] = 1;
        put_i32(&mut out, 102, -840900000);
        put_i32(&mut out, 106, 397400000);
        put_i32(&mut out, 110, -840800000);
        put_i32(&mut out, 114, 397500000);
        out.extend(dir);
        out.extend(meta);
        out.extend(WEBP);
        out.extend(WEBP);
        out
    }

    fn encoded_varint(mut n: u64, out: &mut Vec<u8>) {
        while n >= 128 {
            out.push((n as u8 & 127) | 128);
            n >>= 7;
        }
        out.push(n as u8);
    }
    fn synthetic_directory(entries: &[(u64, u64, u64, u64)]) -> Vec<u8> {
        let mut out = Vec::new();
        encoded_varint(entries.len() as u64, &mut out);
        let mut previous_id = 0;
        for &(id, _, _, _) in entries {
            encoded_varint(id - previous_id, &mut out);
            previous_id = id;
        }
        for &(_, run, _, _) in entries { encoded_varint(run, &mut out); }
        for &(_, _, len, _) in entries { encoded_varint(len, &mut out); }
        let mut last_end = None;
        for &(_, _, len, offset) in entries {
            let wire = if last_end == Some(offset) { 0 } else { offset + 1 };
            encoded_varint(wire, &mut out);
            last_end = Some(offset + len);
        }
        out
    }
    fn zipped(bytes: &[u8]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        let mut writer = GzEncoder::new(Vec::new(), flate2::Compression::default());
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap()
    }
    /// Two WebP tile payloads under a one-level leaf: gzip is actually applied
    /// to root, leaf, metadata, and each individual WebP tile.
    fn leaf_gzip_fixture(parent_start: u64, extra_root: bool) -> Vec<u8> {
        let tile = zipped(WEBP);
        let tile2_offset = tile.len() as u64;
        let tile_entries = [(1, 1, tile.len() as u64, 0),
            (2, 1, tile.len() as u64, tile2_offset)];
        let leaf = zipped(&synthetic_directory(&tile_entries));
        let roots = if extra_root {
            // Second root starts at ID2: child ID2 in first leaf is unreachable.
            synthetic_directory(&[(parent_start, 0, leaf.len() as u64, 0),
                (2, 1, tile.len() as u64, tile2_offset)])
        } else {
            synthetic_directory(&[(parent_start, 0, leaf.len() as u64, 0)])
        };
        let root = zipped(&roots);
        let meta = zipped(br#"{"encoding":"terrarium","attribution":"Synthetic fixture"}"#);
        let root_start = HEADER_BYTES as u64;
        let meta_start = root_start + root.len() as u64;
        let leaf_start = meta_start + meta.len() as u64;
        let tile_start = leaf_start + leaf.len() as u64;
        let mut out = vec![0; HEADER_BYTES];
        out[..7].copy_from_slice(b"PMTiles");
        out[7] = 3;
        put_u64(&mut out, 8, root_start);
        put_u64(&mut out, 16, root.len() as u64);
        put_u64(&mut out, 24, meta_start);
        put_u64(&mut out, 32, meta.len() as u64);
        put_u64(&mut out, 40, leaf_start);
        put_u64(&mut out, 48, leaf.len() as u64);
        put_u64(&mut out, 56, tile_start);
        put_u64(&mut out, 64, tile.len() as u64 * 2);
        put_u64(&mut out, 72, 2);
        put_u64(&mut out, 80, 2);
        put_u64(&mut out, 88, 2);
        out[96] = 1; out[97] = 2; out[98] = 2; out[99] = 4;
        out[100] = 1; out[101] = 1;
        put_i32(&mut out, 102, -840900000);
        put_i32(&mut out, 106, 397400000);
        put_i32(&mut out, 110, -840800000);
        put_i32(&mut out, 114, 397500000);
        out.extend(root); out.extend(meta); out.extend(leaf);
        out.extend(tile.clone()); out.extend(tile);
        out
    }
    #[test]
    fn gzip_leaf_fixture_is_reachable_and_parent_boundaries_are_enforced() {
        let valid = leaf_gzip_fixture(1, false);
        let inspected = inspect(&valid).unwrap();
        assert_eq!(inspected.tiles.len(), 2);
        assert!(inspected.tiles.contains_key(&tile_id(1, 0, 0).unwrap()));
        assert!(inspected.tiles.contains_key(&tile_id(1, 0, 1).unwrap()));
        assert!(inspect(&leaf_gzip_fixture(2, false)).unwrap_err().starts_with("INVALID_SOURCE"));
        assert!(inspect(&leaf_gzip_fixture(1, true)).unwrap_err().starts_with("INVALID_SOURCE"));
        let mut corrupt = valid.clone();
        corrupt[HEADER_BYTES] ^= 0xff; // Deflate root no longer decodes to a directory.
        assert!(inspect(&corrupt).is_err());
        let root = temporary();
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> Result<(), String> {
            let path = root.join("compressed.pmtiles");
            fs::write(&path, &valid).unwrap();
            let source = import_local_pmtiles(&path, &root.join("reg"), &declaration())?;
            assert_eq!(source.read_registered_tile(1, 0, 1)?, WEBP);
            Ok(())
        })();
        let _ = fs::remove_dir_all(root);
        result.unwrap();
    }
    #[test]
    fn restart_hydrates_verified_bytes_checks_coverage_and_detects_missing_or_tampered_index() {
        let root = temporary();
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> Result<(), String> {
            let path = root.join("input.pmtiles");
            fs::write(&path, fixture()).unwrap();
            let registry = root.join("registered");
            let imported = import_local_pmtiles(&path, &registry, &declaration())?;
            let manifest = imported.manifest.clone();
            drop(imported); // Simulate process restart: no cached records.
            let reopened = reopen_registered_pmtiles(&registry, manifest.clone())?;
            let selection: SelectionV1 = serde_json::from_str(include_str!(
                "../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"
            )).unwrap();
            assert_eq!(reopened.verify_selection_coverage(&selection, 1)?, 1);
            assert!(reopened.verify_selection_coverage(&selection, 2).is_err());
            assert!(reopened.verify_required_tiles(&[(1, 1, 0)]).is_err());
            assert_eq!(reopened.read_registered_tile(1, 0, 0)?, WEBP);
            let idx = registry.join("indices")
                .join(format!("{}.idx", manifest.coverage_index_sha256.as_ref().unwrap()));
            fs::remove_file(&idx).map_err(|_| "INVALID_SOURCE: cannot remove synthetic index")?;
            assert!(reopen_registered_pmtiles(&registry, manifest.clone()).is_err());
            assert!(reopened.verify_required_tiles(&[(1, 0, 0)]).is_err());
            let rebuilt = import_local_pmtiles(&path, &registry, &declaration())?;
            assert_eq!(rebuilt.manifest.manifest_sha256, manifest.manifest_sha256);
            Ok(())
        })();
        let _ = fs::remove_dir_all(root);
        result.unwrap();
    }
    #[test]
    fn registration_handles_parallel_imports_and_abandoned_exclusive_stage_files() {
        let root = temporary();
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> Result<(), String> {
            let path = root.join("input.pmtiles");
            fs::write(&path, fixture()).unwrap();
            let registry = root.join("library");
            // A crashed prior import cannot reserve a known staging slot forever.
            fs::create_dir_all(registry.join("assets")).unwrap();
            let crashed = registry.join("assets").join(".stale.stage-previous");
            fs::write(&crashed, b"partial").unwrap();
            let mut workers = Vec::new();
            for _ in 0..5 {
                let src = path.clone();
                let dest = registry.clone();
                workers.push(std::thread::spawn(move || {
                    import_local_pmtiles(&src, &dest, &declaration())
                        .map(|imported| imported.manifest.manifest_sha256)
                }));
            }
            let hashes: Vec<String> = workers.into_iter()
                .map(|thread| thread.join().unwrap().unwrap()).collect();
            assert!(hashes.iter().all(|hash| hash == &hashes[0]));
            assert_eq!(fs::read(crashed).unwrap(), b"partial");
            Ok(())
        })();
        let _ = fs::remove_dir_all(root);
        result.unwrap();
    }

    fn declaration() -> SourceDeclaration {
        SourceDeclaration {
            source_name: "Synthetic Mapterhorn-format".into(),
            release_id: "synthetic-v1".into(),
            archive_id: "z1-west-two".into(),
            expected_archive_sha256: None,
            vertical_datum_id: "SYNTHETIC-RELATIVE-METERS".into(),
            vertical_datum_status: "known".into(),
            license_identifier: "CC0-1.0".into(),
            attribution: "Original artificial fixture, not Mapterhorn data".into(),
            nominal_ground_sample_meters: 12.,
            native_resolution_source: "artificial nominal spacing".into(),
            uncertainty_meters: Some(0.),
            decoder_version: "map2model-data/0.0.1".into(),
        }
    }
    fn temporary() -> PathBuf {
        std::env::temp_dir().join(format!(
            "m2m-a2-01-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn hilbert_tile_ids_follow_pmtiles_v3_reference() {
        for (z, x, y, id) in [
            (0, 0, 0, 0),
            (1, 0, 0, 1),
            (1, 0, 1, 2),
            (1, 1, 1, 3),
            (1, 1, 0, 4),
            (2, 0, 0, 5),
        ] {
            assert_eq!(tile_id(z, x, y).unwrap(), id);
        }
        assert!(tile_id(1, 2, 0).is_err());
    }
    #[test]
    fn imports_real_directory_entries_registers_verified_tokens_and_fails_closed() {
        let root = temporary();
        fs::create_dir_all(&root).unwrap();
        let input = root.join("input.pmtiles");
        let bytes = fixture();
        fs::write(&input, &bytes).unwrap();
        let result = (|| -> Result<(), String> {
            let source = import_local_pmtiles(&input, &root.join("library"), &declaration())?;
            assert_eq!(source.tile_count, 2);
            assert_eq!(source.manifest.source_archive_sha256, digest(&bytes));
            assert_eq!(
                source.manifest.manifest_sha256,
                canonical::manifest_digest(&source.manifest)?
            );
            assert!(source.coverage_index_token.starts_with("m2m-index:"));
            assert!(source.manifest.coverage_index_sha256.is_some());
            source.verify_required_tiles(&[(1, 0, 0), (1, 0, 1)])?;
            assert!(source
                .verify_required_tiles(&[(1, 1, 0)])
                .unwrap_err()
                .starts_with("DATA_MISSING"));
            assert_eq!(source.read_registered_tile(1, 0, 0)?, WEBP);
            // Re-import is deterministic and does not replace an existing registered asset.
            let again = import_local_pmtiles(&input, &root.join("library"), &declaration())?;
            assert_eq!(
                again.manifest.manifest_sha256,
                source.manifest.manifest_sha256
            );
            let registered = root
                .join("library/assets")
                .join(format!("{}.pmtiles", source.manifest.source_archive_sha256));
            let mut changed = bytes.clone();
            *changed.last_mut().unwrap() ^= 1;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&registered, fs::Permissions::from_mode(0o600)).unwrap();
            }
            #[cfg(windows)]
            {
                let mut perms = fs::metadata(&registered).unwrap().permissions();
                // Windows fixture tampering is deliberate; Unix uses narrow mode 0600 above.
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                fs::set_permissions(&registered, perms).unwrap();
            }
            fs::write(&registered, &changed).unwrap();
            assert!(source
                .read_registered_tile(1, 0, 0)
                .unwrap_err()
                .starts_with("HASH_MISMATCH"));
            assert!(
                import_local_pmtiles(&input, &root.join("library"), &declaration())
                    .unwrap_err()
                    .starts_with("HASH_MISMATCH")
            );
            Ok(())
        })();
        let _ = fs::remove_dir_all(&root);
        result.unwrap();
    }
    #[test]
    fn rejects_corruption_unsupported_source_and_unestablished_datum() {
        let input = fixture();
        let mut bad = input.clone();
        bad[7] = 2;
        assert!(inspect(&bad).is_err());
        let mut bad = input.clone();
        bad[99] = 2;
        assert!(inspect(&bad).is_err());
        let mut bad = input.clone();
        put_u64(&mut bad, 16, u64::MAX);
        assert!(inspect(&bad).is_err());
        let mut bad = input.clone();
        bad[HEADER_BYTES + 2] = 0; // duplicate Hilbert IDs
        assert!(inspect(&bad).is_err());
        let mut bad = input.clone();
        bad[HEADER_BYTES + 7] = 127; // bogus tile-data offset
        assert!(inspect(&bad).is_err());
        let mut bad = input.clone();
        bad[HEADER_BYTES + 2 + 7] = 0;
        assert!(inspect(&bad).is_err()); // malformed data byte
        let mut policy = declaration();
        policy.vertical_datum_status = "unknown".into();
        let root = temporary();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("archive.pmtiles"), &input).unwrap();
        assert!(
            import_local_pmtiles(&root.join("archive.pmtiles"), &root.join("lib"), &policy)
                .unwrap_err()
                .starts_with("UNSUPPORTED_DATUM")
        );
        policy = declaration();
        policy.expected_archive_sha256 = Some("0".repeat(64));
        assert!(
            import_local_pmtiles(&root.join("archive.pmtiles"), &root.join("lib"), &policy)
                .unwrap_err()
                .starts_with("HASH_MISMATCH")
        );
        let _ = fs::remove_dir_all(root);
    }
}
