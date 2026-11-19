import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SHARED_VECTOR } from './vectors.mjs';
import { getU64, nsToMs, packProbes, packedSize, putU64, SHARD_FAILED, slices, splitIds, u64Pairs, unwrapOpt } from '../wire.mjs';

const SHARED_RESULTS = [
  {
    truncated: true,
    indexed_upto: 4294967298n,
    pdq: [
      { local_ix: 5, shape: 3, distance: 12 },
      { local_ix: 4294967295, shape: 255, distance: 65535 },
    ],
    watermark: [7n],
    exact: [1099511627777n],
  },
  null,
  { truncated: false, indexed_upto: 0n, pdq: [], watermark: [], exact: [] },
];

test('packProbes matches the vector shared with Kotlin ProbeWireTest', () => {
  const packed = packProbes(SHARED_RESULTS);
  assert.ok(packed instanceof Int32Array);
  assert.deepEqual(Array.from(packed), SHARED_VECTOR);
  assert.equal(packedSize(SHARED_RESULTS), SHARED_VECTOR.length);
});

test('packProbes accepts typed nat64 vectors as decoded by Candid', () => {
  const typed = SHARED_RESULTS.map((r) => r && { ...r, watermark: BigUint64Array.from(r.watermark), exact: BigUint64Array.from(r.exact) });
  assert.deepEqual(Array.from(packProbes(typed)), SHARED_VECTOR);
});

test('failed shard occupies a zeroed header', () => {
  assert.deepEqual(Array.from(packProbes([null])), [1, SHARD_FAILED, 0, 0, 0, 0, 0, 0]);
  assert.deepEqual(Array.from(packProbes([])), [0]);
});

test('u64 halves round trip at the edges', () => {
  const out = new Int32Array(2);
  for (const v of [0n, 1n, 0xffffffffn, 0x100000000n, (1n << 63n) - 1n, (1n << 64n) - 1n]) {
    putU64(out, 0, v);
    assert.equal(getU64(out, 0), v);
  }
  putU64(out, 0, 42);
  assert.equal(getU64(out, 0), 42n);
});

test('slices are zero-copy views', () => {
  const flat = new Uint8Array([1, 2, 3, 4, 5, 6]);
  const parts = slices(flat, 3);
  assert.equal(parts.length, 2);
  assert.deepEqual(Array.from(parts[1]), [4, 5, 6]);
  parts[0][0] = 9;
  assert.equal(flat[0], 9);
  assert.deepEqual(slices(new Uint8Array(0), 32), []);
  assert.throws(() => slices(flat, 4), RangeError);
  assert.throws(() => slices(flat, 0), RangeError);
});

test('u64Pairs decodes hi/lo pairs', () => {
  assert.deepEqual(u64Pairs(Int32Array.of(0x1fabcdef, 0x01234567, 0, -1)), [0x1fabcdef01234567n, 0xffffffffn]);
  assert.deepEqual(u64Pairs(new Int32Array(0)), []);
  assert.throws(() => u64Pairs(Int32Array.of(1)), RangeError);
});

test('splitIds handles empty and single ids', () => {
  assert.deepEqual(splitIds(''), []);
  assert.deepEqual(splitIds('aaaaa-aa'), ['aaaaa-aa']);
  assert.deepEqual(splitIds('a,b'), ['a', 'b']);
});

test('nsToMs truncates', () => {
  assert.equal(nsToMs(1_700_000_000_123_456_789n), 1_700_000_000_123);
  assert.equal(nsToMs(0n), 0);
});

test('unwrapOpt follows Candid opt encoding', () => {
  assert.equal(unwrapOpt([]), null);
  assert.equal(unwrapOpt([7]), 7);
});
