import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fallbackJudge, isFallback, judge, ready } from '../verify.mjs';
import { Outcome, Stage, StageState } from '../wire.mjs';

const record = () => {
  const calls = [];
  return { calls, onProgress: (...a) => calls.push(a) };
};

test('fallback judge reports read then skips the rest', async () => {
  const { calls, onProgress } = record();
  const r = await fallbackJudge(new Uint8Array([1]), onProgress);
  assert.deepEqual(r, { verdict: Outcome.NO_RECORD, recordId: null, flags: 0, fallback: true });
  assert.deepEqual(calls, [
    [Stage.READ_FILE, StageState.RUNNING, 0, 0],
    [Stage.READ_FILE, StageState.DONE, 0, 0],
    [Stage.CREDENTIALS, StageState.SKIPPED, 0, 0],
    [Stage.FINGERPRINTS, StageState.SKIPPED, 0, 0],
    [Stage.WATERMARK, StageState.SKIPPED, 0, 0],
    [Stage.COMPARE, StageState.SKIPPED, 0, 0],
  ]);
});

test('no url means fallback, and readiness is memoized', async () => {
  assert.equal(await isFallback(null), true);
  assert.equal(ready(null), ready(undefined));
});

test('unloadable or judge-less modules fall back', async () => {
  assert.equal(await isFallback('file:///definitely/missing/witness_web.js'), true);
  assert.equal(await isFallback('data:text/javascript,export const nothing = 1;'), true);
});

test('a module exporting judge is used natively', async () => {
  const src = 'export default async function init() {}\nexport async function judge(bytes, p) { p(4, 2, 0, 0); return { verdict: 0, recordId: "WERC8GMRZGE196QV", flags: 65 }; }';
  const url = 'data:text/javascript,' + encodeURIComponent(src);
  assert.equal(await isFallback(url), false);
  const { calls, onProgress } = record();
  const r = await judge(url, new Uint8Array([1]), onProgress);
  assert.deepEqual(r, { verdict: 0, recordId: 'WERC8GMRZGE196QV', flags: 65, fallback: false });
  assert.deepEqual(calls, [[4, 2, 0, 0]]);
});
