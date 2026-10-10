# MAP2MODEL NEXT
## Local-First Geographic-to-3D-Print Application
### Product Requirements & Technical Architecture Specification

**Document ID:** M2M-REQ-001  
**Version:** 1.1 — Multi-agent delivery edition  
**Date:** 10 October 2026  
**Primary platform:** Windows 11 (Windows 10 feasibility to be determined)  
**Status:** Design proposal and multi-agent implementation handoff; not an implementation or approved licensing opinion  
**Audience:** Product owner, desktop engineers, GIS/data engineers, computational geometry engineers, QA, and maintainers

> **Design principle:** A saved project and its pinned local data must remain usable when every outside geographic service is unreachable. Data acquisition, editable modeling, preview, export, and audit history are separate products of the same deterministic pipeline.

---

# 1. Executive decision and design baseline

Map2Model Next is a **new application developed from the ground up**, informed by the successful Earth-mode Map2Model prototype but **not dependent on its mirrored, minified Mini Skyline runtime**. It lets a user browse a map, select a rectangular, circular, or polygonal region, acquire or import permitted geographic data, transform that data into printable geometry, inspect and refine the model, and export a verified 3MF without an online account or network connection after the required region is locally available.

The central architecture is **local-first rather than browser-cache-first**. A local geospatial engine owns data access and model geometry, a versioned data library owns downloaded/imported sources, and the user interface reads those same local resources. The product must distinguish a map that *looks* cached from a region whose elevation, building, road, and land-use inputs are truly sufficient for an offline model.

**Proposed technology baseline:** Electron + TypeScript/React + MapLibre GL JS for a new desktop UI; a separately executable Rust geospatial/mesh engine behind narrow typed IPC; SQLite for catalog, jobs, spatial coverage, and project indexing; read-only PMTiles/GeoParquet/GeoTIFF-derived assets for bulk datasets; an independently validated 3MF writer (prefer lib3mf if packaging and bindings pass the proof of concept). Alternatives are evaluated in §13.

**The first shipping target is Earth, not planets.** The prior prototype's Earth model generation works, while planetary maps and search were reported unreliable. The new app may later introduce celestial bodies via separate datum/provider adapters; nothing in the Earth-only MVP shall silently assume Martian or lunar geodesy.

## 1.1 Decisions that should be made now

| Decision | Proposed baseline | Rationale |
|---|---|---|
| Primary operation | Offline-capable local project and data store | Stable, reproducible printing without cloud availability |
| Main model data | Overture-derived buildings, transportation, base; Mapterhorn terrain | Matches the uploaded 3MF source manifest |
| Bulk data strategy | Official Overture GeoParquet extracts; Mapterhorn PMTiles area extracts | Avoid reliance on inspection-only Overture PMTiles and scraping of public map servers |
| Basemap | Locally hosted licensed OSM/Protomaps-style basemap | Search/display independent of model generation |
| Cache model | Immutable source snapshots + content-addressed objects + spatial manifest | Recoverable, versioned, range-safe and auditable |
| Region acquisition | Select area → coverage estimate → plan → download/import → verify → pin | Highest *native* detail, explicit storage cost, no false completeness |
| Primary output | Multi-component, standards-valid 3MF; STL optional | Material/feature grouping and source provenance |
| User account | Never required for core operations | Local-first/privacy objective |
| Prototype code reuse | Only under confirmed license; refactor conceptually otherwise | Mirrored website's redistribution/reuse rights are unverified |

## 1.2 Evidence versus proposed design

**Observed in user-provided artifacts:** The supplied Mini Skyline 3MF's `Metadata/miniskylines_sources.json` identifies application version 1.206 and a pinned Overture release `2026-08-19.0`. It records Mapterhorn zoom-15 elevation, Overture buildings/transportation/base/addresses, no LiDAR, and an unsuccessful optional OSM bridge supplement. The sample's embedded WGS84 coordinates are near Dayton, Ohio despite the `waikiki-honolulu-1km.3mf` filename. Treat the embedded bounds and source metadata—not the filename—as authoritative for regression tests.

**Provider-documented facts:** Overture publishes official GeoParquet datasets and inspection-oriented PMTiles; it explicitly advises against relying on its published PMTiles as a production interface [S1]. Mapterhorn provides Terrarium-encoded WebP elevation tiles and extractable PMTiles archives including high-zoom regional files [S2]. PMTiles is a read-only tile archive supporting byte-range access [S3]. Overture buildings, base, and transportation are ODbL datasets; other themes have different licensing [S4]. 3MF has a public interoperable specification and conformance suite [S5].

**Recommendations in this specification:** Technology stack, caching scheme, priorities, numeric targets, acceptance tests, download policy, data contracts, and schedule are proposed engineering choices. They have not been validated as already implemented in Map2Model.

# 2. Product objectives, boundaries, and success measures

## 2.1 Goals

G-01. Build printable topographic and city models from Earth terrain plus mapped features with the user controlling physical size, scale, height treatment, minimum thickness, colors/parts, and support/base options.

G-02. Make offline work a first-class state: once a region is marked **Generation Ready**, open, edit, regenerate, preview and export with networking disabled.

G-03. Provide two acquisition modes: **cache as I browse** for authorized resources and **download selected area** at the highest available native resolution that satisfies storage and provider constraints.

G-04. Import externally downloaded Mapterhorn terrain and Overture/OSM-derived regional data; no brittle manual filename hashing, browser cache hacks, or proprietary database required.

G-05. Make model outputs repeatable: save the selection geometry, options, data release IDs, transformation versions, asset hashes, warnings, and print-repair report.

G-06. Export standards-valid 3MF with printable/watertight geometry and feature groups. Provide a source/license summary and an offline-verifiable model bundle.

G-07. Own the user experience and generation engine; treat third-party datasets as replaceable adapters rather than hard dependencies.

## 2.2 Explicit exclusions at first production release

No whole-state or whole-country **single-object** 3MF generation: regional packs enable selecting many smaller print regions. No guaranteed universal building heights, survey-grade terrain, or automated roof photogrammetry. No required account, subscription, telemetry, cloud render farm, shared multiuser server, or remote rendering. No requirement to reproduce Mini Skyline's exact interface, effects, planets, paid features, branding, or proprietary implementation. No blind mass-download of OpenFreeMap public tiles or other endpoints that prohibit automated collection. No unsupported claim that zoom 17 equals a particular ground accuracy. No promise of offline place/address search until a local search dataset has actually been installed.

## 2.3 Success measures and release gates

| Metric | Initial acceptance target (to benchmark) | Gate |
|---|---|---|
| Offline completeness | 100% of required registered inputs present and verified for a marked Generation Ready selection | Ship blocker |
| Offline generation | Same pinned project regenerates with network physically disabled; no external DNS/HTTP calls | Ship blocker |
| Determinism | Canonical mesh geometry / settings / provenance hashes match for repeated runs with same engine and data | Ship blocker |
| 3MF correctness | Valid 3MF structure; meshes pass manifold/watertight checks or are explicitly rejected | Ship blocker |
| Project durability | Restore project and cache after process kill during download/generation | Ship blocker |
| Small-city benchmark | 1.4 km square sample with ~1,040 buildings within an initial 16 GB RAM reference envelope; precise timing TBD | Performance gate |
| Core launch | Opens the project library without any internet; no login wall | Ship blocker |
| Download integrity | Resume interrupted chunks; corruption is detected and never marked verified | Ship blocker |
| Security | No arbitrary filesystem/network access through renderer IPC or imported packages | Ship blocker |

The numeric targets in this document are **acceptance thresholds to validate experimentally**, not measured promises.

# 3. Users, context, and jobs to be done

**Primary user — home 3D-print enthusiast.** Select a neighborhood or hill, choose bed dimensions, maximize usable terrain detail, generate an attractive and printable 3MF, and reopen it months later without surprises.

**Secondary user — power map/model maker.** Organize terabytes of local extracts, work with multiple Overture releases, compare elevation sources, import PMTiles/GeoParquet, pin datasets, batch models, and inspect geometry warnings.

**Secondary user — offline workstation owner.** Download or prepare data on a connected machine, transfer a region pack on removable storage, import it, and generate/export while isolated.

**Maintainer — developer/tester.** Reproduce bug reports from project files, data manifests and logs without sending the user's raw geographic projects to an external service.

## Critical journeys

**J1: Online discovery → offline print.** Open map → browse → draw 1.5 km rectangle → see source coverage → request missing data → verify “Generation Ready” → disconnect → generate preview → set base, Z scale, min walls → export 3MF → open in slicer.

**J2: Download area in maximum available resolution.** Draw existing map selection → choose “Download for Offline Use” → choose Terrain / Buildings / Transportation / Base / Basemap / Search and native-max or custom quality → see real source coverage and byte estimates → approve → pause/resume → verify → pin pack.

**J3: Import external files.** Choose folder or manifest → inspect PMTiles/GeoParquet/COG/OSM PBF → preview geographic coverage, datum, provider/license and release → validate → register as immutable local source → test offline coverage.

