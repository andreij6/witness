import assert from 'node:assert/strict';
import { test } from 'node:test';
import { open, openChunk, randomBytes, seal, sealChunk, sha256 } from '../crypto.mjs';

const hex = (s) => Uint8Array.from(s.match(/../g).map((b) => parseInt(b, 16)));
const toHex = (u) => Buffer.from(u).toString('hex');

const KEY = hex('feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308');
const NONCE = hex('cafebabefacedbaddecaf888');
const PLAIN = hex('d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39');
const AAD = hex('feedfacedeadbeeffeedfacedeadbeefabaddad2');
const CHUNK =
  'cafebabefacedbaddecaf888' +
  '522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662' +
  '76fc6ece0f4e1768cddf8853bb2d551b';

test('sealChunk matches the GCM vector shared with Kotlin', async () => {
  assert.equal(toHex(await sealChunk(KEY, NONCE, PLAIN, AAD)), CHUNK);
});

test('openChunk inverts sealChunk', async () => {
  assert.deepEqual(await openChunk(KEY, hex(CHUNK), AAD), PLAIN);
});

test('openChunk rejects tampering, wrong aad and short input', async () => {
  const bad = hex(CHUNK);
  bad[30] ^= 1;
  assert.equal(await openChunk(KEY, bad, AAD), null);
  assert.equal(await openChunk(KEY, hex(CHUNK), new Uint8Array(1)), null);
  assert.equal(await openChunk(KEY, new Uint8Array(27), AAD), null);
  assert.equal(await open(new Uint8Array(32), NONCE, hex(CHUNK).subarray(12), AAD), null);
});

test('seal of empty plaintext is just the tag', async () => {
  assert.equal((await seal(KEY, NONCE, new Uint8Array(0), new Uint8Array(0))).length, 16);
});

test('sha256 vectors', async () => {
  assert.equal(toHex(await sha256(new Uint8Array(0))), 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
  assert.equal(toHex(await sha256(new TextEncoder().encode('abc'))), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
});

test('randomBytes length', () => {
  assert.equal(randomBytes(12).length, 12);
  assert.equal(randomBytes(0).length, 0);
});
