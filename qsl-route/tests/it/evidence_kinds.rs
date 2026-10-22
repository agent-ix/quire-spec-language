// SPDX-License-Identifier: AGPL-3.0-or-later
//! The probabilistic evidence-kind set on a descriptor (FR-075-AC-10 to
//! AC-14): admission, candidate matching that never substitutes one kind
//! for the other, and the duplicate-registration conflict an evidence
//! difference causes. Also the twelve-kind registration (FR-057).

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, EvidenceKind,
    EvidenceKindsDefect, FairnessKindsMember, ItemEvidence, ManifestDigest, Mode, ProviderOrigin,
    RegistrationCause, RegistrationRefusal, Registry,
};
use qsl_semantics::check::Capability;
use std::collections::BTreeSet;

const PROBABILISTIC: &str = "probabilistic-satisfaction";

fn admit(
    id: &str,
    digest_byte: u8,
    kind: &str,
    evidence: Option<&[&str]>,
) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(BackendId::new(id)),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([digest_byte; 32]),
        [(Some(kind), Some("unbounded"))],
        None,
        // A temporal backend must state its (possibly empty) fairness kinds.
        Some(FairnessKindsMember::List(&[])),
        evidence,
    )
}

fn probabilistic(id: &str, evidence: &[&str]) -> BackendDescriptor {
    admit(id, 1, PROBABILISTIC, Some(evidence)).expect("a well-formed probabilistic descriptor")
}

