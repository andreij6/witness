use witness_canister_common::{ContractError, UploadTarget};
use witness_format::{CaseId, RecordId};

pub const MAX_LABEL: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Case,
    Full,
    Og,
    Preview,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Case, Kind::Full, Kind::Og, Kind::Preview];

    pub const fn ix(self) -> usize {
        self as usize
    }

    pub const fn segment(self) -> &'static str {
        match self {
            Self::Case => "c",
            Self::Full => "f",
            Self::Og => "o",
            Self::Preview => "p",
        }
    }

    pub fn from_segment(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.segment() == s)
    }

    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Og | Self::Preview => "image/jpeg",
            Self::Case | Self::Full => "application/octet-stream",
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Case),
            1 => Some(Self::Full),
            2 => Some(Self::Og),
            3 => Some(Self::Preview),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobPath {
    kind: Kind,
    len: u8,
    label: [u8; MAX_LABEL],
}

impl BlobPath {
    fn new(kind: Kind, label: &[u8]) -> Self {
        let mut buf = [0u8; MAX_LABEL];
        buf[..label.len()].copy_from_slice(label);
        Self {
            kind,
            len: label.len() as u8,
            label: buf,
        }
    }

    pub fn record(kind: Kind, id: &RecordId) -> Self {
        Self::new(kind, &id.to_base32())
    }

    pub fn case(id: &CaseId, ix: u32) -> Self {
        let mut buf = [0u8; MAX_LABEL];
        let text = id.to_base32();
        buf[..text.len()].copy_from_slice(&text);
        buf[text.len()] = b'.';
        let mut n = text.len() + 1;
        let digits = ix.to_string();
        buf[n..n + digits.len()].copy_from_slice(digits.as_bytes());
        n += digits.len();
        Self::new(Kind::Case, &buf[..n])
    }

    pub fn from_target(target: &UploadTarget) -> Result<Self, ContractError> {
        if let Some(r) = target.record_id()? {
            let kind = match target {
                UploadTarget::Preview(_) => Kind::Preview,
                UploadTarget::Og(_) => Kind::Og,
                _ => Kind::Full,
            };
            return Ok(Self::record(kind, &r));
        }
        match (target.case_id()?, target) {
            (Some(c), UploadTarget::CaseChunk { ix, .. }) => Ok(Self::case(&c, *ix)),
            _ => Err(ContractError::Invalid),
        }
    }

    pub fn parse(url: &str) -> Option<Self> {
        let path = url.split(['?', '#']).next().unwrap_or_default();
        let rest = path.strip_prefix('/')?;
        let (seg, label) = rest.split_once('/')?;
        let kind = Kind::from_segment(seg)?;
        let parsed = match kind {
            Kind::Case => {
                let (case, ix) = label.split_once('.')?;
                let ix: u32 = ix.parse().ok()?;
                Self::case(&CaseId::parse(case).ok()?, ix)
            }
            _ => Self::record(kind, &RecordId::parse(label).ok()?),
        };
        (parsed.label() == label.as_bytes()).then_some(parsed)
    }

    pub const fn kind(&self) -> Kind {
        self.kind
    }

    pub fn label(&self) -> &[u8] {
        &self.label[..self.len as usize]
    }

    pub fn case_id(&self) -> Option<CaseId> {
        if self.kind != Kind::Case {
            return None;
        }
        let label = core::str::from_utf8(self.label()).ok()?;
        CaseId::parse(label.split_once('.')?.0).ok()
    }

    pub fn url(&self) -> String {
        let mut s = String::with_capacity(3 + self.len as usize);
        s.push('/');
        s.push_str(self.kind.segment());
        s.push('/');
        s.push_str(core::str::from_utf8(self.label()).unwrap_or_default());
        s
    }

    pub fn key(&self) -> [u8; MAX_LABEL + 1] {
        let mut k = [0u8; MAX_LABEL + 1];
        k[0] = self.kind as u8;
        k[1..].copy_from_slice(&self.label);
        k
    }

