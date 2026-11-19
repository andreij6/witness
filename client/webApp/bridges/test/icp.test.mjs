import assert from 'node:assert/strict';
import { test } from 'node:test';
import { byCapture, caseChunk, caseMeta, getRecord, probeAll, shards } from '../icp.mjs';
import { SHARED_VECTOR } from './vectors.mjs';

const fakeClient = (actors) => ({ agent: { rootKey: new Uint8Array(0) }, registryId: 'reg', actors: new Map(Object.entries(actors)) });

const probeOk = (r, delay = 0) => ({ probe: async (...args) => (await new Promise((res) => setTimeout(res, delay)), r(...args)) });

test('probeAll fans out in parallel and packs results in shard order', async () => {
  const seen = [];
  const client = fakeClient({
    a: probeOk(() => ({ truncated: true, indexed_upto: 4294967298n, pdq: [{ local_ix: 5, shape: 3, distance: 12 }, { local_ix: 4294967295, shape: 255, distance: 65535 }], watermark: [7n], exact: [1099511627777n] }), 20),
    b: { probe: async () => { throw new Error('down'); } },
    c: probeOk(() => ({ truncated: false, indexed_upto: 0n, pdq: [], watermark: [], exact: [] })),
  });
  const packed = await probeAll(client, 'a,b,c', new Uint8Array(32), new Int32Array(0), new Uint8Array(0), 16, (a, t) => seen.push([a, t]));
  assert.deepEqual(Array.from(packed), SHARED_VECTOR);
  assert.equal(seen.length, 3);
  assert.deepEqual(seen.at(-1), [2, 3]);
});

test('probeAll slices flat query buffers', async () => {
  let got;
  const client = fakeClient({ a: probeOk((...args) => ((got = args), { truncated: false, indexed_upto: 1n, pdq: [], watermark: [], exact: [] })) });
  await probeAll(client, 'a', new Uint8Array(64), Int32Array.of(0, 9), new Uint8Array(32), 16, () => {});
  assert.equal(got[0].length, 2);
  assert.equal(got[0][1].length, 32);
  assert.deepEqual(got[1], [9n]);
  assert.equal(got[2].length, 2);
});

test('probeAll with no shards packs an empty result', async () => {
  assert.deepEqual(Array.from(await probeAll(fakeClient({}), '', new Uint8Array(0), new Int32Array(0), new Uint8Array(0), 16, () => {})), [0]);
});

test('byCapture joins record ids', async () => {
  let arg;
  const client = fakeClient({ reg: { by_capture: async (v) => ((arg = v), ['A', 'B']) } });
  assert.equal(await byCapture(client, 0x1fabcdef, 0x01234567), 'A,B');
  assert.equal(arg, 0x1fabcdef01234567n);
});

test('getRecord returns null for a missing record without verifying', async () => {
  const client = fakeClient({ reg: { get: async () => [] } });
  assert.equal(await getRecord(client, 'X'), null);
});

test('getRecord and shards reject unverifiable certificates', async () => {
  const rec = { record_ix: 1n, record_id: 'X', claim: new Uint8Array(0), c2pa_claim: new Uint8Array(0), cose: new Uint8Array(0), anchored_ns: 1n, mmr_proof: [], leaf_count: 1n, certificate: new Uint8Array([1, 2, 3]) };
  const client = fakeClient({ reg: { get: async () => [rec], shards: async () => [{ index: [], media: [] }, new Uint8Array([1])] } });
  client.registryId = 'aaaaa-aa';
  client.actors.set('aaaaa-aa', client.actors.get('reg'));
  await assert.rejects(getRecord(client, 'X'));
  await assert.rejects(shards(client));
});

test('caseMeta maps nat64 fields and caseChunk unwraps opt', async () => {
  const client = fakeClient({
    m: {
      case_meta: async () => [{ count: 2, bytes: 10n, bundle_hash: new Uint8Array(32), expires_ns: 1_700_000_000_123_456_789n }],
      case_chunk: async (_, ix) => (ix === 0 ? [new Uint8Array([1])] : []),
    },
  });
  assert.deepEqual(await caseMeta(client, 'm', new Uint8Array(16)), { count: 2, bytes: 10, bundleHash: new Uint8Array(32), expiresMs: 1_700_000_000_123 });
  assert.deepEqual(await caseChunk(client, 'm', new Uint8Array(16), 0), new Uint8Array([1]));
  assert.equal(await caseChunk(client, 'm', new Uint8Array(16), 1), null);
});

test('caseMeta returns null for unknown cases', async () => {
  assert.equal(await caseMeta(fakeClient({ m: { case_meta: async () => [] } }), 'm', new Uint8Array(16)), null);
});
