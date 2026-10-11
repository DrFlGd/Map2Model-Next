# Agent 2 proposal — exact-archive encoding attestation for attribution-only Mapterhorn PMTiles

**Status:** DESIGN FOR LEAD REVIEW ONLY; not approved, implemented in the production importer or a substitute for source license/vertical datum review. **Scope:** G1 extension inside `crates/data/`, without changing a single frozen G0 V1 contract.

## Verified motivating facts

- The privately supplied **corrected** Interlaken extract is **3,127,295 bytes**, SHA-256 `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0`; the owner reports `pmtiles verify` success, and independent byte parsing identifies 40 valid 512×512 RGB WebPs at z13–17.
- The archive's real JSON metadata has attribution but **no `encoding` declaration**. The current production importer rejects this; the independent format-only inspector may calculate **candidate** Terrarium numbers but never authenticates their meaning, source rights or vertical reference.
- The original malformed-header archive is a **different** SHA (`40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce`) even though its image bytes match. Any attestation for the corrected file MUST NOT cover the original SHA.

## Suggested versioned, non-G0 attestation record

Only after lead approval, define **internal** `VerifiedEncodingEvidenceV1` as a separate immutable JSON record in the trusted native registry (not as part of `TerrainSourceManifestV1`). Example schema illustration (NOT verified evidence):

```json
{
  "schemaVersion": "1",
  "kind": "VerifiedEncodingEvidenceV1",
  "exactArchiveSha256": "75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0",
  "archiveBytes": 3127295,
  "tileCodec": "webp",
  "widthPixels": 512,
  "heightPixels": 512,
  "rgbElevationEncoding": "Terrarium-RGB-1/256m",
  "evidencePublisher": null,
  "independentPublisherDocumentUrl": null,
  "archivedPublisherDocumentSha256": null,
  "recordVersion": null,
  "trustedAttestorKeyId": null,
  "signature": null
}
```

All **null** fields are **mandatory missing evidence**, never defaults. The native host needs a **separately provisioned, immutable allowlist of trusted attestor public keys**, not a renderer-provided label or arbitrary JSON `verified:true`. Verify the signature over a specified canonical form; bind **every required field**, file digest and length; ensure document/release evidence is independently obtained and pinned, not inferred from a general marketing page/attribution. Read the registered source bytes, hash them again and match the exact attested SHA **before** decoding.

This record could authorize the **encoding interpretation only**. It must never set `verticalDatumStatus=known`, `licenseIdentifier`, `nativeResolutionSource`, or `coverage_status` automatically.

## Fail-closed acceptance matrix

| Source format situation | Inspector | Production registry/generation |
|---|---|---|
| Metadata explicitly `encoding=terrarium` + known, verified policy | May decode as format check | Allowed only after independent datum, rights, resolution, archive and tile-index checks |
| Attribution-only metadata, no trusted external encoding attestation | May decode **candidate** RGB values and report unknown | **Reject** (`INVALID_SOURCE`) |
| External signed encoding attestation, exact SHA and data format verified | Mark **encoding evidence** verified only | **Still reject** unless datum, rights, resolution and actual tile presence independently pass |
| Encoding attestation for another file SHA, earlier header version, missing signature, unapproved signer, unsupported version, stale/mismatched document | Reject attestation | **Reject** |
| Unknown/mixed vertical datum, missing regional derivative rights or native-resolution evidence, even with valid encoding attestation | Format-only inspection | **Reject** (`UNSUPPORTED_DATUM` or appropriate source policy reason) |

The V1 source manifest and `ElevationGridV1` remain exactly frozen. The attestation would be stored only in the native registry and referenced internally by code; no new fields in frozen IPC or manifest contracts. No networking by the application (source/evidence acquisition is an explicitly authorized, separate preparation step).

## Test strategy before implementation/acceptance

1. Fully synthetic PMTiles with attribution-only metadata and a signed test attestation. Assert exact-archive binding, schema version, signer allowlist, payload tampering, SHA mismatch (including single changed max-zoom header byte), signer-key rotation and no source fallback.
2. Keep the existing synthetic negative regression: unattributed encoding **does not** register or generate even when pixels are 512×512 Terrarium-shaped.
3. Independently test the private z17 binary *offline* with `terrain_offline inspect` first, using the SHA-pinned `private_mapterhorn.rs` harness and exact independent pixel goldens. This format test does **not** assert trusted encoding or production readiness.
4. Even once encoding evidence is accepted, **keep vertical datum unknown** and `Generation Ready=false` until release/region-specific datum, attribution, derivative-use rights, native accuracy and actual selection XYZ coverage pass; also require disconnected Windows native/GUI smoke.

**Lead decision requested:** whether to authorize this per-file signed encoding evidence and designate the trusted issuer/document; **do not** merge an unsigned self-attestation or silently relax the current importer.


## October 11 provenance update: authoritative encoding publisher identified

See [`INTERLAKEN-PROVENANCE-REVIEW-2026-10-11.md`](INTERLAKEN-PROVENANCE-REVIEW-2026-10-11.md). Source publisher Mapterhorn's upstream repository at commit `775663e9ca58c7fe741dbd53fbe4461cb73a9bb9` provides direct evidence:
- `pipelines/utils.py` Git blob `e2ada1cd53e5ca3b52dba9800cfe9d94b5750eed` **deliberately** writes Terrarium pixels via `save_terrarium_tile`, then `create_archive` uses *attribution-only* PMTiles metadata (WebP tile type, `tile_compression=none`). Missing archive metadata `encoding` is an **upstream design**, not something to be "repaired" by guessing.
- `pipelines/README.md` Git blob `6f3b5a5dd9e69a8458ccb8249a4b06603167a54f` independently documents the mosaic-to-Terrarium process and 512px RGB WebP output.
- `website/examples/pmtiles/example.html` Git blob `2e110119bb4c10ec741bcfc8d14e78e02fb4c49b` explicitly configures **Terrarium encoding and 512px** in a viewer of `6-{x}-{y}.pmtiles`.
- Publisher data-access `https://mapterhorn.com/data-access/` also declares Terrarium/WebP/512px and z13–17 Interlaken PMTiles extraction.

