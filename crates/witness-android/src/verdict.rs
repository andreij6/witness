use crate::claim::ClaimView;
use witness_core::Verdict;
use witness_core::format::{Edit, Outcome, RecordId, VerdictFlags};
use witness_core::verdict::{Anchor, Evidence, Reason, Reference, Region};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum VerdictFlag {
    KeyRevokedAfterAnchor,
    LateAnchor,
    ClockMismatch,
    CouldntCompare,
    NoPublicReference,
    DevAttestation,
    TruncatedCandidates,
}

const FLAGS: [(VerdictFlags, VerdictFlag); 7] = [
    (
        VerdictFlags::KEY_REVOKED_AFTER_ANCHOR,
        VerdictFlag::KeyRevokedAfterAnchor,
    ),
    (VerdictFlags::LATE_ANCHOR, VerdictFlag::LateAnchor),
    (VerdictFlags::CLOCK_MISMATCH, VerdictFlag::ClockMismatch),
    (VerdictFlags::COULDNT_COMPARE, VerdictFlag::CouldntCompare),
    (
        VerdictFlags::NO_PUBLIC_REFERENCE,
        VerdictFlag::NoPublicReference,
    ),
    (VerdictFlags::DEV_ATTESTATION, VerdictFlag::DevAttestation),
    (
        VerdictFlags::TRUNCATED_CANDIDATES,
        VerdictFlag::TruncatedCandidates,
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReferenceView {
    pub record_id: RecordId,
    pub claim: ClaimView,
    pub anchor: Option<Anchor>,
    pub evidence: Evidence,
    pub confidence_q16: u32,
    pub compared_q16: u32,
    pub changed_q16: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerdictView {
    pub version: u16,
    pub outcome: Outcome,
    pub flags: Vec<VerdictFlag>,
    pub reasons: Vec<Reason>,
    pub declared: Vec<Edit>,
    pub regions: Vec<Region>,
    pub reference: Option<ReferenceView>,
    #[uniffi(default = false)]
    pub dev_anchor: bool,
    #[uniffi(default = true)]
    pub credentials_judged: bool,
    #[uniffi(default = false)]
    pub proofs_rejected: bool,
}

pub(crate) fn flag_list(flags: VerdictFlags) -> Vec<VerdictFlag> {
    FLAGS
        .iter()
        .filter(|(bit, _)| flags.contains(*bit))
        .map(|(_, f)| *f)
        .collect()
}

impl From<Reference> for ReferenceView {
    fn from(r: Reference) -> Self {
        Self {
            record_id: r.record_id,
            claim: ClaimView::from(&r.claim),
            anchor: r.anchor,
            evidence: r.evidence,
            confidence_q16: r.confidence_q16,
            compared_q16: r.compared_q16,
            changed_q16: r.changed_q16,
        }
    }
}

impl From<Verdict> for VerdictView {
    fn from(v: Verdict) -> Self {
        Self {
            version: v.version,
            outcome: v.outcome,
            flags: flag_list(v.flags),
            reasons: v.reasons.iter().collect(),
            declared: v.declared,
            regions: v.regions,
            reference: v.reference.map(ReferenceView::from),
            dev_anchor: false,
            credentials_judged: true,
            proofs_rejected: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_core::verdict::{Q16_ONE, Reasons, VERDICT_V};

    #[test]
    fn every_flag_bit_maps_once_in_order() {
        let all = VerdictFlags::from_bits(0x7f).unwrap();
        assert_eq!(flag_list(all), FLAGS.map(|(_, f)| f).to_vec());
        assert!(flag_list(VerdictFlags::EMPTY).is_empty());
        for (bit, f) in FLAGS {
            assert_eq!(flag_list(bit), vec![f]);
        }
    }

    #[test]
    fn verdict_view_keeps_outcome_reasons_and_regions() {
        let reasons: Reasons = [Reason::Unconfirmed, Reason::SealVerified]
            .into_iter()
            .collect();
        let v = Verdict {
            version: VERDICT_V,
            outcome: Outcome::Altered,
            flags: VerdictFlags::COULDNT_COMPARE | VerdictFlags::LATE_ANCHOR,
            reasons,
            declared: vec![Edit::Orient { orientation: 2 }],
            regions: vec![Region {
                x: 1,
                y: 2,
                w: 3,
                h: 4,
            }],
            reference: None,
        };
        let view = VerdictView::from(v);
        assert_eq!(view.version, VERDICT_V);
        assert_eq!(view.outcome, Outcome::Altered);
        assert_eq!(
            view.flags,
            vec![VerdictFlag::LateAnchor, VerdictFlag::CouldntCompare]
        );
        assert_eq!(
            view.reasons,
            vec![Reason::SealVerified, Reason::Unconfirmed]
        );
        assert_eq!(view.declared, vec![Edit::Orient { orientation: 2 }]);
        assert_eq!(
            view.regions,
            vec![Region {
                x: 1,
                y: 2,
                w: 3,
                h: 4
            }]
        );
        assert!(view.reference.is_none());
    }

    #[test]
    fn reference_view_converts_claim() {
        let (_, _, sealed) = crate::seal::tests::sealed_original(21);
        let claim =
            witness_core::format::WitnessClaim::from_cbor(&sealed.submission.claim).unwrap();
        let r = Reference {
            record_id: sealed.record_id,
            claim,
            anchor: Some(Anchor {
                record_ix: 4,
                anchored_ns: 9,
            }),
            evidence: Evidence::Pixels,
            confidence_q16: Q16_ONE,
            compared_q16: 1,
            changed_q16: 2,
        };
        let v = ReferenceView::from(r);
        assert_eq!(v.claim, sealed.claim);
        assert_eq!(v.anchor.unwrap().record_ix, 4);
        assert_eq!(v.evidence, Evidence::Pixels);
        assert_eq!((v.compared_q16, v.changed_q16), (1, 2));
    }
}
