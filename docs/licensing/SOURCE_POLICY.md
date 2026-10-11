# Geodata provenance and licensing register (provisional)

This is an engineering policy, **not legal clearance**. Verify relevant licenses, source-specific restrictions and attribution before publishing software or regional data. **Owner's 2026-10-11 decision:** permit separately labeled, user-initiated, *local private experimental* candidate terrain evaluation under the [private-use disclaimer policy](PRIVATE-LOCAL-TERRAIN-POLICY-2026-10-11.md), without inventing a vertical datum or license. Verified source status and distribution remain separately gated.

| Dataset | Proposed role | Permission/source questions | State |
|---|---|---|---|
| Mapterhorn | User-imported local PMTiles in separate experimental mode; verified mode only when fully attested | Source-specific terms and attribution still apply to private use; no blanket license or accurate datum can be inferred. No raw/derived user assets included in distributed app | Experimental candidate path under review; verified source pending |
| Overture Maps | Phase 2 buildings, transportation, base features | GeoParquet release and theme-specific obligations | Deferred |
| Terrarium/Mapzen | Optional fallback DEM | Credit and caching rights | Deferred |
| OpenStreetMap/Overpass | Optional later features and basemap | ODbL attribution, bulk and hosting restrictions | Deferred |
| OpenFreeMap | Visible map where allowed | Do not bulk-collect shared public tiles | No bulk downloading |
| USGS 3DEP | Optional future LiDAR | Source, vertical reference, redistribution | Deferred |
| Prior Mini Skyline mirror | Behavioral regression reference only | Copying implementation or protected artwork requires separate permission | Not included |

## Source manifest must record
Provider, release/archive ID, data license/attribution, SHA-256, geographic coverage, horizontal and vertical references, original native resolution, decoder version and imported timestamp. Unknown datums must not be silently guessed.

## Release safeguards
- Do not commit user data, source archives or generated 3MF models without provenance and rights review.
- No geographic network fetching when users select Offline.
- Source fallbacks must be explicit and never silently replace a pinned dataset.
- Publish an attribution and component/license report for actual included sources.

## Personal use versus verified data/distribution

- The owner selects a **private, local, non-distributed** experimental terrain workflow with a per-job, unchecked consent checkbox, candidate heights, mandatory attribution and clearly unverified datum/license/native resolution. That **does not grant source rights** or exempt the user from external copying, derivative or caching restrictions. Consult the [full implementation/review policy](PRIVATE-LOCAL-TERRAIN-POLICY-2026-10-11.md).
- The experimental decoder produces a separate typed preview. It must not forge or weaken the known-datum/authorized-rights semantics of frozen G0 TerrainSourceManifestV1 or ElevationGridV1. Generated 3MF, if integrated, will be prominently labeled **experimental**, include warnings and provenance, and remain private to the user.
- **Publishing software** and **sharing third-party data/derived models** are different acts: an unsigned Windows preview may distribute the software and originally generated synthetic fixtures after verifying its own dependency notices; it must never bundle the owner's private real source archives or generated real-source 3MF files.
- Known provider restrictions must be followed even for private evaluation. Do not hardcode guessed provenance; preserve genuinely unknown fields. Source data cannot be called **Verified/Generation Ready** based solely on an acknowledgment or on successful 3MF slicing.
