# G2 Agent 3 review — separate experimental local 3MF bridge

**Verdict: APPROVE the Lead/owner G2 *product policy and separated adapter design*, subject to implementation gates below.** This is a design signoff only, not approval of real-source rights, release publishing, existing verified API changes or merge of draft PR #15.

**Authority:** [G2 policy and review](../../docs/agent-handoff/G2-PRIVATE-USE-EXPERIMENTAL-TERRAIN-REVIEW-2026-10-11.md), merged PR #24; Agent 2's [separate native DTO handoff](https://github.com/DrFlGd/Map2Model-Next/blob/agent/2-A2-02-terrarium-grid/crates/data/EXPERIMENTAL-LOCAL-TERRAIN-HANDOFF.md). All six G0 V1 contracts, current `build_mesh(ElevationGridV1,...)`, `prepare_export`, `export_3mf(BuildOutput, ElevationGridV1, ..., TerrainSourceManifestV1)` and trusted `JobStateV1` wire handling **remain unchanged**.

## Required separate types and function boundaries (proposed signatures)

Agent 2 currently emits a JSON `serde_json::Value` for its separate `ExperimentalLocalTerrainPreviewV1` with real candidate numeric samples. For the Rust-to-Rust experimental bridge, prefer a dedicated deserialized validated type instead of passing untrusted JSON `Value` deeper into the native engine. A typed newtype may parse this JSON, **but never implement conversion into trusted `ElevationGridV1` / `TerrainSourceManifestV1`**.

~~~rust
// Proposed, separately versioned G2-native-only API (NOT implemented yet):
pub struct ExperimentalCandidateGridV1 {
    // Parsed/checked from Agent 2's ExperimentalLocalTerrainPreviewV1;
    // source SHA and candidate digest, number/shape/units, bbox/zoom,
    // selection ID, north-up samples and all unverified-status invariants.
    // No trusted source manifest or canonical grid hash.
    /* private validated fields */
}
pub struct ExperimentalPrintRequestV1 {
    // Native-validated print dimensions/thickness/exaggeration and
    // explicitly local-relative baseline. Not a frozen TerrainBuildRequestV1.
    /* private validated fields */
}
pub struct ExperimentalConsentV1 {
    // Native-issued, single-job/single-operation consumed-once receipt,
    // bound to exact local archive SHA, selection ID, job ID, operation,
    // policy/notice version and explicit checkbox acceptance.
    /* private validated fields */
}
pub struct ExperimentalMeshOutput { /* TerrainMesh + geometry QA + linkage */ }
pub struct ExperimentalExported3mf { /* distinct experimental provenance + 3MF bytes */ }

pub fn build_experimental_mesh(
    candidate: &ExperimentalCandidateGridV1,
    selection: &SelectionV1,
    print: &ExperimentalPrintRequestV1,
) -> Result<ExperimentalMeshOutput, ExperimentalExportError>;

pub fn export_experimental_3mf(
    mesh: &ExperimentalMeshOutput,
    candidate: &ExperimentalCandidateGridV1,
    selection: &SelectionV1,
    print: &ExperimentalPrintRequestV1,
    consent: &ExperimentalConsentV1,
) -> Result<ExperimentalExported3mf, ExperimentalExportError>;

pub fn write_experimental_3mf_atomic(
    destination_from_native_save_dialog: &std::path::Path,
    artifact: &ExperimentalExported3mf,
    is_cancelled: impl FnMut() -> bool,
) -> Result<(), ExperimentalExportError>;
~~~

The *implementation* should extract the existing bounded heightfield triangulation and mesh QA into a shared, internal **already-validated metric heightfield geometry function**, called by the existing trusted `build_mesh` and this separate experimental entry point. Do **not** weaken `validate_build`, fabricate an in-memory trusted manifest with `verticalDatumId=known`, or call `export_3mf` with reconstructed fake G0 source documents. The experimental 3MF writer should reuse the existing **real 3MF** Core ZIP/OPC utilities, geometry QA, canonical mesh SHA, bounded metadata, independent reader, deterministic timestamps, atomic no-clobber stage and cancellation from PR #15. Separate experimental provenance/validator paths must not allow stripping warnings or promoting the package to verified.

**No frozen G0 schema changes are requested.** These are new non-G0, versioned internal experimental-only types. Agent 3 owns geometry and packaging library functions; Agent 2 owns decoded-preview data and its SHA; Lead owns native typed invocation, user consent, trusted file/path selection, worker lifecycle and UI/project persistence.