**J4: Cache-as-you-browse.** With consent and lawful provider access, requested geographic data is persisted in the chosen library; repeated views prefer local data and never confuse browser-rendering cache with print-ready coverage.

**J5: Recover.** Interrupt a 20 GB region download or kill the modeling process → reopen → intact catalog, partial download visible, resumable job, and no false “complete” indicator.

**J6: Reproduce/upgrade.** Open old project with newer code or new data release → stay pinned to prior assets unless user explicitly forks/upgrades → compare geometry/source changes before export.

# 4. Data sources, contracts, and provenance

## 4.1 Source hierarchy

| Data domain | Primary local form | Acquisition / fallback | Purpose / constraints |
|---|---|---|---|
| Terrain | Mapterhorn PMTiles and indexed decoded grid blocks | Mapterhorn `tiles.mapterhorn.com/{z}/{x}/{y}.webp`, allowed metadata; Terrarium backup where legal | Earth elevations; coverage-specific native resolution and datum tracking |
| Buildings | Versioned Overture GeoParquet extract converted into normalized local feature tiles/index | Official Overture release; on-disk feature import | Footprints, parts, height/levels when present; do not invent surveyed heights |
| Transportation | Overture `segment`, `connector` extract in normalized local layer | Official Overture release | Streets, paths, rail, bridges and carriageway profiles |
| Base | Overture water, land, land cover, land use and infrastructure | Official Overture release | Water polygons, vegetation, parks, shorelines, land features |
| Addresses/places | Overture addresses/places or local search index | Optional Photon/OSM regional data importer | Search/labels/preview only; not a prerequisite for most model generation |
| Basemap | OSM-derived regional vector PMTiles from licensed downloadable extract | Protomaps / independently generated OSM PBF tiles | Offline visual navigation, labels, and map context |
| LiDAR | Optional locally extracted USGS 3DEP dataset with coverage and vertical datum | USGS government sources or compatible local tiles | Higher-detail terrain/roof refinement only after validation |
| Bridge supplement | Optional locally extracted OSM geometry | Local OSM extract; online Overpass only if expressly enabled | Enhanced crossings; failure must degrade gracefully |

**Data source selection order:** user-pinned local pack → compatible verified local cache → permitted online retrieval, if enabled → explicitly permitted and labeled fallback → *Missing Data*. Source fallback may never silently replace a higher-quality source without recording the change. Display map tiles must not be used as a substitute for print-generating attributes unless their schema is validated.

## 4.2 Provider adapters

DA-01. Every provider adapter SHALL declare provider identifier, layer, license/attribution, dataset release, archive format, CRS, vertical datum, coverage geometry, resolution units, schema mapping and supported retrieval modes.

DA-02. Every provider shall have a **source manifest** and content checksums; dataset versions are immutable after verification.

DA-03. Overture source import MUST retain building `building`/`building_part` relationships, source height/level tags, identifiers, transport connectivity, water geometry, and field-level missingness; it shall not flatten into a visually pretty but semantically insufficient basemap.

DA-04. Local region builders must use published Overture GeoParquet releases as the authoritative starting point for stable app-owned tiles. The example Overture inspection PMTiles may be supported as a compatibility/import adapter only after schema pinning and acceptance testing [S1].

DA-05. Mapterhorn `WebP` tiles encode Terrarium-style elevation values. Tile zoom controls sampling detail, **not** actual source measurement accuracy or vertical datum; resolution and source-quality labels must remain distinct [S2].

DA-06. Source updates shall never change an existing project without a user-approved upgrade/diff.

DA-07. Remote endpoints, authentication needs, redistribution rights, rate limits and local caching eligibility shall be documented per adapter before enabling unattended fetch.

DA-08. Data licensing must be reviewed theme-by-theme and region-by-region. Overture Buildings, Base and Transportation are ODbL; addresses and places include heterogeneous attribution or permissive terms; Mapterhorn's underlying terrain datasets carry varied source-specific obligations [S4][S6].

DA-09. OpenFreeMap's public service disallows automated data collection without permission, so do not implement public tile scraping as a regional download strategy [S7].

# 5. Core product requirements — geographic workspace

Requirements use priority **P0 = required for initial production release**, **P1 = next committed expansion**, **P2 = optional**. Each has a unique ID, expected behavior, and acceptance evidence.

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| MAP-01 P0 | Render an Earth basemap entirely from a locally registered, licensed map pack when offline. | Map pans/zooms with Wi-Fi disabled and no fallback web calls. |
| MAP-02 P0 | Allow rectangle, circle, free-form polygon, and rotated rectangle selections; show true geographic and print dimensions. | Boundary saved/reloaded at same coordinates, area and rotation. |
| MAP-03 P0 | Preserve WGS84 selection geometry and compute print dimensions in a suitable local metric projection. | Comparison against trusted geodesic test data. |
| MAP-04 P0 | Support mouse, keyboard, accessibility labels, configurable units and undo/redo for selections and edits. | UI automation and keyboard-only acceptance. |
| MAP-05 P0 | Show four separate readiness states: Visual Map, Terrain, Semantic Geometry, and Generation Ready. | Missing buildings do not display as Fully Ready. |
| MAP-06 P0 | Show the exact provider, release, resolution/quality and cache completeness under the selected area. | Coverage panel agrees with catalog and sampled tiles. |
| MAP-07 P0 | Offer explicit “Download Selection for Offline Use” action from existing selection menu. | Selection bounds and polygon mask transferred intact to planner. |
| MAP-08 P1 | Provide local name/coordinate search from installed search packs; optionally geocode online with consent. | Disconnected location lookup returns local entries only. |
| MAP-09 P1 | Show a coverage overlay by layer and quality, distinguishing verified, partial, downloading, obsolete and unavailable. | Toggle overlay matches dataset manifest. |
| MAP-10 P2 | Support multiple simultaneous map/project tabs with independent datasets, selection and undo stacks. | Switching tabs does not cross-contaminate state. |

Navigation shall be useable in an entirely empty library: a neutral grid/coordinate view and import/download instructions are shown rather than a broken map. Crossing the antimeridian requires explicit split bounding boxes and normalized longitude; if a release lacks validated crossing support, it must reject the selection clearly.

# 6. Acquisition, caching, downloads, and imports

## 6.1 Automatic cache requirements

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| CACHE-01 P0 | Let users choose and later move the data-library directory; never store large geographic assets in the app install folder. | Reinstall app and open old library without re-download. |
| CACHE-02 P0 | Persist eligible fetched source responses with provider, URL/archive identity, dataset version, byte range or logical tile ID, ETag, checksum, size, and fetch status. | Inspect catalog and re-read exact matching range. |
| CACHE-03 P0 | Use content-addressed files, atomic temp writes, fsync/rename where supported, and SQLite WAL transactions for catalog updates. | Crash during write yields no visible corrupt completed object. |
| CACHE-04 P0 | Serve verified local data first; offline mode **must not contact remote endpoints** for a cache miss. | Packet capture and disabled-network test. |
| CACHE-05 P0 | Differentiate 200 full response from 206 byte range, validate Content-Range/length, and never mistake one range for a complete archive. | Random range tests for PMTiles header, directories, tiles. |
| CACHE-06 P0 | Make each cached item refer to an immutable source release; pinning prevents cleanup of objects used by a project. | LRU deletion respects project refs and pinned region packs. |
| CACHE-07 P0 | Track source-specific restrictions and honor no-store/private/no-redistribute obligations; never cache credentials/URL secrets. | Policy test denies disallowed hosts/resources. |
| CACHE-08 P0 | Verify actual print-generation inputs, not just map tiles, before marking a region Generation Ready. | Deleting one required semantic tile changes readiness to Partial. |
| CACHE-09 P1 | Show size, quota, duplicate savings, hit/miss rates, free space, pinned volume and repair action. | Clear breakdown by dataset/source. |
| CACHE-10 P1 | Support library relocate, backup/export, integrity scan, orphan cleanup and read-only shared libraries. | Move 10 GB library and remount without breaking projects. |

## 6.2 Area-download planner

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| DL-01 P0 | Reuse the existing selection and allow choosing individual data domains. | Download plan has independent terrain/building/base/basemap layers. |
| DL-02 P0 | Offer Native Maximum, Balanced, and Custom profiles; report actual source resolution, zoom and coverage gaps. | “Max” never merely upscales lower-resolution terrain. |
| DL-03 P0 | Show file/request counts, estimated bytes with uncertainty, peak temporary disk, source version, licenses and approximate time only when rate data exists. | User can review/approve before writing large files. |
| DL-04 P0 | Buffer selection for edge-spanning footprints, transport, terrain interpolation and road continuity, then crop at mesh stage. | Feature crossing boundary is not truncated by acquisition. |
| DL-05 P0 | Enqueue bounded, paced, resumable downloads with persistent state, retry/backoff and checksum validation. | Interrupt process/power and resume without re-fetching verified data. |
| DL-06 P0 | Support cancel, pause, priority, remove, and safe cleanup of abandoned temp objects. | Cancellation leaves prior verified data unchanged. |
| DL-07 P0 | Mark region Complete **only** after per-layer spatial coverage and semantic attribute verification. | No false green state with missing terrain/base geometry. |
| DL-08 P1 | Permit scheduled data refresh and side-by-side dataset upgrades without mutating pinned project sources. | Compare old/new pack and revert reliably. |
| DL-09 P1 | Accept multi-polygon selections, antimeridian splits and approximate input boundaries. | Dateline regression passes when supported. |
| DL-10 P2 | Offer prebuilt download plans transferable to a different networked workstation. | Export/import JSON download plan and pack. |

