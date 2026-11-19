import { Outcome, Stage, StageState } from './wire.mjs';

const STAGES = [Stage.READ_FILE, Stage.CREDENTIALS, Stage.FINGERPRINTS, Stage.WATERMARK, Stage.COMPARE];

export function fallbackJudge(bytes, onProgress) {
  onProgress(Stage.READ_FILE, StageState.RUNNING, 0, 0);
  onProgress(Stage.READ_FILE, StageState.DONE, 0, 0);
  for (const s of STAGES.slice(1)) onProgress(s, StageState.SKIPPED, 0, 0);
  return Promise.resolve({ verdict: Outcome.NO_RECORD, recordId: null, flags: 0, fallback: true });
}

const FALLBACK = Object.freeze({ fallback: true, judge: fallbackJudge });

async function load(url) {
  if (!url) return FALLBACK;
  try {
    const m = await import(/* webpackIgnore: true */ url);
    if (typeof m.default === 'function') await m.default();
    if (typeof m.judge !== 'function') return FALLBACK;
    return { fallback: false, judge: m.judge };
  } catch {
    return FALLBACK;
  }
}

const engines = new Map();

export function ready(url) {
  const key = url ?? '';
  let e = engines.get(key);
  if (!e) {
    e = load(url);
    engines.set(key, e);
  }
  return e;
}

export async function isFallback(url) {
  return (await ready(url)).fallback;
}

export async function judge(url, bytes, onProgress) {
  const engine = await ready(url);
  const r = await engine.judge(bytes, onProgress);
  return { verdict: r.verdict, recordId: r.recordId ?? null, flags: r.flags | 0, fallback: engine.fallback };
}
