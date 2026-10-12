//! Format-only geodata coverage for a *user-picked local* PMTiles archive.
//! This is a visual envelope + indexed-tile check, not elevation, datum,
//! licensing or export verification. No frozen V1 contract is modified.
use crate::experimental::read_private_source;
use crate::{
    digest, err, inspect_archive, tile_id, tile_id_in_zoom, MAX_COVERAGE_QUERY_TILES, MAX_ZOOM,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

/// The desktop may pass a raw *drawn* rectangle before it becomes a hashed
/// SelectionV1. Requiring exact source SHA on every intersection call ensures
/// a file mutated after the file-picker inspection is never silently reused.
#[derive(Debug, Clone)]
pub struct CoverageSelectionQuery {
    pub expected_source_sha256: String,
    pub bbox_wgs84: [f64; 4],
    pub zoom: u8,
}

fn valid_drawn_bbox(b: [f64; 4]) -> bool {
    b.iter().all(|v| v.is_finite())
        && b[0] >= -180.0
        && b[2] <= 180.0
        && b[1] >= -60.0
        && b[3] <= 60.0
        && b[0] < b[2]
        && b[1] < b[3]
        // Archive implementation doesn't support antimeridian-wrapping.
        && b[2] - b[0] < 180.0
}

// Mirror the importer/experimental decoder CLOSED-geographic-rectangle
// edge-ownership rule: both adjacent tiles are required when the bbox edge
// lands exactly on a Web Mercator tile seam.
fn required_xyz_range(b: [f64; 4], zoom: u8) -> Result<(u32, u32, u32, u32), String> {
    if zoom > MAX_ZOOM {
        return Err(err("INVALID_SELECTION", "zoom outside supported PMTiles domain"));
    }
    let n = (1_u64 << zoom) as f64;
    let mx = |lon: f64| ((lon + 180.0) * n / 360.0).clamp(0.0, n);
    let my = |lat: f64| {
        let p = lat.to_radians();
        ((1.0 - (p.tan() + 1.0 / p.cos()).ln() / std::f64::consts::PI) / 2.0 * n)
            .clamp(0.0, n)
    };
    let low = |f: f64| {
        let seam = f.round();
        if (f - seam).abs() <= 1e-9 && seam > 0.0 && seam < n {
            seam as u32 - 1
        } else {
            f.floor().min(n - 1.0) as u32
        }
    };
    let high = |f: f64| {
        let seam = f.round();
        if (f - seam).abs() <= 1e-9 {
            seam.min(n - 1.0) as u32
        } else {
            f.floor().min(n - 1.0) as u32
        }
    };
    let x0 = low(mx(b[0]));
    let x1 = high(mx(b[2]));
    let y0 = low(my(b[3]));
    let y1 = high(my(b[1]));
    if x0 > x1 || y0 > y1 {
        return Err(err("INVALID_SELECTION", "drawn bbox has no tile coverage"));
    }
    Ok((x0, x1, y0, y1))
}

/// Inspect the *declared* WGS84 header boundary for a map overlay, and
/// optionally test a drawn bbox against BOTH the header envelope and actual
/// PMTiles index at the requested zoom. No consent is needed to draw a
/// local file's footprint; explicit per-job consent is still mandatory
/// for experimental elevation sampling and 3MF export.
pub fn inspect_local_pmtiles_coverage(
    file: &Path,
    request: Option<&CoverageSelectionQuery>,
) -> Result<Value, String> {
    // Reuse the same bounded, symlink-rejecting, regular-file read as the
    // experimental candidate decoder. This is deliberately offline.
    let bytes = read_private_source(file)?;
    let sha = digest(&bytes);
    if let Some(q) = request {
        if q.expected_source_sha256 != sha {
            return Err(err(
                "HASH_MISMATCH",
                "local PMTiles changed since coverage file selection",
            ));
        }
        if !valid_drawn_bbox(q.bbox_wgs84) || q.zoom > MAX_ZOOM {
            return Err(err("INVALID_SELECTION", "drawn WGS84 bbox or zoom invalid"));
        }
    }
    // Checks the entire bounded index and WebP container framing; DOES NOT
    // decode pixel alpha masks or establish usable elevations at this point.
    let archive = inspect_archive(&bytes, false)?;
    let h = &archive.header;
    let e = h.bbox;
    let mut indexed_tiles_by_zoom = BTreeMap::<String, usize>::new();
    for id in archive.tiles.keys() {
        let z = (h.min_zoom..=h.max_zoom)
            .find(|z| tile_id_in_zoom(*id, *z, *z))
            .ok_or_else(|| err("INVALID_SOURCE", "unresolvable indexed tile zoom"))?;
        *indexed_tiles_by_zoom.entry(z.to_string()).or_default() += 1;
    }
    let selection_info = if let Some(q) = request {
        let b = q.bbox_wgs84;
        let intersects_declared =
            b[0] < e[2] && b[2] > e[0] && b[1] < e[3] && b[3] > e[1];
        let inside_declared =
            b[0] >= e[0] && b[1] >= e[1] && b[2] <= e[2] && b[3] <= e[3];
        let geographic_intersection = if intersects_declared {
            json!([b[0].max(e[0]), b[1].max(e[1]), b[2].min(e[2]), b[3].min(e[3])])
        } else {
            Value::Null
        };
        let (x0, x1, y0, y1) = required_xyz_range(b, q.zoom)?;
        let required = (u64::from(x1) - u64::from(x0) + 1)
            .checked_mul(u64::from(y1) - u64::from(y0) + 1)
            .ok_or("OUT_OF_MEMORY: coverage query tile count overflow")?;
        if required > MAX_COVERAGE_QUERY_TILES as u64 {
            return Err(err("OUT_OF_MEMORY", "drawn rectangle exceeds indexed-tile query cap"));
        }
        let mut indexed = 0_u64;
        let zoom_in_archive = q.zoom >= h.min_zoom && q.zoom <= h.max_zoom;
        if zoom_in_archive {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if archive.tiles.contains_key(&tile_id(q.zoom, x, y)?) {
                        indexed += 1;
                    }
                }
            }
        }
        // Do NOT call an envelope-only overlap "inside" when a leaf/tile is
        // absent. Also don't declare an "outside" selection "partial" based
        // only on a nonempty bounding-box intersection with zero source tiles.
        let status = if !intersects_declared || indexed == 0 {
            "outside"
        } else if inside_declared && indexed == required {
            "inside"
        } else {
            "partial"
        };
        json!({
            "bboxWgs84": b,
            "requestedZoom": q.zoom,
            "zoomAvailable": zoom_in_archive,
            "status": status,
            "insideDeclaredBbox": inside_declared,
            "intersectsDeclaredBbox": intersects_declared,
            "intersectionWithDeclaredBboxWgs84": geographic_intersection,
            "requiredClosedBoundaryTiles": required,
            "indexedRequiredTiles": indexed,
            "missingRequiredTiles": required - indexed,
            "allRequiredTilesIndexed": required == indexed,
            "classificationScope": "declared-header-envelope-and-real-index-only; alpha/nodata/height samples NOT verified"
        })
    } else {
        Value::Null
    };
    Ok(json!({
        "kind": "LocalPMTilesCoveragePreviewV1",
        "experimentalOnly": true,
        "sourceArchiveSha256": sha,
        "sourceArchiveBytes": bytes.len(),
        "declaredCoverageBboxWgs84": e,
        "boundarySource": "PMTiles-v3-header-reported-bbox; may overstate actual tile coverage",
        "coverageGeoJson": {
            "type": "Feature",
            "properties": {
                "kind": "declared-header-envelope",
                "unverified": true
            },
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [e[0], e[1]],
                    [e[2], e[1]],
                    [e[2], e[3]],
                    [e[0], e[3]],
                    [e[0], e[1]]
                ]]
            }
        },
        "headerMinZoom": h.min_zoom,
        "headerMaxZoom": h.max_zoom,
        "indexedTileCount": archive.tiles.len(),
        "indexedTilesByZoom": indexed_tiles_by_zoom,
        "selection": selection_info,
        "sourceLicenseVerified": false,
        "verticalDatumStatus": "unknown",
        "nativeResolutionVerified": false,
        "productionGenerationReady": false,
        "networkRequests": 0
    }))
}