## Strict input and consent rules

The Lead's real UI checkbox must default **unchecked for EACH job** and be bound to native job ID, operation `experimental-local-3mf-export`, exact source archive SHA, frozen validated selection ID and policy copy/version. Treat the static CLI opt-in string on Agent 2's developer command as **developer test acknowledgment only**: it is NOT a persisted/scopeless authorization token for Electron native export. Native supervisor should reject missing/mismatched/stale/replayed consent and require per-job re-acknowledgment.

The experimental mesh bridge must recompute Agent 2's domain-separated sample SHA directly from bounded `candidateTerrariumHeightsMeters`, checked `sourceArchiveSha256` and `selectionId`, using `M2M-EXPERIMENTAL-UNVERIFIED-HEIGHTS-1\\0` + UTF-8 lowercase source SHA + UTF-8 selection ID + each north-up sample `f64.to_le_bytes()`. Reject signed NaN/Inf, insufficient grid, mask/no-data, dimension mismatch, wrong ordering, source mutation, selection mismatch, nonpositive metric spacings, unsupported clip/exaggeration, output oversize, and altered source attribution. Check `experimentalOnly=true`, `localPersonalUseOnly=true`, `ownerAcknowledgedUnverified=true`, `encodingAuthenticated=false`, `verticalDatumStatus="unknown"`, `sourceLicenseVerified=false`, `nativeResolutionVerified=false`, `redistributionApproved=false`, `productionGenerationReady=false` and `networkRequests=0`. Refuse any attempt to mark unknown source facts as verified. The raster remains a *candidate interpretation*, even if decoding succeeds.

Do **not** infer mean sea level or geoid from decoded values. For Z placement use an explicitly local-relative datum: e.g. `referenceLevelMeters = min(candidateSamples)` over the clipped nonempty finite region; add base thickness and vertical exaggeration to **relative** heights, and label reference zero as *local sample minimum, not geodetic altitude*. Preserve expected millimeter dimensions and the real mesh QA (closed, oriented, no degenerates/nonmanifold edges, single connected component and positive volume). Do not silently cap/delete suspect heights or claim native spatial resolution accuracy.

Before any candidate export, native source-policy gate must block *known* prohibited processing/derivative use even with consent, while unknown source rights stay `unknown`, not `approved`. Local file input must remain bounded, SHA-pinned, offline and private. No background fetch, remote archive URLs, app-driven uploads/telemetry/sharing or redistribution of owner-origin raw/derived files.

## Actual 3MF disclosure and independent verification

The 3MF must be **genuinely printable**, but unmistakably *experimental* if opened without the Map2Model UI. Require BOTH visible Core XML metadata and an embedded JSON provenance part. An optional locally adjacent sidecar may supplement, **never replace**, the data inside the 3MF.

Propose provenance document ID `M2M-EXPERIMENTAL-3MF-PROVENANCE-1` with:

- `experimentalOnly:true`, `localPersonalUseOnly:true`, `productionGenerationReady:false`, `redistributionApproved:false`, `sourceLicenseVerified:false`, `nativeResolutionVerified:false`, `encodingAuthenticated:false`, `verticalDatumStatus:"unknown"`, `sourceLicenseStatus:"unknown"` (unless externally evidenced, do not substitute CC0/public-domain).
- Exact `sourceArchiveSha256` and bytes, **separate `candidateHeightsSha256`**, actual `canonicalGeometrySha256`, `provenanceSha256` **outside the self-hashed provenance JSON**, and final ZIP `packageSha256` (returned after packaging, not self-embedded). Hashes must remain domain-separated.
- `selectionId`, `bboxWgs84`, `zoom`, grid dimension/spacing, `candidateEncoding:"Terrarium interpretation — unverified"`, `sourceMetadataDeclaresTerrarium`, `verticalReference:"unknown"`, `nativeResolutionMeters:null`, `uncertaintyMeters:null` (unless actually known), chosen **local-relative** base reference and mm print dimensions/exaggeration.
- Original `sourceAttributionFromArchive` (or explicit absent/unknown, not fabricated provider credit), any source text URL(s) **only if actually present/known**. Missing links are `null`, never guessed. Preserve user-supplied text as *untrusted* and escape all XML; only safely rendered plain text in UI. Never put local input path or username inside exported metadata.
- `jobId`, explicit `operation`, `acknowledgementPolicyVersion`, SHA-bound consent receipt ID/digest (not private tokens/secrets) and actual disclaimer/warning string from the approved policy.

