// Test-only signatures over ORIGINAL synthetic archive bytes.
// Does not authorize Mapterhorn's contributor licenses, datum or production.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import {
  canonicalize, sha256, signTestEncoding, testOnlyKeypair,
  verifyTestEncoding,
} from '../provenance/test-only-encoding-attestation.mjs';

const syntheticBytes = process.env.A2_SYNTHETIC_ARCHIVE
  ? readFileSync(process.env.A2_SYNTHETIC_ARCHIVE)
  : Buffer.concat([Buffer.from('PMTiles\x03', 'latin1'), Buffer.alloc(160, 0)]);
const evidenceSnapshot = Buffer.from(
  'ORIGINAL TEST SNAPSHOT ONLY: publisher Mapterhorn uses 512px Terrarium/WebP. THIS IS NOT A SIGNED UPSTREAM RELEASE.',
  'utf8'
);
const key1 = testOnlyKeypair('17'.repeat(32));
const key2 = testOnlyKeypair('32'.repeat(32));

function payload() {
  return {
    archiveBytes: syntheticBytes.length,
    archiveSha256: sha256(syntheticBytes),
    archivedPublisherEvidenceSha256: sha256(evidenceSnapshot),
    evidenceSourceName: 'original synthetic fixture only',
    issuerKeyId: 'synthetic-ed25519-1',
    issuerRole: 'test-only-curator',
    keyPolicyRevision: 'test-keyring-revision-1',
    kind: 'VerifiedEncodingEvidenceV1',
    publisherRepoCommitSha: '775663e9ca58c7fe741dbd53fbe4461cb73a9bb9',
    recordSerial: 'synthetic-test-1',
    releasePolicyRevision: 'test-release-policy-1',
    rgbEncoding: 'Terrarium-RGB-1/256m',
    schemaVersion: '1',
    sourceReleaseId: 'ORIGINAL-SYNTHETIC-PMTILES-V1',
    tileCodec: 'webp',
    tileSizePixels: 512,
  };
}
function fixtureOptions(overrides = {}) {
  return {
    archiveBytes: syntheticBytes,
    publisherEvidenceBytes: evidenceSnapshot,
    trustKeys: new Map([['synthetic-ed25519-1', key1.publicKey]]),
    revokedKeyIds: new Set(),
    expectedSourceReleaseId: 'ORIGINAL-SYNTHETIC-PMTILES-V1',
    ...overrides,
  };
}
function okSigned(p = payload()) {
  return signTestEncoding(p, key1.privateKey);
}
function rejection(name, signed, options = {}) {
  assert.throws(
    () => verifyTestEncoding(signed, fixtureOptions(options)),
    /UNTRUSTED_ENCODING_EVIDENCE/,
    name
  );
}

test('RFC8785 canonical flat payload deterministic and Ed25519 verifies test ONLY', () => {
  const p = payload();
  const raw = okSigned(p);
  assert.equal(raw, okSigned(p)); // RFC8032 Ed25519 deterministic signatures
  assert.equal(canonicalize(JSON.parse(raw)), raw);
  assert.equal(raw, signTestEncoding({ ...p }, key1.privateKey));
  const checked = verifyTestEncoding(raw, fixtureOptions());
  assert.equal(checked.sourceEncodingAuthenticatedForExactSha, true);
  assert.equal(checked.sourceLicenseVerified, false);
  assert.equal(checked.verticalDatumVerified, false);
  assert.equal(checked.nativeResolutionVerified, false);
  assert.equal(checked.productionGenerationReady, false);
  console.log(
    'SYNTHETIC_ATTESTATION sourceSha256=' + p.archiveSha256 +
    ' documentSnapshotSha256=' + p.archivedPublisherEvidenceSha256 +
    ' scope=encoding-only keyRevision=' + p.keyPolicyRevision +
    ' releasePolicy=' + p.releasePolicyRevision
  );
});
test('any changed archive byte, e.g. earlier maxzoom 18 header, fails exact SHA', () => {
  const altered = Buffer.from(syntheticBytes);
  altered[101] ^= 1;
  rejection('changed original byte invalidates release binding', okSigned(), { archiveBytes: altered });
  const short = syntheticBytes.subarray(0, syntheticBytes.length - 1);
  rejection('length mismatch', okSigned(), { archiveBytes: short });
});
test('publisher evidence snapshot is SHA-bound, not linked by unverified URL', () => {
  rejection(
    'publisher document tamper',
    okSigned(),
    { publisherEvidenceBytes: Buffer.concat([evidenceSnapshot, Buffer.from('!')]) }
  );
});
test('trusted signer, revocation and key policy revision fail closed', () => {
  rejection('untrusted signer', okSigned(), { trustKeys: new Map() });
  rejection('revoked signer', okSigned(), { revokedKeyIds: new Set(['synthetic-ed25519-1']) });
  rejection('signer-key substitution', okSigned(), {
    trustKeys: new Map([['synthetic-ed25519-1', key2.publicKey]]),
  });
  const rotated = payload();
  rotated.keyPolicyRevision = 'test-keyring-revision-0';
  rejection('old key policy revision', signTestEncoding(rotated, key1.privateKey));
});
test('payload tamper, exact source release mismatch and unsafe rights flags reject', () => {
  const wrongRelease = payload();
  wrongRelease.sourceReleaseId = 'real-mapterhorn-release-UNVERIFIED';
  rejection('source release scope mismatch', signTestEncoding(wrongRelease, key1.privateKey));
  const altered = JSON.parse(okSigned());
  altered.payload.tileCodec = 'png';
  rejection('signed payload altered', canonicalize(altered));
  const fakeRights = JSON.parse(okSigned());
  fakeRights.payload.sourceLicenseVerified = true;
  rejection('unknown authorization field', canonicalize(fakeRights));
  const unknown = JSON.parse(okSigned());
  unknown.payload.verticalDatumId = 'LN02';
  rejection('datum cannot be smuggled into encoding policy', canonicalize(unknown));
});
test('forbidden noncanonical JSON, duplicates, fake algorithm and wrong domain reject', () => {
  const raw = okSigned();
  rejection('whitespace in envelope is not canonical', raw + '\n');
  rejection('duplicate key', raw.replace('{"payload":', '{"payload":null,"payload":'));
  const changed = JSON.parse(raw);
  changed.algorithm = 'Ed25519';
  rejection('unsigned algorithm downgrade', canonicalize(changed));
  const sig = JSON.parse(raw);
  sig.signatureEd25519Base64 = sig.signatureEd25519Base64.replace(/^./, 'A');
  rejection('detached signature altered', canonicalize(sig));
  assert.throws(() => testOnlyKeypair('wrong'), /UNTRUSTED_ENCODING_EVIDENCE/);
});
test('actual consumer trust proof does not change unverified source gates', () => {
  const verified = verifyTestEncoding(okSigned(), fixtureOptions());
  assert.equal(verified.productionGenerationReady, false);
  // No function in this test-only module mutates the production importer,
  // frozen TerrainSourceManifestV1 or native readiness.
  assert.equal(verified.verticalDatumVerified, false);
  assert.equal(verified.sourceLicenseVerified, false);
});
