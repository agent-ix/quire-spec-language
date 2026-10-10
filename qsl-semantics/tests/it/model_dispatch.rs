// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196: dispatch linking (FR-151), the static/link-time subset (D01-D05).
//!
//! D06-D08 need `dispatch.select`/precondition evaluation or the FR-146
//! call graph, both out of scope for `crate::model::dispatch` (see its
//! module docs) — this file does not claim coverage of them.
//!
//! Fixtures follow TC-195/196's own convention: G is F2-shaped (`model.A`,
//! `model.B`, `model.C`, `model.D`; `B`,`C` -> `A`; `D` -> `B`, `D` -> `C`),
//! using this file's own synthetic node identities rather than TC-195's own
//! `test/orders` ground-truth nodes (`tests/model_normalization.rs` owns
//! the exact ground-truth digests; this file only needs G's structure).

use ix_trace_rs::trace;
use qsl_forms::Expression;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    DispatchBridgeRefusal, DispatchRoot, OperationClauses,
};
use qsl_semantics::model::accounting::{ChargePoint, LimitKind, Meter, ModelNormalizationLimits};
use qsl_semantics::model::dispatch::{
    link_dispatch, DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, ObjectTypeRecord,
    OperationEffect, OperationMemberRecord, OperationResult, ValueTypeRef,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize, ModelRefusalCause, NormalizeOutcome};
use quire_exact::ValueType;

fn object_type(identity: &str, supertypes: Vec<&str>) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes
            .into_iter()
            .map(DeclarationKey::fixture)
            .collect(),
    })
}

/// A parameterless, resultless `size`-shaped operation with an empty effect
/// frame, exactly as much signature as dispatch linking itself inspects.
/// `redefines` is `Some(target)` when this operation declares
/// `model-complete.md:162`'s inline `redefines` property (QSpec's own
/// shape, not a separate redefinition record).
fn operation(
    identity: &str,
    owner: &str,
    has_body: bool,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        parameters: Vec::new(),
        result: None,
        effect: OperationEffect::default(),
        has_body,
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

/// G: F2's types/generalizations (`model.A`, `model.B` <= `A`, `model.C` <=
/// `A`, `model.D` <= `B`, `D` <= `C`).
fn fixture_g() -> Vec<DomainPackageRecord> {
    vec![
        object_type("model.A", vec![]),
        object_type("model.B", vec!["model.A"]),
        object_type("model.C", vec!["model.A"]),
        object_type("model.D", vec!["model.B", "model.C"]),
    ]
}

fn effective_view(
    domain_package: &DomainPackage,
) -> qsl_semantics::model::normalize::EffectiveView {
    match normalize(domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn unlimited_meter() -> qsl_semantics::model::accounting::Meter {
    qsl_semantics::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED)
}

/// D01: `A.size`/`B.size` (`B.size` redefines `A.size`) each have a body.
/// Every subtype links: `D -> B.size`, `C -> A.size`, `B -> B.size`, `A ->
/// A.size`, with one `dispatch.subtype` and one `dispatch.candidate` per
/// (subtype, candidate) pair, and `dispatch.dominance` only where `D`/`B`
/// have two applicable candidates.
// Neither FR-151-AC-1 nor FR-151-AC-3 tags this test (retagged, PR #144
// review finding #1): AC-1 is about an effective member's provenance, which
// this happy-path linking test never inspects; AC-3 is entirely about
// refusals, and this test has none.
#[trace("QSpec-TC-196")]
#[test]
fn d01_a_closed_diamond_links_every_subtype_to_its_unique_undominated_candidate() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );

    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = outcome else {
        panic!("expected a linked dispatch table, got {outcome:?}");
    };
    let linked = |subtype: &str| {
        table
            .linked_for(&DeclarationKey::fixture(subtype))
            .map(|c| c.node.clone())
    };
    assert_eq!(linked("model.D"), Some("model.B.size".to_owned()));
    assert_eq!(linked("model.C"), Some("model.A.size".to_owned()));
    assert_eq!(linked("model.B"), Some("model.B.size".to_owned()));
    assert_eq!(linked("model.A"), Some("model.A.size".to_owned()));

    // Eight dispatch.candidate charges (2 candidates x 4 subtypes), one
    // dispatch.subtype per subtype, and dispatch.dominance only for D and B
    // (the two subtypes with more than one applicable candidate).
    let admitted = meter.admitted_charges();
    let candidate_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchCandidate)
        .count();
    let subtype_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchSubtype)
        .count();
    let dominance_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchDominance)
        .count();
    assert_eq!(candidate_count, 8);
    assert_eq!(subtype_count, 4);
    assert_eq!(dominance_count, 2);
}

