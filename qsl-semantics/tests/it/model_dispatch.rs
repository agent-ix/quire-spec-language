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
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::{ChargePoint, LimitKind, ModelNormalizationLimits};
use qsl_semantics::model::dispatch::{
    link_dispatch, DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize, ModelRefusalCause, NormalizeOutcome};

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
        has_own_precondition: false,
        own_postcondition_clauses: Vec::new(),
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
#[trace("TC-196")]
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
        &domain_package,
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
#[trace("TC-196")]
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
        &domain_package,
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
#[trace("TC-196", "FR-151-AC-8")]
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
        &domain_package,
        &view,
        &DeclarationKey::fixture("model.A.size"),
        GeneralizationClosure::Closed,
        &mut meter,
    );

    let LinkCheckOutcome::Incomplete(incomplete) = outcome else {
        panic!("expected an incomplete outcome, got {outcome:?}");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::DispatchCandidates);
    assert_eq!(incomplete.limit, 7);
    assert_eq!(incomplete.consumed, 7);
    assert_eq!(incomplete.next_charge, 8);
    assert_eq!(incomplete.charge_point, ChargePoint::DispatchCandidate);
}

/// D01's `dispatch_candidates` boundary, the other half of FR-151-AC-8: the
/// exact bound (`8`, the real count D01 charges) completes.
#[trace("TC-196", "FR-151-AC-8")]
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
        &domain_package,
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
#[trace("TC-196", "FR-151-AC-3")]
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
        &domain_package,
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
        &resolved_bundle,
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
#[trace("TC-196", "FR-151-AC-3")]
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
        &domain_package,
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
#[trace("TC-196", "FR-151-AC-5")]
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
        &domain_package,
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
#[trace("TC-196", "FR-151-AC-3")]
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
        &domain_package,
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
/// must refuse before either candidate reaches `DispatchIndex`.
#[trace("TC-196")]
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

/// QSL-199: a linear redefinition chain `model.A.op0 <- model.A.op1 <- ... <-
/// model.A.op{depth}` (`op{i}` redefines `op{i-1}`), all owned by the single
/// object type `model.A` so every dispatch/dominance `type_conforms` check
/// this exercises is the trivial `s == t` case -- isolating the family-step
/// count `build_family` itself charges from the unrelated ancestor-step
/// count `walk_ancestors` charges. Only the last operation has a body, so
/// linking has exactly one applicable candidate per subtype (no dominance
/// walk needed either). `original`'s own family has `depth + 1` members.
fn redefinition_chain(depth: usize) -> Vec<DomainPackageRecord> {
    (0..=depth)
        .map(|i| {
            let redefines = if i == 0 {
                None
            } else {
                Some(format!("model.A.op{}", i - 1))
            };
            operation(
                &format!("model.A.op{i}"),
                "model.A",
                i == depth,
                redefines.as_deref(),
            )
        })
        .collect()
}

/// QSL-199 AC-5: a valid dispatch family with more than 128 redefinition
/// steps passes at default (unlimited) limits -- the old hard-coded
/// `MAX_DISPATCH_DEPTH` (128) refused this regardless of the caller's own
/// configured limits (ADR-011 §7.3; NFR-001 "an implementation ceiling is
/// not a domain bound").
#[trace("TC-196", "FR-151-AC-5")]
#[test]
fn a_dispatch_family_with_more_than_128_redefinition_steps_links_at_default_limits() {
    const DEPTH: usize = 129; // original's family has 130 members: op0..op129.
    let mut records = vec![object_type("model.A", vec![])];
    records.extend(redefinition_chain(DEPTH));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.chain"), records);
    let view = effective_view(&domain_package);

    let mut meter = unlimited_meter();
    let outcome = link_dispatch(
        &domain_package,
        &view,
        &DeclarationKey::fixture("model.A.op0"),
        GeneralizationClosure::Closed,
        &mut meter,
    );
    let LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) = outcome else {
        panic!("expected a linked dispatch table under unlimited limits, got {outcome:?}");
    };
    assert_eq!(
        table
            .linked_for(&DeclarationKey::fixture("model.A"))
            .map(|c| c.node.clone()),
        Some(format!("model.A.op{DEPTH}")),
        "must link to the one operation with a body, at the far end of the 130-member family"
    );
}

/// QSL-199 AC-6: exceeding a caller-configured `family_steps` ceiling
/// refuses, naming the limit (`ModelRefusalCause::DispatchFamilyDepth`, tag
/// `resource_exhausted`) and the configured bound (embedded in the
/// refusal's own detail, matching this repo's convention for these two
/// caps -- see `build_family`'s own doc).
#[trace("TC-196", "FR-151-AC-5")]
#[test]
fn exceeding_a_caller_configured_family_steps_ceiling_refuses_naming_the_kind_and_bound() {
    const DEPTH: usize = 129;
    let mut records = vec![object_type("model.A", vec![])];
    records.extend(redefinition_chain(DEPTH));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.chain"), records);
    let view = effective_view(&domain_package);

    let tight_ceiling = 50;
    let mut meter = qsl_semantics::model::accounting::Meter::new(ModelNormalizationLimits {
        family_steps: tight_ceiling,
        ..ModelNormalizationLimits::UNLIMITED
    });
    let outcome = link_dispatch(
        &domain_package,
        &view,
        &DeclarationKey::fixture("model.A.op0"),
        GeneralizationClosure::Closed,
        &mut meter,
    );
    match outcome {
        LinkCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ResourceExhausted);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DispatchFamilyDepth {
                    original: DeclarationKey::fixture("model.A.op0"),
                }
            );
            assert!(
                refusal.detail.contains(&tight_ceiling.to_string()),
                "refusal detail must name the configured bound, got {refusal:?}"
            );
        }
        other => panic!("expected Refused(DispatchFamilyDepth), got {other:?}"),
    }
}
