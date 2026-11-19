import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Principal } from '@icp-sdk/core/principal';
import {
  MAX_PDQ_HITS,
  MAX_CHUNKS,
  MAX_WIRE_CHUNK,
  Stage,
  State,
  createClient,
  emptyFetched,
  fragmentParam,
  fromBase64Url,
  fromHex,
  keyWire,
  mergeProbes,
  preferShard,
  recordWire,
  shardsWire,
  toBase64,
  toHex,
} from '../engine.mjs';

const P = (t) => Principal.fromText(t);
const SHARDS = { index: [{ shard: 0, canister: P('aaaaa-aa'), open: true }, { shard: 1, canister: P('2vxsx-fae'), open: false }], media: [{ shard: 0, canister: P('aaaaa-aa'), open: true }] };
const RECORD = { record_ix: 7n, record_id: 'REC', claim: Uint8Array.of(0xab), c2pa_claim: Uint8Array.of(1), cose: Uint8Array.of(2), anchored_ns: 5_000_000n, mmr_proof: [Uint8Array.of(3)], leaf_count: 8n, certificate: Uint8Array.of(4) };
const KEY = { key_id: Uint8Array.of(9, 9), pubkey: Uint8Array.of(1), security: { Dev: null }, provisioning: { Dev: null }, freshness: { Fresh: null }, patch: 2, enrolled_ns: 1n, active_until_ns: 2n, history: [{ status: { Active: null }, since_ns: 1n }], chain_serials: ['s'] };
const NEED = { keys: [], records: [], recordIxs: [], exact: null, watermark: [], pdq: null, derivatives: [], fingerprints: [], references: [] };
const need = (n) => ({ ...NEED, ...n });
const trust = { rootKey: Uint8Array.of(1), canister: Uint8Array.of(2), now: () => 10n, maxAgeNs: 0n };

function fakeCore(script = []) {
  const calls = { verify: [], judge: [] };
  const certified = (kind) => (json, ...rest) => {
    calls.verify.push([kind, JSON.parse(json), ...rest]);
    if (JSON.parse(json).certificate === 'bad' || JSON.parse(json).bundle === 'bad0') throw new Error('certificate');
    return '{}';
  };
  const core = {
    queryPlan: () => JSON.stringify({ done: script.length === 0, need: script[0] ?? NEED, verdict: { outcome: 'NO_RECORD' }, credentials: { kind: 'witness' } }),
    judge: (_b, _o, fetched) => {
      calls.judge.push(JSON.parse(fetched));
      const n = script[calls.judge.length];
      return JSON.stringify(n ? { done: false, need: n, verdict: {} } : { done: true, need: NEED, verdict: { outcome: 'ORIGINAL', recordId: 'REC', reference: { recordId: 'REC', claim: { lean: true } } } });
    },
    decodeClaim: (bytes) => JSON.stringify({ full: toHex(bytes) }),
    verifyCertifiedRecord: certified('record'),
    verifyCertifiedKey: certified('key'),
    verifyShards: certified('shards'),
    parseCaseBundle: () => JSON.stringify({ title: 'T', summary: null, items: [{ recordId: 'REC', jpeg: { offset: 3, len: 2 }, submission: { claim: 'ab' } }] }),
    CaseOpener: class {
      constructor(fragment, caseId, count, hash) {
        calls.opener = { fragment, caseId, count, hash, added: [] };
        if (fragment.includes('k=bad')) throw new Error('key');
      }
      add(ix, chunk) {
        calls.opener.added.push([ix, chunk.length]);
      }
      finish() {
        return Uint8Array.of(1, 2, 3, 4, 5);
      }
    },
  };
  return { core, calls };
}

function fakeIo(over = {}) {
  const calls = { probe: [], media: [], shards: 0 };
  const io = {
    shards: async () => (calls.shards++, { map: SHARDS, bundle: Uint8Array.of(1) }),
    caRoot: async () => Uint8Array.of(0xca),
    caseInfo: async () => null,
    get: async (id) => (id === 'REC' ? RECORD : null),
    getIx: async (ix) => (ix === 7n ? RECORD : null),
    getKey: async () => ({ key: KEY, smt_proof: Uint8Array.of(1), certificate: Uint8Array.of(2) }),
    probe: async (s, pdq, wm, exact) => {
      calls.probe.push([s.shard, pdq.length, wm, exact.length]);
      return { pdq: [{ record_ix: 7n, distance: s.shard + 3 }], watermark: BigUint64Array.of(7n), exact: BigUint64Array.of(7n), indexed_upto: 9n, truncated_candidates: false };
    },
    derivatives: async (s) => (s.shard === 0 ? BigUint64Array.of(9n, 3n) : [3n]),
    fingerprints: async (s) => (s.shard === 1 ? { grid: Uint8Array.of(1), shapes: [[2, Uint8Array.of(3)]], quality: 80, portrait: true } : null),
    media: async (shards, path) => (calls.media.push(path), path.startsWith('/p/') ? Uint8Array.of(0xff, 0xd8) : null),
    ...over,
  };
  return { io, calls };
}