The planner must not launch thousands of individual requests against an endpoint if the provider offers a sanctioned area-extract archive. Prefer Mapterhorn PMTiles extracts and Overture's published bulk GeoParquet mechanism. High-resolution Mapterhorn requires correct selection of its low-zoom and regional high-zoom files [S2].

## 6.3 External import and region-pack contracts

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| IMP-01 P0 | Import a single verified Mapterhorn PMTiles archive and confirm geographic and zoom coverage. | Offline elevation sample matches source tile. |
| IMP-02 P0 | Import an application-native `.m2mregion` pack containing manifest, checksums, coverage, source data and attribution. | Different Windows machine reads pack without network. |
| IMP-03 P0 | Validate ZIP structure, archive sizes, path traversal, decompression limits, checksums and signatures if present. | Malformed/malicious packages rejected without side effects. |
| IMP-04 P1 | Ingest Overture GeoParquet spatial extracts and normalize them into the app's own semantic source schema. | Footprints/transport/base retain counts, IDs and key attributes. |
| IMP-05 P1 | Import licensed OSM PBF/PMTiles basemaps and construct local search where available. | Search/navigation works offline in imported city. |
| IMP-06 P1 | Merge compatible terrain archives while retaining original archive provenance and per-tile native resolution. | Low/high-zoom coverage can be queried through one mount. |
| IMP-07 P1 | Detect spatial/version conflict and offer keep-both, select-priority or fork-project. | No silent overwriting of a pinned dataset. |
| IMP-08 P2 | Support GeoTIFF/COG, GeoJSON, GPKG, LAS/LAZ or LiDAR-derived rasters behind validated import plugins. | CRS, units and license preserved or import blocked. |

## 6.4 Source and coverage state machines

**Asset:** Planned → Downloading/Importing → Staged → Checksum Verified → Parsed → Indexed → Ready. Any stage may transition to Failed/Quarantined; retry always creates or resumes a temporary staged asset and never overwrites Ready data in place.

**Area:** Not Available → Partial → Visual Ready / Terrain Ready / Semantic Ready → Generation Ready. A separate **Offline Verified** state is awarded only after a forced-offline dry-run can acquire all inputs for selected settings. Changing required layers, selection or data source invalidates readiness as appropriate.

**Job:** Queued → Resolving Sources → Downloading → Validating → Indexing → Complete. Supports Paused, Cancel Requested, Cancelled, Failed (retryable), Failed (permanent). Progress must be based on verified bytes/tiles and show unknown totals honestly.

# 7. Local data library and persistence schema

The data library belongs to the user and can exist on removable or network-backed storage if safe durability can be established. Metadata is SQLite with WAL and periodic integrity checks; very large source files remain outside SQLite. Suggested structure:

```text
<Map2Model Library>/
  library.sqlite                 # jobs, sources, areas, projects, indexes
  objects/sha256/ab/<hash>       # immutable blobs and range objects
  sources/<source-id>/           # metadata, original or normalized archives
  regions/<region-id>/manifest.json
  projects/<project-id>/project.m2m.json
  exports/<project-id>/*.3mf
  staging/<job-id>/              # resumable temp files and journals
  logs/                          # user-controlled local diagnostic logs
```

**Required tables / collections:** `source_release`, `asset`, `asset_range`, `feature_index`, `coverage_cell`, `region`, `region_source`, `project`, `project_asset_ref`, `download_job`, `job_task`, `mesh_artifact`, `export_record`, `license_requirement`, `integrity_event`. Spatial coverage indexes use SQLite RTree or equivalent. Primary keys must not be derived solely from mutable filenames.

**SourceRelease minimum fields:** `source_id`, `provider`, `theme`, `release`, `schema_version`, `format`, `crs`, `vertical_datum`, `resolution_native_m` (nullable), `archive_uri`, `retrieved_at`, `license_ids`, `attribution_text`, `checksum`, `status`.

**AreaCoverage minimum fields:** `area_id`, `selection_wgs84`, `selection_buffer_m`, `layer`, `source_id`, `zoom_or_lod`, `tile_ids`, `expected`, `present`, `verified`, `missing`, `quality_flags`, `verified_at`.

**Project minimum fields:** `project_id`, `schema_version`, `selection_geometry`, `planet='earth'`, `projection`, `model_profile`, `print_profile`, `source_refs`, `engine_version`, `edit_history`, `creation_time`, `last_saved_time`, `output_refs`.

**Invariant LIB-01:** A completed region manifest is portable and sufficient to re-register all files; region identifiers are stable across application restarts. **Invariant LIB-02:** A project never points to a mutable source `latest` alias. **Invariant LIB-03:** Cache garbage collection must be reference-safe and power-loss-safe. **Invariant LIB-04:** The store must work when Windows folder paths contain Unicode, spaces and long filenames.

# 8. Geographic processing and model-generation pipeline

The pipeline is independent of network and UI. Same selection + source hashes + modeling parameters + engine version shall produce geometrically equivalent meshes even after reinstall.

```text
Selection + model settings
        |
Source Resolver -> Verified local source snapshots -> Spatial query / tile decode
        |                     |                               |
        |                     +--> Missing-data report          |
        v                                                     v
Coordinate + vertical datum normalization -> Semantic scene / DEM surface
        |                                      |
        +--> Terrain sampling, water levels    +--> Building/road/land features
                              \               /
                               Printable geometry construction
                                         |
                             Mesh repair + geometric validation
                                         |
                         Interactive preview / 3MF writer / provenance
```

## 8.1 Terrain engineering

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| GEO-01 P0 | Transform lat/lon into a local metric modeling frame with recorded CRS and transform parameters. | Distances and areas match reference within specified tolerance. |
| GEO-02 P0 | Decode Mapterhorn Terrarium WebP and fallback Terrarium images using documented pixel-to-height rules and source metadata. | Unit tests on fixed known pixels and elevations. |
| GEO-03 P0 | Track per-tile no-data masks, datum, interpolation choice, source quality and optional vertical offset. | No-data is warned, never silently rendered as zero sea level. |
| GEO-04 P0 | Stitch tile edges without cracks or duplicate vertices and account for selection buffer. | Seam regression in mixed tile boundaries. |
| GEO-05 P0 | Choose terrain grid spacing based on print dimensions, source resolution and memory budget; preserve aspect ratio. | Requested physical size fits tolerance; no false precision. |
| GEO-06 P0 | Offer raw elevation, locally normalized, exaggerated Z, and water-level modes; expose physical vertical scale. | Users can reproduce measured Z settings. |
| GEO-07 P0 | Construct watertight base, sides and bottom with user-defined thickness, datum floor and optional cutouts. | No open boundaries in export. |
| GEO-08 P1 | Blend overlapping elevation sources by declared priority and seam policy; flag quality discontinuities. | Overlap regression shows no unexplained cliffs. |
| GEO-09 P1 | Generate contour layers, custom cut planes, modular sections and registration joints. | Parts assemble in printer test. |
| GEO-10 P2 | Support LiDAR/COG sources with vertical datum conversion and coverage quality checks. | Verified higher-resolution source supersedes baseline only when selected. |

## 8.2 Semantic geometry engineering

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| SEM-01 P0 | Build 3D walls/roofs from Overture `building` and `building_part` footprints, including holes and multipolygons. | Building regression shapes remain topologically valid. |
| SEM-02 P0 | Apply measured height when provided, else configured levels-derived height, else clearly labeled per-type fallback. | Provenance distinguishes measured/estimated/assumed. |
| SEM-03 P0 | Clip/merge footprints at selection edges without dropping cross-boundary features. | Boundary building remains intact to chosen crop plane. |
| SEM-04 P0 | Convert transportation segments into printable paths/roads with road category width, bridge/tunnel flags, and minimum manufacturable thickness. | Overpasses maintain separation; no mesh self-intersections. |
| SEM-05 P0 | Model water as user-selected inlay, void, recessed surface or separate material, preserving shoreline topology. | Water and terrain parts align; no open seams. |
| SEM-06 P0 | Convert green/forest/land-use polygons into relief, texture geometry or color masks with controllable minimum detail. | Small noisy areas are filtered predictably. |
| SEM-07 P0 | Track source and optional user edit for each semantic object so warnings can highlight individual features. | Click source feature → inspect used geometry/height provenance. |
| SEM-08 P1 | Support separate building/road/water parts for multicolor or assembly prints. | Each export group is valid and correctly positioned. |
| SEM-09 P1 | Include manual add/delete/edit buildings, custom paths, raised text and markers without modifying upstream source files. | Undo/redo and source updates preserve edits. |
| SEM-10 P2 | Enhance bridge superstructure, roof planes, vegetation props and custom architectural detail behind opt-in profiles. | Advanced features do not regress basic city generation. |