/// D05 (`model-complete.md:156`, FR-151-AC-3: "an abstract subtype is
/// never a dispatch target"): the same diamond as
/// [`d01_a_closed_diamond_links_every_subtype_to_its_unique_undominated_candidate`],
/// with `model.B` declared abstract. `B` never enters the linked table at
/// all (not even as a no-applicable refusal); `D` still links to
/// `B.size` (an abstract type's own concrete subtypes remain valid
/// candidates and targets), and the charge counts drop by exactly `B`'s own
/// share (one fewer subtype, two fewer candidate checks, one fewer
/// dominance enumeration since only `D` still has two applicable
/// candidates).
#[trace("QSpec-TC-196")]
#[test]
fn an_abstract_subtype_is_never_linked_as_a_dispatch_target() {
    let mut records = fixture_g();
    for record in &mut records {
        if let DomainPackageRecord::ObjectType(object) = record {
            if object.key == DeclarationKey::fixture("model.B") {
                object.abstract_type = true;
            }
        }
    }
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );

    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = outcome else {
        panic!("expected a linked dispatch table, got {outcome:?}");
    };
    let linked = |subtype: &str| {
        table
            .linked_for(&DeclarationKey::fixture(subtype))
            .map(|c| c.node.clone())
    };
    assert_eq!(
        linked("model.B"),
        None,
        "abstract B is never a dispatch target"
    );
    assert_eq!(linked("model.D"), Some("model.B.size".to_owned()));
    assert_eq!(linked("model.C"), Some("model.A.size".to_owned()));
    assert_eq!(linked("model.A"), Some("model.A.size".to_owned()));

    let admitted = meter.admitted_charges();
    let candidate_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchCandidate)
        .count();
    let subtype_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchSubtype)
        .count();
    let dominance_count = admitted
        .iter()
        .filter(|c| **c == ChargePoint::DispatchDominance)
        .count();
    assert_eq!(candidate_count, 6);
    assert_eq!(subtype_count, 3);
    assert_eq!(dominance_count, 1);
}

/// D01's `dispatch_candidates` boundary: the eighth `dispatch.candidate`
/// charge is denied when the counter's limit is `7`.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-8")]
#[trace("TC-720", "FR-255-AC-1")]
#[test]
fn d01_the_eighth_dispatch_candidate_charge_is_incomplete_at_the_named_limit() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = qsl_semantics::model::accounting::Meter::new(ModelNormalizationLimits {
        dispatch_candidates: 7,
        ..ModelNormalizationLimits::UNLIMITED
    });
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );

    let LinkCheckOutcome::Incomplete(incomplete) = outcome else {
        panic!("expected an incomplete outcome, got {outcome:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::DispatchCandidates);
    assert_eq!(
        incomplete.limit_exceeded().setting(),
        qsl_foundation::Setting::ModelDispatchCandidates
    );
    assert_eq!(incomplete.limit, 7);
    assert_eq!(incomplete.consumed, 7);
    assert_eq!(incomplete.next_charge, 8);
    assert_eq!(incomplete.charge_point, ChargePoint::DispatchCandidate);
}

/// D01's `dispatch_candidates` boundary, the other half of FR-151-AC-8: the
/// exact bound (`8`, the real count D01 charges) completes.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-8")]
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn d01_the_eighth_dispatch_candidate_charge_completes_at_the_exact_limit() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = qsl_semantics::model::accounting::Meter::new(ModelNormalizationLimits {
        dispatch_candidates: 8,
        ..ModelNormalizationLimits::UNLIMITED
    });
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );

    assert!(matches!(
        outcome,
        LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(_))
    ));
}

