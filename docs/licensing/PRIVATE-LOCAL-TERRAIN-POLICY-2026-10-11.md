# Owner decision: private/local experimental terrain, with attribution and uncertainty disclaimer

**Product direction confirmed:** 2026-10-11, by the product owner. **Scope:** personal/local, non-distributed user-generated terrain evaluation. **Status:** lead policy proposal for Agent 2 and Agent 3 implementation review, NOT legal advice, upstream license clearance, verified elevation accuracy, or permission to redistribute source/derived files. This is a deliberate product choice informed by the *local generation + source credits + user responsibility* approach associated with earlier Map2Model behavior. No prior third-party project's code, legal terms or license rights are being copied or adopted.

## Decision: allow two genuinely different paths

| | Verified terrain path | Experimental local personal-use path |
| --- | --- | --- |
| Source | Source identity/rights/coverage/datum evidence approved under the project's release rules | **User-selected, already locally acquired PMTiles**; show known provider, metadata and any source terms; no automatic fetching |
| Rights | Reviewed, attributable data license and release | User affirms local use complies with applicable source terms. **Acknowledgement is not a legal license or a substitute for prohibited source use.** Unknown rights remain clearly marked unknown, not "approved" |
| Datum / heights | Verified datum, units and supported transformations | Candidate Terrarium-decoded heights only; `verticalDatumStatus=unknown`. No absolute elevation/datum, location or survey accuracy certification |
| Trusted status | G0 `TerrainSourceManifestV1` + `ElevationGridV1` can be verified under their real readiness rules | Separate `ExperimentalLocalTerrainPreviewV1` and **explicit experimental mesh/export boundary**. NEVER forge `verticalDatumStatus=known` or a verified V1 manifest to unlock generation |
| 3MF output | Fully verified supported output when all applicable gates pass | Locally created **experimental** 3MF only after an explicit per-job warning/opt-in, with embedded attribution, source SHA and unknown-datum warning, no distribution authorization |
| Publishing/sharing | Governed by source and software terms | No app-driven uploads, public/share links, redistribution, paid usage or publishing of underlying data or derived models via this mode. Recheck actual rights before any such use |
| UI labels | Verified Source / Generation Ready when evidence supports it | **Experimental · private local use · unverified source/datum**, never "Generation Ready" or "licensed/approved" |

**Important:** "For private use" describes this feature's intended scope, **not an automatic exception** to copyright, attribution, access controls, caching, or data derivatives' licensing obligations. The project should neither falsely prohibit lawful personal testing solely because a datum is unknown, nor assure the user that a checkbox cures prohibited access, copying or derivative use. If known source terms expressly prohibit the contemplated processing, copying, caching or derivative creation, block that specific action and explain why; do not invite circumvention. Unknown source rights must remain explicitly `unknown` and user-responsibility warnings should remain visible.

## Local-first privacy and rights implementation

1. **User initiates** import with a trusted native local file picker. No bundled, uploaded, mirrored, public CI or Git-committed real Mapterhorn tiles, contributor rasters, private project archives, or real-source .3mf artifacts. No implicit remote download, bulk scraper, HTTP fallback or permission to extract more than the user already lawfully obtained. Software EXEs may be released separately under their *code/dependency* obligations, with original synthetic fixtures only.
2. Before **each experimental job**, show the full disclaimer and an unchecked-by-default acceptance checkbox. Bind this acknowledgment to exact input archive SHA-256, selection ID, requested operation and job ID. No global remembered permission, no prechecked box, no fabricated source license or datum.
3. Verify a regular, safely sized local file and SHA, actual PMTiles/WebP format and closed selection coverage. Refuse missing/alpha/no-data samples, malformed archives, unexpected encoding declarations, byte mutation or resource-limit violations. Store source attribution exactly when present but render external metadata as **untrusted text** (never executable HTML).
4. Preserve separate status fields: `experimentalOnly=true`, `encodingAuthenticated=false` when an authoritative encoding attestation is absent, `verticalDatumStatus=unknown`, `sourceLicenseVerified=false`, `nativeResolutionVerified=false`, `redistributionApproved=false`, `productionGenerationReady=false`. A successful *numeric candidate* decode is not a verified height measurement. Do not show fake datum, precision or native resolution.
5. Build an actual printable mesh, if the reviewed **separate** experimental bridge supports it, without ever representing the unverified preview as a frozen verified grid. Save local-only 3MF via native Save dialog and atomic no-clobber exporter, include the warning/attribution/source and geometry hashes in package provenance. The app must not remove experimental status in project reopen, export or CLI/IPC.
6. Show and embed the applicable provider attribution and source link(s), the best observed archive metadata, original local SHA, dataset date/release **when known**, selected bbox, candidate decoding convention and uncertainty. Unknown or ambiguous details remain marked **unknown**; do not substitute "public domain", "CC0" or "licensed" for real-world files. Include a noncommercial/personal-use policy notice without asserting it overrides external licensing.
7. Keep project data and model outputs on the user's machine. No app telemetry, uploads, share links or auto-publishing. An app/software binary distribution **must not** include the user's third-party DEM files, extracted real tiles or their generated models. Full Electron/Node/Rust code dependency notices remain a separate software distribution obligation.
8. Never claim that v0.0.4 synthetic PMTiles owner dual-slicer acceptance proves real local experimental source/3MF behavior; physical Windows testing for the new experimental flow remains separate and pending.