## 8.3 Geometry constraints and repair

MESH-01. All modeling is expressed in millimeters after geographic projection, with no unrecorded mixing of geographic degrees and print units.  
MESH-02. Provide minimum wall thickness, minimum positive relief, minimum gap, minimum feature length, elephant-foot allowance and printable roof/road thresholds.  
MESH-03. Validate orientation, degenerate triangles, duplicate vertices, self-intersections, winding consistency, non-manifold edges, watertight volume and disconnected components.  
MESH-04. Attempt deterministic and bounded repairs only; if repair changes geometry beyond user-configured tolerances, require user approval.  
MESH-05. Show estimated dimensions/volume and triangle/part counts before export; do not claim material/time estimates are precise slicer results.  
MESH-06. Preview should share its source snapshot and geometry parameters with export; any lower-detail preview must be prominently labeled as such.  
MESH-07. Long mesh jobs shall be cancellable and memory-budget-aware, with a diagnostic log and recoverable project state.

# 9. Print design, preview and export

## 9.1 Print profiles

Preset profiles must be editable and stored inside project files. Suggested defaults: **Miniature City**, **Topographic Relief**, **Framed Wall Art**, **Multicolor City**, and **Modular Large Map**. They are starting configurations, not independent geometry engines.

Parameters: bed width/depth; physical model width/height; printable overall height; terrain Z multiplier; building-height treatment; minimum base height; no-data policy; feature minimums; wall/shell thickness; color/material grouping; text/labels; frame style; bounding shape; split planes; support/overhang warnings; overlap tolerance; unit selection; output name.

## 9.2 3MF and other deliverables

| ID / Priority | Requirement — the system SHALL... | Observable acceptance |
|---|---|---|
| EXP-01 P0 | Export standards-conformant 3MF with correct mm units, object references, transformed build items and material assignments. | 3MF consortium validation / library parser pass. |
| EXP-02 P0 | Include model settings, source inventory, attribution, license notices, warnings, and build provenance inside 3MF as permitted metadata. | Metadata round-trip and spot-check. |
| EXP-03 P0 | Export the same project to the same geometry with no network if assets are pinned locally. | Air-gapped byte-independent geometry-hash comparison. |
| EXP-04 P0 | Reject invalid/unrepairable printable meshes with explicit issue location and recovery choices. | Negative mesh fixtures rejected. |
| EXP-05 P0 | Open exported files in at least two independent target slicers/viewers; grouped parts align. | End-to-end slicer test report. |
| EXP-06 P1 | Offer STL, OBJ/glTF preview and a separate JSON provenance sidecar with explicit color/group loss warnings. | Correct format/units and user warnings. |
| EXP-07 P1 | Add multi-plate segmentation, pins and keyed alignment parts; optionally use the 3MF Production Extension. | Physical assembly fixture / slicer compatibility. |
| EXP-08 P2 | Offer batch export from queued selections sharing one pinned regional data pack. | Deterministic overnight batch with failure isolation. |

No output should inherit Mini Skyline-specific licensing grants by assumption. The new project's export license decision must be based on its own code and underlying data rights. The 3MF source file demonstrated a separate license and source manifest; use that as a **provenance UX example**, not as legal authority for the redesigned product.

# 10. Desktop interface and interaction specification

**Left rail:** Project Library, Map, Local Data, Downloads, Model Settings, Export History.  
**Center:** Interactive Earth map with local/online state and selection editing; toggleable preview.  
**Right panel:** Selection info, Ready/Partial/Missing coverage by layer, Download or Import, printable geometry settings, warnings.  
**Bottom tray:** Transfer queue, job progress, storage meter and diagnostic events.  
**Persistent status strip:** Offline switch, selected library, active data-release pin, available disk and engine status.

## Required screens

UI-01. **First run:** choose/create library; guided demo/sample region; optional provider configuration; no signup.  
UI-02. **Map/selection:** draw and rotate rectangle/polygon, inspect source coverage, save project, acquire required inputs.  
UI-03. **Download planner:** layers, source, native vs requested resolution, size range, licensing, pause and progress.  
UI-04. **Data library:** pinned packs, imports, integrity, version upgrades, storage usage, remove/relocate with dependency warnings.  
UI-05. **Print editor:** tabs for terrain, buildings, roads, water, base, colors, labels, dimensions; live low-res preview and final geometry run.  
UI-06. **Export/validate:** printability diagnostics, selectable parts, 3MF export, provenance/attribution and slicer test notes.  
UI-07. **Troubleshooter:** exact missing dataset/tile IDs, source version, operation trace, safe redacted log export and repair actions.

### UX rules

No generic “Generation failed, try again” without a causal category. Example error: `DATA_MISSING / Overture transportation release 2026-08-19.0 / tiles 14/4364/6217 ... / Open Download Planner`. Distinguish “currently viewing online,” “locally cached some tiles,” “data downloaded but not indexed,” “offline generation ready,” and “print mesh validated.” Estimated download sizes include uncertainty and metadata overhead; any pack advertised as “maximum available” names the maximum native source resolution for that selection and does not infer it from zoom alone. All destructive operations preview impact on pinned projects.

# 11. Execution architecture and component contracts

## 11.1 Recommended component separation

| Component | Responsibility | Ownership / boundary |
|---|---|---|
| Desktop shell | Windows lifecycle, folder picker, updates, OS dialogs | Electron main process only; no GIS parsing |
| UI / map | MapLibre scene, selection editor, source coverage, project and job panels | Sandboxed React/TypeScript; restricted typed IPC |
| Project service | Schemas, save/load, history, pins, diff, migrations | Local Rust daemon/library |
| Catalog + coverage | Spatial index, file refs, region readiness, integrity | SQLite and immutable object store |
| Acquisition manager | Provider adapters, URL allowlists, retries, checksum, import | Background service, policy-gated networking |
| Geodata decoder | GeoParquet/PMTiles/DEM decoding, CRS transforms, semantic normalization | Independent engine module |
| Mesh engine | Terrain, features, roads, water, thickness, repair | CPU-bound worker pool with budgets |
| Preview bridge | Progressive mesh / glTF and per-object provenance | Geometry service → UI only |
| 3MF writer | Model grouping, metadata, license report, package validation | Engine exporter; lib3mf candidate |
| Diagnostics | Structured local logs, download receipts, crash recovery | Explicit user export, no mandatory telemetry |

IPC and interprocess messages must be versioned and validated. Renderer may request `selectArea`, `planDownload`, `startJob`, `cancelJob`, `loadProject`, `generate`, `export`, `listSources`, or `readCoverage`—not arbitrary filesystem paths or arbitrary URLs. The engine shall be crash-isolated from the UI; an engine panic or process termination must leave verified library assets and project files intact.

## 11.2 Data-flow interfaces (proposed)

```json
{
  "apiVersion": "1.0",
  "selection": {
    "type": "Polygon",
    "crs": "EPSG:4326",
    "coordinates": "<GeoJSON-compatible coordinates>"
  },
  "requiredLayers": ["terrain", "buildings", "transportation", "base"],
  "sourcePolicy": "pinned-or-explicit-fallback",
  "qualityPolicy": "highest-native-within-budget",
  "offlineRequired": true
}
```

**`planDownload(selection, layers, budget)`** returns source IDs, release IDs, spatial coverage, estimated bytes, tile/feature count, source-native resolution, attribution, missing tiles, projected temporary space, warnings and a stable plan hash.  
**`executeDownload(planHash)`** returns persisted job ID and progress events.  
**`verifyRegion(regionId, modelProfile)`** reports per-layer readiness, exact missing identifiers and a proof record bound to requested settings.  
**`generate(projectId, sourceManifestHash)`** returns cancellable job ID; it never upgrades data implicitly.  
**`export3mf(modelJobId, printProfile)`** returns a file in a user-approved export directory plus checksums and validation result.

## 11.3 Background job operations

Use persistent task journals and bounded queues. Acquisition is I/O bound and rate-limited; import/index is disk/CPU bound; mesh generation is CPU/memory bound; 3MF compression is streaming. Prioritize interactive single-model work over discretionary bulk downloads. Record request URL *without private credentials*, response headers relevant to range integrity, cache outcome, source version and measured resource usage.

# 12. Non-functional requirements and operational objectives

