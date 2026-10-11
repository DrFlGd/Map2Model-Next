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