test('hex and base64 helpers', () => {
  assert.equal(toHex(Uint8Array.of(0, 15, 255)), '000fff');
  assert.deepEqual(fromHex('000fFF'), Uint8Array.of(0, 15, 255));
  assert.throws(() => fromHex('abc'));
  assert.throws(() => fromHex('zz'));
  assert.equal(toBase64(Uint8Array.of(0xff, 0xd8)), '/9g=');
  const big = new Uint8Array(100_000).fill(7);
  assert.equal(atob(toBase64(big)).length, big.length);
  assert.deepEqual(fromBase64Url('_-8'), Uint8Array.of(0xff, 0xef));
  assert.deepEqual(fromBase64Url('_-8='), Uint8Array.of(0xff, 0xef));
  assert.equal(fromBase64Url('a b'), null);
  assert.equal(fromBase64Url('a'), null);
});

test('fragment params', () => {
  assert.equal(fragmentParam('#k=abc&h=def', 'k'), 'abc');
  assert.equal(fragmentParam('k=abc&h=def', 'h'), 'def');
  assert.equal(fragmentParam('#kk=1', 'k'), null);
  assert.equal(fragmentParam('', 'k'), null);
});

test('wire mapping matches the witness-web JSON conventions', () => {
  const r = recordWire(RECORD);
  assert.deepEqual(r, { recordIx: '7', recordId: 'REC', claim: 'ab', c2paClaim: '01', cose: '02', anchoredNs: '5000000', mmrProof: ['03'], leafCount: '8', certificate: '04', mediaShard: 0 });
  assert.equal(recordWire({ ...RECORD, media_shard: 3 }).mediaShard, 3);
  const k = keyWire(KEY);
  assert.equal(k.security, 'Dev');
  assert.equal(k.history[0].status, 'Active');
  assert.equal(k.history[0].sinceNs, '1');
  assert.equal(k.keyId, '0909');
  const s = shardsWire(SHARDS, Uint8Array.of(5));
  assert.deepEqual(s.index[0], { shard: 0, canister: '', open: true });
  assert.equal(s.index[1].canister, '04');
  assert.equal(s.bundle, '05');
});

test('mergeProbes keeps the best distance per record, sorted, capped', () => {
  const r = (pdq, extra = {}) => ({ pdq, watermark: [], exact: [], truncated_candidates: false, indexed_upto: 0n, ...extra });
  const m = mergeProbes([r([{ record_ix: 5n, distance: 9 }, { record_ix: 1n, distance: 4 }], { exact: [3n, 1n] }), r([{ record_ix: 5n, distance: 2 }], { exact: [1n], watermark: [8n] })]);
  assert.deepEqual(m.pdq, [{ record_ix: 5n, distance: 2 }, { record_ix: 1n, distance: 4 }]);
  assert.deepEqual(m.exact, [1n, 3n]);
  assert.deepEqual(m.watermark, [8n]);
  assert.equal(m.truncated, false);
  const many = r(Array.from({ length: MAX_PDQ_HITS + 5 }, (_, i) => ({ record_ix: BigInt(i), distance: i })));
  const capped = mergeProbes([many]);
  assert.equal(capped.pdq.length, MAX_PDQ_HITS);
  assert.equal(capped.truncated, true);
  assert.equal(mergeProbes([r([], { truncated_candidates: true })]).truncated, true);
  assert.deepEqual(mergeProbes([]), { pdq: [], watermark: [], exact: [], truncated: false });
});

