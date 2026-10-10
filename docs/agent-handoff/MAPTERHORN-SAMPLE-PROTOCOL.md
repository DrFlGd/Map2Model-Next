# Real Mapterhorn sample acquisition and evidence protocol (A2-01/A2-02)

**Issued:** 2026-10-10. **Status:** confirmed public endpoint/format information, **no downloaded or validated production sample, datum or source-specific license clearance**. The old private Map2Model prototype did not commit a reusable terrain archive; it cached per-request remote geographic responses locally. This protocol uses **official public Mapterhorn sources**, not the proprietary mirrored Mini Skyline implementation.

## Official source information verified

- [Data access / PMTiles regional extraction](https://mapterhorn.com/data-access/): Terrarium-encoded 512x512 terrain RGB tiles packaged as PMTiles; standalone Z/X/Y WebP tiles available at `https://tiles.mapterhorn.com/{z}/{x}/{y}.webp`; TileJSON at `https://tiles.mapterhorn.com/tilejson.json`.
- [Interlaken example](https://mapterhorn.com/data-access/): for Swiss Interlaken, the upstream documentation identifies `https://download.mapterhorn.com/6-33-22.pmtiles` (regional z13-z17 source) and `https://download.mapterhorn.com/planet.pmtiles` (z0-z12). Download catalog: `https://download.mapterhorn.com/download_urls.json`; checksums/archives can change, so snapshot evidence when acquiring.
- [Attribution and upstream DEM sources](https://mapterhorn.com/attribution/): source-specific catalog `https://download.mapterhorn.com/attribution.json`. Mapterhorn **code license is not a blanket terrain-data license**; upstream datasets differ. Document the actual source(s) covering the selected region and derivative/redistribution conditions.
- [Open vertical-reference investigation](https://github.com/mapterhorn/mapterhorn/issues/255): an upstream concern was raised about mixed vertical references. **Do not assume EPSG:3855, EGM2008, geoid or ellipsoidal heights** without reliable, region-specific proof and/or a documented transformation. Horizontal EPSG:4326 tile coordinates do not establish the vertical datum.
- Official [PMTiles CLI](https://github.com/protomaps/go-pmtiles), [PMTiles v3 specification](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md). The archive format and its metadata must be **inspected**, not guessed from URL suffix.

The older Map2Model wrapper's local geographic cache stored URL/range-keyed responses (`%APPDATA%\map2model\GeoCache` by default on Windows). If the owner has a cache, it may contain real standalone WebP samples useful for **format-only** tests, but those blobs do not become a PMTiles archive or a provenance-cleared dataset automatically. Do not copy the old bundled website/application code.

## Proposed bounded real-data fixture (not executed yet)

The official documentation's larger example bounding box is `7.74,46.60,7.96,46.75` (west,south,east,north). For a *smaller candidate* in the same area use:

```bash
# Acquire online only in a separate developer/source-preparation step.
# Verify the current documented PMTiles CLI version and --dry-run semantics.
pmtiles extract --dry-run --bbox=7.88,46.68,7.89,46.69 \
  https://download.mapterhorn.com/6-33-22.pmtiles \
  interlaken-test.pmtiles

# Only execute if the estimate and dataset terms are acceptable:
pmtiles extract --bbox=7.88,46.68,7.89,46.69 \
  https://download.mapterhorn.com/6-33-22.pmtiles \
  interlaken-test.pmtiles

# Record the exact archive bytes, length and CLI version:
pmtiles --version
sha256sum interlaken-test.pmtiles
```

On Windows PowerShell, use `Get-FileHash .\interlaken-test.pmtiles -Algorithm SHA256` and `(Get-Item .\interlaken-test.pmtiles).Length`. This region is only a candidate; confirm extracted presence at intended zooms and archive size. Agent 2's importer currently limits input to **128 MiB** and **200,000 indexed tile IDs**; stop and shrink the bbox/zoom if needed. Neither URL nor proposed bbox is a release-pinned data identity. Do not commit real sample bytes to this **public** repository without explicit clearance.

**Note:** `pmtiles extract` may alter the output archive's layout, root/leaf levels or gzip flags relative to its source. Check actual output header, TileJSON/metadata, WebP signatures and 512x512 decoded dimensions; if importer rejects a valid Mapterhorn extract, report a bounded supported-format incompatibility rather than forging metadata.

## Immutable acquisition worksheet (keep with the test report, not as a forged V1 manifest)

Create a private, structured `acquisition.json` with these fields and populate them from observed evidence (null means unverified):

```json
{
  "fixtureId": "interlaken-2026-10-10-candidate-UNVERIFIED",
  "sourceArchiveUrl": "https://download.mapterhorn.com/6-33-22.pmtiles",
  "sourceArchiveReleaseOrVersion": null,
  "sourceArchiveEtag": null,
  "sourceArchivePublishedChecksum": null,
  "extractionBboxWestSouthEastNorth": [7.88, 46.68, 7.89, 46.69],
  "pmtilesCliVersion": null,
  "acquiredUtc": null,
  "extractedArchiveSha256": null,
  "extractedArchiveBytes": null,
  "verifiedTileZooms": [],
  "webpImageWidth": null,
  "webpImageHeight": null,
  "heightEncoding": "Terrarium (documented; verify actual pixels)",
  "horizontalCrs": "EPSG:4326 (tile geographic bounds)",
  "verticalDatumId": null,
  "verticalDatumEvidenceUrl": null,
  "sourceDatasetIds": [],
  "perSourceLicenseAndAttributionEvidence": [],
  "nativeResolutionMeters": null,
  "nativeResolutionEvidence": null,
  "approvedForRedistribution": false,
  "approvedForProductionGeneration": false
}
```

This worksheet is **not** `TerrainSourceManifestV1`, which has fixed frozen field names and semantic gates. Do not set `verticalDatumStatus=known` or emit a signed/ready V1 manifest using guesses. If the vertical reference cannot be verified, test parsing/image decoding as **untrusted source format**, and exercise the approved `UNSUPPORTED_DATUM` failure on production import/generation.

## Evidence and test plan for Agent 2

1. Save exact provenance above and compute SHA-256 of each acquired/extracted file; verify against any current published checksum when meaningful. Keep retrieval time and upstream response headers if available; do not imply an unversioned URL is immutable.
2. Check actual PMTiles v3 header/metadata, one-level leaf and supported compression modes, tile IDs/coverage and WebP image dimensions (512x512). Decode known pixel samples with the documented Terrarium formula `elevation_m = R*256 + G + B/256 - 32768`, validating signed heights and pixel channel order with independent fixtures.
3. Verify geographic tile coverage for a rectangle **including exact Z/X/Y tile edges**. The decoder sample-to-tile ownership rules must match coverage checks. The importer must reject any absent required neighbor and report `DATA_MISSING`.
4. Verify source-specific provenance, rights to use/derive a terrain model, and trustworthy vertical reference. The attribution catalog by itself is **not** proof of the sample's vertical datum or the exact upstream DEM that contributed at every pixel.
5. Only **after** production source identity/datum/rights are established may the import create a real `TerrainSourceManifestV1` with known datum. Then exercise offline (no HTTP/DNS) file import -> verified coverage -> `ElevationGridV1` (reproducible hash) -> Agent 3 mesh.
6. Produce separate evidence for (a) synthetic fixture passing, (b) real-format image/PMTiles parsing, (c) verified real datum/license, (d) released application offline test. Never label (b) as (c) or (d).

**Deliverables:** PR #14 boundary/integration fix and CI; issue #4 elevation decoder fixtures/tests; exact acquisition report and relevant license/datum evidence in issue #3 without attaching uncleared source data. Owner/lead decides real-source acceptance and release. This protocol does **not** authorize public-data scraping, unchecked redistribution or automatic downloads by the local-first app.
