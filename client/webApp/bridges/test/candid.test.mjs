import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
import { indexIdl, mediaIdl, registryIdl } from '../candid.mjs';

const methods = (factory) => Object.fromEntries(factory({ IDL })._fields.map(([name, func]) => [name, func]));
const did = (path) => readFileSync(new URL(`../../../../canisters/${path}`, import.meta.url), 'utf8');
const queries = (text) => [...text.matchAll(/^\s+(\w+) : \(.*\) query;$/gm)].map((m) => m[1]);
const roundTrip = (types, value) => IDL.decode(types, IDL.encode(types, value));

test('registry methods are queries that exist in registry.did', () => {
  const m = methods(registryIdl);
  const declared = queries(did('registry/registry.did'));
  for (const [name, f] of Object.entries(m)) {
    assert.ok(declared.includes(name), name);
    assert.deepEqual(f.annotations, ['query']);
  }
  assert.deepEqual(Object.keys(m).sort(), ['by_capture', 'ca_root', 'get', 'get_ix', 'get_key', 'shards']);
});

test('index methods are queries that exist in index.did', () => {
  const declared = queries(did('index/index.did'));
  assert.deepEqual(Object.keys(methods(indexIdl)).filter((n) => !declared.includes(n)), []);
});

test('media methods are queries that exist in media.did', () => {
  const declared = queries(did('media/media.did'));
  const m = methods(mediaIdl);
  assert.deepEqual(Object.keys(m), ['case_info']);
  assert.ok(declared.includes('case_info'));
  assert.deepEqual(m.case_info.annotations, ['query']);
});

test('record types in candid.mjs match the field names in the .did files', () => {
  const text = did('registry/registry.did') + did('index/index.did') + did('media/media.did');
  for (const field of ['media_shard', 'expires_ns', 'chunks', 'mmr_proof', 'leaf_count', 'smt_proof', 'chain_serials', 'active_until_ns', 'truncated_candidates', 'indexed_upto', 'portrait', 'quality']) {
    assert.match(text, new RegExp(`\\b${field} :`), field);
  }
});

test('certified record and key round trip', () => {
  const m = methods(registryIdl);
  const rec = { certificate: Uint8Array.of(1), cose: Uint8Array.of(2), claim: Uint8Array.of(3), anchored_ns: 9n, mmr_proof: [Uint8Array.of(4)], c2pa_claim: Uint8Array.of(5), leaf_count: 2n, record_id: 'X', record_ix: 1n, media_shard: 2 };
  const [got] = roundTrip(m.get.retTypes, [[rec]]);
  assert.equal(got[0].record_id, 'X');
  assert.equal(got[0].media_shard, 2);
  assert.equal(got[0].leaf_count, 2n);
  const key = {
    key: { freshness: { Fresh: null }, provisioning: { Dev: null }, chain_serials: ['a'], key_id: Uint8Array.of(1), enrolled_ns: 1n, security: { Tee: null }, history: [{ status: { Active: null }, since_ns: 1n }], pubkey: Uint8Array.of(2), patch: 7, active_until_ns: 3n },
    certificate: Uint8Array.of(9),
    smt_proof: Uint8Array.of(8),
  };
  const [k] = roundTrip(m.get_key.retTypes, [[key]]);
  assert.deepEqual(Object.keys(k[0].key.security), ['Tee']);
  const shards = { index: [{ open: true, shard: 0, canister: Principal.fromText('aaaaa-aa') }], media: [] };
  const [map, bundle] = roundTrip(m.shards.retTypes, [shards, Uint8Array.of(1, 2)]);
  assert.equal(map.index[0].canister.toText(), 'aaaaa-aa');
  assert.equal(bundle.length, 2);
});

test('case_info round trips and stays optional', () => {
  const m = methods(mediaIdl);
  const [some] = roundTrip(m.case_info.retTypes, [[{ expires_ns: 1n << 62n, bytes: 3n, chunks: 256 }]]);
  assert.equal(some[0].expires_ns, 1n << 62n);
  assert.equal(some[0].chunks, 256);
  const [none] = roundTrip(m.case_info.retTypes, [[]]);
  assert.deepEqual(none, []);
  assert.deepEqual(roundTrip(m.case_info.argTypes, ['ABCD']), ['ABCD']);
});

test('probe and fingerprints round trip', () => {
  const m = methods(indexIdl);
  const value = { pdq: [{ distance: 3, record_ix: 1n << 60n }], exact: [2n], watermark: [], indexed_upto: 4n, truncated_candidates: true };
  const [r] = roundTrip(m.probe.retTypes, [value]);
  assert.equal(r.pdq[0].record_ix, 1n << 60n);
  assert.equal(r.truncated_candidates, true);
  const args = roundTrip(m.probe.argTypes, [[new Uint8Array(32)], [5n], []]);
  assert.equal(args[0][0].length, 32);
  const [f] = roundTrip(m.fingerprints.retTypes, [[{ shapes: [[1, Uint8Array.of(7)]], quality: 80, grid: Uint8Array.of(1), portrait: false }]]);
  assert.equal(f[0].quality, 80);
});