    pub fn from_key(k: &[u8; MAX_LABEL + 1]) -> Option<Self> {
        let kind = Kind::from_code(k[0])?;
        let len = k[1..].iter().position(|b| *b == 0).unwrap_or(MAX_LABEL);
        Some(Self::new(kind, &k[1..1 + len]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(b: u8) -> RecordId {
        RecordId::from_bytes([b; 10])
    }

    fn cid(shard: u8) -> CaseId {
        CaseId::new(shard, [7; 15])
    }

    #[test]
    fn kinds_are_sorted_by_segment() {
        let segs: Vec<&str> = Kind::ALL.iter().map(|k| k.segment()).collect();
        let mut sorted = segs.clone();
        sorted.sort_unstable();
        assert_eq!(segs, sorted);
        for (i, k) in Kind::ALL.iter().enumerate() {
            assert_eq!(k.ix(), i);
            assert_eq!(Kind::from_code(i as u8), Some(*k));
            assert_eq!(Kind::from_segment(k.segment()), Some(*k));
        }
        assert_eq!(Kind::from_code(4), None);
        assert_eq!(Kind::from_segment("x"), None);
        assert_eq!(Kind::from_segment(""), None);
    }

    #[test]
    fn content_types() {
        assert_eq!(Kind::Preview.content_type(), "image/jpeg");
        assert_eq!(Kind::Og.content_type(), "image/jpeg");
        assert_eq!(Kind::Full.content_type(), "application/octet-stream");
        assert_eq!(Kind::Case.content_type(), "application/octet-stream");
    }

    #[test]
    fn record_paths_round_trip() {
        for kind in [Kind::Preview, Kind::Og, Kind::Full] {
            let p = BlobPath::record(kind, &rid(3));
            assert_eq!(p.label().len(), 16);
            assert_eq!(BlobPath::parse(&p.url()), Some(p));
            assert_eq!(BlobPath::from_key(&p.key()), Some(p));
        }
    }

    #[test]
    fn case_paths_round_trip() {
        for ix in [0, 1, 9, 10, u32::MAX] {
            let p = BlobPath::case(&cid(2), ix);
            assert_eq!(p.kind(), Kind::Case);
            assert!(p.label().len() <= MAX_LABEL);
            assert_eq!(BlobPath::parse(&p.url()), Some(p));
            assert_eq!(BlobPath::from_key(&p.key()), Some(p));
        }
        assert!(BlobPath::case(&cid(2), 1).url().ends_with(".1"));
    }

    #[test]
    fn case_id_only_for_cases() {
        assert_eq!(BlobPath::case(&cid(4), 9).case_id(), Some(cid(4)));
        assert_eq!(BlobPath::record(Kind::Preview, &rid(1)).case_id(), None);
    }

    #[test]
    fn parse_ignores_query_and_fragment() {
        let p = BlobPath::record(Kind::Preview, &rid(1));
        assert_eq!(BlobPath::parse(&format!("{}?x=1", p.url())), Some(p));
        assert_eq!(BlobPath::parse(&format!("{}#k=abc", p.url())), Some(p));
    }

    #[test]
    fn parse_rejects_malformed() {
        let good = BlobPath::record(Kind::Preview, &rid(1)).url();
        let lower = good.to_lowercase();
        let case = BlobPath::case(&cid(0), 5).url();
        for bad in [
            "",
            "/",
            "/p",
            "/p/",
            "p/ABC",
            "/x/0000000000000000",
            "/p/short",
            &format!("{good}/"),
            &format!("/{}", &good[1..].replace("p/", "p//")),
            &lower,
            &case.replace(".5", ".05"),
            &case.replace(".5", "."),
            &case.replace(".5", ".-1"),
            &case.replace(".5", ".99999999999"),
            &case.replace(".5", ""),
            "/c/0000.1",
        ] {
            assert_eq!(BlobPath::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn from_target_maps_every_kind() {
        let r = rid(9).to_string();
        let cases = [
            (UploadTarget::Preview(r.clone()), Kind::Preview),
            (UploadTarget::Og(r.clone()), Kind::Og),
            (UploadTarget::Full(r.clone()), Kind::Full),
        ];
        for (t, k) in cases {
            assert_eq!(BlobPath::from_target(&t), Ok(BlobPath::record(k, &rid(9))));
        }
        let t = UploadTarget::CaseChunk {
            case_id: cid(1).to_string(),
            ix: 2,
            count: 3,
        };
        assert_eq!(BlobPath::from_target(&t), Ok(BlobPath::case(&cid(1), 2)));
    }

    #[test]
    fn from_target_rejects_bad_ids() {
        assert_eq!(
            BlobPath::from_target(&UploadTarget::Preview("nope".into())),
            Err(ContractError::BadId)
        );
        let t = UploadTarget::CaseChunk {
            case_id: cid(1).to_string(),
            ix: 3,
            count: 3,
        };
        assert_eq!(BlobPath::from_target(&t), Err(ContractError::Invalid));
    }

    #[test]
    fn keys_order_like_labels_within_kind() {
        let a = BlobPath::record(Kind::Preview, &rid(1));
        let b = BlobPath::record(Kind::Preview, &rid(2));
        assert!(a.key() < b.key());
        assert!(a.label() < b.label());
        let c = BlobPath::record(Kind::Og, &rid(1));
        assert_ne!(a.key(), c.key());
    }

    #[test]
    fn from_key_rejects_unknown_kind() {
        let mut k = BlobPath::record(Kind::Preview, &rid(1)).key();
        k[0] = 9;
        assert_eq!(BlobPath::from_key(&k), None);
    }

    #[test]
    fn labels_never_contain_nul() {
        let p = BlobPath::case(&cid(0), 0);
        assert!(!p.label().contains(&0));
        let p = BlobPath::record(Kind::Full, &RecordId::from_bytes([0; 10]));
        assert!(!p.label().contains(&0));
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            BlobPath::case(&cid(3), 7).url(),
            BlobPath::case(&cid(3), 7).url()
        );
    }
}
