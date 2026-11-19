import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PRODUCTION, parseConfig } from '../config.mjs';
import { networkIo } from '../witness.mjs';

test('production never trusts the CA root from an uncertified query', async () => {
  let calls = 0;
  const agent = { call: () => calls++, query: () => calls++ };
  const io = networkIo(agent, parseConfig(JSON.stringify({ ...PRODUCTION, registry: 'aaaaa-aa' })), () => calls++);
  await assert.rejects(io.caRoot(), /no pinned CA anchor/);
  assert.equal(calls, 0);
});
