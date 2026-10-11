# Interlaken z17 — source-provenance findings and non-acceptance ledger

**Status: independent research / review evidence ONLY.** No source-rights or vertical-datum approval, no production registry authorization, no real-source Generation Ready. Not a substitute for release provenance verification, upstream publisher signatures, or legal review. Frozen G0 contracts untouched. Owner-attested Windows format smoke is accepted; **the next OWNER application test is the integrated Windows preview, not another standalone geodata inspector**.

Review date: 2026-10-11. Upstream Mapterhorn code snapshot **`775663e9ca58c7fe741dbd53fbe4461cb73a9bb9`** (GitHub branch `main` observed 2026-10-11); URL-pinned source links below use that SHA instead of mutable main.

## Source bytes: what we actually know

| Item | Evidence | Confidence |
| --- | --- | --- |
| Corrected regional PMTiles | Private `interlaken-z17.pmtiles`, size 3,127,295 B, SHA-256 `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0` | Independent exact-byte SHA in the private session, owner PowerShell SHA matches; owner official PMTiles verify PASSED |
| Original uncorrected PMTiles | SHA `40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce` | Changed **exactly one** header byte at offset 101 (maxZoom 18→17), no image-byte differences. **Not interchangeable** for SHA-bound evidence |
| Region / index | WGS84 bbox `[7.88,46.68,7.89,46.69]`; 40 real indexed tiles at z13=1, z14=2, z15=4, z16=9, z17=24 | Owner-verified native Windows compiled Rust format-only inspector while physically disconnected; 5 independent candidate pixel goldens and 512×512 RGB |
| Upstream archive | Mapterhorn's `6-33-22.pmtiles` at `https://download.mapterhorn.com/6-33-22.pmtiles` | Official route is known, but the **exact regional extraction's immutable source release / upstream parent bytes + verified original release hash are NOT established**. Public catalog MD5 is not a signed provenance guarantee |
| Production source authorization | No signed file-specific, trusted encoding evidence; actual PMTiles JSON has `attribution` only | **NOT APPROVED**; importer must reject absent encoding metadata by current policy |
| Rights, source attribution, vertical datum and true native DEM resolution of all contributing data | Sources may be blended; region-specific aggregation/source provenance not supplied in PMTiles | **UNVERIFIED**; no production Generation Ready |

**Never commit** either private Mapterhorn archive, its real WebP tiles, an extracted raster/grid/mesh, or a derived distributable without independent redistribution clearance.

## Independently authoritative publisher encoding evidence: NOW IDENTIFIED

The original **Mapterhorn publisher** provides *directly relevant* evidence that the PMTiles/WebP source is designed to use Terrarium RGB rather than a speculative format inferred from pixel values:

