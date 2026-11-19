import assert from 'node:assert/strict';
import { test } from 'node:test';
import { IDL } from '@icp-sdk/core/candid';
import { indexIdl, mediaIdl, registryIdl } from '../idl.mjs';

const methods = (factory) => Object.fromEntries(factory({ IDL })._fields.map(([name, func]) => [name, func]));

test('registry exposes read-only queries', () => {
  const m = methods(registryIdl);
  assert.deepEqual(Object.keys(m).sort(), ['by_capture', 'challenge', 'get', 'get_ix', 'shards']);
  for (const f of Object.values(m)) assert.deepEqual(f.annotations, ['query']);
  assert.equal(m.shards.retTypes.length, 2);
});

test('index probe result round trips through Candid', () => {
  const probe = methods(indexIdl).probe;
  const value = {
    pdq: [{ local_ix: 4294967295, shape: 7, distance: 31 }],
    watermark: [1n << 60n],
    exact: [3n],
    truncated: true,
    indexed_upto: 99n,
  };
  const decoded = IDL.decode(probe.retTypes, IDL.encode(probe.retTypes, [value]))[0];
  assert.deepEqual({ ...decoded, watermark: Array.from(decoded.watermark), exact: Array.from(decoded.exact) }, value);
  const args = IDL.encode(probe.argTypes, [[new Uint8Array(32)], [5n], []]);
  assert.equal(IDL.decode(probe.argTypes, args)[0][0].length, 32);
});

test('media case queries', () => {
  const m = methods(mediaIdl);
  assert.deepEqual(Object.keys(m).sort(), ['case_chunk', 'case_meta']);
  const meta = { count: 2, bytes: 10n, bundle_hash: new Uint8Array(32), expires_ns: 5n };
  const decoded = IDL.decode(m.case_meta.retTypes, IDL.encode(m.case_meta.retTypes, [[meta]]))[0][0];
  assert.equal(decoded.count, 2);
  assert.equal(decoded.expires_ns, 5n);
});