| ID / Priority | Non-functional requirement | Acceptance / verification |
|---|---|---|
| NFR-01 P0 | Full functionality for an already verified Earth region with external networking blocked. | Air-gapped Windows end-to-end test. |
| NFR-02 P0 | Support 16 GB RAM reference system with graceful memory-budget enforcement and progress. | Golden-city fixture completes or explicit bounded downgrade; no OOM. |
| NFR-03 P0 | Handle datasets and projects beyond 4 GB and Win32-style file-size assumptions. | Multi-GB import and export fixture. |
| NFR-04 P0 | All writes transactional, resumable or atomically recoverable; no committed corrupted asset. | Fault injection at 10+ state transitions. |
| NFR-05 P0 | The renderer cannot execute untrusted web content with Node access; filesystem access via narrow validated IPC. | Threat-model and automated security tests. |
| NFR-06 P0 | Zero login, zero required telemetry and zero third-party requests when Offline toggle is active. | Process/network interception fixture. |
| NFR-07 P0 | Reproducible source pinning, geometry signatures and per-export provenance. | Re-run golden project after reinstall. |
| NFR-08 P0 | Offline updates and source-schema migrations do not rewrite pinned geodata without consent. | Upgrade/downgrade migration fixture. |
| NFR-09 P1 | Local map UI remains responsive during downloads and geometry generation. | Input latency target p95 < 150 ms on reference PC outside GPU saturation. |
| NFR-10 P1 | Clear source quality and warning messages; WCAG-style keyboard navigation and readable contrast. | Accessibility audit and keyboard-only task. |
| NFR-11 P1 | Project and cache catalog integrity scan; protect against silent bit rot with object hashes. | Inject mutated bytes then read as corrupt. |
| NFR-12 P1 | Maintain source adapter and engine API version compatibility for two minor releases or provide automatic migration. | Compatibility matrix in CI. |
| NFR-13 P1 | Support 2560×1440 monitors and 125–200% Windows display scaling. | Visual tests at multiple DPIs. |
| NFR-14 P2 | Optional CLI for batch imports, pack verification and render/export. | Headless workflow on runner. |

**Reference hardware to benchmark:** Windows 11 x64, 6–8 physical CPU cores, 16 GB RAM, 100 GB free SSD, mainstream graphics; additionally test 8 GB (degraded mode) and 32 GB (high-detail). Publish measured runtime/peak memory/size for standard fixtures rather than guessing. Target architecture shall be compatible with future ARM64/macOS/Linux work but these are not MVP release criteria.

# 13. Architecture choices and recorded trade-offs

| Decision | Option A | Option B | Recommendation / proof required |
|---|---|---|---|
| Shell | Electron | Tauri | **Electron** for v1 developer velocity and existing workflow; isolate native engine and harden IPC. Re-evaluate footprint after MVP. |
| Geometry | Rust CPU native | TypeScript/WASM browser workers | **Rust engine** for deterministic streaming and memory control; benchmark WASM when parity matters. |
| Data import | DuckDB spatial/GeoParquet builder | Overture inspection PMTiles direct | **Official GeoParquet-first** for semantic fidelity/stability; accept PMTiles as versioned compatibility format only. |
| Bulk tile format | PMTiles archives | Millions of loose ZXY files | **PMTiles** for immutable region packs, content-addressed range objects for opportunistic online caching. |
| Catalog | SQLite + RTree | Flat manifest only | **SQLite** for queries/jobs/recovery plus portable JSON region manifest. |
| 3MF | lib3mf | Hand-written ZIP/XML writer | Prefer **lib3mf** if license/FFI/Windows packaging validated; otherwise spec-tested writer, not untested XML strings. |
| Search | Local SQLite FTS/OSM extract | Public Photon dependency | **Local optional index**; online provider optional with consent. |
| Preview | MapLibre + local mesh preview | Reuse Mini Skyline minified engine | **New render pipeline**; do not depend on proprietary bundle. |

## ADR-001: Why not simply extend v0.2.0?

That prototype is a valuable reference for UI and source behavior but its bundled website is compiled and the prior issues (missing worker scripts, dynamic imports, nonfunctional planetary maps and search) illustrate update fragility. The redesign must not continue fetching unpredictable bundles from the website at application build time. Retain lessons, observed source contracts and test artifacts; rewrite owned components with tests and documentation.

### ADR-002: What can be reused?

Potentially reusable with verification: Electron packaging/configuration patterns, safe localhost serving concepts, the user-facing selection/download workflow, model print parameter ideas, prior 3MF QA fixtures, source manifests, checksum tests, and independently licensed libraries. Do **not** copy minified application code, assets, branding, copyrighted UI or upstream exporter routines into a distributable product until permissions are explicitly established. “Same data sources” does not mean “same application code.”

# 14. Security, privacy, rights and provider policy

SEC-01. Use packaged local UI assets, `contextIsolation`, renderer sandboxing, no remote Node integration, CSP, checked navigation and narrow sender-validated IPC, aligned with current Electron guidance [S8].

SEC-02. Provider downloaders shall have explicit trusted allowlists or signed source definitions, strict HTTPS, redirect checking, download size caps, timeouts, backoff, header validation and SSRF protection. Imported manifests do not gain arbitrary URL-fetch privileges.

SEC-03. Region archives shall be treated as untrusted: reject zip-slip paths, symlinks to outside folders, duplicate archive entries, compression bombs, executable content, foreign schema versions, suspicious coordinate/geometry counts and oversized raster expansion.

SEC-04. Apply immutable snapshots and file permissions to protect pinned projects. No background auto-update of user data from unofficial endpoints.

SEC-05. Log locally with opt-in export; redact secrets, filenames containing personal identifiers when exporting publicly, and exact user coordinates if user selects privacy-redaction mode.

SEC-06. Track licensing at source-feature/theme/region level. Generate user-facing attribution in map UI, region manifests, exports and sharing instructions. Resolve ODbL and downstream Produced Work questions with legal review before public distribution.

SEC-07. No “free for the public” interpretation as automatic permission to scrape, redistribute or sell a data pack. Check Mapterhorn's underlying source attribution for each region and review Overture theme-by-theme [S4][S6].

SEC-08. Maintain software bill of materials, build provenance and dependency patch process for packaged desktop releases; code-sign Windows installers when distribution begins.

# 15. Verification strategy and golden fixtures

## 15.1 User-provided baseline fixture

**Fixture ID:** GOLDEN-DAYTON-001. The uploaded `waikiki-honolulu-1km.3mf` is mislabeled geographically; internal bbox is approximately **[-84.095746, 39.738573, -84.079262, 39.751249]**, near Dayton, Ohio. Selection is about **1,409.5 m × 1,409.5 m**. Its browser-generation metadata reports:

| Recorded field | Observed value | Intended regression use |
|---|---|---|
| Terrain | Mapterhorn z15, 6 ZXY tiles | Elevation decoding and tile seam verification |
| Terrain grid | 1.8366 m reported grid spacing | Verify sampling policy vs native source quality |
| Buildings | Overture `2026-08-19.0`, 4 logical z14 building tiles | Counts/locations and source projection |
| Buildings matched | 1,040 | Semantic feature completeness comparison |
| Transportation | 16 logical z14 tiles, 153 selected features | Road type/geometry validation |
| Base | 2 logical z13 tiles, 61 selected features | Land-use and feature mapping validation |
| Addresses | 715 points loaded; preview-only | Optional display/search separation |
| Bridge supplement | 13 requested, failed | Must degrade gracefully, not block export |
| LiDAR | Not requested / not used | Baseline depends only on normal sources |

**Caution:** Overture logical identifiers `archive.pmtiles#z/x/y` are not HTTP byte ranges. The 3MF is an output mesh with provenance, not a raw source cache. Its counts are useful sanity anchors; differences caused by a newly normalized Overture schema or revised geometry policies must be recorded rather than forced into a false exact match.

## 15.2 Test pyramid and release checklist

| Test ID | Fixture / action | Pass condition |
|---|---|---|
| TEST-01 | Terrarium pixel fixtures, tiles with negative elevations/no-data | Correct elevation math and explicit no-data |
| TEST-02 | Mixed-zoom adjacent terrain tiles | No seams, orientation mistakes or illegal extrapolation |
| TEST-03 | PMTiles header/index and randomized byte-range fixtures | Correct cache keying, Content-Range and length validation |
| TEST-04 | Polygon/rectangle/rotated/datelined selections | CRS and selection identity stable |
| TEST-05 | Overture building and building_part with holes | Valid footprint and roof/wall topology |
| TEST-06 | Roads, bridges, tunnels and water intersection cases | No unintended merged walls or artifacts |
| TEST-07 | Malformed imported pack, zip-slip/zip bomb, malicious URL | Rejected, no writes outside library |
| TEST-08 | Kill acquisition process mid-write and mid-index | Recovered job; no partial Ready states |
| TEST-09 | Delete one required Overture or Mapterhorn tile from ready pack | Correct missing-data warning and readiness downgrade |
| TEST-10 | Airgap golden Dayton project | Preview and 3MF export succeed, external request count zero |
| TEST-11 | Compare two runs with same pinned sources and engine | Canonical mesh geometry signatures match |
| TEST-12 | Reinstall/upgrade UI while keeping external library | Project opens with identical source pins and geometry |
| TEST-13 | Open 3MF in two independent viewers/slicers | Valid volume, units, colors/parts, no fatal issues |
| TEST-14 | Restore library from transferred `.m2mregion` pack | Offline coverage and source counts preserved |
| TEST-15 | Simulate corrupt file/hash mismatch | Quarantine and targeted repair; do not serve corrupt data |
| TEST-16 | Large park/water/coastal/high-rise and steep-mountain fixtures | Graceful warnings and print-safe geometry |
| TEST-17 | Source version upgrade diff | Old project remains pinned; user-approved fork changes source map only |
| TEST-18 | Provider quota, forbidden caching, flaky network | Respect policy, exponential retry, no source spoofing |

