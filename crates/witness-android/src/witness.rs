use crate::client::{Begun, Enrolled, Enrollment, IcpClient};
use crate::error::WitnessError;
use crate::oracle::NetOracle;
use crate::seal::{KeystoreSigner, sign_es256};
use crate::transport::{AgentTransport, Transport, identity_seed};
use crate::trustmark::Trustmark;
use crate::verdict::VerdictView;
use candid::Principal;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::runtime::{Builder, Runtime};
use witness_canister_common::UNPINNED_ANCHOR_DER;
use witness_canister_common::limits;
use witness_canister_common::{
    BeginUpload, CaseInfo, CertifiedKey, CertifiedRecord, Chunk, EntitlementGrant, EraseAuth,
    EraseRequest, ProbeResult, Shards, Submission, SubmitResult, UploadTarget,
};
use witness_core::c2pa::TrustAnchor;
use witness_core::format::RecordId;
use witness_core::{Options, judge};
use witness_proof::{DEFAULT_MAX_AGE_NS, Trust as ProofTrust};

const NS_PER_S: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WitnessConfig {
    pub replica_url: String,
    pub registry: Principal,
    pub local: bool,
    pub ca_root: Option<Vec<u8>>,
    pub identity_seed: Option<Vec<u8>>,
    #[uniffi(default = None)]
    pub max_cert_age_s: Option<u64>,
}

fn wall_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

