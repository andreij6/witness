export const registryIdl = ({ IDL }) => {
  const Blob = IDL.Vec(IDL.Nat8);
  const KeyStatus = IDL.Variant({ Active: IDL.Null, Suspended: IDL.Null, Retired: IDL.Null, Revoked: IDL.Null });
  const KeyInfo = IDL.Record({
    freshness: IDL.Variant({ Late: IDL.Null, Fresh: IDL.Null }),
    provisioning: IDL.Variant({ Dev: IDL.Null, Rkp: IDL.Null, Factory: IDL.Null }),
    chain_serials: IDL.Vec(IDL.Text),
    key_id: Blob,
    enrolled_ns: IDL.Nat64,
    security: IDL.Variant({ Dev: IDL.Null, Tee: IDL.Null, StrongBox: IDL.Null }),
    history: IDL.Vec(IDL.Record({ status: KeyStatus, since_ns: IDL.Nat64 })),
    pubkey: Blob,
    patch: IDL.Nat32,
    active_until_ns: IDL.Nat64,
  });
  const CertifiedKey = IDL.Record({ key: KeyInfo, certificate: Blob, smt_proof: Blob });
  const CertifiedRecord = IDL.Record({
    certificate: Blob,
    cose: Blob,
    claim: Blob,
    anchored_ns: IDL.Nat64,
    mmr_proof: IDL.Vec(Blob),
    c2pa_claim: Blob,
    leaf_count: IDL.Nat64,
    record_id: IDL.Text,
    record_ix: IDL.Nat64,
    media_shard: IDL.Nat8,
  });
  const ShardInfo = IDL.Record({ open: IDL.Bool, shard: IDL.Nat8, canister: IDL.Principal });
  const Shards = IDL.Record({ media: IDL.Vec(ShardInfo), index: IDL.Vec(ShardInfo) });
  return IDL.Service({
    by_capture: IDL.Func([IDL.Nat64], [IDL.Vec(IDL.Text)], ['query']),
    ca_root: IDL.Func([], [IDL.Opt(Blob)], ['query']),
    get: IDL.Func([IDL.Text], [IDL.Opt(CertifiedRecord)], ['query']),
    get_ix: IDL.Func([IDL.Nat64], [IDL.Opt(CertifiedRecord)], ['query']),
    get_key: IDL.Func([Blob], [IDL.Opt(CertifiedKey)], ['query']),
    shards: IDL.Func([], [Shards, Blob], ['query']),
  });
};

export const indexIdl = ({ IDL }) => {
  const Blob = IDL.Vec(IDL.Nat8);
  const ProbeResult = IDL.Record({
    pdq: IDL.Vec(IDL.Record({ distance: IDL.Nat32, record_ix: IDL.Nat64 })),
    exact: IDL.Vec(IDL.Nat64),
    watermark: IDL.Vec(IDL.Nat64),
    indexed_upto: IDL.Nat64,
    truncated_candidates: IDL.Bool,
  });
  const Fingerprints = IDL.Record({ shapes: IDL.Vec(IDL.Tuple(IDL.Nat8, Blob)), quality: IDL.Nat8, grid: Blob, portrait: IDL.Bool });
  return IDL.Service({
    derivatives: IDL.Func([IDL.Nat64], [IDL.Vec(IDL.Nat64)], ['query']),
    fingerprints: IDL.Func([IDL.Nat64], [IDL.Opt(Fingerprints)], ['query']),
    probe: IDL.Func([IDL.Vec(Blob), IDL.Vec(IDL.Nat64), IDL.Vec(Blob)], [ProbeResult], ['query']),
  });
};

export const mediaIdl = ({ IDL }) => {
  const CaseInfo = IDL.Record({ expires_ns: IDL.Nat64, bytes: IDL.Nat64, chunks: IDL.Nat32 });
  return IDL.Service({
    case_info: IDL.Func([IDL.Text], [IDL.Opt(CaseInfo)], ['query']),
  });
};