fn ids(outcome: CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("expected a computed candidate set, got {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

fn item(registry: &Registry, evidence: &ItemEvidence, named: Option<&BackendId>) -> Vec<String> {
    ids(registry.candidates_for_evidence(Capability::ProbabilisticSatisfaction, evidence, named))
}

fn kinds(labels: &[EvidenceKind]) -> BTreeSet<EvidenceKind> {
    labels.iter().copied().collect()
}

/// FR-075-AC-10: `{statistical}`, `{exact}` and both are admitted and the
/// descriptor retains exactly the supplied set.
#[test]
#[trace("FR-075-AC-10")]
fn each_admitted_evidence_set_is_retained_exactly() {
    let cases: [(&[&str], &[EvidenceKind]); 3] = [
        (&["statistical"], &[EvidenceKind::Statistical]),
        (&["exact"], &[EvidenceKind::Exact]),
        (
            &["statistical", "exact"],
            &[EvidenceKind::Statistical, EvidenceKind::Exact],
        ),
    ];
    for (labels, expected) in cases {
        let from_manifest = probabilistic("p", labels);
        assert_eq!(
            from_manifest.evidence_kinds(),
            &kinds(expected),
            "{labels:?}"
        );
        let typed = BackendDescriptor::new(
            Candidate::new(BackendId::new("p")),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest([1; 32]),
            [(Capability::ProbabilisticSatisfaction, Mode::Unbounded)],
            None,
            Some(kinds(expected)),
        )
        .expect("the typed constructor admits the same set");
        assert_eq!(typed, from_manifest, "{labels:?}");
    }
}

/// FR-075-AC-11: an absent set, an empty set, or a member other than the
/// exact labels is refused `invalid_capability`/`unknown-evidence-kind`,
/// keyed by the backend identity; the refusal names the first unknown label.
#[test]
#[trace("FR-075-AC-11")]
fn a_defective_evidence_set_is_refused_keyed_by_backend() {
    let cases: [(Option<&[&str]>, EvidenceKindsDefect); 7] = [
        (None, EvidenceKindsDefect::Absent),
        (Some(&[]), EvidenceKindsDefect::Empty),
        (
            Some(&["Statistical"]),
            EvidenceKindsDefect::Unknown("Statistical".into()),
        ),
        (Some(&[""]), EvidenceKindsDefect::Unknown(String::new())),
        (
            Some(&["exact "]),
            EvidenceKindsDefect::Unknown("exact ".into()),
        ),
        (
            Some(&["statistical", "approximate", "other"]),
            EvidenceKindsDefect::Unknown("approximate".into()),
        ),
        (
            Some(&["exact", "sampled"]),
            EvidenceKindsDefect::Unknown("sampled".into()),
        ),
    ];
    for (labels, defect) in cases {
        let refusal = admit("p", 1, PROBABILISTIC, labels).expect_err("must refuse");
        assert_eq!(refusal.identity().as_str(), "p", "{labels:?}");
        assert_eq!(
            refusal.cause(),
            &RegistrationCause::UnknownEvidenceKind(defect),
            "{labels:?}"
        );
        assert_eq!(
            refusal.catalog_code().to_string(),
            "invalid_capability/unknown-evidence-kind"
        );
    }
    let typed = BackendDescriptor::new(
        Candidate::new(BackendId::new("p")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([1; 32]),
        [(Capability::ProbabilisticSatisfaction, Mode::Unbounded)],
        None,
        Some(BTreeSet::new()),
    )
    .expect_err("an empty typed set refuses");
    assert_eq!(
        typed.cause(),
        &RegistrationCause::UnknownEvidenceKind(EvidenceKindsDefect::Empty)
    );
}

/// FR-075-AC-11: a refusal happens before registry insertion, so an earlier
/// registration under the identity stands.
#[test]
#[trace("FR-075-AC-11")]
fn a_refused_evidence_set_leaves_the_earlier_registration_standing() {
    let mut registry = Registry::new();
    registry
        .register(probabilistic("p", &["statistical"]))
        .unwrap();
    let refused = admit("p", 1, PROBABILISTIC, Some(&["bogus"]));
    assert!(refused.is_err());
    // The driver registers only an admitted descriptor, so the refused one
    // never reaches the registry.
    let statistical = ItemEvidence::Kind(EvidenceKind::Statistical);
    assert_eq!(item(&registry, &statistical, None), ["p"]);
    assert_eq!(registry.refusals().count(), 0);
}

/// FR-075-AC-12: a backend advertising no probabilistic kind is admitted
/// without evidence labels, whichever of the other eleven kinds it
/// advertises.
#[test]
#[trace("FR-075-AC-12")]
fn a_non_probabilistic_descriptor_needs_no_evidence_kinds() {
    let mut others = 0;
    for &kind in Capability::ALL {
        if kind == Capability::ProbabilisticSatisfaction {
            continue;
        }
        others += 1;
        let admitted = admit("p", 1, kind.to_wire(), None)
            .unwrap_or_else(|refusal| panic!("{kind:?} refused: {refusal}"));
        assert!(admitted.evidence_kinds().is_empty(), "{kind:?}");
    }
    assert_eq!(others, 11);
}

/// FR-075-AC-13: a probabilistic item's candidate set is exactly the
/// registrations advertising its evidence kind; neither kind substitutes for
/// the other, including when it is the only registration.
#[test]
#[trace("TC-980", "TC-983", "FR-075-AC-1", "FR-075-AC-13")]
fn candidates_match_the_exact_evidence_kind() {
    let statistical = ItemEvidence::Kind(EvidenceKind::Statistical);
    let exact = ItemEvidence::Kind(EvidenceKind::Exact);

    let mut both = Registry::new();
    both.register(probabilistic("stat", &["statistical"]))
        .unwrap();
    both.register(probabilistic("ex", &["exact"])).unwrap();
    assert_eq!(item(&both, &statistical, None), ["stat"]);
    assert_eq!(item(&both, &exact, None), ["ex"]);

    let mut only_stat = Registry::new();
    only_stat
        .register(probabilistic("stat", &["statistical"]))
        .unwrap();
    assert!(item(&only_stat, &exact, None).is_empty());

    let mut only_ex = Registry::new();
    only_ex.register(probabilistic("ex", &["exact"])).unwrap();
    assert!(item(&only_ex, &statistical, None).is_empty());

    // A named backend with the nonmatching kind is an empty set, not the
    // unknown-backend marker; an unregistered name stays the marker.
    assert!(item(&both, &exact, Some(&BackendId::new("stat"))).is_empty());
    assert_eq!(item(&both, &exact, Some(&BackendId::new("ex"))), ["ex"]);
    assert_eq!(
        both.candidates_for_evidence(
            Capability::ProbabilisticSatisfaction,
            &exact,
            Some(&BackendId::new("zzz"))
        ),
        CandidateOutcome::UnknownBackend(BackendId::new("zzz"))
    );

    // A backend advertising both kinds serves either item.
    let mut either = Registry::new();
    either
        .register(probabilistic("both", &["statistical", "exact"]))
        .unwrap();
    assert_eq!(item(&either, &statistical, None), ["both"]);
    assert_eq!(item(&either, &exact, None), ["both"]);

    // A backend that does not advertise the kind is no candidate whatever
    // its evidence.
    let mut other_kind = Registry::new();
    other_kind
        .register(admit("v", 2, "value-validity", None).unwrap())
        .unwrap();
    assert!(item(&other_kind, &statistical, None).is_empty());
}

/// FR-075-AC-13: an absent or unknown item evidence yields no candidate and
/// is preserved on the set for negotiation to refuse.
#[test]
#[trace("FR-075-AC-13")]
fn missing_or_unknown_item_evidence_is_preserved_not_substituted() {
    let mut registry = Registry::new();
    registry
        .register(probabilistic("both", &["statistical", "exact"]))
        .unwrap();
    for evidence in [
        ItemEvidence::from_label(None),
        ItemEvidence::from_label(Some("Statistical")),
        ItemEvidence::from_label(Some("")),
    ] {
        let CandidateOutcome::Candidates(set) = registry.candidates_for_evidence(
            Capability::ProbabilisticSatisfaction,
            &evidence,
            None,
        ) else {
            panic!("expected a computed set");
        };
        assert!(set.is_empty(), "{evidence:?}");
        assert_eq!(set.evidence(), Some(&evidence));
    }
    assert_eq!(
        ItemEvidence::from_label(Some("Statistical")),
        ItemEvidence::Unknown("Statistical".to_owned())
    );
    // `candidates` carries no evidence, so a probabilistic item through it is
    // an absent-evidence item and matches nobody.
    assert!(ids(registry.candidates(Capability::ProbabilisticSatisfaction, None)).is_empty());
    // An item of another kind carries no evidence value.
    let CandidateOutcome::Candidates(other) = registry.candidates(Capability::ValueValidity, None)
    else {
        panic!("expected a computed set");
    };
    assert_eq!(other.evidence(), None);
}

/// FR-075-AC-14: descriptors identical but for their evidence sets conflict:
/// the identity is refused `duplicate-backend`, withdrawn, and has no
/// candidate; equal descriptors repeat harmlessly.
#[test]
#[trace("TC-981", "TC-983", "FR-075-AC-14")]
fn an_evidence_difference_is_a_duplicate_backend_conflict() {
    let statistical = ItemEvidence::Kind(EvidenceKind::Statistical);
    let exact = ItemEvidence::Kind(EvidenceKind::Exact);

    let mut repeat = Registry::new();
    repeat
        .register(probabilistic("p", &["statistical"]))
        .unwrap();
    repeat
        .register(probabilistic("p", &["statistical"]))
        .unwrap();
    assert_eq!(item(&repeat, &statistical, None), ["p"]);

    for reversed in [false, true] {
        let a = probabilistic("p", &["statistical"]);
        let b = probabilistic("p", &["exact"]);
        let (first, second) = if reversed { (b, a) } else { (a, b) };
        let mut registry = Registry::new();
        registry.register(first).unwrap();
        let refusals = registry.register(second).expect_err("conflict");
        assert!(refusals
            .iter()
            .all(|r| r.catalog_code().to_string() == "invalid_capability/duplicate-backend"));
        assert!(
            registry.descriptors().next().is_none(),
            "reversed={reversed}"
        );
        assert!(item(&registry, &statistical, None).is_empty());
        assert!(item(&registry, &exact, None).is_empty());
        assert_eq!(
            registry.candidates_for_evidence(
                Capability::ProbabilisticSatisfaction,
                &exact,
                Some(&BackendId::new("p"))
            ),
            CandidateOutcome::UnknownBackend(BackendId::new("p"))
        );
    }
}

/// FR-057: the two kinds added to the vocabulary register and are
/// candidates like any other kind.
#[test]
#[trace("TC-155", "FR-057-AC-8")]
fn every_one_of_the_twelve_kinds_registers_and_routes() {
    let mut registry = Registry::new();
    for (index, &kind) in Capability::ALL.iter().enumerate() {
        let evidence: Option<&[&str]> =
            (kind == Capability::ProbabilisticSatisfaction).then_some(&["statistical"][..]);
        let id = format!("backend-{index}");
        registry
            .register(admit(&id, 1, kind.to_wire(), evidence).expect("admitted"))
            .unwrap();
    }
    assert_eq!(Capability::ALL.len(), 12);
    for (index, &kind) in Capability::ALL.iter().enumerate() {
        let got = ids(registry.candidates_for_evidence(
            kind,
            &ItemEvidence::Kind(EvidenceKind::Statistical),
            None,
        ));
        assert_eq!(got, [format!("backend-{index}")], "{kind:?}");
    }
}