1. Official Mapterhorn data access: `https://mapterhorn.com/data-access/` explicitly states 512px **Terrarium-encoded WebP** tiles and describes extracting `6-33-22.pmtiles` for Interlaken at zoom 13–17.
2. Published pipeline documentation pinned at [`pipelines/README.md`](https://github.com/mapterhorn/mapterhorn/blob/775663e9ca58c7fe741dbd53fbe4461cb73a9bb9/pipelines/README.md): aggregation warps/mosaics/blends source DEM inputs into EPSG:3857 tiles, uses **Terrarium encoding** for elevations, stores WebP RGB, then downsamples and bundles into `6-{x}-{y}.pmtiles` for z13+.
3. Actual publisher implementation pinned at [`pipelines/utils.py`](https://github.com/mapterhorn/mapterhorn/blob/775663e9ca58c7fe741dbd53fbe4461cb73a9bb9/pipelines/utils.py): `save_terrarium_tile` applies z-dependent value rounding, offsets by 32768, writes `R/G/B` bytes and **lossless WebP**; `create_archive` explicitly finalizes a PMTiles with WebP and an **attribution-only metadata JSON** (no `encoding` field). This means lack of `encoding` in the extracted archive **matches upstream producer design** and is *not*, by itself, corruption.
4. Publisher's [PMTiles viewer example](https://github.com/mapterhorn/mapterhorn/blob/775663e9ca58c7fe741dbd53fbe4461cb73a9bb9/website/examples/pmtiles/example.html) selects `6-{x}-{y}.pmtiles` with `encoding: "terrarium"`, `tileSize: 512`.

**Scope limit:** These prove **upstream data encoding conventions**. They are **NOT signed metadata about these exact 3,127,295 extracted bytes** and do not establish individual DEM contributing files, vertical reference, licenses, native horizontal sample resolution or a released-parent SHA. Internal release evidence must still bind a provider document snapshot and exact archive SHA before a lead-approved encoding-only attestation can be considered.

## Candidate source attribution and independent swisstopo terms

The pinned [Mapterhorn source catalog `swissalti3d/metadata.json`](https://github.com/mapterhorn/mapterhorn/blob/775663e9ca58c7fe741dbd53fbe4461cb73a9bb9/source-catalog/swissalti3d/metadata.json) identifies **Swiss Federal Office of Topography swisstopo** and **swissALTI3D**, nominal 0.5 m, access year 2026, license *Open Government Data*. Published Git blob SHA for this metadata is `23b54587f2925eabb20bf4346252a17c02d5794c`. The catalog's source list and `LICENSE.pdf` exist, but they are **not yet traced to each actual raster/aggregation tile** in our exact extract. The catalog's 0.5 m figure is *a candidate source property's nominal resolution*, not verified native DEM resolution at this Interlaken location.

Primary original producer documents (not merely a third-party summary):

- [swisstopo swissALTI3D product details](https://www.swisstopo.admin.ch/en/height-model-swissalti3d): product is available with **LV95 / LN02** coordinates; 0.5m **or** 2m grid spacing depending on the area; a nominal 0.5m output can contain **oversampled** original measurements; accuracy depends on data-generation method. A contributor's documented LN02 datum must NOT automatically be used as the final Mapterhorn output's known datum.
- [swisstopo original OGD usage terms](https://www.swisstopo.admin.ch/en/terms-of-use-free-geodata-and-geoservices): allow use, processing, redistribution and commercial use of qualifying open geodata; **source acknowledgement is mandatory** (e.g. `©swisstopo`). These terms apply to relevant swisstopo products, **not all other contributors** in the Mapterhorn mosaic.
- [swisstopo national vertical reference explanation](https://www.swisstopo.admin.ch/en/local-swiss-reference-frames): the Swiss **LN02** and **LHN95** height frames differ; they cannot be treated as interchangeable.
- [Mapterhorn upstream vertical references issue #255](https://github.com/mapterhorn/mapterhorn/issues/255): open unresolved discussion requests vertical provenance/transformations across contributing datasets. Do **not** infer that Mapterhorn harmonizes every tile to LN02, EGM2008, or ellipsoidal heights.

Caution: the swissALTI3D **sample-download page** restricts its example datasets to testing and disallows inclusion in products. Do not confuse restrictions on specific sample products with the published OGD terms for qualifying production downloads. Dataset-specific rights, attribution text and constituent raster versions must be verified for each confirmed contributor.

## Provenance chain still missing

1. **Real parent release identity:** obtain verified publish-version metadata, immutable parent URL/ETag or strong content digest; match the regional extraction to that snapshot and record exact go-pmtiles CLI version/flags. Our regional extract SHA only proves the *derived local file*, not its parent's authenticated origin.
2. **Region-specific contributor map:** determine which source-catalog entries/actual TIFFs contributed to the z13–17 Interlaken macrotiles at the pinned Mapterhorn release. Mapterhorn can blend multiple sources; a single country's location alone does not prove sole-source coverage. Pin the specific covering/aggregation CSV IDs, source item filenames and source metadata versions.
3. **Datum policy per constituent:** for *every* source, establish native vertical CRS, whether Mapterhorn reprojected vertically or only horizontally, any mixed-datum blending and mathematical reprojection chain. Until independently attested, **UNSUPPORTED_DATUM** and no production grid.
4. **Native spatial accuracy:** separate zoom 17 Web Mercator output *pixel spacing* from source DEM grid spacing and accuracy. Obtain the contributing 0.5/2m original pixel-grid distribution and relevant survey method; record authoritative native resolution/uncertainty with actual region-specific map, not the maximum tile zoom.
5. **Source-specific redistribution and derivation rights:** confirm all contributors' versions, required credits, whether transformed and exported 3MF derivatives can be distributed; source license for code (BSD-3) never grants DEM rights by itself.
6. **Encoding approval:** a trusted/authorized signer's exact-archive-SHA record with independently archived publisher encoding documentation can verify **encoding only**. Signer authority/provisioning, canonical signed bytes, revocation and negative tests require lead approval (separate design proposal).

The operator should **not** perform another standalone inspector test. The next Windows application test is owned by the integrator's full desktop synthetic-terrain→mesh→3MF preview. Provenance research and safeguards proceed independently on Agent 2's draft PR #16 without a build dependency or any new user task.
