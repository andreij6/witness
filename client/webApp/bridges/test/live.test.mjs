import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseConfig } from '../config.mjs';
import { open } from '../witness.mjs';

const local = new URL('../../../../.icp-local/', import.meta.url);
const seedFile = new URL('web-seed.json', local);
const live = process.env.WITNESS_LIVE === '1' && existsSync(seedFile);
const read = (name) => readFileSync(new URL(name, local));

test('live local replica round trip', { skip: !live && 'set WITNESS_LIVE=1 after running the web seed' }, async () => {
  const ids = JSON.parse(read('ids.json'));
  const seed = JSON.parse(read('web-seed.json'));
  const config = parseConfig(JSON.stringify({ host: ids.gateway, registry: ids.canisters.registry.id, canister: ids.gateway.replace('://', '://*.'), local: true }));
  const wasm = readFileSync(new URL('../wasm/witness_web_bg.wasm', import.meta.url));
  const viaGateway = (url) => {
    const u = new URL(url);
    const [id] = u.hostname.split('.');
    return fetch(`${ids.gateway}${u.pathname}?canisterId=${id}`);
  };
  const c = await open(config, wasm, viaGateway);

  const proof = JSON.parse(await c.proof(seed.edited));
  assert.equal(proof.recordId, seed.edited);
  assert.equal(proof.claim.kind.type, 'derivative');
  assert.equal(proof.claim.kind.edits[0].kind, 'blur');
  assert.equal(await c.proof('0000000000000000'), null);
  assert.ok((await c.key(proof.claim.keyId)).includes(proof.claim.keyId));
  const preview = await c.preview(seed.post);
  assert.deepEqual([...preview.subarray(0, 2)], [0xff, 0xd8]);

  const stages = [];
  const verdict = async (file) => JSON.parse(await c.check(new Uint8Array(read(`web-seed/${file}`)), (...s) => stages.push(s)));
  const original = await verdict('original.jpg');
  assert.equal(original.outcome, 'ORIGINAL');
  assert.equal(original.recordId, seed.original);
  assert.ok('fpCommit' in original.reference.claim);
  assert.equal((await verdict('recompressed.jpg')).outcome, 'MATCHES_WITH_CHANGES');
  assert.equal((await verdict('unrelated.jpg')).outcome, 'NO_RECORD');
  assert.ok(stages.some(([s, st, done, total]) => s === 2 && st === 2 && total >= 1 && done === total));

  const [, fragment] = seed.urls.case_web.split('#');
  const opened = await c.openCase(seed.case_id, `#${fragment}`);
  assert.equal(opened.status, 'opened');
  const bundle = JSON.parse(opened.json);
  assert.equal(bundle.title, 'Fender bender');
  assert.deepEqual(bundle.items.map((i) => i.recordId), [seed.original, seed.post, seed.edited]);
  assert.ok(bundle.items.every((i) => /^\d+$/.test(i.wallMs)));
  assert.ok(/^\d+$/.test(bundle.expiresMs) && Number(bundle.expiresMs) > Date.now(), bundle.expiresMs);
  assert.equal((await c.openCase(seed.case_id, `#${fragment.replace(/h=./, 'h=A')}`)).status, 'tampered');
  assert.equal((await c.openCase(seed.case_id.replace(/.$/, '0'), `#${fragment}`)).status, 'not_found');
});
