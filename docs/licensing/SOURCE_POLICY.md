# Geodata provenance and licensing register (provisional)

This is an engineering policy, **not legal clearance**. Verify relevant licenses, source-specific restrictions and attribution before publishing software or regional data.

| Dataset | Proposed role | Permission/source questions | State |
|---|---|---|---|
| Mapterhorn | User-imported, version-pinned terrain PMTiles | Archive redistribution rights; underlying terrain/DEM attribution and datum accuracy | Pending agent review |
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
