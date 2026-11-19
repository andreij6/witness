import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { initSync, judge, queryPlan, parseCaseBundle, CaseOpener, verifyCertifiedRecord, version } from "../pkg/witness_web.js";

const wasm = readFileSync(fileURLToPath(new URL("../pkg/witness_web_bg.wasm", import.meta.url)));
initSync({ module: wasm });
assert.match(version(), /^\d+\.\d+\.\d+$/);

const fixture = JSON.parse(readFileSync(process.argv[2], "utf8"));
const options = JSON.stringify(fixture.options);
for (const c of fixture.cases) {
  const input = Buffer.from(c.input, "base64");
  const plan = JSON.parse(queryPlan(input, options));
  assert.equal(plan.done, false, c.name);
  const step = JSON.parse(judge(input, options, JSON.stringify(c.fetched)));
  assert.equal(step.done, true, c.name);
  assert.deepEqual(step.verdict, c.verdict, c.name);
  console.log(`${c.name}: ${step.verdict.outcome} in ${c.rounds} rounds`);
}

assert.throws(() => queryPlan(new Uint8Array([1]), "{"), /invalid json/);
assert.throws(() => queryPlan(new Uint8Array([1]), '{"anchor":"00"}'), /malformed anchor/);
assert.throws(() => parseCaseBundle(new Uint8Array([0xa0])), /malformed case bundle/);
assert.throws(() => new CaseOpener("#x=1", "0".repeat(26), 1, new Uint8Array(32)), /key missing/);
assert.throws(() => verifyCertifiedRecord("{}", new Uint8Array(133), new Uint8Array(10), 0n, 0n), /invalid json/);
console.log("smoke ok");
