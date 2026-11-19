export const Stage = Object.freeze({ READ_FILE: 0, CREDENTIALS: 1, FINGERPRINTS: 2, WATERMARK: 3, COMPARE: 4 });
export const State = Object.freeze({ WAITING: 0, RUNNING: 1, DONE: 2, SKIPPED: 3, FAILED: 4 });

export const MAX_ROUNDS = 12;
export const MAX_PDQ_HITS = 32;
export const MAX_DERIVATIVES = 32;
export const MAX_AGE_NS = 300_000_000_000n;
export const MAX_WIRE_CHUNK = 1 << 20;
export const MAX_CHUNKS = 256;
export const NS_PER_MS = 1_000_000n;
export const HASH_BYTES = 32;

const HEX = Array.from({ length: 256 }, (_, i) => i.toString(16).padStart(2, '0'));

export function toHex(bytes) {
  let s = '';
  for (let i = 0; i < bytes.length; i++) s += HEX[bytes[i]];
  return s;
}

export function fromHex(text) {
  if (text.length % 2 !== 0 || /[^0-9a-f]/i.test(text)) throw new Error('bad hex');
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(text.substr(i * 2, 2), 16);
  return out;
}

export function toBase64(bytes) {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

export function fromBase64Url(text) {
  if (!/^[A-Za-z0-9_-]*={0,2}$/.test(text)) return null;
  try {
    const bin = atob(text.replace(/=+$/, '').replace(/-/g, '+').replace(/_/g, '/'));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

export function fragmentParam(fragment, name) {
  const prefix = `${name}=`;
  return fragment.replace(/^#/, '').split('&').find((p) => p.startsWith(prefix))?.slice(prefix.length) ?? null;
}

export const unwrapOpt = (opt) => (opt.length === 0 ? null : opt[0]);
export const variant = (v) => Object.keys(v)[0];

export const recordWire = (r) => ({
  recordIx: String(r.record_ix),
  recordId: r.record_id,
  claim: toHex(r.claim),
  c2paClaim: toHex(r.c2pa_claim),
  cose: toHex(r.cose),
  anchoredNs: String(r.anchored_ns),
  mmrProof: r.mmr_proof.map(toHex),
  leafCount: String(r.leaf_count),
  certificate: toHex(r.certificate),
  mediaShard: r.media_shard ?? 0,
});

export const keyWire = (k) => ({
  keyId: toHex(k.key_id),
  pubkey: toHex(k.pubkey),
  security: variant(k.security),
  provisioning: variant(k.provisioning),
  freshness: variant(k.freshness),
  patch: k.patch,
  enrolledNs: String(k.enrolled_ns),
  activeUntilNs: String(k.active_until_ns),
  history: k.history.map((c) => ({ status: variant(c.status), sinceNs: String(c.since_ns) })),
  chainSerials: [...k.chain_serials],
});

export const certifiedKeyWire = (c) => ({ key: keyWire(c.key), smtProof: toHex(c.smt_proof), certificate: toHex(c.certificate) });

const shardWire = (s) => ({ shard: s.shard, canister: toHex(s.canister.toUint8Array()), open: s.open });

export const shardsWire = (map, bundle) => ({ index: map.index.map(shardWire), media: map.media.map(shardWire), bundle: toHex(bundle) });

export const fingerprintsWire = (f) => ({
  grid: toHex(f.grid),
  shapes: f.shapes.map(([tag, hash]) => [tag, toHex(hash)]),
  quality: f.quality,
  portrait: f.portrait,
});

const sortedUnique = (list) => [...new Set(list)].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));

export function mergeProbes(results) {
  const best = new Map();
  let truncated = false;
  const watermark = [];
  const exact = [];
  for (const r of results) {
    truncated ||= r.truncated_candidates;
    for (const h of r.pdq) {
      const prev = best.get(h.record_ix);
      if (prev === undefined || h.distance < prev) best.set(h.record_ix, h.distance);
    }
    watermark.push(...r.watermark);
    exact.push(...r.exact);
  }
  const pdq = [...best]
    .map(([record_ix, distance]) => ({ record_ix, distance }))
    .sort((a, b) => a.distance - b.distance || (a.record_ix < b.record_ix ? -1 : a.record_ix > b.record_ix ? 1 : 0));
  truncated ||= pdq.length > MAX_PDQ_HITS;
  return { pdq: pdq.slice(0, MAX_PDQ_HITS), watermark: sortedUnique(watermark), exact: sortedUnique(exact), truncated };
}

export const emptyFetched = () => ({
  keys: {},
  records: {},
  absentIxs: [],
  exact: null,
  watermark: {},
  pdq: null,
  derivatives: {},
  fingerprints: {},
  references: {},
  indexPending: false,
});

const CREDENTIAL_KINDS = new Set(['witness', 'other', 'invalid', 'unverified']);
const credentialStage = (kind) => (CREDENTIAL_KINDS.has(kind) ? State.DONE : State.SKIPPED);

export const preferShard = (shards, hint) =>
  hint === undefined ? shards : [...shards.filter((s) => s.shard === hint), ...shards.filter((s) => s.shard !== hint)];

export function createClient(core, io, trust, anchorHex = '', local = false) {
  let shards;
  let anchor = anchorHex ? Promise.resolve({ anchor: anchorHex }) : local ? null : Promise.resolve({ anchor: '' });
  const mediaShards = new Map();

  const certify = (fn, wire) => fn(JSON.stringify(wire), trust.rootKey, trust.canister, trust.now(), trust.maxAgeNs);

  const shardMap = () =>
    (shards ??= io.shards().then(
      ({ map, bundle }) => {
        certify(core.verifyShards, shardsWire(map, bundle));
        return map;
      },
      (e) => {
        shards = undefined;
        throw e;
      },
    ));

  const trustAnchor = () =>
    (anchor ??= io.caRoot().then(
      (root) => {
        if (!root) throw new Error('registry has no CA root');
        return { anchor: toHex(root), devAnchor: true };
      },
      (e) => {
        anchor = null;
        throw e;
      },
    ));

  const verifiedRecord = (r) => {
    if (!r) return null;
    const wire = recordWire(r);
    certify(core.verifyCertifiedRecord, wire);
    mediaShards.set(wire.recordId, wire.mediaShard);
    return wire;
  };

  const record = async (id) => verifiedRecord(await io.get(id));

  const recordAt = async (ix) => {
    const wire = verifiedRecord(await io.getIx(BigInt(ix)));
    if (wire && wire.recordIx !== String(ix)) throw new Error('record index mismatch');
    return wire;
  };

  const key = async (hex) => {
    const k = await io.getKey(fromHex(hex));
    if (!k) return null;
    const wire = certifiedKeyWire(k);
    certify(core.verifyCertifiedKey, wire);
    return wire.key;
  };

  const media = async (path, hint) => io.media(preferShard((await shardMap()).media, hint), path);

  const preview = (id) => media(`/p/${id}`, mediaShards.get(id));

  async function lookup(need, f, stage) {
    const index = (await shardMap()).index;
    let answered = 0;
    stage(Stage.FINGERPRINTS, State.RUNNING, 0, index.length);
    const pdq = (need.pdq ?? []).map(fromHex);
    const watermarks = need.watermark.map((c) => BigInt(c));
    const exact = need.exact ? [need.exact.asset, need.exact.pixel].filter(Boolean).map(fromHex) : [];
    const results = await Promise.all(
      index.map((s) =>
        io.probe(s, pdq, watermarks, exact).then((r) => {
          stage(Stage.FINGERPRINTS, State.RUNNING, ++answered, index.length);
          return r;
        }),
      ),
    );
    const m = mergeProbes(results);
    if (need.pdq) f.pdq = { hits: m.pdq.map((h) => ({ ix: String(h.record_ix), distance: h.distance })), truncated: m.truncated };
    if (need.exact) f.exact = m.exact.map(String);
    for (const c of need.watermark) f.watermark[c] = m.watermark.map(String);
    stage(Stage.FINGERPRINTS, State.DONE, answered, index.length);
  }

  async function derivatives(ix) {
    const index = (await shardMap()).index;
    const all = (await Promise.all(index.map((s) => io.derivatives(s, BigInt(ix))))).flatMap((list) => [...list]);
    return sortedUnique(all).slice(0, MAX_DERIVATIVES).map(String);
  }

  async function fingerprints(ix) {
    const index = (await shardMap()).index;
    const found = (await Promise.all(index.map((s) => io.fingerprints(s, BigInt(ix))))).find(Boolean);
    return found ? fingerprintsWire(found) : null;
  }

  async function fulfil(need, f, stage) {
    const tasks = [];
    const later = need.recordIxs.length + need.derivatives.length + need.fingerprints.length + need.references.length > 0;
    if (later) stage(Stage.COMPARE, State.RUNNING, 0, 0);
    for (const k of need.keys) if (!(k in f.keys)) tasks.push(key(k).then((v) => (f.keys[k] = v)));
    for (const id of need.records) if (!(id in f.records)) tasks.push(record(id).then((v) => (f.records[id] = v)));
    if (need.exact || need.pdq || need.watermark.length > 0) tasks.push(lookup(need, f, stage));
    for (const ix of need.recordIxs) tasks.push(recordAt(ix).then((w) => (w ? (f.records[w.recordId] = w) : f.absentIxs.push(ix))));
    for (const ix of need.derivatives) tasks.push(derivatives(ix).then((l) => (f.derivatives[ix] = l)));
    for (const ix of need.fingerprints) tasks.push(fingerprints(ix).then((v) => (f.fingerprints[ix] = v)));
    for (const id of need.references) tasks.push(preview(id).then((b) => (f.references[id] = b ? toBase64(b) : null)));
    await Promise.all(tasks);
  }

  const fullClaim = (wire) => JSON.parse(core.decodeClaim(fromHex(wire.claim)));

  async function check(bytes, onStage) {
    const stage = (s, st, done = 0, total = 0) => onStage(s, st, done, total);
    stage(Stage.READ_FILE, State.RUNNING);
    const options = JSON.stringify(await trustAnchor());
    let step = JSON.parse(core.queryPlan(bytes, options));
    stage(Stage.READ_FILE, State.DONE);
    stage(Stage.CREDENTIALS, credentialStage(step.credentials?.kind));
    stage(Stage.WATERMARK, State.SKIPPED);
    const f = emptyFetched();
    for (let round = 0; !step.done; round++) {
      if (round >= MAX_ROUNDS) throw new Error('check did not converge');
      await fulfil(step.need, f, stage);
      step = JSON.parse(core.judge(bytes, options, JSON.stringify(f)));
    }
    const v = step.verdict;
    const wire = v.reference && f.records[v.reference.recordId];
    if (wire) v.reference.claim = fullClaim(wire);
    return JSON.stringify(v);
  }

  async function proof(id) {
    const wire = await record(id);
    if (!wire) return null;
    return JSON.stringify({ recordIx: wire.recordIx, recordId: wire.recordId, anchoredNs: wire.anchoredNs, claim: fullClaim(wire) });
  }

  async function certifiedKey(hex) {
    const k = await key(hex);
    return k ? JSON.stringify(k) : null;
  }

  async function caseInfo(caseId) {
    try {
      return await io.caseInfo((await shardMap()).media, caseId);
    } catch {
      return null;
    }
  }

  async function chunks(caseId, limit) {
    const out = [];
    for (let ix = 0; ix < limit; ix++) {
      const c = await media(`/c/${caseId}.${ix}`);
      if (!c) break;
      out.push(c);
      if (c.length < MAX_WIRE_CHUNK) break;
    }
    return out;
  }

  async function openCase(caseText, fragment) {
    const caseId = caseText.replace(/-/g, '').toUpperCase();
    const hash = fromBase64Url(fragmentParam(fragment, 'h') ?? '');
    if (!fragmentParam(fragment, 'k') || hash?.length !== HASH_BYTES) return { status: 'tampered' };
    const info = await caseInfo(caseId);
    const parts = await chunks(caseId, info ? Math.min(Number(info.chunks), MAX_CHUNKS) : MAX_CHUNKS);
    if (parts.length === 0) return { status: 'not_found' };
    let bundle;
    let parsed;
    let items;
    try {
      const opener = new core.CaseOpener(fragment, caseId, parts.length, hash);
      parts.forEach((c, ix) => opener.add(ix, c));
      bundle = opener.finish();
      parsed = JSON.parse(core.parseCaseBundle(bundle));
      items = parsed.items.map((i) => ({
        recordId: i.recordId,
        offset: i.jpeg.offset,
        len: i.jpeg.len,
        wallMs: i.submission ? JSON.parse(core.decodeClaim(fromHex(i.submission.claim))).wallMs ?? null : null,
      }));
    } catch {
      return { status: 'tampered' };
    }
    const expiresMs = info ? String(BigInt(info.expires_ns) / NS_PER_MS) : null;
    return { status: 'opened', json: JSON.stringify({ title: parsed.title, summary: parsed.summary, items, expiresMs }), bundle };
  }

  return { check, proof, key: certifiedKey, preview, openCase, record, shardMap };
}
