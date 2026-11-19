import init, {
  CaseOpener,
  decodeClaim,
  judge,
  parseCaseBundle,
  queryPlan,
  verifyCertifiedKey,
  verifyCertifiedRecord,
  verifyShards,
} from './wasm/witness_web.js';
import { Actor, AnonymousIdentity, HttpAgent } from '@icp-sdk/core/agent';
import { Principal } from '@icp-sdk/core/principal';
import { indexIdl, mediaIdl, registryIdl } from './candid.mjs';
import { canisterUrl, readConfig } from './config.mjs';
import { MAX_AGE_NS, createClient, unwrapOpt } from './engine.mjs';

const core = { CaseOpener, decodeClaim, judge, parseCaseBundle, queryPlan, verifyCertifiedKey, verifyCertifiedRecord, verifyShards };

export function networkIo(agent, config, fetchFn = globalThis.fetch.bind(globalThis)) {
  const registry = Actor.createActor(registryIdl, { agent, canisterId: config.registry });
  const actors = new Map();
  const actor = (idl, s) => {
    const id = s.canister.toText();
    let a = actors.get(id);
    if (!a) actors.set(id, (a = Actor.createActor(idl, { agent, canisterId: id })));
    return a;
  };
  const index = (s) => actor(indexIdl, s);
  return {
    get: async (id) => unwrapOpt(await registry.get(id)),
    getIx: async (ix) => unwrapOpt(await registry.get_ix(ix)),
    getKey: async (keyId) => unwrapOpt(await registry.get_key(keyId)),
    caRoot: async () => {
      if (!config.local) throw new Error('witness: no pinned CA anchor');
      return unwrapOpt(await registry.ca_root());
    },
    shards: async () => {
      const [map, bundle] = await registry.shards();
      return { map, bundle };
    },
    probe: (s, pdq, watermarks, exact) => index(s).probe(pdq, watermarks, exact),
    derivatives: (s, ix) => index(s).derivatives(ix),
    fingerprints: async (s, ix) => unwrapOpt(await index(s).fingerprints(ix)),
    caseInfo: async (shards, caseId) => {
      for (const s of shards) {
        const info = unwrapOpt(await actor(mediaIdl, s).case_info(caseId));
        if (info) return info;
      }
      return null;
    },
    media: async (shards, path) => {
      for (const s of shards) {
        const r = await fetchFn(canisterUrl(config, s.canister.toText(), path));
        if (r.ok) return new Uint8Array(await r.arrayBuffer());
      }
      return null;
    },
  };
}

export async function open(config, wasm, fetchFn) {
  if (!config.registry) throw new Error('witness: no registry configured');
  await init(wasm === undefined ? undefined : { module_or_path: wasm });
  const agent = await HttpAgent.create({ host: config.host, identity: new AnonymousIdentity(), shouldFetchRootKey: config.local });
  if (!agent.rootKey) throw new Error('witness: no root key');
  const trust = {
    rootKey: new Uint8Array(agent.rootKey),
    canister: Principal.fromText(config.registry).toUint8Array(),
    now: () => BigInt(Date.now()) * 1_000_000n,
    maxAgeNs: MAX_AGE_NS,
  };
  return createClient(core, networkIo(agent, config, fetchFn), trust, config.anchor, config.local);
}

let client;

export function connect() {
  client ??= open(readConfig(globalThis.document)).catch((e) => {
    client = undefined;
    throw e;
  });
  return client;
}

export const check = async (bytes, onStage) => (await connect()).check(bytes, onStage);
export const proof = async (recordId) => (await connect()).proof(recordId);
export const key = async (keyIdHex) => (await connect()).key(keyIdHex);
export const preview = async (recordId) => (await connect()).preview(recordId);
export const openCase = async (caseId, fragment) => (await connect()).openCase(caseId, fragment);