Additionally run property-based tests on polygon clipping, mesh manifoldness, geographic wrap, and cache metadata; run reproducibility tests on multiple machines; and include end-to-end visual tests across Windows scaling factors. A passing CI build is **not** proof of offline model generation without an air-gapped runtime test.

## 15.3 Benchmark corpus

Start with the user-provided Dayton sample. Add a verified coastal/shoreline fixture in Honolulu or Waikiki, a dense urban tall-building area, a rural low-building area, a mountain terrain-only region, a river crossing/bridge region, a high-latitude region, a selection spanning data-quality changes, and a dataset version-upgrade pair. Record source releases and file hashes for every fixture. Never label coordinates by filenames without validating spatial metadata.

# 16. Delivery roadmap, backlog and release criteria

**Sizing caution:** The following sequence is a proposed engineering plan, not a firm time estimate. A single experienced engineer with GIS/geometry experience may require multiple months for the complete program; third-party source adapter complexity and print-mesh QA dominate risk. Each stage has a verifiable exit rather than a date promise.

| Stage | Deliverables | Definition of done |
|---|---|---|
| Phase 0 — Discovery & rights | Provider policy matrix; geometry/3MF spike; repo structure; test fixture set; license decisions | Source contracts and clean-room ownership approved |
| Phase 1 — Local kernel | New app shell, project file, catalog, local map/selection, DEM decoder, basic terrain/3MF | Simple selected terrain printed entirely offline from a local pack |
| Phase 2 — City semantics | Overture importer, normalized buildings/roads/base, local geometry engine and repair | Golden Dayton model generated offline with source audit |
| Phase 3 — Cache and area planning | Browse-as-cache, source release pinning, selected-area planner, resumable downloads, readiness overlay | Download → disconnect → regenerate → export, including interruption recovery |
| Phase 4 — External import and usability | PMTiles extract import, native packs, library manager, optional local search/basemap, settings UI | Transfer a data pack to another PC and run fully offline |
| Phase 5 — Production readiness | Multi-part 3MF, slicer conformance, accessibility, code signing, migration, performance/security test suite | All P0 requirements and release-blocking tests pass |
| Phase 6 — Optional enhancements | LiDAR, advanced bridge/roof/vegetation, modular print, planet adapters | Feature-specific validation without regressing Earth workflow |

## Recommended initial repository structure

```text
map2model-next/
  apps/desktop/                  # Electron shell, React/MapLibre UI
  crates/engine/                 # terrain / geometry / mesh kernel
  crates/data/                   # source readers, decoders, normalized features
  crates/catalog/                # SQLite migrations, coverage, cache library
  crates/acquisition/            # bulk download, import, policies, jobs
  crates/export/                 # 3MF, provenance, geometry validation
  packages/contracts/            # versioned JSON/TypeScript types
  packages/ui/                   # map, selection, settings components
  tools/region-builder/          # offline source pack generation / validation
  fixtures/golden/               # hashes, exact source pin definitions
  docs/architecture/             # ADRs, provider licenses, operational guides
  .github/workflows/             # unit/integration/security/Windows release
```

### Initial vertical slice (first demonstrable milestone)

1. Install the new Windows app with no web code copied from Mini Skyline.
2. Mount one small published Mapterhorn extract as a local data source.
3. Draw a selection, sample elevation, construct a watertight relief with base and export valid 3MF.
4. Close networking completely and repeat the project generation.
5. Save a project plus source manifest and show that source hashes and mesh geometry match after restart.

Do not start by reproducing dozens of UI toggles; protect the offline data/print pipeline first.

**Execution ownership and gates:** The first vertical slice is delivered through the three-agent operating model in §21. Agent 1 establishes the contracts and test harness at G0; Agents 1–3 then work in parallel against those frozen interfaces and converge at an air-gapped acceptance gate.

# 17. Traceability matrix and approval criteria

| Outcome | Requirements | Must-pass tests |
|---|---|---|
| Navigate/choose a geographic region | MAP-01..07 | TEST-04, TEST-09, TEST-10 |
| Store/cache/reuse local geodata | CACHE-01..08, LIB invariants | TEST-03, TEST-08..10, TEST-15 |
| Download selected max-quality region | DL-01..07 | TEST-02, TEST-08, TEST-09, TEST-18 |
| Import externally downloaded packs | IMP-01..03, DA-01..09 | TEST-03, TEST-07, TEST-14 |
| Build print-safe 3D geometry | GEO-01..07, SEM-01..07, MESH-01..07 | TEST-01, TEST-02, TEST-05, TEST-06, TEST-10..13 |
| Export auditable 3MF | EXP-01..05 | TEST-10..13 |
| Ship a safe, durable Windows application | NFR-01..08, SEC-01..08 | TEST-07..18 plus security and code-sign checks |

**Definition of the initial product:** all P0 requirements have a named test and a passing artifact. There is a distributable signed Windows installer, a reproducible build description, a self-contained sample local region, a 3MF golden fixture, complete license notices, a clean-room ownership review and a bug-report workflow without compulsory telemetry.

# 18. Open design questions and explicit assumptions

| ID | Decision needed before production | Proposed default / owner |
|---|---|---|
| OQ-01 | Which printer(s) and slicer(s) define the minimum printable dimension and material compatibility? | Select two common Windows slicers and a reference FDM printer; product owner |
| OQ-02 | Which maximum selection size and physical bed size must be supported in first release? | Use configurable print bed, cap geometry by measured complexity; product owner |
| OQ-03 | Which licensing grants apply to copied UI ideas, site code or exporter routines? | None assumed; legal/owner review |
| OQ-04 | Should building heights default to levels-based estimates or per-building-type constants when absent? | Levels first, configurable fallback and conspicuous provenance; GIS/UX |
| OQ-05 | Should high-quality terrain favor less interpolation or smooth visually attractive relief? | Expose modes, default terrain-faithful; modeling team |
| OQ-06 | Is optional local search required for v1? | P1; coordinate entry and offline map selection P0 |
| OQ-07 | Are external pack imports intended for personal, private team or public redistribution? | Personal/local first; rights need specific review for redistribution |
| OQ-08 | Which terrain vertical datums and LiDAR sources must be supported? | Mapterhorn datum as observed/declared, USGS optional; GIS team |
| OQ-09 | Should region downloads store source GeoParquet verbatim, derived semantic tiles, or both? | Derived compact local tiles + enough original metadata for audit; evaluate disk cost |
| OQ-10 | What 3MF materials/production extensions must be mandatory for specific slicers? | Core + basic materials first; test extensions separately |
| OQ-11 | How are local cache sources shared between multiple local Windows accounts? | Default per-user, explicit read-only shared library mount in P1 |
| OQ-12 | Which automatic-updater and signing infrastructure is available? | Manual signed install first, optional auto-update with consent |

# 19. Terms and definitions

**Local-first:** user owns the canonical project and geodata; core operations function with no network when required data is locally verified.  
**Offline Verified:** a selection and setting profile have passed input completeness and air-gapped generation readiness checks.  
**PMTiles:** immutable single-file tile pyramid format with on-demand byte-range access [S3].  
**GeoParquet:** spatially enabled columnar Parquet dataset used by Overture as the official core release format [S1].  
**Native resolution:** source measurement detail supported in that location, independent of output pixel spacing or map tile zoom.  
**Generation Ready:** all declared terrain and semantic data are present, indexed and valid for the current selection and source policy.  
**Pinned source:** immutable source release/file hash that the project is bound to.  
**Produced Work:** a rendered output derived from a database, subject to applicable underlying license analysis; not a universal legal exemption.  
**3MF:** ZIP-packaged standard for additive-manufacturing models, parts and metadata [S5].  
**Provenance:** reproducible history of dataset names, versions, hashes, transforms and model settings.  
**Vertical datum:** reference used to interpret elevations; must not be silently conflated with another system.

# 20. Source register and review notes

The following official/project sources were checked on **10 October 2026**. URLs are included for engineering follow-up, and individual provider terms must be rechecked at the time of implementation.