test('check drives the plan/judge loop and fills every need', async () => {
  const script = [
    need({ keys: ['0909'], records: ['REC'], exact: { asset: '11'.repeat(32), pixel: null }, pdq: ['22'.repeat(32)], watermark: ['42'] }),
    need({ recordIxs: ['7', '8'], derivatives: ['7'] }),
    need({ fingerprints: ['7'], references: ['REC'] }),
  ];
  const { core, calls } = fakeCore(script);
  const { io, calls: ioCalls } = fakeIo();
  const stages = [];
  const c = createClient(core, io, trust);
  const out = JSON.parse(await c.check(Uint8Array.of(1), (...s) => stages.push(s)));
  assert.equal(out.outcome, 'ORIGINAL');
  assert.deepEqual(out.reference.claim, { full: 'ab' });
  const f = calls.judge.at(-1);
  assert.equal(f.keys['0909'].keyId, '0909');
  assert.equal(f.records.REC.recordIx, '7');
  assert.deepEqual(f.exact, ['7']);
  assert.deepEqual(f.watermark, { 42: ['7'] });
  assert.deepEqual(f.pdq, { hits: [{ ix: '7', distance: 3 }], truncated: false });
  assert.deepEqual(f.absentIxs, ['8']);
  assert.deepEqual(f.derivatives, { 7: ['3', '9'] });
  assert.equal(f.fingerprints['7'].quality, 80);
  assert.equal(f.references.REC, '/9g=');
  assert.equal(calls.judge.length, 3);
  assert.deepEqual(ioCalls.probe.map((p) => p.slice(0, 2)), [[0, 1], [1, 1]]);
  assert.deepEqual(ioCalls.probe[0][2], [42n]);
  assert.equal(ioCalls.probe[0][3], 1);
  assert.equal(ioCalls.shards, 1);
  assert.deepEqual(stages.slice(0, 4), [[Stage.READ_FILE, State.RUNNING, 0, 0], [Stage.READ_FILE, State.DONE, 0, 0], [Stage.CREDENTIALS, State.DONE, 0, 0], [Stage.WATERMARK, State.SKIPPED, 0, 0]]);
  assert.ok(stages.some((s) => s[0] === Stage.FINGERPRINTS && s[1] === State.RUNNING && s[2] === 2 && s[3] === 2));
  assert.ok(stages.some((s) => s[0] === Stage.FINGERPRINTS && s[1] === State.DONE));
  assert.ok(stages.some((s) => s[0] === Stage.COMPARE && s[1] === State.RUNNING));
  const kinds = calls.verify.map((v) => v[0]);
  assert.deepEqual([...new Set(kinds)].sort(), ['key', 'record', 'shards']);
  assert.deepEqual(calls.verify[0].slice(2), [trust.rootKey, trust.canister, 10n, 0n]);
});

test('check pins the configured anchor, queries the replica only in local DEV mode, and otherwise judges without one', async () => {
  const opts = [];
  const { core } = fakeCore();
  const spy = { ...core, queryPlan: (b, o) => (opts.push(JSON.parse(o)), core.queryPlan(b, o)) };
  let asked = 0;
  const counting = fakeIo({ caRoot: async () => (asked++, Uint8Array.of(0xca)) }).io;
  await createClient(spy, counting, trust, 'beef').check(Uint8Array.of(1), () => {});
  await createClient(spy, counting, trust, 'beef', true).check(Uint8Array.of(1), () => {});
  await createClient(spy, counting, trust).check(Uint8Array.of(1), () => {});
  assert.equal(asked, 0);
  const dev = createClient(spy, counting, trust, '', true);
  await dev.check(Uint8Array.of(1), () => {});
  await dev.check(Uint8Array.of(1), () => {});
  assert.equal(asked, 1);
  assert.deepEqual(opts, [{ anchor: 'beef' }, { anchor: 'beef' }, { anchor: '' }, { anchor: 'ca', devAnchor: true }, { anchor: 'ca', devAnchor: true }]);
  const none = createClient(core, fakeIo({ caRoot: async () => null }).io, trust, '', true);
  await assert.rejects(none.check(Uint8Array.of(1), () => {}), /CA root/);
  let n = 0;
  const flaky = createClient(core, fakeIo({ caRoot: async () => { if (n++ === 0) throw new Error('down'); return Uint8Array.of(1); } }).io, trust, '', true);
  await assert.rejects(flaky.check(Uint8Array.of(1), () => {}), /down/);
  await flaky.check(Uint8Array.of(1), () => {});
});

test('credentials judged without an anchor still count as a finished stage', async () => {
  for (const [kind, want] of [['unverified', State.DONE], ['witness', State.DONE], ['other', State.DONE], ['invalid', State.DONE], ['none', State.SKIPPED], [undefined, State.SKIPPED]]) {
    const { core } = fakeCore();
    const plan = { ...core, queryPlan: () => JSON.stringify({ done: true, need: NEED, verdict: { outcome: 'NO_RECORD' }, credentials: kind ? { kind } : undefined, anchor: 'missing' }) };
    const seen = [];
    await createClient(plan, fakeIo().io, trust).check(Uint8Array.of(1), (s, st) => seen.push([s, st]));
    assert.deepEqual(seen.find(([s]) => s === Stage.CREDENTIALS), [Stage.CREDENTIALS, want], String(kind));
  }
});

