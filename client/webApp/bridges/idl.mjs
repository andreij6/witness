export const registryIdl = ({ IDL }) => {
  const Blob = IDL.Vec(IDL.Nat8);
  const ShardInfo = IDL.Record({ canister_id: IDL.Principal, base_ix: IDL.Nat64, indexed_upto: IDL.Nat64 });
  const ShardMap = IDL.Record({ index: IDL.Vec(ShardInfo), media: IDL.Vec(ShardInfo) });
  const CertifiedRecord = IDL.Record({
    record_ix: IDL.Nat64,
    record_id: IDL.Text,
    claim: Blob,
    c2pa_claim: Blob,
    cose: Blob,
    anchored_ns: IDL.Nat64,
    mmr_proof: IDL.Vec(Blob),
    leaf_count: IDL.Nat64,
    certificate: Blob,
  });
  return IDL.Service({
    challenge: IDL.Func([], [Blob], ['query']),
    get: IDL.Func([IDL.Text], [IDL.Opt(CertifiedRecord)], ['query']),
    get_ix: IDL.Func([IDL.Nat64], [IDL.Opt(CertifiedRecord)], ['query']),
    by_capture: IDL.Func([IDL.Nat64], [IDL.Vec(IDL.Text)], ['query']),
    shards: IDL.Func([], [ShardMap, Blob], ['query']),
  });
};

export const indexIdl = ({ IDL }) => {
  const Blob = IDL.Vec(IDL.Nat8);
  const PdqHit = IDL.Record({ local_ix: IDL.Nat32, shape: IDL.Nat8, distance: IDL.Nat16 });
  const ProbeResult = IDL.Record({
    pdq: IDL.Vec(PdqHit),
    watermark: IDL.Vec(IDL.Nat64),
    exact: IDL.Vec(IDL.Nat64),
    truncated: IDL.Bool,
    indexed_upto: IDL.Nat64,
  });
  const Fingerprints = IDL.Record({ grid: Blob, shapes: IDL.Vec(IDL.Tuple(IDL.Nat8, Blob)) });
  return IDL.Service({
    probe: IDL.Func([IDL.Vec(Blob), IDL.Vec(IDL.Nat64), IDL.Vec(Blob)], [ProbeResult], ['query']),
    fingerprints: IDL.Func([IDL.Nat64], [IDL.Opt(Fingerprints)], ['query']),
  });
};

export const mediaIdl = ({ IDL }) => {
  const Blob = IDL.Vec(IDL.Nat8);
  const CaseMeta = IDL.Record({ count: IDL.Nat32, bytes: IDL.Nat64, bundle_hash: Blob, expires_ns: IDL.Nat64 });
  return IDL.Service({
    case_meta: IDL.Func([Blob], [IDL.Opt(CaseMeta)], ['query']),
    case_chunk: IDL.Func([Blob, IDL.Nat32], [IDL.Opt(Blob)], ['query']),
  });
};
