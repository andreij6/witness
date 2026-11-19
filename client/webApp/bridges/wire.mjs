export const SHARD_OK = 0;
export const SHARD_FAILED = 1;
export const SHARD_HEADER = 7;

export const Stage = Object.freeze({ READ_FILE: 0, CREDENTIALS: 1, FINGERPRINTS: 2, WATERMARK: 3, COMPARE: 4 });
export const StageState = Object.freeze({ WAITING: 0, RUNNING: 1, DONE: 2, SKIPPED: 3 });
export const Outcome = Object.freeze({ ORIGINAL: 0, MATCHES_WITH_CHANGES: 1, ALTERED: 2, NO_RECORD: 3, SEALED_UNANCHORED: 4 });

const MASK32 = 0xffffffffn;

export function putU64(out, at, value) {
  const v = BigInt(value);
  out[at] = Number(BigInt.asIntN(32, (v >> 32n) & MASK32));
  out[at + 1] = Number(BigInt.asIntN(32, v & MASK32));
  return at + 2;
}

export function getU64(ints, at) {
  return (BigInt(ints[at] >>> 0) << 32n) | BigInt(ints[at + 1] >>> 0);
}

export function packedSize(results) {
  let size = 1;
  for (const r of results) {
    size += SHARD_HEADER;
    if (r) size += r.pdq.length * 3 + (r.watermark.length + r.exact.length) * 2;
  }
  return size;
}

export function packProbes(results) {
  const out = new Int32Array(packedSize(results));
  out[0] = results.length;
  let o = 1;
  for (const r of results) {
    if (!r) {
      out[o] = SHARD_FAILED;
      o += SHARD_HEADER;
      continue;
    }
    out[o] = SHARD_OK;
    out[o + 1] = r.truncated ? 1 : 0;
    o = putU64(out, o + 2, r.indexed_upto);
    out[o++] = r.pdq.length;
    out[o++] = r.watermark.length;
    out[o++] = r.exact.length;
    for (const h of r.pdq) {
      out[o++] = h.local_ix | 0;
      out[o++] = h.shape;
      out[o++] = h.distance;
    }
    for (const ix of r.watermark) o = putU64(out, o, ix);
    for (const ix of r.exact) o = putU64(out, o, ix);
  }
  return out;
}

export function slices(flat, stride) {
  if (stride <= 0 || flat.length % stride !== 0) throw new RangeError(`length ${flat.length} is not a multiple of ${stride}`);
  const out = new Array(flat.length / stride);
  for (let i = 0; i < out.length; i++) out[i] = flat.subarray(i * stride, (i + 1) * stride);
  return out;
}

export function u64Pairs(ints) {
  if (ints.length % 2 !== 0) throw new RangeError('odd u64 pair array');
  const out = new Array(ints.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = getU64(ints, i * 2);
  return out;
}

export function splitIds(joined) {
  return joined.length === 0 ? [] : joined.split(',');
}

export function nsToMs(ns) {
  return Number(BigInt(ns) / 1_000_000n);
}

export function unwrapOpt(opt) {
  return opt.length === 0 ? null : opt[0];
}