test('previews are fetched from the record media shard first', async () => {
  const media = [{ shard: 0, canister: P('aaaaa-aa'), open: false }, { shard: 1, canister: P('2vxsx-fae'), open: true }];
  assert.deepEqual(preferShard(media, 1).map((s) => s.shard), [1, 0]);
  assert.deepEqual(preferShard(media, 0).map((s) => s.shard), [0, 1]);
  assert.deepEqual(preferShard(media, 9).map((s) => s.shard), [0, 1]);
  assert.equal(preferShard(media, undefined), media);
  const { core } = fakeCore();
  const order = [];
  const { io } = fakeIo({
    shards: async () => ({ map: { ...SHARDS, media }, bundle: Uint8Array.of(1) }),
    get: async () => ({ ...RECORD, media_shard: 1 }),
    media: async (shards, path) => (order.push([path, shards.map((s) => s.shard)]), Uint8Array.of(0xff)),
  });
  const c = createClient(core, io, trust);
  await c.preview('REC');
  await c.proof('REC');
  await c.preview('REC');
  assert.deepEqual(order, [['/p/REC', [0, 1]], ['/p/REC', [1, 0]]]);
});

test('check fails on a tampered certificate instead of trusting the reply', async () => {
  const { core } = fakeCore([need({ records: ['REC'] })]);
  const { io } = fakeIo({ get: async () => ({ ...RECORD, certificate: 'bad' }) });
  const bad = { ...io, get: async () => RECORD };
  const strict = { ...core, verifyCertifiedRecord: () => { throw new Error('certificate'); } };
  await assert.rejects(createClient(strict, bad, trust).check(Uint8Array.of(1), () => {}), /certificate/);
});

test('check rejects a get_ix reply for a different index', async () => {
  const { core } = fakeCore([need({ recordIxs: ['8'] })]);
  const { io } = fakeIo({ getIx: async () => RECORD });
  await assert.rejects(createClient(core, io, trust).check(Uint8Array.of(1), () => {}), /index mismatch/);
});

test('check stops after too many rounds', async () => {
  const { core } = fakeCore(Array.from({ length: 20 }, () => need({ keys: ['0909'] })));
  await assert.rejects(createClient(core, fakeIo().io, trust).check(Uint8Array.of(1), () => {}), /converge/);
});

test('shard map is retried after a failure and verified', async () => {
  let n = 0;
  const { core } = fakeCore();
  const { io } = fakeIo({ shards: async () => { if (n++ === 0) throw new Error('down'); return { map: SHARDS, bundle: Uint8Array.of(1) }; } });
  const c = createClient(core, io, trust);
  await assert.rejects(c.preview('REC'), /down/);
  assert.deepEqual(await c.preview('REC'), Uint8Array.of(0xff, 0xd8));
  const tampered = createClient(core, fakeIo({ shards: async () => ({ map: SHARDS, bundle: Uint8Array.of(0xba, 0xd0) }) }).io, trust);
  await assert.rejects(tampered.preview('REC'));
});

test('proof returns the verified record with its full claim', async () => {
  const { core } = fakeCore();
  const c = createClient(core, fakeIo().io, trust);
  assert.deepEqual(JSON.parse(await c.proof('REC')), { recordIx: '7', recordId: 'REC', anchoredNs: '5000000', claim: { full: 'ab' } });
  assert.equal(await c.proof('NOPE'), null);
  assert.equal(JSON.parse(await c.key('0909')).keyId, '0909');
  assert.equal(await createClient(core, fakeIo({ getKey: async () => null }).io, trust).key('0909'), null);
});

const HASH = 'A'.repeat(43);

test('openCase discovers chunks, decrypts and lists items', async () => {
  const { core, calls } = fakeCore();
  const sizes = [MAX_WIRE_CHUNK, 10];
  const { io, calls: ioCalls } = fakeIo({ media: async (_s, path) => (ioCalls.media.push(path), path.startsWith('/c/') ? new Uint8Array(sizes[Number(path.split('.').pop())] ?? 0) : null) });
  const out = await createClient(core, io, trust).openCase('abcd-ef', `#k=key&h=${HASH}`);
  assert.equal(out.status, 'opened');
  assert.deepEqual(ioCalls.media, ['/c/ABCDEF.0', '/c/ABCDEF.1']);
  assert.equal(calls.opener.count, 2);
  assert.equal(calls.opener.caseId, 'ABCDEF');
  assert.equal(calls.opener.hash.length, 32);
  assert.deepEqual(calls.opener.added, [[0, MAX_WIRE_CHUNK], [1, 10]]);
  assert.deepEqual(JSON.parse(out.json), { title: 'T', summary: null, items: [{ recordId: 'REC', offset: 3, len: 2, wallMs: null }], expiresMs: null });
  assert.equal(out.bundle.length, 5);
});