/// D02: `C.size`/`D.size` also redefine `A.size`. Without a body for
/// `D.size`, `D` is a three-way tie among `A.size`, `B.size`, `C.size`
/// (`B.size`/`C.size` each dominate `A.size`, but neither dominates the
/// other): `ambiguous_dispatch`/`multiple-undominated` for `D` only, and no
/// dispatch table. With a body for `D.size`, `D.size`'s owner strictly
/// dominates every other owner and the whole operation links, `D ->
/// D.size`.
#[trace("TC-223", "TC-224", "FR-083-AC-2", "FR-083-AC-3")]
#[test]
fn d02_an_undominated_multi_way_tie_refuses_and_a_strict_descendant_resolves_it() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    records.push(operation(
        "model.C.size",
        "model.C",
        true,
        Some("model.A.size"),
    ));
    records.push(operation(
        "model.D.size",
        "model.D",
        false,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records.clone());
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Ambiguous(refusals)) = outcome else {
        panic!("expected an ambiguous outcome, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 1);
    let refusal = &refusals[0];
    assert_eq!(refusal.subtype.node, "model.D");
    assert_eq!(refusal.code, Code::AmbiguousDispatch);
    assert_eq!(refusal.cause, ModelRefusalCause::MultipleUndominated);
    let mut candidate_names: Vec<&str> =
        refusal.candidates.iter().map(|c| c.node.as_str()).collect();
    candidate_names.sort_unstable();
    assert_eq!(
        candidate_names,
        vec!["model.A.size", "model.B.size", "model.C.size"]
    );
    let dominance_names: std::collections::HashSet<(String, String)> = refusal
        .dominance_pairs
        .iter()
        .map(|pair| (pair.dominant.node.clone(), pair.dominated.node.clone()))
        .collect();
    assert!(dominance_names.contains(&("model.B.size".to_owned(), "model.A.size".to_owned())));
    assert!(dominance_names.contains(&("model.C.size".to_owned(), "model.A.size".to_owned())));
    assert_eq!(dominance_names.len(), 2);

    // With a body for D.size, D.size's owner (D) strictly dominates A, B
    // and C, so it uniquely wins at every subtype it is applicable for.
    let mut resolved_records = records;
    for record in &mut resolved_records {
        if let DomainPackageRecord::OperationMember(op) = record {
            if op.key.node == "model.D.size" {
                op.has_body = true;
            }
        }
    }
    let resolved_bundle =
        DomainPackage::new(DomainPackageRef::fixture("bundle.g"), resolved_records);
    let resolved_view = effective_view(&resolved_bundle);
    let mut resolved_meter = unlimited_meter();
    let resolved_outcome = link_dispatch(
        &resolved_view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut resolved_meter,
    );
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = resolved_outcome else {
        panic!("expected a linked dispatch table, got {resolved_outcome:?}");
    };
    assert_eq!(
        table
            .linked_for(&DeclarationKey::fixture("model.D"))
            .map(|c| c.node.as_str()),
        Some("model.D.size")
    );
}

/// D03: neither `A.size` nor `B.size` has a body. Every subtype has zero
/// applicable candidates: four `ambiguous_dispatch`/`no-applicable`
/// refusals, one per subtype, and no `dispatch.candidate` charge at all
/// (there is nothing to test a subtype against).
#[trace("TC-223", "FR-083-AC-2")]
#[test]
fn d03_no_candidate_with_a_body_refuses_every_subtype_as_no_applicable() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", false, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        false,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Ambiguous(refusals)) = outcome else {
        panic!("expected an ambiguous outcome, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 4);
    let subtypes: Vec<&str> = refusals.iter().map(|r| r.subtype.node.as_str()).collect();
    // Subtype order follows the view's own ascending effective-identity
    // order, i.e. each type's computed hash.
    assert_eq!(subtypes, vec!["model.A", "model.D", "model.C", "model.B"]);
    assert!(refusals.iter().all(|r| r.code == Code::AmbiguousDispatch));
    assert!(refusals
        .iter()
        .all(|r| r.cause == ModelRefusalCause::NoApplicable));
    assert!(refusals.iter().all(|r| r.candidates.is_empty()));
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::DispatchCandidate));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|c| **c == ChargePoint::DispatchSubtype)
            .count(),
        4
    );
}

