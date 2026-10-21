// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-134: the registry carries each backend's fairness kinds to its
//! candidates and leaves out a backend lacking a kind the item uses.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_foundation::CatalogCode;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, CandidateSet,
    FairnessKindsDefect, FairnessKindsMember, FairnessLabel, ManifestDigest, Mode, ProviderOrigin,
    RegistrationCause, Registry,
};
use qsl_semantics::check::Capability;
use quire_semantic_value::fairness::FairnessKind;
use quire_semantic_value::fairness::FairnessKind::{Strong, Weak};

fn candidate(id: &str) -> Candidate {
    Candidate::new(BackendId::new(id))
}

/// A temporal backend advertising (`temporal-satisfaction`, `unbounded`)
/// and the given fairness kinds.
fn temporal(id: &str, seed: u8, kinds: &[FairnessKind]) -> BackendDescriptor {
    BackendDescriptor::new(
        candidate(id),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([seed; 32]),
        [(Capability::TemporalSatisfaction, Mode::Unbounded)],
        None,
    )
    .expect("a well-formed descriptor")
    .with_fairness_kinds(kinds.iter().copied())
}

fn registry(backends: impl IntoIterator<Item = BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    for backend in backends {
        registry.register(backend).expect("registration admitted");
    }
    registry
}

fn set_for(registry: &Registry, required: &[FairnessKind]) -> CandidateSet {
    let required: BTreeSet<_> = required.iter().copied().collect();
    match registry.candidates_for_fairness(Capability::TemporalSatisfaction, None, &required) {
        CandidateOutcome::Candidates(set) => set,
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}

/// FR-134-AC-1: a weak constraint keeps both backends, each with its own
/// kinds; a strong constraint keeps only the backend advertising `Strong`.
#[test]
#[trace("TC-535", "FR-134-AC-1")]
fn a_backend_without_the_requested_kind_is_not_a_candidate() {
    let registry = registry([
        temporal("EN-1", 1, &[Weak, Strong]),
        temporal("weak-only", 2, &[Weak]),
    ]);

    let weak = set_for(&registry, &[Weak]);
    assert_eq!(
        weak.candidates(),
        [candidate("EN-1"), candidate("weak-only")]
    );
    assert_eq!(
        weak.fairness_kinds(&candidate("EN-1")),
        Some(&BTreeSet::from([Weak, Strong]))
    );
    assert_eq!(
        weak.fairness_kinds(&candidate("weak-only")),
        Some(&BTreeSet::from([Weak]))
    );

    let strong = set_for(&registry, &[Strong]);
    assert_eq!(strong.candidates(), [candidate("EN-1")]);
    assert_eq!(strong.fairness_kinds(&candidate("weak-only")), None);
}

/// FR-134-AC-1 (filter is by every kind the item uses): an item using both
/// kinds needs a backend advertising both.
#[test]
#[trace("TC-535", "FR-134-AC-1")]
fn every_kind_in_the_fairness_set_must_be_advertised() {
    let registry = registry([
        temporal("strong-only", 1, &[Strong]),
        temporal("both", 2, &[Weak, Strong]),
    ]);
    assert_eq!(
        set_for(&registry, &[Weak, Strong]).candidates(),
        [candidate("both")]
    );
}

/// FR-134 Behavior: an item that uses no fairness kind is filtered by no
/// kind, so a backend advertising none stays a candidate.
#[test]
fn an_item_without_fairness_keeps_every_capable_backend() {
    let registry = registry([
        temporal("plain", 1, &[]),
        temporal("EN-1", 2, &[Weak, Strong]),
    ]);
    let set = set_for(&registry, &[]);
    assert_eq!(set.candidates(), [candidate("EN-1"), candidate("plain")]);
    assert!(set.missing_fairness_kinds().is_empty());
}

/// FR-134-AC-2, registry half only (not tagged to the AC, which also
/// requires the `Unsupported` settlement that `negotiate_*` owns): with only the weak backend, a strong item
/// has an empty set that names the missing `Strong` kind. Settling it
/// `unsupported` is `negotiate_*`'s work, not the registry's.
#[test]
#[trace("TC-535")]
fn an_emptied_set_names_the_missing_fairness_kind() {
    let registry = registry([temporal("weak-only", 1, &[Weak])]);
    let set = set_for(&registry, &[Strong]);
    assert!(set.is_empty());
    assert_eq!(set.kind(), Capability::TemporalSatisfaction);
    assert_eq!(set.missing_fairness_kinds(), &BTreeSet::from([Strong]));
}

/// FR-076: an empty set from backend absence names no fairness kind, so it
/// is told apart from one the filter emptied.
#[test]
fn backend_absence_names_no_missing_fairness_kind() {
    let set = set_for(&Registry::new(), &[Strong]);
    assert!(set.is_empty());
    assert!(set.missing_fairness_kinds().is_empty());
}

/// Two registrations of one identity that differ only in fairness kinds
/// conflict like any other differing member.
#[test]
fn registrations_differing_in_fairness_kinds_conflict() {
    let mut registry = Registry::new();
    registry
        .register(temporal("EN-1", 1, &[Weak]))
        .expect("first admitted");
    assert!(registry
        .register(temporal("EN-1", 1, &[Weak, Strong]))
        .is_err());
    assert_eq!(registry.backends().count(), 0);
}

use FairnessLabel::{Other, Text};

type Admission = Result<BackendDescriptor, qsl_route::RegistrationRefusal>;

/// A manifest for `id` advertising `pairs` over `domains`, with the
/// `fairness_kinds` member as supplied (`None` = absent).
fn admit(
    id: &str,
    pairs: &[(&str, &str)],
    domains: Option<&[&str]>,
    member: Option<FairnessKindsMember<'_>>,
) -> Admission {
    descriptor_from_manifest(
        candidate(id),
        ProviderOrigin::Process,
        ManifestDigest::from_digest([9; 32]),
        pairs.iter().map(|&(kind, mode)| (Some(kind), Some(mode))),
        domains,
        member,
    )
}

const TEMPORAL: &[(&str, &str)] = &[("temporal-satisfaction", "unbounded")];

/// A temporal manifest with the `fairness_kinds` list `labels`.
fn listed(labels: &[FairnessLabel<'_>]) -> Admission {
    admit("b", TEMPORAL, None, Some(FairnessKindsMember::List(labels)))
}

fn manifest(id: &str, seed: u8, labels: &[FairnessLabel<'_>]) -> BackendDescriptor {
    descriptor_from_manifest(
        candidate(id),
        ProviderOrigin::Process,
        ManifestDigest::from_digest([seed; 32]),
        [(Some("temporal-satisfaction"), Some("unbounded"))],
        None,
        Some(FairnessKindsMember::List(labels)),
    )
    .expect("a valid manifest")
}

/// The refusal's catalog code.
fn refused_code(admission: Admission) -> CatalogCode {
    let refusal = admission.expect_err("the manifest is refused");
    assert_eq!(refusal.identity(), &BackendId::new("b"));
    refusal.catalog_code()
}

/// The defect of a refusal that must be `invalid_capability`/
/// `invalid-fairness-kinds`.
fn refused(admission: Admission) -> FairnessKindsDefect {
    let refusal = admission.expect_err("the manifest is refused");
    assert_eq!(refusal.identity(), &BackendId::new("b"));
    assert_eq!(refusal.catalog_code(), code("invalid-fairness-kinds"));
    match refusal.cause() {
        RegistrationCause::InvalidFairnessKinds(defect) => defect.clone(),
        other => panic!("not a fairness refusal: {other:?}"),
    }
}

fn code(cause: &'static str) -> CatalogCode {
    CatalogCode::new("invalid_capability", cause)
}

/// TC-535 V1 to V3: admitted manifests advertise exactly their kinds, and a
/// present empty list is a candidate only for an item with no fairness.
#[test]
#[trace("TC-535", "FR-134-AC-5")]
fn v1_to_v3_admitted_lists_advertise_their_kinds() {
    assert_eq!(
        listed(&[Text("weak")]).unwrap().fairness_kinds(),
        &BTreeSet::from([Weak])
    );
    assert_eq!(
        listed(&[Text("weak"), Text("strong")])
            .unwrap()
            .fairness_kinds(),
        &BTreeSet::from([Weak, Strong])
    );
    let registry = registry([listed(&[]).unwrap()]);
    assert!(listed(&[]).unwrap().fairness_kinds().is_empty());
    assert!(set_for(&registry, &[Weak]).is_empty());
    assert!(set_for(&registry, &[Strong]).is_empty());
    assert_eq!(set_for(&registry, &[]).candidates().len(), 1);
}

/// TC-535 V4 to V6, V11, V14: an element not byte-equal to `weak` or `strong`
/// is refused `invalid-fairness-kinds`/`unknown-label`, naming the first such element.
#[test]
#[trace("TC-535", "FR-134-AC-3")]
fn v4_v5_v6_v11_v14_an_unknown_element_is_refused_naming_the_first() {
    for (labels, named) in [
        (vec![Text("Weak")], "Weak"),
        (vec![Text("weak"), Text("eventual")], "eventual"),
        (vec![Text("")], ""),
        (vec![Text("weak"), Other("1")], "1"),
        (vec![Text("x"), Text("y")], "x"),
    ] {
        assert_eq!(
            refused(listed(&labels)),
            FairnessKindsDefect::UnknownLabel(named.to_owned())
        );
    }
}

/// TC-535 V12 and V13: a member that is not a list is refused
/// `invalid-fairness-kinds`/`not-a-list`, naming the member as supplied.
#[test]
#[trace("TC-535", "FR-134-AC-3")]
fn v12_v13_a_member_that_is_not_a_list_is_refused() {
    for value in ["null", "\"weak\"", "{}"] {
        let admission = admit(
            "b",
            TEMPORAL,
            None,
            Some(FairnessKindsMember::NotAList(value)),
        );
        assert_eq!(
            refused(admission),
            FairnessKindsDefect::NotAList(value.to_owned())
        );
    }
}

/// TC-535 V7, V8, V10, V15: a repeat is refused `invalid-fairness-kinds`/`duplicate-label`
/// naming the first element that repeats an earlier one, and is never
/// collapsed; an unknown element wins over a repeat.
#[test]
#[trace("TC-535", "FR-134-AC-4")]
fn v7_v8_v10_v15_a_repeat_is_refused_and_an_unknown_element_wins() {
    for labels in [
        vec![Text("weak"), Text("weak")],
        vec![Text("weak"), Text("strong"), Text("weak")],
        vec![Text("strong"), Text("weak"), Text("weak"), Text("strong")],
    ] {
        assert_eq!(
            refused(listed(&labels)),
            FairnessKindsDefect::DuplicateLabel("weak".to_owned())
        );
    }
    assert_eq!(
        refused(listed(&[Text("weak"), Text("weak"), Text("eventual")])),
        FairnessKindsDefect::UnknownLabel("eventual".to_owned())
    );
}

/// TC-535 V9: a temporal manifest with no `fairness_kinds` is refused
/// `invalid-fairness-kinds`/`absent`.
#[test]
#[trace("TC-535", "FR-134-AC-5")]
fn v9_an_absent_member_of_a_temporal_backend_is_refused() {
    assert_eq!(
        refused(admit("b", TEMPORAL, None, None)),
        FairnessKindsDefect::Absent
    );
}

/// TC-535 V16 and V17: a pair or `domains` defect is judged first.
#[test]
#[trace("TC-535", "FR-134-AC-6")]
fn v16_v17_a_pair_or_domains_defect_wins_over_a_fairness_defect() {
    let both = [("temporal-satisfaction", "unbounded"), ("Foo", "unbounded")];
    let bad_member = Some(FairnessKindsMember::List(&[Text("Weak")]));
    assert_eq!(
        refused_code(admit("b", &both, None, bad_member)),
        code("unknown-kind")
    );
    assert_eq!(
        refused_code(admit(
            "b",
            &[("temporal-satisfaction", "bounded")],
            None,
            None
        )),
        code("invalid-domains")
    );
}

/// TC-535 step 4: a refused manifest under an identity that already holds a
/// registration leaves that registration and its candidates standing.
#[test]
#[trace("TC-535", "FR-134-AC-5")]
fn a_refusal_leaves_the_earlier_registration_standing() {
    let registry = registry([manifest("b", 1, &[Text("weak"), Text("strong")])]);
    for member in [
        Some(FairnessKindsMember::List(&[Text("Weak")])),
        Some(FairnessKindsMember::List(&[Text("weak"), Text("weak")])),
        None,
    ] {
        assert!(admit("b", TEMPORAL, None, member).is_err());
        assert_eq!(set_for(&registry, &[Strong]).candidates(), [candidate("b")]);
    }
}

/// TC-535 step 5: a backend with no temporal pair is admitted whatever its
/// `fairness_kinds` member, which is not validated and keeps no kind.
#[test]
#[trace("TC-535", "FR-134-AC-5")]
fn a_non_temporal_backend_ignores_the_fairness_member() {
    let pairs = [("local-projection", "unbounded")];
    for member in [
        None,
        Some(FairnessKindsMember::List(&[Text("Weak")])),
        Some(FairnessKindsMember::NotAList("null")),
    ] {
        let descriptor = admit("b", &pairs, None, member).expect("admitted");
        assert!(descriptor.fairness_kinds().is_empty());
    }
}

/// QSpec FR-368 through the registry: manifest-registered backends are
/// filtered by their `fairness_kinds`.
#[test]
#[trace("TC-535")]
fn manifest_fairness_kinds_filter_candidates() {
    let registry = registry([
        manifest("both", 1, &[Text("weak"), Text("strong")]),
        manifest("weak-only", 2, &[Text("weak")]),
    ]);
    assert_eq!(
        set_for(&registry, &[Strong]).candidates(),
        [candidate("both")]
    );
    assert_eq!(
        set_for(&registry, &[Weak]).candidates(),
        [candidate("both"), candidate("weak-only")]
    );
}
