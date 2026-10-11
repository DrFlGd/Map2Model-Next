// TEST-ONLY reference — NOT a trusted production attestation verifier.
// Uses built-in Node Ed25519 and the RFC 8785 JSON canonicalization rules
// on a strict, flat, bounded string/safe-integer payload. No external keys,
// downloaded trust roots, third-party source bytes or real-data approvals.
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';

export const DOMAIN = Buffer.from('MAP2MODEL-TERRARIUM-ENCODING-ATTESTATION-V1\0', 'ascii');
export const FIELDS = Object.freeze([
  'archiveBytes',
  'archiveSha256',
  'archivedPublisherEvidenceSha256',
  'evidenceSourceName',
  'issuerKeyId',
  'issuerRole',
  'keyPolicyRevision',
  'kind',
  'publisherRepoCommitSha',
  'recordSerial',
  'releasePolicyRevision',
  'rgbEncoding',
  'schemaVersion',
  'sourceReleaseId',
  'tileCodec',
  'tileSizePixels',
]);
const SHA = /^[a-f0-9]{64}$/;
const COMMIT_SHA = /^[a-f0-9]{40}$/;

function fail(message) {
  throw new Error('UNTRUSTED_ENCODING_EVIDENCE: ' + message);
}
export function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}
export function canonicalize(value) {
  // RFC 8785 JSON canonicalization for the intentionally restricted test schema:
  // strings, integers, booleans, null, arrays, and unique-key plain objects.
  // UTF-16 lexical sorting matches JCS object property key ordering.
  if (value === null || typeof value === 'string' || typeof value === 'boolean') {
    return JSON.stringify(value);
  }
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value)) fail('only safe integers supported');
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return '[' + value.map(canonicalize).join(',') + ']';
  }
  if (typeof value === 'object' && Object.getPrototypeOf(value) === Object.prototype) {
    return '{' + Object.keys(value).sort().map((key) =>
      JSON.stringify(key) + ':' + canonicalize(value[key])
    ).join(',') + '}';
  }
  fail('unsupported payload type');
}
export function validateEncodingPayload(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) fail('payload not an object');
  const found = Object.keys(payload).sort();
  if (JSON.stringify(found) !== JSON.stringify(FIELDS)) fail('missing/unknown payload property');
  const strings = [
    'evidenceSourceName', 'issuerKeyId', 'issuerRole', 'keyPolicyRevision',
    'kind', 'publisherRepoCommitSha', 'recordSerial', 'releasePolicyRevision',
    'rgbEncoding', 'schemaVersion', 'sourceReleaseId', 'tileCodec',
    'archiveSha256', 'archivedPublisherEvidenceSha256',
  ];
  for (const name of strings) {
    if (typeof payload[name] !== 'string' || !payload[name] || payload[name].length > 256) {
      fail('invalid required string: ' + name);
    }
  }
  if (!SHA.test(payload.archiveSha256) || !SHA.test(payload.archivedPublisherEvidenceSha256)) {
    fail('source/evidence SHA must be lowercase 64 hex');
  }
  if (!COMMIT_SHA.test(payload.publisherRepoCommitSha)) fail('invalid publisher commit');
  if (!Number.isSafeInteger(payload.archiveBytes) || payload.archiveBytes < 127 || payload.archiveBytes > 128 * 1024 * 1024) {
    fail('archive byte bound mismatch');
  }
  if (payload.tileSizePixels !== 512 || payload.rgbEncoding !== 'Terrarium-RGB-1/256m' || payload.tileCodec !== 'webp') {
    fail('unsupported binary format');
  }
  if (payload.kind !== 'VerifiedEncodingEvidenceV1' || payload.schemaVersion !== '1') {
    fail('unknown evidence schema');
  }
  if (payload.issuerRole !== 'test-only-curator') fail('role cannot be promoted');
  if (payload.keyPolicyRevision !== 'test-keyring-revision-1' ||
      payload.releasePolicyRevision !== 'test-release-policy-1') {
    fail('incorrect release/key trust revision');
  }
}
export function signedBytes(payload) {
  validateEncodingPayload(payload);
  return Buffer.concat([DOMAIN, Buffer.from(canonicalize(payload), 'utf8')]);
}
export function signTestEncoding(payload, privateKey) {
  return canonicalize({
    payload,
    signatureEd25519Base64: sign(null, signedBytes(payload), privateKey).toString('base64'),
  });
}
export function verifyTestEncoding(jsonText, {
  archiveBytes, publisherEvidenceBytes, trustKeys, revokedKeyIds,
  expectedSourceReleaseId,
}) {
  // Enforce canonical envelope; duplicate raw JSON properties cannot normalize.
  let envelope;
  try { envelope = JSON.parse(jsonText); } catch { fail('invalid JSON'); }
  if (canonicalize(envelope) !== jsonText) fail('noncanonical/duplicate JSON envelope');
  if (Object.keys(envelope).sort().join(',') !== 'payload,signatureEd25519Base64') {
    fail('unknown envelope field');
  }
  validateEncodingPayload(envelope.payload);
  const p = envelope.payload;
  if (p.sourceReleaseId !== expectedSourceReleaseId) fail('wrong release scope');
  if (p.archiveBytes !== archiveBytes.length || p.archiveSha256 !== sha256(archiveBytes)) {
    fail('exact source archive SHA/size mismatch');
  }
  if (p.archivedPublisherEvidenceSha256 !== sha256(publisherEvidenceBytes)) {
    fail('publisher document snapshot mismatch');
  }
  if (revokedKeyIds.has(p.issuerKeyId)) fail('signing key revoked');
  const key = trustKeys.get(p.issuerKeyId);
  if (!key) fail('untrusted signer key');
  const signature = Buffer.from(envelope.signatureEd25519Base64, 'base64');
  if (signature.length !== 64 ||
      !verify(null, signedBytes(p), key, signature)) {
    fail('Ed25519 signature mismatch');
  }
  return Object.freeze({
    sourceEncodingAuthenticatedForExactSha: true,
    sourceLicenseVerified: false,
    verticalDatumVerified: false,
    nativeResolutionVerified: false,
    productionGenerationReady: false,
  });
}
// RFC8032-style PKCS8 test-only key; the private seed is PUBLIC in the source
// and MUST NOT appear in a shipping trust keyring.
export function testOnlyKeypair(hex32) {
  if (!/^[a-f0-9]{64}$/.test(hex32)) fail('test seed size');
  const prefix = Buffer.from('302e020100300506032b657004220420', 'hex');
  const privateKey = createPrivateKey({
    key: Buffer.concat([prefix, Buffer.from(hex32, 'hex')]),
    format: 'der',
    type: 'pkcs8',
  });
  return { privateKey, publicKey: createPublicKey(privateKey) };
}