test('openCase stops after a full final chunk and caps the count', async () => {
  const { core, calls } = fakeCore();
  const full = fakeIo({ media: async (_s, path) => (path.endsWith('.0') ? new Uint8Array(MAX_WIRE_CHUNK) : null) }).io;
  await createClient(core, full, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.equal(calls.opener.count, 1);
  const endless = fakeIo({ media: async () => new Uint8Array(MAX_WIRE_CHUNK) }).io;
  await createClient(core, endless, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.equal(calls.opener.count, MAX_CHUNKS);
  assert.equal(MAX_CHUNKS, 256);
});

test('openCase uses media case_info for the chunk count and the canister expiry', async () => {
  const { core, calls } = fakeCore();
  const asked = [];
  const fetched = [];
  const { io } = fakeIo({
    caseInfo: async (shards, id) => (asked.push([shards.map((s) => s.shard), id]), { expires_ns: 1_700_000_000_123_456_789n, bytes: 10n, chunks: 2 }),
    media: async (_s, path) => (fetched.push(path), new Uint8Array(MAX_WIRE_CHUNK)),
  });
  const out = await createClient(core, io, trust).openCase('ab-cd', `#k=key&h=${HASH}`);
  assert.deepEqual(asked, [[[0], 'ABCD']]);
  assert.deepEqual(fetched, ['/c/ABCD.0', '/c/ABCD.1']);
  assert.equal(calls.opener.count, 2);
  assert.equal(JSON.parse(out.json).expiresMs, '1700000000123');
  const huge = fakeIo({ caseInfo: async () => ({ expires_ns: 0n, bytes: 0n, chunks: 100_000 }), media: async () => new Uint8Array(MAX_WIRE_CHUNK) }).io;
  await createClient(core, huge, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.equal(calls.opener.count, MAX_CHUNKS);
});

test('openCase still opens when case_info is unavailable, without inventing an expiry', async () => {
  const { core } = fakeCore();
  const down = fakeIo({ caseInfo: async () => { throw new Error('down'); }, media: async () => Uint8Array.of(1) }).io;
  const out = await createClient(core, down, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.equal(out.status, 'opened');
  assert.equal(JSON.parse(out.json).expiresMs, null);
});

test('openCase reports missing, damaged and incomplete links', async () => {
  const { core } = fakeCore();
  const none = createClient(core, fakeIo({ media: async () => null }).io, trust);
  assert.deepEqual(await none.openCase('C', `#k=key&h=${HASH}`), { status: 'not_found' });
  const some = createClient(core, fakeIo({ media: async () => Uint8Array.of(1) }).io, trust);
  assert.deepEqual(await some.openCase('C', '#k=key'), { status: 'tampered' });
  assert.deepEqual(await some.openCase('C', '#h=' + HASH), { status: 'tampered' });
  assert.deepEqual(await some.openCase('C', '#k=key&h=AAAA'), { status: 'tampered' });
  assert.deepEqual(await some.openCase('C', `#k=bad&h=${HASH}`), { status: 'tampered' });
  const failing = { ...core, parseCaseBundle: () => { throw new Error('bundle'); } };
  assert.deepEqual(await createClient(failing, fakeIo({ media: async () => Uint8Array.of(1) }).io, trust).openCase('C', `#k=key&h=${HASH}`), { status: 'tampered' });
});

test('openCase reads capture times from item submissions', async () => {
  const { core } = fakeCore();
  const timed = { ...core, decodeClaim: () => JSON.stringify({ wallMs: '1700000000000' }) };
  const out = await createClient(timed, fakeIo({ media: async () => Uint8Array.of(1) }).io, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.equal(JSON.parse(out.json).items[0].wallMs, '1700000000000');
});

test('openCase treats an undecodable item claim as tampered instead of throwing', async () => {
  const { core } = fakeCore();
  const broken = { ...core, decodeClaim: () => { throw new Error('claim'); } };
  const out = await createClient(broken, fakeIo({ media: async () => Uint8Array.of(1) }).io, trust).openCase('C', `#k=key&h=${HASH}`);
  assert.deepEqual(out, { status: 'tampered' });
});

test('emptyFetched has every field witness-web expects', () => {
  assert.deepEqual(Object.keys(emptyFetched()).sort(), ['absentIxs', 'derivatives', 'exact', 'fingerprints', 'indexPending', 'keys', 'pdq', 'records', 'references', 'watermark']);
});