Required in `3D/3dmodel.model` top-level Core metadata are unmistakable text `EXPERIMENTAL — PRIVATE LOCAL USE — UNVERIFIED`, unknown vertical datum/native accuracy, local use/distribution restriction, attribution (or missing indication) and links as plain escaped text. Include the following policy notice verbatim or Lead-approved equivalent:

> EXPERIMENTAL — USER-SUPPLIED TERRAIN. Source permissions not independently verified; vertical datum and native accuracy unknown. Private local evaluation only; third-party license and attribution requirements still apply. Not a certified elevation model. Redistribution not approved by Map2Model Next.

Reuse the current Core package source license fields **only in verified mode**. A guessed CC0, known vertical datum, `Generation Ready`, or only a transient UI badge is a **G2 failure**. Nothing about a valid ZIP, watertight mesh or successful PrusaSlicer/Bambu Studio slice verifies rights, datum, survey-grade accuracy or license status.

## Test plan before Lead can label G2 export integrated

- Positive: **Agent 2's original generated, redistribution-safe 512px WebP/PMTiles fixture** → experimental candidate preview → separate mesh → actual 3MF; independently reopen OPC, source/candidate/geometry/provenance SHA, unknown flags, text/attribution/XML escaping and mm bounds; verify deterministic ZIP bytes on Windows and Ubuntu.
- Consent: absent, stale, false, reused, wrong job, source SHA, selection ID, operation, notice version; deny before file staging.
- Source: wrong SHA/mutated file, symlink, malformed PMTiles, unsupported/conflicting encoding, missing indexed seam neighbors, alpha/no-data and overbudget samples; source restrictions known to prohibit use; no fallback to fabricated heights/rights.
- Mesh/package: wrong candidate SHA, NaN/Inf, wrong grid shape/geographic extent, manipulated geometry digest, invalid triangles/winding, compressed/oversize archive, lost disclaimer/metadata, altered attribution, wrong package SHA.
- Publish: cancelled before stage/after write, worker crash/timeout (orphan stage cleanup), destination outside native authorized location, no-clobber race, after-publication file SHA mismatch; never emit `succeeded` or `Generation Ready` on any such path. Lead owns terminal state/cancel arbitration and safe output.
- Security/privacy: XML controls/HTML/script-bearing attribution, no local path/filename leakage, no telemetry or data upload; original synthetic fixture only in public CI/EXE.
- Physical Windows **new G2 integrated build**: user-chosen private local input and warnings, check exact consent and save path, 3MF in PrusaSlicer and Bambu Studio, per-source attribution and both in-file warnings; owner already passed v0.0.4 synthetic exports, which must not be counted as this new G2 test.

**Errors:** Return allowed frozen native `JobStateV1.reasonCode` values only (e.g. `EXPERIMENTAL_CONSENT_REQUIRED` is *experimental internal* and maps to existing `INVALID_REQUEST` or `EXPORT_FAILED` with safe user-facing message; `INVALID_SOURCE`, `DATA_MISSING`, `HASH_MISMATCH`, `UNSUPPORTED_DATUM`, `INVALID_GEOMETRY`, `CANCELLED` where appropriate). No mutation of JobStateV1 allowed. Enforce cancellation before atomic hard-link; after commit require true native disk SHA verification and correct final status.

## Legal and distribution boundaries

This review **approves the design direction only**: *private/local evaluation does not override upstream data rights or attribution*. Source rights, external contributors, vertical datum and native DEM accuracy for Interlaken remain **unknown/unapproved**. No real-source archive, WebP tile, intermediate private raster, or derived 3MF should enter GitHub, CI/release assets, telemetry or bundled installer. Rust Cargo license/NOTICE staging checks already pass in PR #15; actual packaged Windows Electron/Node/Rust notices still require Lead verification and human release-policy review. They are *software code dependencies*, legally separate from the user's source data or derived terrain rights.

**G2 implementation owner:** Agent 3 builds a separate API after interface agreement, Agent 2 keeps candidate decoding/DTO and optional genuinely sourced URL/attribution fields, Lead owns native consent and job state. The existing draft PR remains unmerged pending Lead reviews and CI.