| Ref | Source / authoritative topic | URL |
|---|---|---|
| S1 | Overture official catalog, GeoParquet, inspection-only PMTiles, release formats | https://docs.overturemaps.org/getting-data/cloud-sources/ |
| S2 | Mapterhorn terrain tile encoding and PMTiles region extracts | https://mapterhorn.com/data-access/ |
| S3 | Protomaps PMTiles concepts and range-based archives | https://docs.protomaps.com/pmtiles/ |
| S4 | Overture licensing / attribution by theme and upstream source | https://docs.overturemaps.org/attribution/ |
| S5 | 3MF Consortium specifications and conformance reference | https://3mf.io/spec/ |
| S6 | Mapterhorn regional coverage, mixed terrain source attribution | https://mapterhorn.com/ and https://mapterhorn.com/attribution/ |
| S7 | OpenFreeMap service terms; automated data collection restrictions | https://openfreemap.org/tos/ |
| S8 | Electron security guidance for sandbox/context isolation/IPC | https://www.electronjs.org/docs/latest/tutorial/security/ |
| S9 | Overture GeoParquet -> own PMTiles example and themes | https://docs.overturemaps.org/examples/build-a-map/ |
| S10 | DuckDB spatial extension overview, potential Overture import tool | https://duckdb.org/docs/current/core_extensions/spatial/overview |
| S11 | Protomaps downloadable OSM-derived basemap | https://docs.protomaps.com/basemaps/downloads |
| S12 | 3MF Core specification and packaging rules | https://github.com/3MFConsortium/spec_core |
| U1 | User-provided `waikiki-honolulu-1km.3mf` | Contains original `Metadata/miniskylines_sources.json` and attribution text |
| U2 | User-provided `map2model-waikiki-file-geodata-manifest.json` | Derived summary of U1; see inconsistent filename/bounds warning |
| U3 | Existing private Map2Model prototype, v0.2.0 | Implementation reference only, not stable reusable upstream source |

# 21. Multi-agent execution plan and work ownership

**Status and relationship to roadmap:** This section is the proposed operating plan for **Phase 0 (foundation)** and **Phase 1 (first offline terrain-to-3MF vertical slice)** in §16. It adds coordination and handoff requirements without changing the underlying product scope, architecture proposal, rights caveats, or release gates in §§1–20. All implementation choices remain subject to the Phase 0 architecture decision records (ADRs). No feature implementation is considered approved merely by being assigned here.

## 21.1 Team model, authority, and ownership

Use **three agents with a single accountable integrator**. Agent 1 is the lead architect and integration owner, Agent 2 is the geographic data engine owner, and Agent 3 is the geometry/3MF engine owner. The product owner approves user-visible scope, accepted licensing, printer/slicer targets, and stage gates; the lead owns code-level decisions and integration. Agents may comment on one another's PRs, but a single named owner is accountable for each shared contract and test.

| Agent | Primary ownership (first milestone) | Deliverable and acceptance responsibility |
|---|---|---|
| **Agent 1 — Lead Architect & Integrator** | `apps/desktop/`, `packages/contracts/`, `packages/ui/`, `.github/workflows/`, integration tests, release assembly; owns architecture and PR merges | New Windows shell; selection/project workflow; versioned contracts and IPC; reliable sidecar startup; offline end-to-end test; installer; integration report. |
| **Agent 2 — Geographic Data Engine** | `crates/data/`, terrain importer/decoder, its fixtures and tests; data-side code in `tools/region-builder/` | Import a permitted local Mapterhorn extract; decode/normalize elevation and datum; return validated `ElevationGridV1`; deterministic raster/sampling tests. |
| **Agent 3 — Geometry & 3MF Export** | `crates/engine/`, `crates/export/`, mesh/export fixtures and tests | Build printable relief with base from `ElevationGridV1`; validate manifold geometry; export standards-valid 3MF and print-provenance report; run slicer checks. |

**Ownership rules:** Only Agent 1 changes shared contracts, package manifests, installer workflows, integration harnesses, and architecture docs unless a contract-change PR is explicitly approved. Agents 2 and 3 independently own their crates and fixtures. Agent 1 is the sole person/agent who integrates completed contributions into the protected main branch. Use the existing Map2Model v0.2.0 repository and its user-supplied 3MF as **behavioral references and regression fixtures only**, not as a source of proprietary implementation code absent confirmed permission (§13–14).

## 21.2 Execution stages, dependency gates, and parallel work

| Gate / order | Responsible agent(s) | Work / required exit evidence |
|---|---|---|
| **G0 — Foundation (sequential)** | **Agent 1 leads; Agents 2–3 review contracts only** | Create new repository skeleton; pin toolchains; document clean-room/code ownership, dataset license policy, error taxonomy and data/projection decisions; commit `SelectionV1`, `ElevationGridV1`, generation/export contracts, fixture manifest, stub CLI/IPC and test runners. **Do not start parallel feature PRs before G0 is accepted.** |
| **G1 — Parallel implementation** | **Agents 1, 2 and 3** | Agent 1 builds desktop shell/selection/project/UI; Agent 2 implements offline terrain import and grid generation; Agent 3 implements terrain mesh and 3MF from a **synthetic grid fixture**. Agents work on separate branches and directories, behind the frozen V1 contracts. |
| **G2 — Integration** | **Agent 1 integrates; Agents 2–3 fix owner-area defects** | Connect selection → local terrain grid → mesh → 3MF; run integration fixture and check attribution/provenance, absent-data diagnostics and determinism; complete review and merge each PR. |
| **G3 — Air-gapped milestone acceptance** | **Agent 1 operates tests; all agents sign off their components; product owner accepts** | Windows install; import local Mapterhorn subset; select rectangle; export and open 3MF in reference slicer; disconnect DNS/network; reopen project and regenerate; compare pinned input hashes and canonical mesh hashes; archive logs and conformance evidence. |

The full **Phase 1** milestone is not complete merely because a Windows executable launches or a 3MF can be produced online. It is complete only when G3 passes with **network physically blocked** and with no silent source fallback or access to the old mirrored web application.

## 21.3 Frozen cross-agent interface contracts (G0 deliverables)

Agent 1 shall commit schema files, Rust types, TypeScript equivalents, sample JSON, contract tests, and a changelog. Agents 2 and 3 implement against those files rather than exchanging ad hoc objects. Proposed V1 contracts below are implementation proposals to ratify in G0, not facts about existing code.

| Contract ID | Producer → consumer | Required fields and invariants |
|---|---|---|
| **C-01 `SelectionV1`** | Agent 1 UI → Agent 2 data | Earth/WGS84 EPSG:4326 rectangular bbox `[west,south,east,north]` with valid latitude/longitude and non-wrapping longitude; a pinned selection ID; the requested terrain profile. Polygon/circle support remains later in §5. |
| **C-02 `TerrainSourceManifestV1`** | Agent 2 → catalog, Agent 1, Agent 3 | Source name, license and attribution, source release/archive ID, local file URI through approved engine access, SHA-256, coverage/bounds, horizontal/vertical reference, original native resolution, import/decoder version; no guessed datums. |
| **C-03 `ElevationGridV1`** | Agent 2 → Agent 3 | Version, `width`, `height` (each ≥2), row-major elevations in **meters**, validity mask with one entry per sample, local metric XY basis anchored to WGS84, north-up row order, positive X/Y spacing, declared vertical datum, transform ID, selection ID and source-manifest hash. Invalid/incomplete data must fail explicitly, not interpolate silently. |
| **C-04 `TerrainBuildRequestV1`** | Agent 1 → Agent 3 | Hash of `ElevationGridV1`, rectangular clip/extent, model XY physical dimensions in mm, vertical exaggeration, terrain Z reference, base thickness, minimum feature thickness, deterministic engine version and export location token (no arbitrary renderer path). |
| **C-05 `MeshAnd3mfResultV1`** | Agent 3 → Agent 1 | Print-space millimeter units, closed and oriented triangle mesh/parts, non-manifold/degenerate checks, min-thickness report, export file hash, canonical geometry hash, 3MF conformance outcome, warnings, embedded provenance reference. |
| **C-06 `JobStateV1` and error taxonomy** | All engines → Agent 1 UI | Cancellable job ID, typed phase/progress, terminal state, correlation ID and stable reason code (`DATA_MISSING`, `INVALID_CRS`, `UNSUPPORTED_DATUM`, `OUT_OF_MEMORY`, `NON_MANIFOLD_MESH`, `EXPORT_FAILED`, etc.). Never reduce this to a generic “try again.” |

**Grid convention to freeze:** For the first milestone choose an explicitly documented local East/North metric projection/basis. Define grid sample `(col,row)` as `x = minX + col*dx`, `y = maxY − row*dy`. Width × height is the exact length of the samples and mask. A positive elevation is in meters relative to the recorded datum—not automatically sea level. No-data samples inside the chosen print rectangle block a “Ready” state unless a disclosed fill policy is requested. Projection details, datum transformation rules and precision tolerance are recorded as ADRs before either engine is implemented. The model engine converts horizontal/vertical meters to physical millimeters using the frozen print profile; all mesh output units are millimeters.

**Cross-platform testing:** Core decoding and mesh/export must be callable from a **headless engine CLI** as well as via versioned Electron IPC. Synthetic-grid and 3MF tests must not require Windows GUI, provider network connections, private tokens, or proprietary copied site bundles. Agent 1 supplies this test harness at G0.

## 21.4 Initial issue / task allocation