## Proposed in-product disclosure (draft UX copy; Lead owns final wording)

**Title:** Experimental terrain — private local use only

**Body:**

> This file was supplied from your computer. Map2Model Next has not independently verified the terrain's source licenses, contributors, native resolution or vertical datum. Elevation values are provisional and may be inaccurate or measured against a different vertical reference. Models produced in this mode are for your own local evaluation, not certified geographical measurements. You are responsible for obtaining and using the source under its applicable terms, including attribution and any restrictions on copying or derivative use. Keep source and model files private; do not publish, sell, share or redistribute them without checking the underlying rights. No data is uploaded by Map2Model Next.

**Checkbox (unchecked by default):**

> I acquired this file under applicable terms, accept the unverified terrain limitations, and will use this experimental export only for my private local evaluation.

**Persistent export banner / embedded 3MF notice:**

> EXPERIMENTAL — USER-SUPPLIED TERRAIN. Source permissions not independently verified; vertical datum and native accuracy unknown. Private local evaluation only; third-party license and attribution requirements still apply. Not a certified elevation model. Redistribution not approved by Map2Model Next.

*Policy copy is a design draft, not a legal opinion. Review it with counsel before a broader public/paid/commercial release if that becomes the product scope.*

## Relationship to Agent 2's existing work and frozen contracts

Agent 2's already-implemented [experimental draft policy](https://github.com/DrFlGd/Map2Model-Next/blob/agent/2-A2-02-terrarium-grid/crates/data/EXPERIMENTAL-LOCAL-TERRAIN-POLICY.md) and [native handoff](https://github.com/DrFlGd/Map2Model-Next/blob/agent/2-A2-02-terrarium-grid/crates/data/EXPERIMENTAL-LOCAL-TERRAIN-HANDOFF.md) remain the correct **starting point**, subject to both agents' review. The `decode_experimental_local_pmtiles` API already emits `ExperimentalLocalTerrainPreviewV1` with owner consent and unknown/unverified flags. Do not build a second conflicting decoder or silently wire its unverified preview into the verified `export_3mf` by inventing a `TerrainSourceManifestV1`.

The six frozen G0 V1 schema contracts and existing verified import/export paths must **not change silently**. Lead and Agent 3 should design a separate experimental-only mesh/export adapter and explicit job/provenance envelope; Agent 2 should maintain the local-bound decoder and warnings. Any contractual changes require separate versioned review.

## Primary reference pointers — informative, not clearance

- [Mapterhorn upstream code and terrain-data attribution](https://github.com/mapterhorn/mapterhorn) / [current provider attribution](https://mapterhorn.com/attribution/). The code's open-source license is **not** a blanket grant to all underlying elevation datasets.
- [Existing Map2Model local-first source policy](SOURCE_POLICY.md) and [real-sample provenance protocol](../agent-handoff/MAPTERHORN-SAMPLE-PROTOCOL.md).
- The Map2Model concept of local model generation, attribution, disclaimers, and user responsibility is an **approach**, not borrowed permission. Do not import legal language, code, artwork, branding or copyrighted content from prior Map2Model versions.
