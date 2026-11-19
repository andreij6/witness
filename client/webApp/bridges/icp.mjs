import { Actor, AnonymousIdentity, Certificate, HttpAgent, lookupResultToBuffer } from '@icp-sdk/core/agent';
import { Principal } from '@icp-sdk/core/principal';
import { indexIdl, mediaIdl, registryIdl } from './idl.mjs';
import { nsToMs, packProbes, slices, splitIds, u64Pairs, unwrapOpt } from './wire.mjs';

export const PDQ_BYTES = 32;

export async function connect(host, fetchRootKey, registryId) {
  const agent = await HttpAgent.create({ host, identity: new AnonymousIdentity(), shouldFetchRootKey: fetchRootKey });
  return { agent, registryId, actors: new Map() };
}

function actor(client, idl, canisterId) {
  const key = `${canisterId}`;
  let a = client.actors.get(key);
  if (!a) {
    a = Actor.createActor(idl, { agent: client.agent, canisterId });
    client.actors.set(key, a);
  }
  return a;
}

const registry = (client) => actor(client, registryIdl, client.registryId);

export async function certifiedData(client, canisterId, certificate) {
  const principal = Principal.fromText(canisterId);
  const cert = await Certificate.create({ certificate, rootKey: client.agent.rootKey, principal: { canisterId: principal } });
  return lookupResultToBuffer(cert.lookup_path(['canister', principal.toUint8Array(), 'certified_data'])) ?? null;
}

async function verified(client, certificate) {
  const data = await certifiedData(client, client.registryId, certificate);
  if (!data) throw new Error('registry certificate carries no certified data');
  return data;
}

function shardOut(s) {
  return { canisterId: s.canister_id.toText(), baseIx: String(s.base_ix), indexedUpto: String(s.indexed_upto) };
}

function recordOut(r) {
  return {
    recordIx: String(r.record_ix),
    recordId: r.record_id,
    claim: r.claim,
    c2paClaim: r.c2pa_claim,
    cose: r.cose,
    anchoredNs: String(r.anchored_ns),
    mmrProof: r.mmr_proof,
    leafCount: String(r.leaf_count),
    certificate: r.certificate,
  };
}

export async function getRecord(client, recordId) {
  const r = unwrapOpt(await registry(client).get(recordId));
  if (!r) return null;
  await verified(client, r.certificate);
  return recordOut(r);
}

export async function byCapture(client, captureHi, captureLo) {
  const ids = await registry(client).by_capture(u64Pairs(Int32Array.of(captureHi, captureLo))[0]);
  return ids.join(',');
}

export async function shards(client) {
  const [map, certificate] = await registry(client).shards();
  await verified(client, certificate);
  return { index: map.index.map(shardOut), media: map.media.map(shardOut) };
}

export async function probeAll(client, canisterIds, pdq, watermarks, exact, exactStride, onShard) {
  const ids = splitIds(canisterIds);
  const args = [slices(pdq, PDQ_BYTES), u64Pairs(watermarks), exact.length === 0 ? [] : slices(exact, exactStride)];
  let answered = 0;
  const results = await Promise.all(
    ids.map((id) =>
      actor(client, indexIdl, id)
        .probe(...args)
        .then(
          (r) => {
            answered++;
            onShard(answered, ids.length);
            return r;
          },
          () => {
            onShard(answered, ids.length);
            return null;
          },
        ),
    ),
  );
  return packProbes(results);
}

export async function caseMeta(client, mediaId, caseId) {
  const m = unwrapOpt(await actor(client, mediaIdl, mediaId).case_meta(caseId));
  if (!m) return null;
  return { count: m.count, bytes: Number(m.bytes), bundleHash: m.bundle_hash, expiresMs: nsToMs(m.expires_ns) };
}

export async function caseChunk(client, mediaId, caseId, ix) {
  return unwrapOpt(await actor(client, mediaIdl, mediaId).case_chunk(caseId, ix));
}