**Source-of-truth hierarchy:** first-party publisher code/docs prove an *upstream encoding convention*; a verified local file SHA proves **exact archive bytes**; a trusted downstream attestation can bind these **two claims** *only after the publisher evidence has been independently snapshotted and hashed*. The historical main-branch Git commit and Git *blob SHA-1s* above are **not**, by themselves, a signed upstream dataset release or evidence-document SHA-256, and must never be substituted for one. A source file's own PMTiles attribution JSON is **not** a digital signature.

### Proposed deterministic trust specification (review target, not shipping authorization)

**Issuer roles:** Mapterhorn is authoritative for the documented encoding convention; an internal *Map2Model source curator* could, after independent evidence review, issue a file-specific signed binding. **The curator's signature is not falsely described as Mapterhorn's signature.** The product owner/lead approves an initial allowlist of curator Ed25519 verification keys through a signed application release, **not** through user-supplied PMTiles, renderer IPC, arbitrary registry paths or an automatically downloaded manifest.

1. Proposed signature algorithm: **Ed25519 (RFC 8032)**, no alternative algorithms/downgrade; verify with a pinned native-only keyring. Domain-separated signing message: ASCII `MAP2MODEL-TERRARIUM-ENCODING-ATTESTATION-V1` followed by a NUL octet and **RFC 8785 canonical UTF-8 JSON** of the strict *payload object only* (all fields defined and type checked; no `signature` field inside signed payload). Disable duplicate properties/unknown fields, non-finite values, unsafe integers and ambiguous number representations; prefer strings and bounded integers. Treat canonical bytes as the *only* signature input. The current `canonical::manifest_digest` pertains to frozen G0 objects, **not** this new signature domain.
2. Payload proposed mandatory fields: `kind`, `schemaVersion`, `exactArchiveSha256`, `archiveBytes`, `tileCodec`, `tileSizePixels`, `rgbEncoding`, `issuerKeyId`, `issuerRole`, `publishedAtUtc`, `evidenceSourceName`, `publisherRepoCommitSha`, `publisherCodeBlobSha`, `publisherDocumentationBlobSha`, `archivedPublisherEvidenceSha256`, `recordSerial`. Constrain source SHA to **64 lowercase hex** and numeric bytes to safe bounded positive integer; timestamps RFC3339 UTC; no permissive aliases. `signatureEd25519Base64` is the detached signature over the payload.
3. **Trust provisioning and rotation:** ship allowed `(issuerRole,issuerKeyId,pubkey,notBeforeUtc,notAfterUtc)` tuples inside the lead-owned, code-signed installer. No TOFU ("trust on first use") and no loading keys from renderer-supplied JSON. Add/retire keys only through a signed application update reviewed by the lead. Historical attestations must be tied to a stable release trust snapshot; if a key is revoked for compromise, fail closed even if the signature was valid previously. An offline client needs a **bundled** policy/revocation revision and should not silently assert that it has the *latest* online revocations.
4. **Exact SHA binding:** authenticate the original PMTiles file as a bounded native local handle, SHA-256 its *actual bytes* and match the signed `exactArchiveSha256`, `archiveBytes`, declared WebP/Terrarium constraints, real indexed tile proof and pinned provider documentation hash. The original invalid maxZoom=18 file `40f452bd…` must always reject when presented with the corrected z17 attestation `75e03d8e…`.
5. **Separation of concerns:** a successfully verified encoding signature may set an internal `sourceEncodingAuthenticated` flag ONLY. It must **never** set `sourceLicenseVerified`, `verticalDatumStatus=known`, `nativeResolutionVerified` or `productionGenerationReady`; those have different independent mandatory evidence and the frozen manifest validator still rejects unknown/unsupported datum. No mesh/3MF is rendered from unauthenticated real-source data.
6. **No live network for trust:** publisher documents and source attestation should be retrieved in a separately controlled lawful provenance-preparation phase, snapshotted, hashed, peer-reviewed and included in a signed release. All offline Windows acceptance paths must work without network.

**Minimum negative tests** *before this design is ever approved or shipped* (synthetic data and test-only keys, no Mapterhorn archive committed): untrusted/no key; altered SHA even a one-byte PMTiles header change; same WebP payload on wrong archive; wrong archive length; stale/revoked key; key ID replacement/unknown issuer role; duplicate/unknown JSON fields; non-canonical payload; unsupported attestation schema/algorithm; modified publisher-document hash/commit; modified `tileSizePixels`/`rgbEncoding`; wrong domain prefix; detached signature tampering; issuer key from renderer path; old release trust list; downgraded attestation; and a positive **encoding-only** signature that still fails `UNSUPPORTED_DATUM` for a genuinely unverified source.

**Lead decision outstanding:** Authorize the curator role, trust-root provisioning/revocation strategy, release-audited evidence-document SHA-256 and final canonical signing specification **before** implementing an acceptance path. Source-specific rights/attribution, vertical reference, upstream release identity and regional native source resolution are still separate blockers. No product-owner request for another standalone inspector is appropriate: **next app test = integrated Windows preview**.