/// D04: D01 with every domain package record supplied in reverse declared order and
/// `B.size` declared before `A.size`. Linking depends only on the
/// `DeclarationKey` (`package`, `node`)-sorted family and the effective
/// view's own ascending-identity subtype order, never on registration or
/// source order (FR-151-AC-5): the same table results.
#[trace("TC-222", "FR-083-AC-1")]
#[test]
fn d04_registration_order_does_not_change_the_linked_table() {
    let mut records = fixture_g();
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    records.push(operation("model.A.size", "model.A", true, None));
    records.reverse();
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = outcome else {
        panic!("expected a linked dispatch table, got {outcome:?}");
    };
    let linked = |subtype: &str| {
        table
            .linked_for(&DeclarationKey::fixture(subtype))
            .map(|c| c.node.as_str())
    };
    assert_eq!(linked("model.D"), Some("model.B.size"));
    assert_eq!(linked("model.C"), Some("model.A.size"));
    assert_eq!(linked("model.B"), Some("model.B.size"));
    assert_eq!(linked("model.A"), Some("model.A.size"));
}

/// D05: an open generalization closure has no dispatch table at all — the
/// incomplete outcome `incomplete_population`/`unclosed-method-set`, not a
/// refusal, and no dispatch charge of any kind.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-3")]
#[test]
fn d05_an_open_generalization_closure_is_incomplete_before_any_dispatch_charge() {
    let mut records = fixture_g();
    records.push(operation("model.A.size", "model.A", true, None));
    records.push(operation(
        "model.B.size",
        "model.B",
        true,
        Some("model.A.size"),
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.g"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Open,
        &mut meter,
    );
    let LinkCheckOutcome::OpenClosure(unclosed) = outcome else {
        panic!("expected an open-closure outcome, got {outcome:?}");
    };
    assert_eq!(unclosed.cause, ModelRefusalCause::UnclosedMethodSet);
    assert_eq!(unclosed.operation.node, "model.A.size");
    assert!(meter.admitted_charges().is_empty());
}

/// FR-154: "Two nodes share one identity" refuses
/// `invalid_model_binding`/`conflicting-binding`. Retargets the pre-#131
/// `f2_operation_members_sharing_an_identity_but_differing_in_revision_both_survive`
/// regression test: under the dropped `revision`/`digest` fields, two
/// `OperationMember` records that once differed only in `revision` now
/// share the exact same `DeclarationKey`, and `normalize` -- which every
/// real pipeline runs before `link_dispatch` ever sees a domain package --
/// must refuse before either candidate reaches `link_dispatch`.
#[trace("QSpec-TC-196")]
#[test]
fn two_operation_members_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-dispatch-conflict"),
        vec![
            object_type("model.A", vec![]),
            operation("model.A.size", "model.A", true, None),
            operation("model.A.size", "model.A", false, None),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingBinding {
                    key: DeclarationKey::fixture("model.A.size"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}

/// One proper ancestor owner per redefinition edge. Original members keep
/// their real owner identities; only the terminal operation has a body.
fn chain_owner(index: u64) -> String {
    if index == 0 { "model.A".to_owned() } else { format!("model.A{index}") }
}

fn chain_operation(index: u64) -> String { format!("{}.op", chain_owner(index)) }

fn redefinition_chain(edges: u64) -> Vec<DomainPackageRecord> {
    (0..=edges).map(|index| {
        let parent = index.checked_sub(1).map(chain_operation);
        query_operation(&chain_operation(index), &chain_owner(index), index == edges, parent.as_deref())
    }).collect()
}

/// [`operation`], with a declared `model.A` result so the checked-dispatch
/// bridge admits it as a query.
fn query_operation(
    identity: &str,
    owner: &str,
    has_body: bool,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    let DomainPackageRecord::OperationMember(mut record) =
        operation(identity, owner, has_body, redefines)
    else {
        unreachable!("operation() always builds an operation member");
    };
    record.result = Some(OperationResult {
        value_type: ValueTypeRef::Package(DeclarationKey::fixture("model.A")),
        multiplicity: Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        },
    });
    DomainPackageRecord::OperationMember(record)
}

fn link_chain(domain_package: &DomainPackage, limits: ModelNormalizationLimits) -> LinkCheckOutcome {
    qsl_semantics::model::intake::SelectedModels::fixture("M",
        qsl_foundation::Span { start: 0, end: 0 }, domain_package.clone(), limits)
        .expect("the proper-ancestor chain genuinely admits before linking")
        .fixture_link_dispatch(&DeclarationKey::fixture(chain_operation(0)), GeneralizationClosure::Closed)
}

fn chain_package(edges: u64) -> DomainPackage {
    let mut records: Vec<_> = (0..=edges).map(|index| {
        let parent = index.checked_sub(1).map(chain_owner);
        object_type(&chain_owner(index), parent.as_deref().into_iter().collect())
    }).collect();
    records.extend(redefinition_chain(edges));
    DomainPackage::new(DomainPackageRef::fixture("bundle.chain"), records)
}

fn assert_links_chain_receiver_to(outcome: LinkCheckOutcome, receiver: &str, winner: &str) {
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = outcome else {
        panic!("expected a linked dispatch table, got {outcome:?}");
    };
    assert_eq!(
        table
            .linked_for(&DeclarationKey::fixture(receiver))
            .map(|c| c.node.clone()),
        Some(winner.to_owned()),
    );
}

/// A dispatch family of more than 128 redefinition steps
/// links at default (unlimited) limits. The removed fixed ceiling of 128
/// refused this whatever the caller configured (ADR-011 §7.3; NFR-001 "an
/// implementation ceiling is not a domain bound").
#[trace("TC-225", "FR-083-AC-4")]
#[test]
fn a_dispatch_family_with_more_than_128_redefinition_steps_links_at_default_limits() {
    const EDGES: u64 = 130;
    assert_links_chain_receiver_to(
        link_chain(&chain_package(EDGES), ModelNormalizationLimits::UNLIMITED),
        &chain_owner(EDGES),
        &chain_operation(EDGES),
    );
}

/// End to end: the same family of more than 128 redefinition
/// steps also passes the checked-dispatch bridge, whose
/// effective-precondition walk follows the winner's whole `redefines`
/// chain (every operation declares its own precondition, so no link of the
/// chain is skipped).
#[trace("TC-225", "FR-083-AC-4")]
#[test]
fn a_dispatch_family_with_more_than_128_redefinition_steps_passes_the_checked_bridge() {
    const EDGES: u64 = 130;
    let domain_package = chain_package(EDGES);
    let mut clauses = OperationClauses::default();
    let root = DeclarationKey::fixture(chain_operation(0));
    clauses.member.insert(root.clone(), "op".to_owned());
    for i in 0..=EDGES {
        let key = DeclarationKey::fixture(chain_operation(i));
        clauses
            .parameters
            .insert(key.clone(), vec![("self".to_owned(), ValueType::Boolean)]);
        clauses.result.insert(key.clone(), ValueType::Boolean);
        clauses
            .own_precondition
            .insert(key.clone(), Expression::boolean(true));
        if i == EDGES {
            clauses.own_body.insert(key, Expression::boolean(true));
        }
    }
    let selected = qsl_semantics::model::intake::SelectedModels::fixture("M",
        qsl_foundation::Span { start: 0, end: 0 }, domain_package,
        ModelNormalizationLimits::UNLIMITED).expect("proper-ancestor chain genuinely admits");
    let declarations = qsl_semantics::check::checked_dispatch_selected_operation(
        selected,
        &DispatchRoot {
            key: root,
            closure: GeneralizationClosure::Closed,
        },
        &clauses,
        qsl_semantics::check::admitted_source(
            qsl_foundation::SourceIdentity::new("agent-ix", "qsl-semantics", "git", "1"),
            b"",
        ),
        qsl_foundation::IdentityLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("expected a checked dispatch family, got {refusal:?}"));
    assert_eq!(declarations.dispatch_tables.len(), 1);
}

/// FR-083-AC-4: a 10,000-long `redefines` chain links on a thread with a
/// 512 KiB stack once `family_steps` fits its edges: the walks are iterative
/// and the limit counts edges, so no chain depth refuses it.
#[trace("TC-225", "FR-083-AC-4")]
#[test]
fn a_ten_thousand_long_redefines_chain_links_on_a_small_stack() {
    const EDGES: u64 = 10_000;
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let limits = ModelNormalizationLimits {
                family_steps: EDGES,
                ..ModelNormalizationLimits::UNLIMITED
            };
            assert_links_chain_receiver_to(
                link_chain(&chain_package(EDGES), limits),
                &chain_owner(EDGES),
                &chain_operation(EDGES),
            );
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("linking must not overflow a 512 KiB stack");
}

/// FR-083: `family_steps` charges the effective-precondition walks as well
/// as the family walk, and an edge both follow counts once. Dispatching on
/// `op10` of a chain of 20 edges, the family walk follows the 10 edges below
/// it and the winner's precondition walk follows all 20 up to `op0`, 10 of
/// them the family walk's. At 19 the bridge refuses although the family
/// walk alone fits, so the precondition walk is charged; at 20 it admits,
/// so the 10 shared edges are not charged twice (30). That the walks share
/// one count rather than each having its own is
/// `family_steps_counts_the_edges_of_every_walk_together`'s.
#[trace("TC-225", "FR-083-AC-4")]
#[test]
fn the_checked_bridge_counts_every_walk_against_one_family_steps() {
    const EDGES: u64 = 20;
    const ROOT: u64 = 10;
    let domain_package = chain_package(EDGES);
    let mut clauses = OperationClauses::default();
    let root = DeclarationKey::fixture(chain_operation(ROOT));
    clauses.member.insert(root.clone(), "op".to_owned());
    for i in 0..=EDGES {
        let key = DeclarationKey::fixture(chain_operation(i));
        clauses
            .parameters
            .insert(key.clone(), vec![("self".to_owned(), ValueType::Boolean)]);
        clauses.result.insert(key.clone(), ValueType::Boolean);
        clauses
            .own_precondition
            .insert(key.clone(), Expression::boolean(true));
        if i == EDGES {
            clauses.own_body.insert(key, Expression::boolean(true));
        }
    }
    let run = |family_steps: u64| {
        let selected = qsl_semantics::model::intake::SelectedModels::fixture("M",
            qsl_foundation::Span { start: 0, end: 0 }, domain_package.clone(),
            ModelNormalizationLimits { family_steps, ..ModelNormalizationLimits::UNLIMITED })
            .expect("proper-ancestor chain genuinely admits before family traversal");
        qsl_semantics::check::checked_dispatch_selected_operation(
            selected,
            &DispatchRoot {
                key: root.clone(),
                closure: GeneralizationClosure::Closed,
            },
            &clauses,
            qsl_semantics::check::admitted_source(
                qsl_foundation::SourceIdentity::new("agent-ix", "qsl-semantics", "git", "1"),
                b"",
            ),
            qsl_foundation::IdentityLimits::default(),
        )
    };
    assert!(
        run(EDGES).is_ok(),
        "every walk follows the same {EDGES} edges, counted once"
    );
    match run(EDGES - 1) {
        Err(DispatchBridgeRefusal::FamilyStepsExceeded(refusal)) => assert_eq!(
            refusal.cause,
            ModelRefusalCause::family_steps(root, EDGES - 1)
        ),
        other => panic!(
            "expected FamilyStepsExceeded at {}, got {other:?}",
            EDGES - 1
        ),
    }
}

/// A fully admitted chain at the bound links; a fully admitted chain
/// with one additional edge refuses without exposing a partial winner.
#[trace("TC-720", "FR-255-AC-1")]
#[trace("TC-225", "FR-083-AC-4")]
#[test]
fn a_family_at_the_configured_bound_links_and_one_step_more_refuses() {
    const BOUND: u64 = 50;
    let limits = ModelNormalizationLimits {
        family_steps: BOUND,
        ..ModelNormalizationLimits::UNLIMITED
    };

    assert_links_chain_receiver_to(
        link_chain(&chain_package(BOUND), limits),
        &chain_owner(BOUND),
        &chain_operation(BOUND),
    );

    let over = chain_package(BOUND + 1);
    assert_links_chain_receiver_to(link_chain(&over, ModelNormalizationLimits::UNLIMITED),
        &chain_owner(BOUND + 1), &chain_operation(BOUND + 1));
    match link_chain(&over, limits) {
        LinkCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ResourceExhausted);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::family_steps(DeclarationKey::fixture(chain_operation(0)), BOUND)
            );
            let exceeded = refusal
                .cause
                .limit_exceeded()
                .expect("a step ceiling is a stage limit");
            assert_eq!(
                exceeded.setting(),
                qsl_foundation::Setting::ModelFamilySteps
            );
            assert_eq!(
                exceeded.actual(),
                u128::from(match &refusal.cause {
                    ModelRefusalCause::AncestorSteps { limit, .. }
                    | ModelRefusalCause::FamilySteps { limit, .. } => *limit,
                    other => panic!("unexpected {other:?}"),
                }) + 1
            );
            assert_eq!(refusal.cause.as_str(), "family-steps");
            assert!(
                refusal.detail.contains(&BOUND.to_string()),
                "refusal detail must name the configured bound, got {refusal:?}"
            );
        }
        other => panic!("expected Refused(FamilySteps), got {other:?}"),
    }
}