| Task ID / owner | Depends on | Required PR artifact / reviewer |
|---|---|---|
| **A1-01** Lead: repository, CI, code ownership, ADR and license register | None | New clean repository; pinned Rust/Node toolchains; lint/test workflow; Agent 2/3 review. |
| **A1-02** Lead: frozen `contracts/v1` + synthetic fixtures | A1-01 | Schema, tests, CLI/IPC stubs; acceptance by Agents 2 and 3 **before G1**. |
| **A2-01** Data: local terrain pack reader and asset validation | A1-02 | Offline Mapterhorn PMTiles import; bounds, checksum and source/license manifest tests; Lead review. |
| **A2-02** Data: tile/elevation decode, coordinate normalization | A2-01 | Golden samples incl. negative heights/no-data/edge sampling; `ElevationGridV1` fixture; Lead review. |
| **A3-01** Geometry: synthetic-grid relief and printable base | A1-02 | Closed mesh with correct winding, XY/Z scaling and deterministic tests; Lead review. |
| **A3-02** Export: 3MF writer and validation | A3-01 | Standards-valid test 3MF, slicer-open report, mesh checksum and provenance; Lead review. |
| **A1-03** Lead: new Windows shell and selection UI | A1-02 | Local project workflow, rectangular map selection, disabled-network startup; Agent 2 review. |
| **A1-04** Lead: engine bridge, source manifests and job UI | A1-03, A2-02, A3-02 | End-to-end integration, typed error display and cancellation; Agents 2 and 3 review. |
| **A1-05** Lead: signed/unsigned test installer and G3 evidence | A1-04 | Windows build, repeatable offline test record, 3MF and conformance report; product owner sign-off. |

Tasks involving Overture city semantics, broad background auto-caching, region download queues, offline search, planet renderers, or UI parity with Mini Skyline are **not part of this first vertical slice** and remain scheduled by §16 after acceptance.

## 21.5 Branch, pull-request, and review protocol

- Each task has a GitHub issue with owner, dependencies, contract version, affected requirements, fixture, test command, and precise “done when” checks. Use branch names such as `agent/lead-A1-03-desktop`, `agent/data-A2-02-elevation`, `agent/mesh-A3-02-3mf`.
- Each agent submits a focused PR to an integration branch (or protected main as configured by the lead) and **does not independently merge**. Agent 1 reviews all PRs; Agent 2 reviews any projection/datum changes; Agent 3 reviews any mesh or 3MF data-contract changes.
- No agent may change `packages/contracts/` or shared IPC schema without a written change proposal, corresponding tests, consumer acknowledgement, and a version bump. Do not silently reshape fields just to make a PR pass.
- Every PR includes: change summary, input/source license note, linked requirement IDs, tests run and observed outputs, deterministic fixture hashes where applicable, failure modes, and a reproducible demo/test command. Keep datasets and generated binaries out of Git history unless rights and size rules permit; use versioned fixtures and release artifacts.
- Required green checks before merge: formatting/lint, unit/contract tests, test-fixture integrity, offline-only CLI tests, and Windows package/integration smoke tests for affected components. Lead runs the full G3 test after integration, not only PR-level unit tests.
- Record decisions in `docs/architecture/ADR-###.md`; unresolved conflicts and off-contract assumptions go to the lead as explicit blockers. Agent 1 documents rejected alternatives and signs the integration checklist.

## 21.6 Milestone-1 acceptance checklist (must-pass evidence)

| Acceptance ID | Evidence required for handoff / sign-off |
|---|---|
| **M1-AT-01** | Clean Windows installer/portable test package starts without the old Mini Skyline runtime or external connection; versioned engine is alive and sandbox/IPC checks pass. |
| **M1-AT-02** | Import one **lawfully usable, pinned** small Mapterhorn terrain extract while disconnected; manifest records archive hash, bounds, resolution and datum, or rejects unsupported datum explicitly. |
| **M1-AT-03** | Select a rectangle in WGS84, transform to a documented local metric grid, and obtain the same `ElevationGridV1` samples and validity mask on repeat runs. |
| **M1-AT-04** | Generate a closed, oriented terrain relief with a solid base, correct mm dimensions and configured vertical exaggeration from a synthetic fixture and an imported elevation fixture. |
| **M1-AT-05** | 3MF opens in the agreed reference slicer(s), passes structural/conformance checks, and includes source/license and engine provenance; invalid geometry fails clearly. |
| **M1-AT-06** | With external networking disabled (not merely simulated cache hits), reopen the project, regenerate and export; audit confirms no DNS/HTTP attempt and matching source and canonical mesh hashes. |
| **M1-AT-07** | Missing tiles, malformed PMTiles, hash mismatch, unsupported vertical datum and cancelled job produce typed actionable errors without falsely marking the region Generation Ready. |
| **M1-AT-08** | Repository contains build instructions, sample fixtures, test logs, screenshots of UI, 3MF inspection output, rights/attribution record, and known limitations; all three agents record component sign-off. |

**Release decision:** Agent 1 presents one consolidated evidence bundle and outstanding-risk register to the product owner. A green CI build alone is insufficient. Any missing M1-AT evidence keeps Phase 1 open; acceptance of the terrain-only slice is not acceptance of offline city-building support or broad source caching.

## 21.7 Ready-to-send agent briefs

### Agent 1 brief — Lead Architect, Desktop, and Integration

**Mission:** Create an independent Windows Map2Model Next application and own its end-to-end integration. Start alone at G0. Establish a clean-room repository and lock `SelectionV1`, `TerrainSourceManifestV1`, `ElevationGridV1`, `TerrainBuildRequestV1`, `MeshAnd3mfResultV1`, and `JobStateV1` with schema fixtures and engine-CLI/IPC stubs. After Agents 2 and 3 approve the contracts, build the React/MapLibre/Electron shell, local project persistence, rectangular selection, status/error UX, and safe engine process bridge. Merge approved PRs, create installer artifacts, and run the complete offline test. **Deliver:** G0 contract PR, desktop integration PR, reproducible build instructions, M1-AT-01 through M1-AT-08 evidence, and the consolidated acceptance report. **Do not:** fork or copy the minified Mini Skyline runtime, override GIS/mesh ownership without review, or claim “offline ready” without a no-network test.

### Agent 2 brief — Geodata Import, Terrain Decoder, and Coverage

**Mission:** Build the independent offline data path for the first Earth terrain slice. Wait for signed G0 contracts, then implement a local licensed Mapterhorn PMTiles import/reader, tile index and integrity checks, elevation decoding, spatial clipping/sampling, WGS84-to-local-metric conversion and explicit vertical datum handling. Produce deterministic `ElevationGridV1` responses and manifest hashes for both synthetic and real terrain fixtures. Own decode/no-data/edge tests and document any mismatch between claimed and actual native source resolution. **Deliver:** data-crate PR, fixtures, sample CLI outputs, source/license records, deterministic grid and error tests. **Do not:** embed download-on-demand into generation, assume zoom is native accuracy, bulk-scrape public tiles, or silently interpolate a missing terrain cell.

### Agent 3 brief — Geometry Kernel, Mesh Validation, and 3MF

**Mission:** Build the offline printable-terrain engine against the frozen `ElevationGridV1` contract. Start with a synthetic elevation grid so work proceeds independently of Agent 2; generate a clipped relief, watertight sides and base, unit-correct dimensions, controlled Z scaling and print-quality diagnostics. Use a standards-conformant 3MF writer, document the library/license decision, embed source and engine provenance, and verify outputs in selected slicers. **Deliver:** mesh/export crate PRs, synthetic/real-grid fixtures, manifold/3MF validation reports, deterministic canonical mesh hashes and reproducible headless CLI demonstration. **Do not:** couple to UI JavaScript, consume raw PMTiles or undocumented geodata shapes, substitute arbitrary heights, or accept non-manifold geometry as printable.

## 21.8 Kickoff and coordination record

**Before agents receive implementation access**, the lead opens A1-01 and A1-02, shares this versioned requirements plan, nominates the reference printer/slicers, and records any pending license/provenance assumptions. Agents 2 and 3 may review draft interfaces but must not implement against unstable assumptions. At G0 the lead posts a signed **contract-freeze record** (commit SHA, V1 schema checksums, sample fixture hashes, environment/toolchain versions, and review acknowledgements). This record unlocks G1 parallel work.

Use a brief shared async check-in for blockers, contract/fixture changes, test failures, and next PR handoffs. Escalate conflicting CRS/vertical-datum assumptions or changes in data licensing immediately. For every integration failure record the source version, fixture identifier, failing stage, and accountable code owner; the lead drives triage rather than asking agents to guess across boundaries.

**Decision rule:** Do not optimize headcount or UI breadth at the cost of the first verifiable offline print. The sole non-negotiable target is: **local terrain input → validated elevation grid → printable 3MF → repeated offline regeneration on Windows**, with traceable inputs and a clear owner for every contract.

## Closing recommendation

**Approve the project as an independent, local-first engine with a clean data-source boundary.** Prioritize one complete, air-gapped printable city-model workflow using official Overture extracts and Mapterhorn terrain before adding every styling control. Reuse the best lessons from Map2Model 0.2.0, but do not make a robust successor dependent on the old website's minified JavaScript, transient tile endpoints or unclear redistribution permissions.