fn max_cert_age_ns(seconds: Option<u64>) -> u64 {
    seconds.map_or(DEFAULT_MAX_AGE_NS, |s| s.saturating_mul(NS_PER_S).max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct CaseLimits {
    pub max_chunks: u32,
    pub max_bytes: u64,
    pub plus_max_chunks: u32,
    pub plus_max_bytes: u64,
    pub chunk_bytes: u32,
}

#[uniffi::export]
pub fn case_limits() -> CaseLimits {
    CaseLimits {
        max_chunks: limits::MAX_CASE_CHUNKS,
        max_bytes: limits::MAX_CASE_BYTES,
        plus_max_chunks: limits::MAX_PLUS_CASE_CHUNKS,
        plus_max_bytes: limits::MAX_PLUS_CASE_BYTES,
        chunk_bytes: limits::MAX_CHUNK_BYTES as u32,
    }
}

#[uniffi::export]
pub async fn case_info(
    witness: Arc<Witness>,
    shard: u8,
    case_id: String,
) -> Result<Option<CaseInfo>, WitnessError> {
    witness.case_info(shard, case_id).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trust {
    Pinned,
    Dev,
    Missing,
}

#[derive(uniffi::Object)]
pub struct Witness {
    rt: Runtime,
    client: IcpClient,
    local: bool,
    shards: Mutex<Option<Arc<Shards>>>,
    anchor: Mutex<Option<(Arc<TrustAnchor>, Trust)>>,
    root_key: Mutex<Option<Arc<Vec<u8>>>>,
    max_cert_age_ns: u64,
    clock: fn() -> u64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn parse_anchor(der: &[u8]) -> Result<Arc<TrustAnchor>, WitnessError> {
    TrustAnchor::from_der(der)
        .map(Arc::new)
        .map_err(|e| WitnessError::invalid(format!("ca root: {e:?}")))
}

impl Witness {
    pub(crate) fn with_transport(
        transport: Arc<dyn Transport>,
        registry: Principal,
        ca_root: Option<&[u8]>,
        local: bool,
    ) -> Result<Self, WitnessError> {
        let pinned = ca_root
            .filter(|der| !der.is_empty())
            .map(|der| parse_anchor(der).map(|a| (a, Trust::Pinned)))
            .transpose()?;
        let rt = Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("witness-io")
            .enable_all()
            .build()
            .map_err(WitnessError::network)?;
        Ok(Self {
            rt,
            client: IcpClient::new(transport, registry),
            local,
            shards: Mutex::new(None),
            anchor: Mutex::new(pinned),
            root_key: Mutex::new(None),
            max_cert_age_ns: DEFAULT_MAX_AGE_NS,
            clock: wall_ns,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    pub(crate) fn with_max_cert_age_s(mut self, seconds: Option<u64>) -> Self {
        self.max_cert_age_ns = max_cert_age_ns(seconds);
        self
    }

    async fn root_key(&self) -> Result<Arc<Vec<u8>>, WitnessError> {
        if let Some(k) = lock(&self.root_key).clone() {
            return Ok(k);
        }
        let key = Arc::new(self.io(|c| async move { c.root_key().await }).await?);
        *lock(&self.root_key) = Some(key.clone());
        Ok(key)
    }

    async fn io<T, F>(&self, f: impl FnOnce(IcpClient) -> F) -> Result<T, WitnessError>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, WitnessError>> + Send + 'static,
    {
        self.rt
            .spawn(f(self.client.clone()))
            .await
            .map_err(WitnessError::network)?
    }

    async fn verified<T>(
        &self,
        found: Option<T>,
        holds: impl FnOnce(&ProofTrust<'_>, &T) -> bool,
    ) -> Result<Option<T>, WitnessError> {
        let Some(v) = found else {
            return Ok(None);
        };
        let root_key = self.root_key().await?;
        let registry = self.client.registry();
        let trust = ProofTrust {
            root_key: &root_key,
            canister: registry.as_slice(),
            now_ns: (self.clock)(),
            max_age_ns: self.max_cert_age_ns,
        };
        if holds(&trust, &v) {
            Ok(Some(v))
        } else {
            Err(WitnessError::Rejected {
                reason: "registry answer failed proof verification".into(),
            })
        }
    }

    async fn cached_shards(&self) -> Result<Arc<Shards>, WitnessError> {
        if let Some(s) = lock(&self.shards).clone() {
            return Ok(s);
        }
        self.shards().await?;
        lock(&self.shards)
            .clone()
            .ok_or_else(|| WitnessError::malformed("shards"))
    }

    async fn media(&self, shard: u8) -> Result<Principal, WitnessError> {
        self.cached_shards()
            .await?
            .media_shard(shard)
            .map(|s| s.canister)
            .ok_or_else(|| WitnessError::invalid(format!("unknown media shard {shard}")))
    }

    pub(crate) async fn case_info(
        &self,
        shard: u8,
        case_id: String,
    ) -> Result<Option<CaseInfo>, WitnessError> {
        let media = self.media(shard).await?;
        self.io(|c| async move { c.case_info(media, &case_id).await })
            .await
    }

    async fn trust_anchor(&self) -> Result<(Arc<TrustAnchor>, Trust), WitnessError> {
        if let Some(a) = lock(&self.anchor).clone() {
            return Ok(a);
        }
        let found = if self.local {
            let der = self
                .io(|c| async move { c.ca_root().await })
                .await?
                .ok_or_else(|| WitnessError::Rejected {
                    reason: "registry has no CA root yet".into(),
                })?;
            let anchor = parse_anchor(&der).map_err(|_| WitnessError::malformed("ca root"))?;
            (anchor, Trust::Dev)
        } else {
            (parse_anchor(UNPINNED_ANCHOR_DER)?, Trust::Missing)
        };
        *lock(&self.anchor) = Some(found.clone());
        Ok(found)
    }
}

#[uniffi::export]
impl Witness {
    #[uniffi::constructor]
    pub fn new(config: WitnessConfig) -> Result<Arc<Self>, WitnessError> {
        let seed = identity_seed(config.identity_seed.as_deref())?;
        let transport = AgentTransport::new(&config.replica_url, config.local, seed)?;
        Self::with_transport(
            Arc::new(transport),
            config.registry,
            config.ca_root.as_deref(),
            config.local,
        )
        .map(|w| Arc::new(w.with_max_cert_age_s(config.max_cert_age_s)))
    }

    pub async fn challenge(&self) -> Result<Vec<u8>, WitnessError> {
        self.io(|c| async move { c.challenge().await }).await
    }

    pub async fn ca_root(&self) -> Result<Option<Vec<u8>>, WitnessError> {
        self.io(|c| async move { c.ca_root().await }).await
    }

    pub async fn enroll(&self, enrollment: Enrollment) -> Result<Enrolled, WitnessError> {
        self.io(|c| async move { c.enroll(enrollment).await }).await
    }

    pub async fn submit(&self, batch: Vec<Submission>) -> Result<Vec<SubmitResult>, WitnessError> {
        self.io(|c| async move { c.submit(batch).await }).await
    }

    pub async fn get(&self, record_id: String) -> Result<Option<CertifiedRecord>, WitnessError> {
        let wanted = RecordId::parse(&record_id).map_err(|_| WitnessError::invalid("record id"))?;
        let found = self.io(|c| async move { c.get(&record_id).await }).await?;
        self.verified(found, |t, r| {
            t.certified_record(r).is_ok()
                && RecordId::parse(&r.record_id).is_ok_and(|id| id == wanted)
        })
        .await
    }

    pub async fn get_key(&self, key_id: Vec<u8>) -> Result<Option<CertifiedKey>, WitnessError> {
        let wanted = key_id.clone();
        let found = self.io(|c| async move { c.get_key(&key_id).await }).await?;
        self.verified(found, |t, k| {
            t.certified_key(k)
                .is_ok_and(|(info, _)| info.key_id.as_bytes().as_slice() == wanted.as_slice())
        })
        .await
    }

    pub async fn by_capture(&self, capture_id: u64) -> Result<Vec<String>, WitnessError> {
        self.io(|c| async move { c.by_capture(capture_id).await })
            .await
    }

    pub async fn shards(&self) -> Result<Shards, WitnessError> {
        let s = self.io(|c| async move { c.shards().await }).await?;
        *lock(&self.shards) = Some(Arc::new(s.clone()));
        Ok(s)
    }

    pub async fn probe(
        &self,
        pdq: Vec<Vec<u8>>,
        watermarks: Vec<u64>,
        exact: Vec<Vec<u8>>,
    ) -> Result<ProbeResult, WitnessError> {
        let shards = self.cached_shards().await?;
        self.io(|c| async move { c.probe(&shards.index, &pdq, &watermarks, &exact).await })
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn begin_upload(
        &self,
        shard: u8,
        target: UploadTarget,
        size: u64,
        sha256: Vec<u8>,
        key_id: Vec<u8>,
        signer: Arc<dyn KeystoreSigner>,
        grant: Option<EntitlementGrant>,
    ) -> Result<Begun, WitnessError> {
        let mut req = BeginUpload {
            target,
            size,
            sha256,
            key_id,
            signature: Vec::new(),
            grant,
        };
        let message = req
            .signing_bytes()
            .map_err(|e| WitnessError::invalid(e.to_string()))?;
        req.signature = sign_es256(signer.as_ref(), &message)?;
        let media = self.media(shard).await?;
        self.io(|c| async move { c.begin_upload(media, req).await })
            .await
    }

    pub async fn put_chunk(
        &self,
        shard: u8,
        upload: u64,
        ix: u32,
        bytes: Vec<u8>,
    ) -> Result<(), WitnessError> {
        let media = self.media(shard).await?;
        self.io(|c| async move { c.put_chunk(media, Chunk { upload, ix, bytes }).await })
            .await
    }

    pub async fn commit(&self, shard: u8, upload: u64) -> Result<String, WitnessError> {
        let media = self.media(shard).await?;
        self.io(|c| async move { c.commit(media, upload).await })
            .await
    }

    pub async fn erase_mine(
        &self,
        record_id: String,
        nonce: Vec<u8>,
        signer: Arc<dyn KeystoreSigner>,
    ) -> Result<(), WitnessError> {
        let mut req = EraseRequest {
            record_id,
            nonce,
            auth: EraseAuth::Ii,
        };
        let message = req
            .signing_bytes()
            .map_err(|e| WitnessError::invalid(e.to_string()))?;
        req.auth = EraseAuth::Device(sign_es256(signer.as_ref(), &message)?);
        self.io(|c| async move { c.erase_mine(req).await }).await
    }

    pub async fn check(
        &self,
        image: Vec<u8>,
        watermark: Option<Arc<Trustmark>>,
    ) -> Result<VerdictView, WitnessError> {
        let (anchor, trust) = self.trust_anchor().await?;
        let shards = self.cached_shards().await?;
        let root_key = self.root_key().await?;
        let now_ns = (self.clock)();
        let max_age_ns = self.max_cert_age_ns;
        let client = self.client.clone();
        let handle = self.rt.handle().clone();
        self.rt
            .spawn_blocking(move || {
                let mut options = Options::android(&anchor);
                if let Some(t) = watermark.as_deref() {
                    options = options.with_watermarker(t.watermarker());
                }
                let registry = client.registry();
                let proofs = ProofTrust {
                    root_key: &root_key,
                    canister: registry.as_slice(),
                    now_ns,
                    max_age_ns,
                };
                let oracle = NetOracle::new(&handle, &client, &shards, proofs);
                let verdict = judge(&image, &oracle, &options);
                let proofs_rejected = oracle.rejected();
                match oracle.into_failure() {
                    Some(e) => Err(e),
                    None => Ok(VerdictView {
                        dev_anchor: trust == Trust::Dev,
                        credentials_judged: trust != Trust::Missing,
                        proofs_rejected,
                        ..verdict.into()
                    }),
                }
            })
            .await
            .map_err(WitnessError::network)?
    }
}

#[cfg(test)]
mod tests;
