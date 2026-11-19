use crate::certtree::WILDCARD;
use crate::certtree::{CertTree, Hash};
use crate::http::{blob_response_hash, expr_hash, not_found_hash};
use crate::pagestore::{PageStore, pages_for};
use crate::paths::{BlobPath, Kind};
use crate::policy::*;
use crate::records::*;
use candid::{CandidType, Deserialize, Principal};
use ic_certification::HashTree;
use ic_http_certification::HttpCertificationPath;
use ic_http_certification::utils::more_specific_wildcards_for;
use ic_stable_structures::memory_manager::MemoryId;
use ic_stable_structures::{Memory, StableBTreeMap, StableBTreeSet, StableCell};
pub use witness_canister_common::Allow;
use witness_canister_common::limits::{
    MAX_CASE_BYTES, MAX_CASE_CHUNKS, MAX_PLUS_CASE_BYTES, MAX_PLUS_CASE_CHUNKS,
};
use witness_canister_common::memory::{local, media as ids};
use witness_canister_common::{
    BeginUpload, CaseInfo, Chunk, ContractError, EntitlementGrant, MediaErr,
};
use witness_format::crypto::Es256Key;
use witness_format::{CaseId, KeyId, RecordId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ctx {
    pub now_ns: u64,
    pub caller: Principal,
    pub shard: u8,
    pub signer: Option<[u8; 32]>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Begun {
    pub upload: u64,
    pub missing: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServeInfo {
    pub path: BlobPath,
    pub blob: u64,
    pub size: u64,
}

enum Charge {
    Daily(QuotaKey),
    Case {
        id: [u8; 16],
        meta: CaseMeta,
        new_case: Option<QuotaKey>,
        plus: Option<(Principal, Usage)>,
    },
    Plus {
        owner: Principal,
        usage: Usage,
    },
}

pub const KEYS: MemoryId = local(0);
pub const RECORDS: MemoryId = local(1);
pub const CASES: MemoryId = local(2);
pub const CHECKS: MemoryId = local(3);

pub struct Media<M: Memory> {
    pages: PageStore<M>,
    index: StableBTreeMap<PathKey, u64, M>,
    blobs: StableBTreeMap<u64, Meta, M>,
    quotas: StableBTreeMap<QuotaKey, u32, M>,
    usage: StableBTreeMap<Principal, Usage, M>,
    expiry: StableBTreeSet<(u64, u64), M>,
    tree: CertTree<M>,
    state: StableCell<State, M>,
    keys: StableBTreeMap<KeyBytes, Vec<u8>, M>,
    records: StableBTreeMap<RecordBytes, KeyBytes, M>,
    cases: StableBTreeMap<CaseBytes, CaseMeta, M>,
    checks: StableBTreeMap<RecordBytes, u64, M>,
}

fn malformed(s: &str) -> MediaErr {
    MediaErr::Malformed(s.to_owned())
}

pub const fn case_limits(plus: bool) -> (u32, u64) {
    if plus {
        (MAX_PLUS_CASE_CHUNKS, MAX_PLUS_CASE_BYTES)
    } else {
        (MAX_CASE_CHUNKS, MAX_CASE_BYTES)
    }
}

fn grant_err(e: ContractError) -> MediaErr {
    match e {
        ContractError::WrongCaller => MediaErr::Unauthorized,
        _ => MediaErr::GrantInvalid,
    }
}

impl<M: Memory> Media<M> {
    pub fn init(mem: impl Fn(MemoryId) -> M) -> Self {
        Self {
            pages: PageStore::init(mem(ids::BLOBS)),
            index: StableBTreeMap::init(mem(ids::BLOB_INDEX)),
            blobs: StableBTreeMap::init(mem(ids::UPLOADS)),
            quotas: StableBTreeMap::init(mem(ids::QUOTAS)),
            usage: StableBTreeMap::init(mem(ids::GRANT_USAGE)),
            expiry: StableBTreeSet::init(mem(ids::EXPIRY)),
            tree: CertTree::init(mem(ids::CERT_TREE), expr_hash(), Some(not_found_hash())),
            state: StableCell::init(mem(ids::STATE), State::default()),
            keys: StableBTreeMap::init(mem(KEYS)),
            records: StableBTreeMap::init(mem(RECORDS)),
            cases: StableBTreeMap::init(mem(CASES)),
            checks: StableBTreeMap::init(mem(CHECKS)),
        }
    }

    pub fn root_hash(&self) -> Hash {
        self.tree.root_hash()
    }

    pub fn begin(&mut self, req: &BeginUpload, ctx: &Ctx) -> Result<Begun, MediaErr> {
        req.check_shape()?;
        let path = BlobPath::from_target(&req.target)?;
        if path.kind() == Kind::Full && req.size > MAX_FULL_BYTES {
            return Err(MediaErr::TooLarge);
        }
        let key_id = req.key_id()?.to_bytes();
        let spki = self
            .keys
            .get(&key_id)
            .ok_or(MediaErr::RetryAfterNs(SYNC_RETRY_NS))?;
        req.verify(&spki)?;
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&req.sha256);
        if let Some(id) = self.index.get(&path.key()) {
            let mut meta = self.meta(id);
            let same = meta.sha256 == sha256 && meta.size == req.size && meta.key_id == key_id;
            if meta.committed && same {
                return Ok(Begun {
                    upload: id,
                    missing: Vec::new(),
                });
            }
            if meta.committed || meta.key_id != key_id {
                return Err(malformed("path taken"));
            }
            if let Some(wait) = meta.held_for(&ctx.caller, ctx.now_ns, STALE_UPLOAD_NS) {
                return Err(MediaErr::RetryAfterNs(wait));
            }
            if same {
                meta.uploader = ctx.caller;
                meta.touched_ns = ctx.now_ns;
                let missing = meta.missing();
                self.blobs.insert(id, meta);
                return Ok(Begun {
                    upload: id,
                    missing,
                });
            }
            self.delete_blob(id);
        }
        let charge = self.plan(&path, req, &key_id, ctx)?;
        let pages = self
            .pages
            .alloc(pages_for(req.size))
            .map_err(|_| MediaErr::TooLarge)?;
        let owner = self.apply(charge);
        let mut st = *self.state.get();
        let id = st.next_id;
        st.next_id += 1;
        self.state.set(st);
        let expires_ns = ctx.now_ns.saturating_add(PENDING_TTL_NS);
        let meta = Meta {
            path,
            size: req.size,
            sha256,
            key_id,
            uploader: ctx.caller,
            owner,
            created_ns: ctx.now_ns,
            expires_ns,
            committed: false,
            received: Vec::new(),
            pages,
            touched_ns: ctx.now_ns,
        };
        let missing = meta.missing();
        self.blobs.insert(id, meta);
        self.index.insert(path.key(), id);
        self.expiry.insert((expires_ns, id));
        Ok(Begun {
            upload: id,
            missing,
        })
    }

    fn plan(
        &self,
        path: &BlobPath,
        req: &BeginUpload,
        key_id: &KeyBytes,
        ctx: &Ctx,
    ) -> Result<Charge, MediaErr> {
        let kind = path.kind();
        let today = day(ctx.now_ns);
        if let Some(r) = req.target.record_id()? {
            match self.records.get(r.as_bytes()) {
                None => return Err(MediaErr::RetryAfterNs(SYNC_RETRY_NS)),
                Some(k) if &k != key_id => return Err(MediaErr::Unauthorized),
                Some(_) => {}
            }
        }
        match kind {
            Kind::Preview | Kind::Og => {
                let q = quota_key(today, kind as u8, key_id);
                self.under_limit(&q, kind)?;
                Ok(Charge::Daily(q))
            }
            Kind::Case => {
                let case = path.case_id().ok_or_else(|| malformed("case id"))?;
                if case.shard() != ctx.shard {
                    return Err(malformed("wrong shard"));
                }
                let count = match &req.target {
                    witness_canister_common::UploadTarget::CaseChunk { count, .. } => *count,
                    _ => return Err(malformed("case target")),
                };
                let plus = match &req.grant {
                    Some(g) => Some((g.ii_principal, self.plus_usage(g, req.size, ctx)?)),
                    None => None,
                };
                let (max_chunks, max_bytes) = case_limits(plus.is_some());
                if count > max_chunks {
                    return Err(MediaErr::TooLarge);
                }
                let (mut meta, new_case) = match self.cases.get(case.as_bytes()) {
                    Some(m) => {
                        if &m.key_id != key_id || m.count != count {
                            return Err(MediaErr::Unauthorized);
                        }
                        (m, None)
                    }
                    None => {
                        let q = quota_key(today, kind as u8, key_id);
                        self.under_limit(&q, kind)?;
                        let m = CaseMeta {
                            key_id: *key_id,
                            count,
                            reserved: 0,
                            live: 0,
                            expires_ns: ctx.now_ns.saturating_add(CASE_TTL_NS),
                        };
                        (m, Some(q))
                    }
                };
                meta.reserved = meta.reserved.saturating_add(req.size);
                if meta.reserved > max_bytes {
                    return Err(MediaErr::QuotaExceeded);
                }
                meta.live += 1;
                Ok(Charge::Case {
                    id: case.to_bytes(),
                    meta,
                    new_case,
                    plus,
                })
            }
            Kind::Full => {
                let grant = req.grant.as_ref().ok_or(MediaErr::GrantInvalid)?;
                let usage = self.plus_usage(grant, req.size, ctx)?;
                Ok(Charge::Plus {
                    owner: grant.ii_principal,
                    usage,
                })
            }
        }
    }

    fn plus_usage(
        &self,
        grant: &EntitlementGrant,
        size: u64,
        ctx: &Ctx,
    ) -> Result<Usage, MediaErr> {
        let signer = ctx.signer.ok_or(MediaErr::GrantInvalid)?;
        grant
            .verify(&signer, &ctx.caller, ctx.now_ns)
            .map_err(grant_err)?;
        let mut u = self.usage.get(&grant.ii_principal).unwrap_or_default();
        u.used = u.used.saturating_add(size);
        if u.used > grant.quota_bytes {
            return Err(MediaErr::QuotaExceeded);
        }
        u.expires_ns = u.expires_ns.max(grant.expires_ns);
        Ok(u)
    }

    fn under_limit(&self, q: &QuotaKey, kind: Kind) -> Result<(), MediaErr> {
        let used = self.quotas.get(q).unwrap_or(0);
        match daily_limit(kind) {
            Some(limit) if used >= limit => Err(MediaErr::QuotaExceeded),
            _ => Ok(()),
        }
    }

    fn bump(&mut self, q: QuotaKey) {
        let n = self.quotas.get(&q).unwrap_or(0);
        self.quotas.insert(q, n.saturating_add(1));
    }

    fn apply(&mut self, charge: Charge) -> Option<Principal> {
        match charge {
            Charge::Daily(q) => {
                self.bump(q);
                None
            }
            Charge::Case {
                id,
                meta,
                new_case,
                plus,
            } => {
                if let Some(q) = new_case {
                    self.bump(q);
                }
                self.cases.insert(id, meta);
                plus.map(|(owner, usage)| {
                    self.usage.insert(owner, usage);
                    owner
                })
            }
            Charge::Plus { owner, usage } => {
                self.usage.insert(owner, usage);
                Some(owner)
            }
        }
    }

    fn meta(&self, id: u64) -> Meta {
        self.blobs
            .get(&id)
            .unwrap_or_else(|| ic_cdk::trap("blob index corrupt"))
    }

    pub fn put_chunk(
        &mut self,
        c: &Chunk,
        caller: &Principal,
        now_ns: u64,
    ) -> Result<(), MediaErr> {
        let mut meta = self.blobs.get(&c.upload).ok_or(MediaErr::NotFound)?;
        if meta.committed {
            return Ok(());
        }
        if meta.uploader != *caller {
            return Err(MediaErr::Unauthorized);
        }
        if !c.within_limits() || meta.chunk_len(c.ix) != Some(c.bytes.len()) {
            return Err(MediaErr::BadChunk);
        }
        let offset = c.ix as u64 * witness_canister_common::limits::MAX_CHUNK_BYTES as u64;
        self.pages.write(&meta.pages, offset, &c.bytes);
        meta.mark(c.ix);
        meta.touched_ns = meta.touched_ns.max(now_ns);
        self.blobs.insert(c.upload, meta);
        Ok(())
    }

    pub fn commit(&mut self, id: u64, now_ns: u64) -> Result<String, MediaErr> {
        let mut meta = self.blobs.get(&id).ok_or(MediaErr::NotFound)?;
        if meta.committed {
            return Ok(meta.path.url());
        }
        if !meta.missing().is_empty() {
            return Err(MediaErr::Incomplete);
        }
        if self.pages.sha256(&meta.pages, meta.size) != meta.sha256 {
            meta.received.clear();
            self.blobs.insert(id, meta);
            return Err(MediaErr::HashMismatch);
        }
        let kind = meta.path.kind();
        self.tree.insert(
            kind,
            meta.path.label(),
            blob_response_hash(kind, &meta.sha256),
        );
        self.expiry.remove(&(meta.expires_ns, id));
        meta.expires_ns = match kind {
            Kind::Case => meta
                .path
                .case_id()
                .and_then(|c| self.cases.get(c.as_bytes()))
                .map_or(now_ns, |c| c.expires_ns),
            Kind::Full => meta
                .owner
                .and_then(|o| self.usage.get(&o))
                .map_or(now_ns, |u| u.expires_ns.saturating_add(PLUS_GRACE_NS)),
            Kind::Preview | Kind::Og => 0,
        };
        if meta.expires_ns != 0 {
            self.expiry.insert((meta.expires_ns, id));
        }
        meta.committed = true;
        meta.received = Vec::new();
        let url = meta.path.url();
        self.blobs.insert(id, meta);
        Ok(url)
    }

    fn delete_blob(&mut self, id: u64) -> bool {
        let Some(meta) = self.blobs.remove(&id) else {
            return false;
        };
        self.index.remove(&meta.path.key());
        if meta.committed {
            self.tree.remove(meta.path.kind(), meta.path.label());
        }
        self.expiry.remove(&(meta.expires_ns, id));
        self.pages.free(&meta.pages);
        if let Some(case) = meta.path.case_id()
            && let Some(mut c) = self.cases.get(case.as_bytes())
        {
            c.reserved = c.reserved.saturating_sub(meta.size);
            c.live = c.live.saturating_sub(1);
            if c.live == 0 {
                self.cases.remove(case.as_bytes());
            } else {
                self.cases.insert(case.to_bytes(), c);
            }
        }
        if let Some(owner) = meta.owner
            && let Some(mut u) = self.usage.get(&owner)
        {
            u.used = u.used.saturating_sub(meta.size);
            self.usage.insert(owner, u);
        }
        true
    }

    pub fn erase_record(&mut self, record_id: &str) -> Result<u32, MediaErr> {
        let r = RecordId::parse(record_id).map_err(|_| malformed("record id"))?;
        self.records.remove(r.as_bytes());
        self.checks.remove(r.as_bytes());
        let mut n = 0;
        for kind in [Kind::Preview, Kind::Og, Kind::Full] {
            if let Some(id) = self.index.get(&BlobPath::record(kind, &r).key()) {
                n += self.delete_blob(id) as u32;
            }
        }
        Ok(n)
    }

    pub fn sync(&mut self, items: &[Allow]) -> u32 {
        let mut n = 0;
        for item in items.iter().take(MAX_SYNC_BATCH) {
            n += match item {
                Allow::Key { key_id, pubkey } => self.allow_key(key_id, pubkey),
                Allow::Record { record_id, key_id } => self.allow_record(record_id, key_id),
                Allow::Revoke(key_id) => match <KeyBytes>::try_from(key_id.as_slice()) {
                    Ok(k) => {
                        self.keys.remove(&k);
                        true
                    }
                    Err(_) => false,
                },
            } as u32;
        }
        n
    }

    fn allow_key(&mut self, key_id: &[u8], pubkey: &[u8]) -> bool {
        let Ok(k) = KeyBytes::try_from(key_id) else {
            return false;
        };
        let ok = pubkey.len() <= MAX_SPKI_BYTES
            && KeyId::from_spki(pubkey).to_bytes() == k
            && Es256Key::from_spki(pubkey).is_ok();
        if ok {
            self.keys.insert(k, pubkey.to_vec());
        }
        ok
    }

    fn allow_record(&mut self, record_id: &str, key_id: &[u8]) -> bool {
        match (RecordId::parse(record_id), KeyBytes::try_from(key_id)) {
            (Ok(r), Ok(k)) => {
                self.records.insert(r.to_bytes(), k);
                true
            }
            _ => false,
        }
    }

    pub fn expire(&mut self, now_ns: u64, budget: usize) -> u32 {
        let mut deleted = 0;
        for _ in 0..budget {
            let Some((deadline, id)) = self.expiry.first() else {
                break;
            };
            if deadline > now_ns {
                break;
            }
            self.expiry.remove(&(deadline, id));
            let Some(mut meta) = self.blobs.get(&id) else {
                continue;
            };
            let renewed = match (meta.committed, meta.owner, meta.path.kind()) {
                (true, Some(o), Kind::Full) => self
                    .usage
                    .get(&o)
                    .map(|u| u.expires_ns.saturating_add(PLUS_GRACE_NS))
                    .filter(|d| *d > now_ns),
                _ => None,
            };
            if let Some(d) = renewed {
                meta.expires_ns = d;
                self.expiry.insert((d, id));
                self.blobs.insert(id, meta);
                continue;
            }
            deleted += self.delete_blob(id) as u32;
        }
        self.purge_quotas(day(now_ns), budget);
        deleted
    }

    fn purge_quotas(&mut self, today: u32, budget: usize) {
        let stale: Vec<QuotaKey> = self
            .quotas
            .keys_range(..quota_key(today, 0, &[0; 16]))
            .take(budget)
            .collect();
        for k in stale {
            self.quotas.remove(&k);
        }
    }

    pub fn count_checks(&mut self, ids: &[String], now_ns: u64) -> u32 {
        let mut st = *self.state.get();
        let window = now_ns / EXPIRY_PERIOD_NS;
        if st.check_window != window {
            st.check_window = window;
            st.check_used = 0;
        }
        let mut seen: Vec<RecordBytes> = Vec::with_capacity(MAX_CHECK_BATCH);
        for id in ids.iter().take(MAX_CHECK_BATCH) {
            if st.check_used >= CHECK_BUDGET {
                break;
            }
            let Ok(r) = RecordId::parse(id) else {
                continue;
            };
            let r = r.to_bytes();
            if seen.contains(&r) || !self.records.contains_key(&r) {
                continue;
            }
            seen.push(r);
            let n = self.checks.get(&r).unwrap_or(0);
            self.checks.insert(r, n.saturating_add(1));
            st.check_used += 1;
        }
        let counted = seen.len() as u32;
        self.state.set(st);
        counted
    }

    pub fn checks(&self, ids: &[String]) -> Vec<u64> {
        ids.iter()
            .take(MAX_CHECK_BATCH)
            .map(|id| {
                RecordId::parse(id)
                    .ok()
                    .and_then(|r| self.checks.get(r.as_bytes()))
                    .unwrap_or(0)
            })
            .collect()
    }

    pub fn case_info(&self, case_id: &str) -> Option<CaseInfo> {
        let case = CaseId::parse(case_id).ok()?;
        self.cases.get(case.as_bytes()).map(|c| CaseInfo {
            expires_ns: c.expires_ns,
            bytes: c.reserved,
            chunks: c.count,
        })
    }

    pub fn lookup(&self, url: &str) -> Option<ServeInfo> {
        let path = BlobPath::parse(url)?;
        let blob = self.index.get(&path.key())?;
        let meta = self.blobs.get(&blob)?;
        meta.committed.then_some(ServeInfo {
            path,
            blob,
            size: meta.size,
        })
    }

    pub fn read(&self, blob: u64, offset: u64, len: u64) -> Option<Vec<u8>> {
        let meta = self.blobs.get(&blob).filter(|m| m.committed)?;
        let end = offset.checked_add(len).filter(|e| *e <= meta.size)?;
        let mut out = vec![0u8; (end - offset) as usize];
        self.pages.read(&meta.pages, offset, &mut out);
        Some(out)
    }

    pub fn blob_size(&self, blob: u64) -> Option<u64> {
        self.blobs
            .get(&blob)
            .filter(|m| m.committed)
            .map(|m| m.size)
    }

    pub fn witness(&self, path: &BlobPath) -> Option<HashTree> {
        self.tree.witness(path.kind(), path.label())
    }

    pub fn absence_witness(&self, url_path: &str) -> HashTree {
        let segments = |p: Vec<String>| -> Vec<Vec<u8>> {
            p.into_iter().skip(1).map(String::into_bytes).collect()
        };
        let req = segments(HttpCertificationPath::exact(url_path).to_expr_path());
        let responding = vec![WILDCARD.to_vec()];
        let mut proofs = more_specific_wildcards_for(&req, &responding);
        proofs.push(req);
        self.tree.witness_paths(&proofs)
    }

    pub fn metrics(&self) -> Vec<(String, u64)> {
        [
            ("blobs", self.blobs.len()),
            ("pages_total", self.pages.total_pages()),
            ("pages_free", self.pages.free_pages()),
            ("cert_nodes", self.tree.node_count()),
            ("keys", self.keys.len()),
            ("records", self.records.len()),
            ("cases", self.cases.len()),
            ("expiry_queue", self.expiry.len()),
            ("plus_accounts", self.usage.len()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect()
    }

    #[cfg(test)]
    pub(crate) fn usage_of(&self, p: &Principal) -> Option<Usage> {
        self.usage.get(p)
    }

    #[cfg(test)]
    pub(crate) fn case_of(&self, c: &witness_format::CaseId) -> Option<CaseMeta> {
        self.cases.get(c.as_bytes())
    }

    #[cfg(test)]
    pub(crate) fn quota_entries(&self) -> u64 {
        self.quotas.len()
    }

    #[cfg(test)]
    pub(crate) fn tree_contains(&self, path: &BlobPath) -> bool {
        self.tree.contains(path.kind(), path.label())
    }
}

#[cfg(test)]
mod tests;
