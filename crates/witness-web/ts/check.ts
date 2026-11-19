import init, {
  CaseOpener,
  judge,
  parseCaseBundle,
  queryPlan,
  verifyCertifiedKey,
  verifyCertifiedRecord,
  verifyShards,
  version,
} from "../pkg/witness_web.js";

type Outcome = "ORIGINAL" | "MATCHES_WITH_CHANGES" | "ALTERED" | "NO_RECORD" | "SEALED_UNANCHORED";

interface Need {
  keys: string[];
  records: string[];
  recordIxs: string[];
  exact: { asset: string; pixel: string | null } | null;
  watermark: string[];
  pdq: string[] | null;
  derivatives: string[];
  fingerprints: string[];
  references: string[];
}

interface VerdictJson {
  version: number;
  outcome: Outcome;
  outcomeCode: number;
  flags: number;
  flagNames: string[];
  reasons: string[];
  recordId: string | null;
}

interface Step {
  done: boolean;
  need: Need;
  verdict: VerdictJson;
}

type Fetch = (need: Need, fetched: Record<string, unknown>) => Promise<void>;

export async function check(bytes: Uint8Array, options: string, fetch: Fetch): Promise<VerdictJson> {
  await init();
  let step = JSON.parse(queryPlan(bytes, options)) as Step;
  const fetched: Record<string, unknown> = {};
  for (let round = 0; round < 12 && !step.done; round++) {
    await fetch(step.need, fetched);
    step = JSON.parse(judge(bytes, options, JSON.stringify(fetched))) as Step;
  }
  return step.verdict;
}

export function certified(record: string, key: string, shards: string, rootKey: Uint8Array, registry: Uint8Array, nowNs: bigint): string[] {
  const maxAge: bigint = 300_000_000_000n;
  return [
    verifyCertifiedRecord(record, rootKey, registry, nowNs, maxAge),
    verifyCertifiedKey(key, rootKey, registry, nowNs, maxAge),
    verifyShards(shards, rootKey, registry, nowNs, maxAge),
  ];
}

export function openCase(fragment: string, caseId: string, chunks: Uint8Array[], bundleHash: Uint8Array): string {
  const opener = new CaseOpener(fragment, caseId, chunks.length, bundleHash);
  chunks.forEach((c, ix) => opener.add(ix, c));
  const missing: Uint32Array = opener.missing();
  if (missing.length > 0) throw new Error("missing chunks");
  const bundle: Uint8Array = opener.finish();
  opener.free();
  return parseCaseBundle(bundle);
}

export const crateVersion: string = version();