/// Incomparable genuine inherited replacements refuse in normalization,
/// before any bridge or partial dispatch table is exposed.
#[trace("TC-225", "QSpec-TC-196", "FR-082-AC-2")]
#[test]
fn a_genuine_sibling_join_refuses_with_both_redefinition_paths_before_dispatch() {
    use qsl_semantics::model::intake::{SelectedModels, UnitIntakeCause};
    for reverse in [false, true] {
        let mut package = chain_package(3);
        package.records.push(object_type("model.A4", vec!["model.A2"]));
        package.records.push(object_type("model.Join", vec!["model.A3", "model.A4"]));
        package.records.push(query_operation("model.A4.op", "model.A4", true, Some("model.A2.op")));
        if reverse { package.records.reverse(); }
        let refused = SelectedModels::fixture("M", qsl_foundation::Span { start: 0, end: 0 },
            package, ModelNormalizationLimits::UNLIMITED).expect_err("R07 refuses the contested inherited target before linking");
        assert!(refused.additional.is_empty());
        let UnitIntakeCause::Refused(refusals) = refused.cause else { panic!("expected completed normalization refusal, got {refused:?}"); };
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code, Code::InvalidModelBinding);
        assert_eq!(refusals[0].cause, ModelRefusalCause::DerivationConflict {
            type_: DeclarationKey::fixture("model.Join"),
            member: DeclarationKey::fixture("model.A2.op"),
            redefiners: vec![DeclarationKey::fixture("model.A3.op"), DeclarationKey::fixture("model.A4.op")],
        });
        assert_eq!(refusals[0].detail, "type model.Join has 2 undominated redefinitions of model.A2.op: [model.A3, model.A3.op, model.A2.op] and [model.A4, model.A4.op, model.A2.op]");
    }
}
