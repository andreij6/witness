use crate::error::WitnessError;
use crate::transport::Transport;
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use candid::{CandidType, Principal, decode_args, encode_args};
use futures::future::join_all;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use std::sync::Arc;
use witness_canister_common::index::probe_within_limits;
use witness_canister_common::limits::{MAX_CHUNK_BYTES, MAX_SUBMIT_BATCH};
use witness_canister_common::{
    BeginUpload, CaseInfo, CertifiedKey, CertifiedRecord, Chunk, EraseRequest, Fingerprints,
    MediaErr, PdqHit, ProbeResult, ShardInfo, Shards, Submission, SubmitResult,
};

pub(crate) const MAX_PDQ_HITS: usize = 32;
const HTTP_OK: u16 = 200;

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LateOpening {
    #[serde(with = "serde_bytes")]
    pub install_nonce: Vec<u8>,
    pub wall_ms: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Enrollment {
    pub key_id: Option<ByteBuf>,
    pub chain: Vec<ByteBuf>,
    pub late: Option<LateOpening>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Enrolled {
    #[serde(with = "serde_bytes")]
    pub key_id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub c2pa_cert: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub ca_cert: Vec<u8>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnrollResult {
    #[serde(rename = "ok")]
    Ok(Enrolled),
    #[serde(rename = "err")]
    Err(String),
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum EraseResult {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "err")]
    Err(String),
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Begun {
    pub missing: Vec<u32>,
    pub upload: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
    pub certificate_version: Option<u16>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpResponse {
    pub status_code: u16,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub(crate) struct IcpClient {
    transport: Arc<dyn Transport>,
    registry: Principal,
}

fn rejected(reason: String) -> WitnessError {
    WitnessError::Rejected { reason }
}

pub(crate) fn merge_probes(results: Vec<ProbeResult>) -> ProbeResult {
    let mut pdq: Vec<PdqHit> = Vec::new();
    let mut watermark = Vec::new();
    let mut exact = Vec::new();
    let mut truncated = false;
    let mut indexed_upto = u64::MAX;
    for r in &results {
        pdq.extend_from_slice(&r.pdq);
        watermark.extend_from_slice(&r.watermark);
        exact.extend_from_slice(&r.exact);
        truncated |= r.truncated_candidates;
        indexed_upto = indexed_upto.min(r.indexed_upto);
    }
    pdq.sort_unstable_by_key(|h| (h.record_ix, h.distance));
    pdq.dedup_by_key(|h| h.record_ix);
    pdq.sort_unstable_by_key(|h| (h.distance, h.record_ix));
    truncated |= pdq.len() > MAX_PDQ_HITS;
    pdq.truncate(MAX_PDQ_HITS);
    watermark.sort_unstable();
    watermark.dedup();
    exact.sort_unstable();
    exact.dedup();
    ProbeResult {
        pdq,
        watermark,
        exact,
        truncated_candidates: truncated,
        indexed_upto: if results.is_empty() { 0 } else { indexed_upto },
    }
}

impl IcpClient {
    pub(crate) fn new(transport: Arc<dyn Transport>, registry: Principal) -> Self {
        Self {
            transport,
            registry,
        }
    }

    pub(crate) fn registry(&self) -> Principal {
        self.registry
    }

    pub(crate) async fn root_key(&self) -> Result<Vec<u8>, WitnessError> {
        self.transport.root_key().await
    }

    async fn query<A: ArgumentEncoder, R: for<'de> ArgumentDecoder<'de>>(
        &self,
        canister: Principal,
        method: &str,
        args: A,
    ) -> Result<R, WitnessError> {
        let reply = self
            .transport
            .query(canister, method, encode_args(args)?)
            .await?;
        Ok(decode_args(&reply)?)
    }

    async fn update<A: ArgumentEncoder, R: for<'de> ArgumentDecoder<'de>>(
        &self,
        canister: Principal,
        method: &str,
        args: A,
    ) -> Result<R, WitnessError> {
        let reply = self
            .transport
            .update(canister, method, encode_args(args)?)
            .await?;
        Ok(decode_args(&reply)?)
    }

    pub(crate) async fn challenge(&self) -> Result<Vec<u8>, WitnessError> {
        let (c,): (ByteBuf,) = self.query(self.registry, "challenge", ()).await?;
        Ok(c.into_vec())
    }

    pub(crate) async fn ca_root(&self) -> Result<Option<Vec<u8>>, WitnessError> {
        let (r,): (Option<ByteBuf>,) = self.query(self.registry, "ca_root", ()).await?;
        Ok(r.map(ByteBuf::into_vec))
    }

    pub(crate) async fn enroll(&self, e: Enrollment) -> Result<Enrolled, WitnessError> {
        if e.chain.is_empty() {
            return Err(WitnessError::invalid("attestation chain is empty"));
        }
        match self.update(self.registry, "enroll", (e,)).await? {
            (EnrollResult::Ok(k),) => Ok(k),
            (EnrollResult::Err(m),) => Err(rejected(m)),
        }
    }

    pub(crate) async fn submit(
        &self,
        batch: Vec<Submission>,
    ) -> Result<Vec<SubmitResult>, WitnessError> {
        if batch.is_empty() || batch.len() > MAX_SUBMIT_BATCH {
            return Err(WitnessError::invalid("submit batch size"));
        }
        let n = batch.len();
        let (r,): (Vec<SubmitResult>,) = self.update(self.registry, "submit", (batch,)).await?;
        if r.len() != n {
            return Err(WitnessError::malformed("submit result count"));
        }
        Ok(r)
    }

    pub(crate) async fn get(
        &self,
        record_id: &str,
    ) -> Result<Option<CertifiedRecord>, WitnessError> {
        let (r,) = self.query(self.registry, "get", (record_id,)).await?;
        Ok(r)
    }

    pub(crate) async fn get_ix(
        &self,
        record_ix: u64,
    ) -> Result<Option<CertifiedRecord>, WitnessError> {
        let (r,) = self.query(self.registry, "get_ix", (record_ix,)).await?;
        Ok(r)
    }

    pub(crate) async fn get_key(
        &self,
        key_id: &[u8],
    ) -> Result<Option<CertifiedKey>, WitnessError> {
        let (r,) = self
            .query(self.registry, "get_key", (ByteBuf::from(key_id),))
            .await?;
        Ok(r)
    }

    pub(crate) async fn by_capture(&self, capture_id: u64) -> Result<Vec<String>, WitnessError> {
        let (r,) = self
            .query(self.registry, "by_capture", (capture_id,))
            .await?;
        Ok(r)
    }

    pub(crate) async fn shards(&self) -> Result<Shards, WitnessError> {
        let (s, _): (Shards, ByteBuf) = self.query(self.registry, "shards", ()).await?;
        Ok(s)
    }

    pub(crate) async fn erase_mine(&self, req: EraseRequest) -> Result<(), WitnessError> {
        match self.update(self.registry, "erase_mine", (req,)).await? {
            (EraseResult::Ok,) => Ok(()),
            (EraseResult::Err(m),) => Err(rejected(m)),
        }
    }

    pub(crate) async fn records(&self, ixs: &[u64]) -> Result<Vec<CertifiedRecord>, WitnessError> {
        let found = join_all(ixs.iter().map(|&ix| self.get_ix(ix))).await;
        found.into_iter().filter_map(Result::transpose).collect()
    }

    pub(crate) async fn probe(
        &self,
        index: &[ShardInfo],
        pdq: &[Vec<u8>],
        watermarks: &[u64],
        exact: &[Vec<u8>],
    ) -> Result<ProbeResult, WitnessError> {
        let pdq: Vec<ByteBuf> = pdq.iter().map(|h| ByteBuf::from(h.as_slice())).collect();
        let exact: Vec<ByteBuf> = exact.iter().map(|h| ByteBuf::from(h.as_slice())).collect();
        if !probe_within_limits(&pdq, watermarks, &exact) {
            return Err(WitnessError::invalid("probe exceeds limits"));
        }
        let calls = index.iter().map(|s| {
            self.query::<_, (ProbeResult,)>(s.canister, "probe", (&pdq, watermarks, &exact))
        });
        let results = join_all(calls)
            .await
            .into_iter()
            .map(|r| r.map(|(p,)| p))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(merge_probes(results))
    }

    pub(crate) async fn derivatives(
        &self,
        index: &[ShardInfo],
        root_ix: u64,
        cap: usize,
    ) -> Result<Vec<u64>, WitnessError> {
        let calls = index
            .iter()
            .map(|s| self.query::<_, (Vec<u64>,)>(s.canister, "derivatives", (root_ix,)));
        let mut all = Vec::new();
        for r in join_all(calls).await {
            all.extend(r?.0);
        }
        all.sort_unstable();
        all.dedup();
        all.truncate(cap);
        Ok(all)
    }

    pub(crate) async fn fingerprints(
        &self,
        index: &[ShardInfo],
        record_ix: u64,
    ) -> Result<Option<Fingerprints>, WitnessError> {
        let calls = index.iter().map(|s| {
            self.query::<_, (Option<Fingerprints>,)>(s.canister, "fingerprints", (record_ix,))
        });
        let mut found = None;
        for r in join_all(calls).await {
            found = found.or(r?.0);
        }
        Ok(found)
    }

    pub(crate) async fn http_get(
        &self,
        canister: Principal,
        path: &str,
    ) -> Result<Option<Vec<u8>>, WitnessError> {
        let req = HttpRequest {
            method: "GET".into(),
            url: path.into(),
            headers: Vec::new(),
            body: Vec::new(),
            certificate_version: Some(2),
        };
        let (r,): (HttpResponse,) = self.query(canister, "http_request", (req,)).await?;
        Ok((r.status_code == HTTP_OK).then_some(r.body))
    }

    pub(crate) async fn http_get_any(
        &self,
        shards: &[ShardInfo],
        path: &str,
    ) -> Result<Option<Vec<u8>>, WitnessError> {
        let calls = shards.iter().map(|s| self.http_get(s.canister, path));
        let mut found = None;
        for r in join_all(calls).await {
            found = found.or(r?);
        }
        Ok(found)
    }

    pub(crate) async fn begin_upload(
        &self,
        media: Principal,
        req: BeginUpload,
    ) -> Result<Begun, WitnessError> {
        let (r,): (Result<Begun, MediaErr>,) = self.update(media, "begin_upload", (req,)).await?;
        Ok(r?)
    }

    pub(crate) async fn put_chunk(
        &self,
        media: Principal,
        chunk: Chunk,
    ) -> Result<(), WitnessError> {
        if !chunk.within_limits() {
            return Err(WitnessError::invalid(format!(
                "chunk must be 1..={MAX_CHUNK_BYTES} bytes"
            )));
        }
        let (r,): (Result<(), MediaErr>,) = self.update(media, "put_chunk", (chunk,)).await?;
        Ok(r?)
    }

    pub(crate) async fn case_info(
        &self,
        media: Principal,
        case_id: &str,
    ) -> Result<Option<CaseInfo>, WitnessError> {
        let (r,): (Option<CaseInfo>,) = self.query(media, "case_info", (case_id,)).await?;
        Ok(r)
    }

    pub(crate) async fn commit(
        &self,
        media: Principal,
        upload: u64,
    ) -> Result<String, WitnessError> {
        let (r,): (Result<String, MediaErr>,) = self.update(media, "commit", (upload,)).await?;
        Ok(r?)
    }
}

#[cfg(test)]
pub(crate) mod tests;
