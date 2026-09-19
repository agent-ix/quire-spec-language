// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-195: model normalization provenance (FR-150).
//!
//! Fixtures F1/F2 and expected identities are taken verbatim from
//! `resources/complete-value/quire-specification/spec/test-cases/TC-195-model-normalization-provenance.md`
//! and its ground-truth vectors,
//! `resources/complete-value/quire-specification/proposals/checked-package-v2/model-effective-declaration-vectors.json`.

use ix_trace_rs::trace;
use quire_spec_language::model::accounting::{
    ChargePoint, Incomplete, LimitKind, ModelNormalizationLimits,
};
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, RedefinitionRecord,
};
use quire_spec_language::model::key::{EffectiveId, ProducerKey, RULE_REDEFINE};
use quire_spec_language::model::normalize::{
    normalize, normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};

const MULTIPLICITY_0_1: Multiplicity = Multiplicity {
    lower: 0,
    upper: Some(1),
    ordered: false,
    unique: true,
};

fn object_type(identity: &str) -> BundleRecord {
    BundleRecord::ObjectType(ObjectTypeRecord {
        key: ProducerKey::fixture(identity),
        interface_features: None,
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> BundleRecord {
    BundleRecord::FieldMember(FieldMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        value_type: ProducerKey::fixture(value_type),
        multiplicity: MULTIPLICITY_0_1,
    })
}

/// A minimal operation member: no parameters, no result, no effect frame,
/// no postcondition -- enough to exist as a redefinable member without
/// pulling in `crate::model::dispatch`/`conformance`'s own richer fixtures.
fn operation_member(identity: &str, owner: &str) -> BundleRecord {
    BundleRecord::OperationMember(OperationMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        parameters: Vec::new(),
        result: None,
        effect: OperationEffect {
            modifies: Vec::new(),
            creates: Vec::new(),
            deletes: Vec::new(),
        },
        has_own_precondition: false,
        own_postcondition_clauses: Vec::new(),
        has_body: true,
    })
}

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

fn redefinition(identity: &str, owner: &str, redefining: &str, redefined: &str) -> BundleRecord {
    BundleRecord::Redefinition(RedefinitionRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        redefining: ProducerKey::fixture(redefining),
        redefined: ProducerKey::fixture(redefined),
    })
}

/// F1 (bundle `bundle.n01`): types `A`, `B`; field `A.x` of `A`; generalization `B` -> `A`.
fn fixture_f1() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.n01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            field_member("model.A.x", "model.A", "model.A"),
            generalization("model.gen.B-A", "model.B", "model.A"),
        ],
    )
}

/// F2 (bundle `bundle.n02`): F1's records plus types `C`, `D` and
/// generalizations `C` -> `A`, `D` -> `B`, `D` -> `C`.
fn fixture_f2() -> Bundle {
    let mut records = fixture_f1().records;
    records.push(object_type("model.C"));
    records.push(object_type("model.D"));
    records.push(generalization("model.gen.C-A", "model.C", "model.A"));
    records.push(generalization("model.gen.D-B", "model.D", "model.B"));
    records.push(generalization("model.gen.D-C", "model.D", "model.C"));
    Bundle::new(ModelSelection::fixture("bundle.n02"), records)
}

/// F2 plus field members `B.x2`/`C.x3` of type `A` (both of `A`'s own value
/// type) and redefinition records `redef.B` (`B`, `B.x2` redefines `A.x`)
/// and `redef.C` (`C`, `C.x3` redefines `A.x`) — TC-195 N06's first stage:
/// two undominated redefiners of `A.x` reach `D` through sibling owners `B`
/// and `C`.
fn fixture_n06_conflict() -> Bundle {
    let mut records = fixture_f2().records;
    records.push(field_member("model.B.x2", "model.B", "model.A"));
    records.push(field_member("model.C.x3", "model.C", "model.A"));
    records.push(redefinition(
        "model.redef.B",
        "model.B",
        "model.B.x2",
        "model.A.x",
    ));
    records.push(redefinition(
        "model.redef.C",
        "model.C",
        "model.C.x3",
        "model.A.x",
    ));
    Bundle::new(ModelSelection::fixture("bundle.n06"), records)
}

/// TC-195 N06's second stage: `fixture_n06_conflict` plus field `D.x4` of
/// `A`'s value type and record `redef.D` (`D`, `D.x4` redefines `A.x`). `D`
/// is a proper descendant of both `B` and `C`, so `D.x4` dominates every
/// other redefiner of `A.x` and the conflict resolves.
fn fixture_n06_resolved() -> Bundle {
    let mut records = fixture_n06_conflict().records;
    records.push(field_member("model.D.x4", "model.D", "model.A"));
    records.push(redefinition(
        "model.redef.D",
        "model.D",
        "model.D.x4",
        "model.A.x",
    ));
    Bundle::new(ModelSelection::fixture("bundle.n06"), records)
}

/// `fixture_n06_conflict` plus a type `E` with no generalization, field
/// `E.y` of `A`'s value type, and redefinition record `redef.E` (`E`,
/// `E.y` redefines `A.x`). `E` does not inherit `A`, so `A.x` is not an
/// effective member of `E` -- this redefinition record's own target is
/// unreachable from its owner, distinct from N06's own dominance conflict
/// at `D`.
fn fixture_n06_conflict_with_unreachable_redefiner() -> Bundle {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("model.E"));
    records.push(field_member("model.E.y", "model.E", "model.A"));
    records.push(redefinition(
        "model.redef.E",
        "model.E",
        "model.E.y",
        "model.A.x",
    ));
    Bundle::new(ModelSelection::fixture("bundle.n06e"), records)
}

/// `fixture_n06_conflict` (`D` conflicts on `A.x`) plus a second, unrelated
/// diamond: root `M` (field `M.w`), `M1` and `M2` (both `<- M`,
/// `M1.w1`/`M2.w2` redefine `M.w` -- sibling owners, neither dominating the
/// other), and `B9` (`<- M1`, `<- M2`, so it inherits both undominated
/// redefiners of `M.w`). `B9` sorts before `D`.
fn fixture_n06_conflict_with_a_second_diamond_sorting_first() -> Bundle {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("model.M"));
    records.push(object_type("model.M1"));
    records.push(object_type("model.M2"));
    records.push(object_type("model.B9"));
    records.push(field_member("model.M.w", "model.M", "model.M"));
    records.push(generalization("model.gen.M1-M", "model.M1", "model.M"));
    records.push(generalization("model.gen.M2-M", "model.M2", "model.M"));
    records.push(generalization("model.gen.B9-M1", "model.B9", "model.M1"));
    records.push(generalization("model.gen.B9-M2", "model.B9", "model.M2"));
    records.push(field_member("model.M1.w1", "model.M1", "model.M"));
    records.push(field_member("model.M2.w2", "model.M2", "model.M"));
    records.push(redefinition(
        "model.redef.M1",
        "model.M1",
        "model.M1.w1",
        "model.M.w",
    ));
    records.push(redefinition(
        "model.redef.M2",
        "model.M2",
        "model.M2.w2",
        "model.M.w",
    ));
    Bundle::new(ModelSelection::fixture("bundle.n06.two-diamonds"), records)
}

/// `E` (no generalization) with `redef.Ey` (`E`, `E.y` redefines `A.x`):
/// `A.x` is not an effective member of `E`. `Da` (`<- E`) inherits the
/// identical record and fails the identical check for the identical
/// reason. `Da` sorts before `E` in `type_keys`' ascending identity order.
fn fixture_unreachable_target_reached_by_owner_and_an_earlier_sorted_descendant() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.rank02"),
        vec![
            object_type("model.A"),
            object_type("model.E"),
            object_type("model.Da"),
            field_member("model.A.x", "model.A", "model.A"),
            generalization("model.gen.Da-E", "model.Da", "model.E"),
            field_member("model.E.y", "model.E", "model.A"),
            redefinition("model.redef.Ey", "model.E", "model.E.y", "model.A.x"),
        ],
    )
}

/// A second, independently-built instance of the same ranking shape as
/// `fixture_n06_conflict_with_unreachable_redefiner`: the
/// `normalize.conflict-check`-stage owner sorts before the
/// `normalize.redefinition-check`-stage owner in `type_keys`' ascending
/// order, to pin the ranking rule itself rather than any one bundle's own
/// identities. Types `G` (field `G.g`), `H` and `I` (both `<- G`, with
/// `H.h2`/`I.i3` redefining `G.g` -- sibling owners, neither dominating the
/// other), `J` (`<- H`, `<- I`, so it inherits both undominated redefiners
/// of `G.g`), and `K` (no generalization, `K.k` redefines `G.g`). `J`'s own
/// dominance conflict over `G.g` and `K`'s own unreachable target (`K` does
/// not inherit `G`) coexist; `J` sorts before `K`.
fn fixture_conflict_check_owner_sorts_before_redefinition_check_owner() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.rank01"),
        vec![
            object_type("model.G"),
            object_type("model.H"),
            object_type("model.I"),
            object_type("model.J"),
            object_type("model.K"),
            field_member("model.G.g", "model.G", "model.G"),
            generalization("model.gen.H-G", "model.H", "model.G"),
            generalization("model.gen.I-G", "model.I", "model.G"),
            generalization("model.gen.J-H", "model.J", "model.H"),
            generalization("model.gen.J-I", "model.J", "model.I"),
            field_member("model.H.h2", "model.H", "model.G"),
            field_member("model.I.i3", "model.I", "model.G"),
            redefinition("model.redef.h2", "model.H", "model.H.h2", "model.G.g"),
            redefinition("model.redef.i3", "model.I", "model.I.i3", "model.G.g"),
            field_member("model.K.k", "model.K", "model.G"),
            redefinition("model.redef.k", "model.K", "model.K.k", "model.G.g"),
        ],
    )
}

/// TC-196 R07's second shape (origin/main, after QSpec #86): `B` itself
/// (not two sibling lineages) declares two distinct redefining members,
/// `B/z` and `B/z2`, both `redefines: A/x` — the identical single inherited
/// target contended by two redefiners under one owner, distinct from N06's
/// diamond conflict (two different owners, neither dominating the other).
fn fixture_r07_same_owner_contending_redefiners() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.r07"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.x", "model.A", "model.A"),
            field_member("model.B.z", "model.B", "model.A"),
            field_member("model.B.z2", "model.B", "model.A"),
            redefinition("model.redef.z", "model.B", "model.B.z", "model.A.x"),
            redefinition("model.redef.z2", "model.B", "model.B.z2", "model.A.x"),
        ],
    )
}

/// Re-review of PR #157's round-3 fix: `C <= A` with `C/w` (owned by `C`)
/// `redefines: A/x`; `B <= C` with `B/z` and `B/z2` (both owned by `B`)
/// also `redefines: A/x`. `B`'s edges already dominate `C`'s single edge
/// (a more-derived owner beating a less-derived one, exactly as it would
/// if `B` had only one redefiner), so `C`'s edge takes no part in the
/// "one owner, several redefiners" test — only `B`'s two edges do, and
/// they share the identical owner.
fn fixture_r07_dominated_owner_takes_no_part_in_the_same_owner_test() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.r07d"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            generalization("model.gen.C-A", "model.C", "model.A"),
            generalization("model.gen.B-C", "model.B", "model.C"),
            field_member("model.A.x", "model.A", "model.A"),
            field_member("model.C.w", "model.C", "model.A"),
            field_member("model.B.z", "model.B", "model.A"),
            field_member("model.B.z2", "model.B", "model.A"),
            redefinition("model.redef.w", "model.C", "model.C.w", "model.A.x"),
            redefinition("model.redef.z", "model.B", "model.B.z", "model.A.x"),
            redefinition("model.redef.z2", "model.B", "model.B.z2", "model.A.x"),
        ],
    )
}

/// Finds the effective member declared with original identity
/// `original_identity` under owner effective type `owner`, regardless of
/// its `visible` bit — phase 4 retains hidden entries in the view.
fn find_member<'a>(
    view: &'a quire_spec_language::model::normalize::EffectiveView,
    owner: &EffectiveId,
    original_identity: &str,
) -> &'a quire_spec_language::model::normalize::ViewEntry {
    view.declarations
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.as_ref() == Some(owner)
                && entry.preimage.original.identity == original_identity
        })
        .unwrap_or_else(|| panic!("no member {original_identity} owned by {owner:?} in {view:?}"))
}

fn completed(
    bundle: &Bundle,
    limits: ModelNormalizationLimits,
) -> quire_spec_language::model::normalize::EffectiveView {
    match normalize(bundle, limits) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed view, got {other:?}"),
    }
}

fn find<'a>(
    view: &'a quire_spec_language::model::normalize::EffectiveView,
    short_hex: &str,
) -> &'a quire_spec_language::model::normalize::ViewEntry {
    view.declarations
        .iter()
        .find(|entry| entry.effective_id.short_hex() == short_hex)
        .unwrap_or_else(|| panic!("no declaration with identity {short_hex} in {view:?}"))
}
#[trace("TC-195", "FR-150-AC-1", "FR-150-AC-3")]
#[test]
fn n01_normalizes_f1_to_the_exact_ground_truth_identities() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.declarations.len(), 4);

    let type_a = find(&view, "f1cc59cd");
    assert_eq!(
        type_a.effective_id.hex(),
        "f1cc59cd925687bda1e9e91e9ddd01b3fdd862ea0632bd497a42a75f18a627ca"
    );
    assert_eq!(type_a.preimage.owner_effective_type, None);
    assert_eq!(type_a.preimage.derivation.len(), 1);

    let type_b = find(&view, "b9953c43");
    assert_eq!(
        type_b.effective_id.hex(),
        "b9953c43aa0fdd40ca23f80ea2932b8e3908d6dc6af10a3a955c518d47989560"
    );
    assert_eq!(type_b.preimage.derivation.len(), 2);

    let member_a_x = find(&view, "4c06822f");
    assert_eq!(
        member_a_x.effective_id.hex(),
        "4c06822f2d05d603e97c7ca356701a0d0be955ccd7cf4f30a6a21c8f1082d292"
    );
    assert_eq!(
        member_a_x.preimage.owner_effective_type,
        Some(type_a.effective_id.clone())
    );

    let member_b_x = find(&view, "e8a29d61");
    assert_eq!(
        member_b_x.effective_id.hex(),
        "e8a29d61b4872863400f23b859adffb27449909de4ed74141c9db0619466d410"
    );
    assert_eq!(
        member_b_x.preimage.owner_effective_type,
        Some(type_b.effective_id.clone())
    );
    assert_eq!(member_b_x.preimage.derivation.len(), 1);

    assert_eq!(
        view.identity().hex(),
        "9ae1232acbb9dbdf69fb1dd20e2acd1eded73c246161b6d774e9949abe941749"
    );

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_eq!(universe.root_types, vec![type_a.effective_id.clone()]);
    assert_eq!(
        universe.identity().hex(),
        "0873083c49d8eb4a97733bab8353061f896626bffdfb7ebcff6dcc34b6e3bccf"
    );
}

#[trace("TC-195", "FR-150-AC-6")]
#[test]
fn n02_normalizes_f2_diamond_inheritance_to_the_exact_ground_truth_identities() {
    let view = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.declarations.len(), 8);

    let expected: &[(&str, &str)] = &[
        (
            "f1cc59cd",
            "f1cc59cd925687bda1e9e91e9ddd01b3fdd862ea0632bd497a42a75f18a627ca",
        ),
        (
            "b9953c43",
            "b9953c43aa0fdd40ca23f80ea2932b8e3908d6dc6af10a3a955c518d47989560",
        ),
        (
            "7c28ad04",
            "7c28ad04592a5d54e06cac985e9bbc495cdcd7c69ddc04b06c013065414be4b3",
        ),
        (
            "51796212",
            "517962120a9ded35208eac39e5e69eb9275145578307c657d19a3189d587fc8e",
        ),
        (
            "4c06822f",
            "4c06822f2d05d603e97c7ca356701a0d0be955ccd7cf4f30a6a21c8f1082d292",
        ),
        (
            "e8a29d61",
            "e8a29d61b4872863400f23b859adffb27449909de4ed74141c9db0619466d410",
        ),
        (
            "118a8e09",
            "118a8e09ebeb0d8335da657fef1417ddabfb4c70055362ac91fb95ce35b045bd",
        ),
        (
            "13a71b44",
            "13a71b442efbafa2131edc2e74d3dd580cc928f01979455a955c7a3b18457dce",
        ),
    ];
    for (short, full) in expected {
        assert_eq!(find(&view, short).effective_id.hex(), *full, "{short}");
    }

    let type_d = find(&view, "51796212");
    assert_eq!(
        type_d.preimage.derivation.len(),
        5,
        "qualify plus four inherit facts"
    );

    let member_d_x = find(&view, "13a71b44");
    assert_eq!(
        member_d_x.preimage.derivation.len(),
        2,
        "both diamond paths retained"
    );

    assert_eq!(
        view.identity().hex(),
        "d3a497977547c8d1e888018b6271690b4d6f6be5113b214b1e3ff8997f270c20"
    );

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f2()).unwrap();
    assert_eq!(
        universe.identity().hex(),
        "ed29d7101f52f874a508050e7bc384edc0a17b79f9ad30b63813df0452fa91b1"
    );
}

#[trace("TC-195", "FR-150-AC-4")]
#[test]
fn n07_record_order_does_not_affect_identity_or_view() {
    let mut reversed = fixture_f2();
    reversed.records.reverse();
    let view = completed(&reversed, ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        view.identity().hex(),
        "d3a497977547c8d1e888018b6271690b4d6f6be5113b214b1e3ff8997f270c20"
    );
}

#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n01_exact_limits_complete_and_the_charge_totals_match_ground_truth() {
    let exact = ModelNormalizationLimits {
        producer_records: 4,
        derivation_facts: 5,
        effective_declarations: 4,
        dispatch_candidates: 0,
        hashed_bytes: 9989,
        work_units: 20,
    };
    let (outcome, meter) = normalize_with_meter(&fixture_f1(), exact);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 20);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 9989);
    assert_eq!(meter.consumed(LimitKind::ProducerRecords), 4);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 5);
    assert_eq!(meter.consumed(LimitKind::EffectiveDeclarations), 4);
    assert_eq!(
        meter.admitted_charges().last(),
        Some(&ChargePoint::NormalizeHash)
    );
}

#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n01_one_less_work_unit_is_incomplete_at_the_view_hash() {
    let mut limits = ModelNormalizationLimits {
        producer_records: 4,
        derivation_facts: 5,
        effective_declarations: 4,
        dispatch_candidates: 0,
        hashed_bytes: 9989,
        work_units: 19,
    };
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 19);
            assert_eq!(incomplete.consumed, 19);
            assert_eq!(incomplete.next_charge, 1);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeHash);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }

    limits.work_units = 20;
    limits.hashed_bytes = 9988;
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::HashedBytes);
            assert_eq!(incomplete.limit, 9988);
            assert_eq!(incomplete.consumed, 4709);
            assert_eq!(incomplete.next_charge, 5280);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeHash);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }

    limits.hashed_bytes = 9989;
    limits.derivation_facts = 4;
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 4);
            assert_eq!(incomplete.consumed, 4);
            assert_eq!(incomplete.next_charge, 5);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }
}

#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n05_digest_domain_mismatch_refuses_before_any_effective_view() {
    let mut bundle = fixture_f1();
    let BundleRecord::FieldMember(member) = &mut bundle.records[2] else {
        panic!("fixture_f1 records[2] is not a field member");
    };
    assert_eq!(member.key.identity, "model.A.x");
    member.key.digest.domain = "quire-native-bytes-1".to_owned();

    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::StaleDependency
            );
            {
                let mut key = ProducerKey::fixture("model.A.x");
                key.digest.domain = "quire-native-bytes-1".to_owned();
                assert_eq!(
                    refusal.cause,
                    ModelRefusalCause::DigestDomainMismatch {
                        key,
                        domain: "quire-native-bytes-1".to_owned(),
                    }
                );
            }
            assert!(refusal.detail.contains("model.A.x"));
            assert!(refusal.detail.contains("filament-canonical-json-1"));
            assert!(refusal.detail.contains("quire-native-bytes-1"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-195")]
#[test]
fn n09_effective_and_universe_identities_never_collide_with_a_producer_key() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();

    let mut producer_digests: Vec<String> = fixture_f1()
        .records
        .iter()
        .map(|record| record.key().digest.sha256)
        .map(|bytes| quire_spec_language::model::key::hex(&bytes))
        .collect();
    producer_digests.sort();

    for entry in &view.declarations {
        assert!(
            !producer_digests.contains(&entry.effective_id.hex()),
            "effective identity {} collided with a producer digest",
            entry.effective_id.hex()
        );
    }
    assert!(!producer_digests.contains(&view.identity().hex()));
    assert!(!producer_digests.contains(&universe.identity().hex()));
}

#[trace("TC-195")]
#[test]
fn a_field_member_naming_an_undeclared_owner_refuses_instead_of_dropping() {
    let mut bundle = fixture_f1();
    bundle.records.push(field_member(
        "model.orphan.x",
        "model.no-such-type",
        "model.A",
    ));
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownOwner {
                    member: ProducerKey::fixture("model.orphan.x"),
                    owner: ProducerKey::fixture("model.no-such-type"),
                }
            );
            assert!(refusal.detail.contains("model.orphan.x"));
            assert!(refusal.detail.contains("model.no-such-type"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-195")]
#[test]
fn a_generalization_naming_an_undeclared_specific_refuses_instead_of_being_ignored() {
    let mut bundle = fixture_f1();
    bundle.records.push(generalization(
        "model.gen.orphan",
        "model.no-such-type",
        "model.A",
    ));
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownSpecific {
                    generalization: ProducerKey::fixture("model.gen.orphan"),
                    specific: ProducerKey::fixture("model.no-such-type"),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-195")]
#[test]
fn a_generalization_naming_an_undeclared_general_refuses_instead_of_panicking() {
    let mut bundle = fixture_f1();
    bundle.records.push(generalization(
        "model.gen.orphan",
        "model.A",
        "model.no-such-type",
    ));
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownGeneral {
                    generalization: ProducerKey::fixture("model.gen.orphan"),
                    general: ProducerKey::fixture("model.no-such-type"),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-195")]
#[test]
fn unsupported_interface_version_refuses_before_any_charge() {
    let mut bundle = fixture_f1();
    bundle.model_selection.contract_version.interface_version = "1.4.0".to_owned();
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::UnknownWire
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnsupportedWire {
                    version: "1.4.0".to_string(),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

// Retagged (PR #144 review finding #1): this is the conflict refusal AC-3
// names ("a conflicting phase 4 derivation... refuses with its named
// cause") and, together, the perfect test for AC-5 ("conflicting
// derivations report both rule paths and expose no chosen effective
// member") — asserting the refusal names both contending redefiners' rule
// paths is exactly that. Not AC-1: nothing here links a normalized
// identity to its contributing declarations.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-5")]
#[test]
fn n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict() {
    match normalize(&fixture_n06_conflict(), ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DerivationConflict {
                    type_: ProducerKey::fixture("model.D"),
                    member: ProducerKey::fixture("model.A.x"),
                    redefiners: vec![
                        ProducerKey::fixture("model.B.x2"),
                        ProducerKey::fixture("model.C.x3"),
                    ],
                }
            );
            assert!(refusal.detail.contains("model.gen.D-B"));
            assert!(refusal.detail.contains("model.redef.B"));
            assert!(refusal.detail.contains("model.gen.D-C"));
            assert!(refusal.detail.contains("model.redef.C"));
            assert!(refusal.detail.contains("model.A.x"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// QSL #145: `build`'s own `fact_budget_exceeded` gate skips phase-4
/// resolution entirely once phase 2/3's own fact budget is already
/// exhausted, so a truncated ancestor-path set can never derive a false
/// phase-4 refusal from partial data --
/// `charge_all`'s own replay of the phase 2/3 facts that triggered the
/// truncation always denies, in charge order, before phase 4's own charges
/// are ever consulted (FR-150's "exhaustion ends checking"). Under
/// `ModelNormalizationLimits::UNLIMITED`, `fixture_n06_conflict` refuses
/// `derivation-conflict` (the test above); under a tight `derivation_facts`
/// budget the outcome is `Incomplete` instead, at every point along the
/// sweep, never that refusal or the `redefinition-unreachable` refusal a
/// still-tighter budget's truncated `D` would otherwise expose.
///
/// Revert-probe: removing the `if !fact_budget_exceeded(...)` guard around
/// the per-type phase-4 loop in `build()` (running `apply_redefinitions`
/// unconditionally again) turns both cases below back into their pre-fix
/// refusals — confirmed locally, then restored.
#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n06_conflict_under_a_tight_fact_budget_is_incomplete_not_a_phase4_refusal() {
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.derivation_facts = 10;
    match normalize(&fixture_n06_conflict(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 10);
            assert_eq!(incomplete.consumed, 10);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!(
            "expected Incomplete at normalize.fact, not a phase-4 derivation-conflict refusal, got {other:?}"
        ),
    }

    limits.derivation_facts = 0;
    match normalize(&fixture_n06_conflict(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 0);
            assert_eq!(incomplete.consumed, 0);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!(
            "expected Incomplete at normalize.fact, not a redefinition-unreachable refusal, got {other:?}"
        ),
    }
}

/// TC-196 R07's second shape, run through `normalize`'s own phase 4
/// (`apply_redefinitions`) rather than `model::conformance`'s
/// `resolve_redefinition_target` in isolation — the real boundary a model
/// actually normalizes through. Two redefining members owned by the
/// identical type both redefine the same inherited target: refuses
/// `redefinition-target`, not `derivation-conflict` (N06's diamond shape
/// above uses two different owners, neither dominating the other).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_two_redefiners_owned_by_the_same_type_refuse_redefinition_target_through_normalize() {
    match normalize(
        &fixture_r07_same_owner_contending_redefiners(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(refusal.cause, ModelRefusalCause::RedefinitionTarget);
            assert!(
                refusal.detail.contains("model.B.z2"),
                "detail must name B/z2's own declaration key: {}",
                refusal.detail
            );
            assert!(
                refusal.detail.contains("model.B.z") && !refusal.detail.contains("model.redef.z"),
                "detail must name the redefining members' own keys, not the redefinition records' keys: {}",
                refusal.detail
            );
            assert!(
                refusal.detail.contains("model.A.x"),
                "detail must name the contended target: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
}

/// Re-review of PR #157's round-3 fix: the same-owner test must look only
/// at the *most-derived* owners among the undominated edges, not every
/// edge's owner. `C/w` (owner `C`) and `B/z`/`B/z2` (owner `B <= C`) all
/// redefine `A/x`: `C`'s edge is already dominated by `B`'s (either of
/// them), so it takes no part in the ambiguity test, leaving only `B`'s
/// two edges — one owner, refuses `redefinition-target`, not
/// `derivation-conflict`.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_a_less_derived_owners_redefiner_is_excluded_from_the_same_owner_test() {
    match normalize(
        &fixture_r07_dominated_owner_takes_no_part_in_the_same_owner_test(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(refusal.cause, ModelRefusalCause::RedefinitionTarget);
            assert!(
                refusal.detail.contains("model.B.z") && refusal.detail.contains("model.B.z2"),
                "detail must name both of B's own redefining members: {}",
                refusal.detail
            );
            assert!(
                !refusal.detail.contains("model.C.w"),
                "C's already-dominated edge must take no part in the ambiguity: {}",
                refusal.detail
            );
            assert!(
                refusal.detail.contains("model.A.x"),
                "detail must name the contended target: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
}

// Retagged (PR #144 review finding #1): every assertion here checks that
// the winner's/hidden target's derivation facts link to every contributing
// original declaration and rule — AC-1's own language. It never checks
// replay/record-order independence, which is what AC-4 actually requires.
#[trace("TC-195", "FR-150-AC-1")]
#[test]
fn n06_a_strictly_more_derived_redefiner_resolves_the_conflict_and_hides_every_contender() {
    let view = completed(&fixture_n06_resolved(), ModelNormalizationLimits::UNLIMITED);

    // Type-level identities are unaffected by phase 4 (field-only): D's
    // identity is exactly N02's ground-truth "51796212" type.
    let type_d = find(&view, "51796212");
    let owner_d = type_d.effective_id.clone();

    let winner = find_member(&view, &owner_d, "model.D.x4");
    assert!(
        winner.visible,
        "D.x4 must win outright: D is a descendant of both B and C"
    );
    let winner_redefine: Vec<_> = winner
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(winner_redefine.len(), 1);
    assert_eq!(
        winner_redefine[0].inputs,
        vec![
            ProducerKey::fixture("model.redef.D"),
            ProducerKey::fixture("model.A.x"),
        ]
    );

    let target = find_member(&view, &owner_d, "model.A.x");
    assert!(!target.visible, "A.x is retained for provenance but hidden");
    let target_redefine: Vec<_> = target
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(
        target_redefine.len(),
        3,
        "one redefine fact per competing redefiner: B, C, D"
    );
    assert_eq!(
        target_redefine[0].inputs,
        vec![
            ProducerKey::fixture("model.gen.D-B"),
            ProducerKey::fixture("model.redef.B"),
            ProducerKey::fixture("model.A.x"),
        ]
    );
    assert_eq!(
        target_redefine[1].inputs,
        vec![
            ProducerKey::fixture("model.gen.D-C"),
            ProducerKey::fixture("model.redef.C"),
            ProducerKey::fixture("model.A.x"),
        ]
    );
    assert_eq!(
        target_redefine[2].inputs,
        vec![
            ProducerKey::fixture("model.redef.D"),
            ProducerKey::fixture("model.A.x"),
        ]
    );

    let loser_b = find_member(&view, &owner_d, "model.B.x2");
    assert!(
        !loser_b.visible,
        "B.x2 loses to D.x4's more-derived redefinition"
    );
    let loser_b_redefine: Vec<_> = loser_b
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(loser_b_redefine.len(), 1, "only B.x2's own edge, not D's");
    assert_eq!(
        loser_b_redefine[0].inputs,
        vec![
            ProducerKey::fixture("model.gen.D-B"),
            ProducerKey::fixture("model.redef.B"),
            ProducerKey::fixture("model.A.x"),
        ]
    );

    let loser_c = find_member(&view, &owner_d, "model.C.x3");
    assert!(
        !loser_c.visible,
        "C.x3 loses to D.x4's more-derived redefinition"
    );
    let loser_c_redefine: Vec<_> = loser_c
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(loser_c_redefine.len(), 1, "only C.x3's own edge, not D's");
    assert_eq!(
        loser_c_redefine[0].inputs,
        vec![
            ProducerKey::fixture("model.gen.D-C"),
            ProducerKey::fixture("model.redef.C"),
            ProducerKey::fixture("model.A.x"),
        ]
    );
}

// Retagged (PR #144 review finding #1): a dangling redefinition target
// refuses with a named cause, closer to AC-3 than to AC-1 (which is about
// linking a normalized identity to its contributing declarations — there
// is no normalized identity here at all, only a refusal).
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n06_redefinition_target_absent_from_the_bundle_refuses_instead_of_dropping() {
    let mut bundle = fixture_f2();
    bundle.records.push(redefinition(
        "model.redef.orphan",
        "model.B",
        "model.B.no-such-member",
        "model.A.x",
    ));
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownMember {
                    record: ProducerKey::fixture("model.redef.orphan"),
                    member: ProducerKey::fixture("model.B.no-such-member"),
                }
            );
            assert!(refusal.detail.contains("model.B.no-such-member"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// PR #140 F2 regression: two `ObjectType` records that share a display
/// identity but differ in revision are distinct original declarations under
/// `ProducerKey`'s full-key equality and must both survive as distinguishable
/// effective declarations, not collapse to one. This is also FR-150-AC-2's
/// real test (`unsupported_interface_version_refuses_before_any_charge`
/// above was mistagged with this AC; it actually tests N08's second clause).
/// On `759968d` `Index::build` keyed `types` by display identity alone, so
/// this collapsed to a single declaration.
#[trace("TC-195", "FR-150-AC-2")]
#[test]
fn f2_producer_keys_sharing_an_identity_but_differing_in_revision_both_survive() {
    let mut second_revision = ProducerKey::fixture("model.T");
    second_revision.revision.value = "2".to_owned();
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.f2-revision"),
        vec![
            BundleRecord::ObjectType(ObjectTypeRecord {
                key: ProducerKey::fixture("model.T"),
                interface_features: None,
            }),
            BundleRecord::ObjectType(ObjectTypeRecord {
                key: second_revision,
                interface_features: None,
            }),
        ],
    );
    let view = completed(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        view.declarations.len(),
        2,
        "both revisions of model.T must survive as distinct effective types"
    );
    let identities: std::collections::HashSet<_> = view
        .declarations
        .iter()
        .map(|e| e.effective_id.clone())
        .collect();
    assert_eq!(
        identities.len(),
        2,
        "the two declarations must be distinguishable, not merged"
    );
}

/// PR #140 F2 regression: two `FieldMember` records under the same owner that
/// share a display identity but differ in revision must also both survive.
/// On `759968d` `member_preimages` was keyed `(String, String)` (display
/// identities), so the second record silently overwrote the first while the
/// meter still charged for both — a silent drop, not just a merge.
#[trace("TC-195")]
#[test]
fn f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive() {
    let mut second_revision = ProducerKey::fixture("model.A.x");
    second_revision.revision.value = "2".to_owned();
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.f2-member-revision"),
        vec![
            object_type("model.A"),
            BundleRecord::FieldMember(FieldMemberRecord {
                key: ProducerKey::fixture("model.A.x"),
                owner: ProducerKey::fixture("model.A"),
                value_type: ProducerKey::fixture("model.A"),
                multiplicity: MULTIPLICITY_0_1,
            }),
            BundleRecord::FieldMember(FieldMemberRecord {
                key: second_revision,
                owner: ProducerKey::fixture("model.A"),
                value_type: ProducerKey::fixture("model.A"),
                multiplicity: MULTIPLICITY_0_1,
            }),
        ],
    );
    let view = completed(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        view.declarations.len(),
        3,
        "type A plus both revisions of member A.x must all survive"
    );
}

/// PR #140 F1 regression: 15 types in a chain, each specific type generalizing
/// to its predecessor via TWO parallel generalization records (43 records
/// total: 15 `ObjectType` + 28 `Generalization`) — the exact shape the review
/// reproduced (diamond/parallel generalization causes an ancestor-path count
/// exponential in chain depth). Under a tight budget this must return a typed
/// `Incomplete` quickly rather than enumerate every path first. On `759968d`
/// this exact 43-record shape took ~51s (unbounded `ancestor_paths`); it must
/// complete in low single-digit seconds here.
fn fixture_deep_parallel_generalization_chain() -> Bundle {
    let mut records: Vec<BundleRecord> = (0..15)
        .map(|i| object_type(&format!("model.T{i}")))
        .collect();
    for i in 1..15 {
        records.push(generalization(
            &format!("model.gen.T{i}-T{}-a", i - 1),
            &format!("model.T{i}"),
            &format!("model.T{}", i - 1),
        ));
        records.push(generalization(
            &format!("model.gen.T{i}-T{}-b", i - 1),
            &format!("model.T{i}"),
            &format!("model.T{}", i - 1),
        ));
    }
    assert_eq!(records.len(), 43, "15 types + 28 generalizations");
    Bundle::new(
        ModelSelection::fixture("bundle.deep-parallel-chain"),
        records,
    )
}

#[trace("TC-195")]
#[test]
fn f1_deep_parallel_generalization_bounds_enumeration_instead_of_exploding() {
    let bundle = fixture_deep_parallel_generalization_chain();
    let tight = ModelNormalizationLimits {
        producer_records: 43,
        derivation_facts: 1,
        effective_declarations: 1,
        dispatch_candidates: 0,
        hashed_bytes: 1,
        work_units: 1,
    };
    let start = std::time::Instant::now();
    let outcome = normalize(&bundle, tight);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "normalize took {elapsed:?} under a saturated budget; ancestor-path \
         enumeration is not bounded during the walk (F1 regression)"
    );
    match outcome {
        NormalizeOutcome::Incomplete(_) => {}
        other => panic!("expected a typed Incomplete under limits of 1, got {other:?}"),
    }
}

/// PR #140 F6: a producer key with an absent revision (empty
/// `revision.namespace`/`revision.value`) refuses `wrong-model-selection`
/// rather than normalizing as if the revision label were simply blank.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n04_absent_revision_refuses_wrong_model_selection() {
    let mut bundle = fixture_f1();
    let BundleRecord::Generalization(gen) = &mut bundle.records[3] else {
        panic!("fixture_f1 records[3] is not a generalization");
    };
    assert_eq!(gen.key.identity, "model.gen.B-A");
    gen.key.revision.namespace.clear();
    gen.key.revision.value.clear();

    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            {
                let mut key = ProducerKey::fixture("model.gen.B-A");
                key.revision.namespace.clear();
                key.revision.value.clear();
                assert_eq!(
                    refusal.cause,
                    ModelRefusalCause::WrongModelSelection { key }
                );
            }
            assert!(refusal.detail.contains("model.gen.B-A"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// PR #140 F4 / TC-195 N08: a producer interface `1.2.0` bundle yields
/// exactly one `unsupplied-producer-record` refusal per missing FR-150
/// capability item, in the fixed order, after three `normalize.record` and
/// ten `normalize.unsupplied-item` charges — never the `unknown-wire`
/// refusal a plain unsupported-version bundle gets.
#[trace("TC-195", "FR-150-AC-7")]
#[test]
fn n08_interface_1_2_0_refuses_every_missing_capability_in_fixed_order() {
    let mut selection = ModelSelection::fixture("bundle.n01");
    selection.contract_version.interface_version = "1.2.0".to_owned();
    let bundle = Bundle::new(
        selection,
        vec![
            object_type("model.A"),
            object_type("model.B"),
            field_member("model.A.x", "model.A", "model.A"),
        ],
    );

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    let refusals = match outcome {
        NormalizeOutcome::UnsupportedCapabilities(refusals) => refusals,
        other => panic!("expected UnsupportedCapabilities, got {other:?}"),
    };

    let expected: &[(&str, &str)] = &[
        ("subtype-closure", "bundle.n01"),
        ("subsetting-closure", "bundle.n01"),
        ("redefinition-closure", "bundle.n01"),
        ("generalization", "model.A"),
        ("generalization", "model.B"),
        ("redefinition", "model.A.x"),
        ("subsetting", "model.A.x"),
        ("interface-signature", "model.A"),
        ("interface-signature", "model.B"),
        ("typed-multiplicity", "model.A.x"),
    ];
    assert_eq!(refusals.len(), expected.len());
    for (refusal, (name, subject)) in refusals.iter().zip(expected) {
        assert_eq!(
            refusal.code,
            quire_spec_language::diagnostic::Code::InvalidModelBinding
        );
        assert_eq!(refusal.cause, ModelRefusalCause::UnsuppliedProducerRecord);
        assert!(refusal.detail.contains(name), "{}", refusal.detail);
        assert!(refusal.detail.contains(subject), "{}", refusal.detail);
        assert!(refusal.detail.contains("1.3.0"), "{}", refusal.detail);
        assert!(refusal.detail.contains("1.2.0"), "{}", refusal.detail);
    }

    assert_eq!(meter.consumed(LimitKind::ProducerRecords), 3);
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted.len(),
        13,
        "three normalize.record + ten normalize.unsupplied-item"
    );
    assert!(admitted[..3]
        .iter()
        .all(|point| *point == ChargePoint::NormalizeRecord));
    assert!(admitted[3..]
        .iter()
        .all(|point| *point == ChargePoint::NormalizeUnsuppliedItem));
}

/// PR #140 F5 / TC-195 N10: the three `invalid_mutations` named "refused by
/// the semantic check" over an already-constructed effective declaration or
/// view. The other three N10 mutations (`stale-digest`,
/// `cross-domain-producer-digest`, `owner-as-producer-key`) are "refused by
/// schema" against the wire `model-effective-declaration.schema.json` this
/// rung does not decode from wire bytes (`Bundle`'s fields are already typed
/// Rust, not JSON); they are honestly uncovered here for that reason.
#[trace("TC-195")]
#[test]
fn n10_unsorted_derivation_refuses_by_the_semantic_check() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let type_b = find(&view, "b9953c43");
    let mut mutated = type_b.preimage.clone();
    assert!(
        mutated.derivation.len() >= 2,
        "type B has a qualify fact plus at least one inherit fact"
    );
    mutated.derivation.swap(0, 1);
    let (cause, _detail) = mutated
        .validate_derivation()
        .expect_err("a derivation whose ordinals no longer match array position must be refused");
    assert_eq!(
        cause,
        ModelRefusalCause::UnsortedDerivation {
            original: mutated.original.clone(),
            position: 0,
            ordinal: 1,
        }
    );
}

#[trace("TC-195")]
#[test]
fn n10_duplicate_path_refuses_by_the_semantic_check() {
    let view = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    let member_d_x = find(&view, "13a71b44");
    let mut mutated = member_d_x.preimage.clone();
    assert_eq!(mutated.derivation.len(), 2, "both diamond paths retained");
    let duplicate_inputs = mutated.derivation[0].inputs.clone();
    mutated.derivation[1].inputs = duplicate_inputs;
    let (cause, _detail) = mutated
        .validate_derivation()
        .expect_err("a derivation retaining the same input path twice must be refused");
    assert_eq!(
        cause,
        ModelRefusalCause::DuplicatePath {
            original: mutated.original.clone(),
            earlier: 0,
            later: 1,
        }
    );
}

#[trace("TC-195")]
#[test]
fn n10_unsorted_view_refuses_by_the_semantic_check() {
    let mut view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    assert!(view.declarations.len() >= 2);
    view.declarations.swap(0, 1);
    let refusal = view
        .validate_order()
        .expect_err("a view whose declarations are no longer ascending must be refused");
    assert_eq!(
        refusal.cause,
        ModelRefusalCause::UnsortedView {
            at: view.declarations[1].effective_id.clone(),
        }
    );
    assert_eq!(
        refusal.code,
        quire_spec_language::diagnostic::Code::InvalidModelBinding
    );
}

/// PR #140 F12: the exact TC-195 N01 charge sequence and per-declaration JCS
/// lengths — four `normalize.record`; three phase-2 `normalize.fact`
/// (qualify A, qualify B, qualify A.x); one `normalize.cycle-check` then a
/// phase-3 `normalize.fact` (inherit B); one more phase-3 `normalize.fact`
/// (inherit B.x, no cycle-check — cycle-check is charged only for phase-3
/// type-level facts); four `(normalize.declaration, normalize.hash)` pairs
/// with JCS lengths 739 (A), 1380 (B), 864 (A.x), 1135 (B.x); then
/// `normalize.hash` of the universe (591) and of the view (5280) — 9989
/// hashed bytes and 20 work units total, verified against the running
/// preimage's own `jcs_bytes()`, not just against the meter's own bookkeeping.
#[trace("TC-195", "FR-150-AC-1", "FR-150-AC-8")]
#[test]
fn n01_charges_the_exact_ground_truth_sequence_in_order() {
    let (outcome, meter) = normalize_with_meter(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let view = match outcome {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected Completed, got {other:?}"),
    };

    use ChargePoint::{
        NormalizeCycleCheck, NormalizeDeclaration, NormalizeFact, NormalizeHash, NormalizeRecord,
    };
    let expected = vec![
        NormalizeRecord,
        NormalizeRecord,
        NormalizeRecord,
        NormalizeRecord,
        NormalizeFact,
        NormalizeFact,
        NormalizeFact,
        NormalizeCycleCheck,
        NormalizeFact,
        NormalizeFact,
        NormalizeDeclaration,
        NormalizeHash,
        NormalizeDeclaration,
        NormalizeHash,
        NormalizeDeclaration,
        NormalizeHash,
        NormalizeDeclaration,
        NormalizeHash,
        NormalizeHash,
        NormalizeHash,
    ];
    assert_eq!(meter.admitted_charges().to_vec(), expected);

    assert_eq!(find(&view, "f1cc59cd").preimage.jcs_bytes().len(), 739);
    assert_eq!(find(&view, "b9953c43").preimage.jcs_bytes().len(), 1380);
    assert_eq!(find(&view, "4c06822f").preimage.jcs_bytes().len(), 864);
    assert_eq!(find(&view, "e8a29d61").preimage.jcs_bytes().len(), 1135);

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_eq!(universe.jcs_bytes().len(), 591);

    assert_eq!(meter.consumed(LimitKind::HashedBytes), 9989);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 20);
}

/// PR #140 F12: TC-195 N02's fifteen `normalize.fact` charges (five phase-2
/// qualify facts for A, B, C, D and A.x; ten phase-3 inherit facts) and six
/// `normalize.cycle-check` charges (one per phase-3 type-level ancestor
/// path: B->A, C->A, D->B, D->C via B, D->C via C, D->A via each of D's two
/// two-hop paths through B and C — six total across the diamond), for fifty
/// work units and 27969 hashed bytes.
#[trace("TC-195", "FR-150-AC-4", "FR-150-AC-6")]
#[test]
fn n02_charges_fifteen_facts_and_six_cycle_checks() {
    let (outcome, meter) = normalize_with_meter(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));

    let admitted = meter.admitted_charges();
    let fact_count = admitted
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeFact)
        .count();
    let cycle_check_count = admitted
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeCycleCheck)
        .count();
    assert_eq!(fact_count, 15, "five qualify plus ten inherit facts");
    assert_eq!(
        cycle_check_count, 6,
        "one cycle-check per phase-3 type-level ancestor path"
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 50);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 27969);
}

/// TC-196 R01: a closing generalization cycle (`model.A` -> `model.B` ->
/// `model.A`) refuses `invalid_model_binding`/`specialization-cycle` naming
/// every contributing declaration in the cycle, rotated to start at its
/// least key (`model.A`), regardless of which type's own walk closes it
/// first — see `src/model/normalize.rs`'s module docs for this rung's own
/// recorded scope decision (the refusal's shape is reproduced exactly; the
/// exact six-`normalize.cycle-check`-charge accounting across both types'
/// walks is not).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r01_a_closing_generalization_cycle_names_the_full_rotated_chain() {
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.r01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.A-B", "model.A", "model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
        ],
    );
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: ProducerKey::fixture("model.A"),
                    via: ProducerKey::fixture("model.gen.B-A"),
                }
            );
            assert!(
                refusal.detail.contains("[model.A, model.B]"),
                "the chain must be rotated to start at its least key regardless of \
                 which type's own walk closes the cycle first, got: {}",
                refusal.detail
            );
            assert!(refusal.detail.contains("generalizes back to itself via"));
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// TC-196 R01, the prefix-plus-rotation shape review found the R01 test above
/// cannot catch: `model.A` -> `model.C`, `model.C` -> `model.B`,
/// `model.B` -> `model.C`. `model.A` is not part of the cycle at all — it is
/// only how the walk *reaches* it — so the listing must name just the cycle
/// itself, `[model.B, model.C]`, never the whole path from the walk's root
/// (`[model.A, model.C, model.B]`).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r01b_the_cycle_listing_excludes_a_type_that_only_leads_into_it() {
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.r01b"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            generalization("model.gen.A-C", "model.A", "model.C"),
            generalization("model.gen.C-B", "model.C", "model.B"),
            generalization("model.gen.B-C", "model.B", "model.C"),
        ],
    );
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: ProducerKey::fixture("model.C"),
                    via: ProducerKey::fixture("model.gen.B-C"),
                }
            );
            assert!(
                refusal.detail.contains("[model.B, model.C]"),
                "the listing must name only the cycle itself, excluding model.A, \
                 which only leads into it, got: {}",
                refusal.detail
            );
            assert!(
                !refusal.detail.contains("model.A"),
                "model.A is not part of the cycle and must not be named, got: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// F2 plus a single, uncontested redefiner owned by `B2` (`B2 <= A`,
/// `B2.x2 redefines A.x`) -- exactly
/// `fixture_n06_resolved`'s own diamond, reused so the exact `m + r` here
/// (`value-accounting.md:455`) is cross-checked against N06's own already
/// hand-verified `f(o)`/`m` values below, not derived from a fresh fixture.
fn fixture_single_redefiner_no_conflict() -> Bundle {
    let mut records = fixture_f1().records;
    records.push(object_type("model.B2"));
    records.push(generalization("model.gen.B2-A", "model.B2", "model.A"));
    records.push(field_member("model.B2.x2", "model.B2", "model.A"));
    records.push(redefinition(
        "model.redef.single",
        "model.B2",
        "model.B2.x2",
        "model.A.x",
    ));
    Bundle::new(ModelSelection::fixture("bundle.single-redefiner"), records)
}

/// `Owner` declares `n_parents` direct generalizations (one to `Base`, the
/// rest to unrelated, ancestor-less filler types) and a single field
/// (`Owner.x2 redefines Base.x`) with no other redefiner contesting
/// `Base.x` -- an uncontested redefinition group (`edges.len() == 1`) whose
/// owner's own breadth alone, before this fix, was enough to exceed
/// `MAX_CONFORMANCE_DEPTH` (128) computing a dominance closure no single
/// redefiner ever needs.
fn fixture_wide_ancestry_single_redefiner(n_parents: usize) -> Bundle {
    let mut records = vec![
        object_type("model.Base"),
        field_member("model.Base.x", "model.Base", "model.Base"),
    ];
    for i in 0..n_parents.saturating_sub(1) {
        records.push(object_type(&format!("model.P{i}")));
    }
    records.push(object_type("model.Owner"));
    records.push(generalization(
        "model.gen.Owner-Base",
        "model.Owner",
        "model.Base",
    ));
    for i in 0..n_parents.saturating_sub(1) {
        records.push(generalization(
            &format!("model.gen.Owner-P{i}"),
            "model.Owner",
            &format!("model.P{i}"),
        ));
    }
    records.push(field_member("model.Owner.x2", "model.Owner", "model.Base"));
    records.push(redefinition(
        "model.redef.wide",
        "model.Owner",
        "model.Owner.x2",
        "model.Base.x",
    ));
    Bundle::new(
        ModelSelection::fixture(format!("bundle.wide-{n_parents}")),
        records,
    )
}

/// TC-195 N06's own resolved diamond (`fixture_n06_resolved`, three
/// redefiners of `A.x` -- `B`, `C`, and the
/// dominating winner `D` -- own owners `B`/`C`/`D`) charges exactly one
/// `normalize.conflict-check` at `Σ (c − 1) × f(o)`
/// (`value-accounting.md:456`), not a flat one work unit per group:
///
/// - `c = 3` (three redefiners of `A.x` reachable at `D`'s own effective
///   view: `redef.B`, `redef.C`, `redef.D`).
/// - `f(B) = 2`: `B`'s own type-level derivation is one `qualify` fact plus
///   one `inherit` fact for its single ancestor path `B -> A`.
/// - `f(C) = 2`: symmetric to `B`, one ancestor path `C -> A`.
/// - `f(D) = 5`: one `qualify` fact plus four `inherit` facts, one per
///   distinct ancestor path (`D -> B`, `D -> C`, `D -> A` via `B`, `D -> A`
///   via `C` -- the same four paths TC-195 N02's own ground truth charges
///   four of its six `normalize.cycle-check` charges against).
/// - `Σ (c − 1) × f(o) = (3 − 1) × (f(B) + f(C) + f(D)) = 2 × (2 + 2 + 5)
///   = 18`.
///
/// `normalize.redefinition-check` (`value-accounting.md:455`) charges
/// `m + r` once per redefinition record in the whole bundle, ascending by
/// the record's own producer key -- never once per (record, effective type
/// reaching it) pair (QSL #145): `redef.B`
/// (`r = 0`, `m(B) = 2`) charges `2`; `redef.C` (`r = 1`, `m(C) = 2`)
/// charges `3`; `redef.D` (`r = 2`, `m(D) = 4`: `D`'s own effective members
/// are `A.x`, `B.x2`, `C.x3`, all inherited, plus its own direct `D.x4`)
/// charges `6`. Three charges, `2 + 3 + 6 = 11` total -- not five charges
/// re-examining `redef.B`/`redef.C` a second time at `D`'s own pass, which
/// is what a per-(type, record) charge wrongly did before this fix.
///
/// Cross-checked against the crate by running it directly: with every
/// other limit unlimited, `work_units = 54` (43 for every phase-2/3
/// `normalize.record`/`normalize.fact`/`normalize.cycle-check` charge, plus
/// the three `normalize.redefinition-check` charges' own `2 + 3 + 6 = 11`
/// work) is exactly enough to admit every charge up to and including the
/// last `normalize.redefinition-check`. It no longer denies at
/// `normalize.conflict-check` there (QSL #169): ten phase-4 redefine facts
/// -- two per redefinition record reaching the type that resolves it,
/// `2` at `B` + `2` at `C` + `6` at `D` (see
/// `n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks`
/// below for the full count) -- are now charged as `normalize.fact` first,
/// so `54` denies at the first of those instead. `work_units = 64`
/// (`54 + 10`) is the boundary that now lands exactly before
/// `normalize.conflict-check`, whose reported `next_charge` is still
/// exactly `18`.
///
/// Revert probe: reverting the `Σ (c − 1) × f(o)` charge back to a flat,
/// unconditional `Charge::new(ChargePoint::NormalizeConflictCheck)` (no
/// `.work(...)` override, i.e. PR #167's own pre-fix shape) makes both
/// assertions below fail -- the exact-bound one because `work_units = 64`
/// then completes outright (a flat charge of 1 fits), and the total
/// because `110` no longer matches. Confirmed by hand: reintroducing that
/// exact one-line regression locally reproduces both failures, then
/// removing it again restores this test to green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_conflict_check_charges_exactly_sigma_c_minus_1_times_f_o() {
    let bundle = fixture_n06_resolved();

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        3,
        "one normalize.redefinition-check per redefinition record in the \
         whole bundle -- redef.B, redef.C and redef.D each examined exactly \
         once, ascending by the record's own producer key, never once per \
         (record, effective type reaching it) pair"
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        1,
        "exactly one contested group (A.x, reachable at D) across the whole build"
    );
    // Includes the ten phase-4 redefine facts' own `normalize.fact` charges
    // (see `n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks`
    // for their derivation).
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 110);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 64;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 64);
            assert_eq!(incomplete.consumed, 64);
            assert_eq!(incomplete.next_charge, 18);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// Phase-4 redefine facts (`RULE_REDEFINE`) are themselves derivation facts
/// and are charged as `normalize.fact`
/// (`value-accounting.md:453`: "each derivation fact before it is formed:
/// phase 2, then phase 3, then phase 4 ... `derivation_facts=k`"), in the
/// order `value-accounting.md:455`/`:456` fix relative to phase 4's own two
/// checks: `normalize.redefinition-check` fires "before its first
/// `normalize.fact`" and `normalize.conflict-check` fires "after its last
/// `normalize.fact`" -- so the phase-4 charge sequence is
/// redefinition-check, then every phase-4 `normalize.fact` (one charge per
/// fact, continuing the same `derivation_facts` running total phase 2/3
/// already charge), then conflict-check.
///
/// `fixture_n06_resolved` (reused from
/// `n06_conflict_check_charges_exactly_sigma_c_minus_1_times_f_o` above,
/// same hand-verified `m`/`f(o)` values) produces exactly ten phase-4
/// redefine facts -- two per redefinition record reaching the type that
/// resolves it (`quire.model.normalize.redefine/v1`,
/// `model-complete.md:231`: "one fact on (T, redefining feature) and one on
/// (T, redefined feature)"), for every record reaching that type, contested
/// or not:
///
/// - at `B`: `redef.B` reaches only `B` -- 2 facts (`B.x2`, `A.x`).
/// - at `C`: `redef.C` reaches only `C` -- 2 facts (`C.x3`, `A.x`).
/// - at `D`: `redef.B`, `redef.C` and `redef.D` all reach `D` -- 3 records
///   x 2 facts = 6 facts (`B.x2`, `A.x`, `C.x3`, `A.x`, `D.x4`, `A.x`).
///
/// Total: `2 + 2 + 6 = 10`.
///
/// Cross-checked against the crate: a completed (`UNLIMITED`) run charges
/// `work_units = 110` and peaks at `derivation_facts = 30` (the `20`
/// phase-2/3 facts plus these ten). `work_units = 109` is exactly one short
/// of that total, `Incomplete` at the run's own last charge; `work_units =
/// 110` completes.
///
/// Revert probe: dropping the phase-4 `normalize.fact` charge loop out of
/// `charge_all` makes `work_units` read `100`, not `110`, and moves the
/// `work_units = 54` boundary's denial to `normalize.conflict-check` --
/// confirmed by hand: reverting the change locally reproduces both,
/// restoring it returns this test to green.
#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks() {
    let bundle = fixture_n06_resolved();

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    match outcome {
        NormalizeOutcome::Completed(view) => assert_eq!(view.declarations.len(), 13),
        other => panic!("expected Completed, got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 110);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 30);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 54;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 54,
                    consumed: 54,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                },
                "the first phase-4 normalize.fact charge, never once with a \
                 flat cost other than one work unit"
            );
        }
        other => {
            panic!("expected Incomplete at the first phase-4 normalize.fact charge, got {other:?}")
        }
    }

    // Exact-bound pair (FR-150-AC-8): one short of the full charge total
    // denies at the run's own last charge; the full total completes.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 109;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 109,
                    consumed: 109,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeHash,
                }
            );
        }
        other => panic!("expected Incomplete one work unit short of completion, got {other:?}"),
    }
    limits.work_units = 110;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Completed(view) => assert_eq!(view.declarations.len(), 13),
        other => panic!("expected Completed at work_units=110, got {other:?}"),
    }
}

/// A redefinition target with only one redefiner reaching it (`c = 1`)
/// admits zero `normalize.conflict-check`
/// charges -- `value-accounting.md:456`'s own `c >= 2` condition -- and no
/// dominance closure is ever computed for it, rather than the pre-fix
/// shape that walked one unconditionally regardless of `edges.len()`.
///
/// Cross-checked by running the crate directly: with every other limit
/// unlimited, this bundle completes at exactly `work_units = 39` (`37`
/// before QSL #169: the single redefinition record still produces its own
/// two phase-4 redefine facts -- one on `B2.x2`, one on the redefined
/// `A.x` -- now charged as `normalize.fact`, `+2`), and its
/// one `normalize.redefinition-check` charge is exactly `2` (`m = 2`:
/// `B2`'s own effective members are `A.x`, inherited, and `B2.x2`, direct;
/// `r = 0`: the only redefinition record examined in this build).
///
/// Revert probe: reverting the `edges.len() < 2` guard in
/// `apply_redefinitions` back to computing `ensure_owner_closures` and a
/// `Σ (c − 1) × f(o)` charge unconditionally makes the conflict-check-count
/// assertion fail (it becomes 1, charged at `(1 − 1) × f(B2) = 0` work
/// units under the new formula, or a flat 1 under the older pre-#167
/// shape) -- confirmed by hand: removing the guard locally reproduces the
/// failure, restoring it returns this test to green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_a_single_redefiner_admits_no_conflict_check_charge() {
    let bundle = fixture_single_redefiner_no_conflict();

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        1
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        0,
        "a single redefiner has nothing to dominate and admits no conflict-check charge"
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 39);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 19;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 19);
            assert_eq!(incomplete.next_charge, 2);
            assert_eq!(
                incomplete.charge_point,
                ChargePoint::NormalizeRedefinitionCheck
            );
        }
        other => panic!("expected Incomplete at normalize.redefinition-check, got {other:?}"),
    }
}

/// Before this fix, resolving a redefinition group unconditionally
/// computed every contesting owner's ancestor-dominance closure -- even a
/// group with a single, uncontested
/// redefiner, which has nothing to dominate. A redefiner's owner with 128
/// or more *direct* generalizations (never a deep chain; a single wide
/// fan-out is enough) hit `crate::model::conformance`'s own
/// `MAX_CONFORMANCE_DEPTH` (128) ceiling computing that unneeded closure,
/// wrongly refusing `conformance-depth` on a bundle with no actual
/// dominance question to resolve. Both 128 (the exact boundary: 128 direct
/// generalizations plus the owner itself is the 129th node the walk would
/// visit) and 200 (comfortably over) must complete cleanly and quickly.
///
/// Revert probe: reverting the `edges.len() < 2` guard in
/// `apply_redefinitions` (skipping `ensure_owner_closures` entirely for a
/// single-edge group) back to calling it unconditionally reproduces the
/// original refusal at both widths -- confirmed by hand: removing the
/// guard locally makes this test fail with a `conformance-depth` refusal
/// at both 128 and 200, restoring it returns this test to green.
#[trace("TC-195", "TC-196", "FR-151-AC-2")]
#[test]
fn n06_wide_ancestry_with_a_single_uncontested_redefiner_completes() {
    for n_parents in [128usize, 200usize] {
        let bundle = fixture_wide_ancestry_single_redefiner(n_parents);
        let start = std::time::Instant::now();
        let outcome = normalize(&bundle, ModelNormalizationLimits::UNLIMITED);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "normalize took {elapsed:?} for {n_parents} direct generalizations \
             with a single uncontested redefiner"
        );
        let view = match outcome {
            NormalizeOutcome::Completed(view) => view,
            other => panic!(
                "expected Completed for {n_parents} direct generalizations, got {other:?} \
                 (an uncontested redefiner's owner breadth alone must never \
                 force a dominance-closure walk)"
            ),
        };

        let winner = view
            .declarations
            .iter()
            .find(|entry| {
                entry.preimage.original.identity == "model.Owner.x2"
                    && entry.preimage.owner_effective_type.is_some()
            })
            .unwrap_or_else(|| {
                panic!("no declaration for model.Owner.x2 in {n_parents}-parent view")
            });
        assert!(
            winner.visible,
            "Owner.x2 is the only redefiner and must win outright"
        );
    }
}

/// The `edges.len() < 2` guard above only ever fixed the *uncontested*
/// wide-ancestry case. `Owner` (`O`) still declares `n_parents` direct
/// generalizations, one of them to `Base` (`G000`), but now `G000` itself
/// also declares a second field, `G000.w`, that redefines `G000.x` — a
/// genuine two-redefiner contest (`c = 2`: `O.z` and `G000.w`) whose
/// dominance resolution, before this fix, still built `G000`'s own
/// [`crate::model::conformance::ancestor_closure`] and exceeded
/// `MAX_CONFORMANCE_DEPTH` on `O`'s breadth alone, exactly as the
/// uncontested case did. `O` is a proper descendant of `G000` (one of its
/// `n_parents` direct generalizations), so `O` dominates `G000` and `O.z`
/// wins outright.
fn fixture_wide_ancestry_contested_redefiners(n_parents: usize) -> Bundle {
    let mut records = vec![
        object_type("model.G000"),
        field_member("model.G000.x", "model.G000", "model.G000"),
        field_member("model.G000.w", "model.G000", "model.G000"),
        redefinition(
            "model.redef.w",
            "model.G000",
            "model.G000.w",
            "model.G000.x",
        ),
    ];
    for i in 1..n_parents {
        records.push(object_type(&format!("model.G{i:03}")));
    }
    records.push(object_type("model.O"));
    records.push(generalization("model.gen.O-G000", "model.O", "model.G000"));
    for i in 1..n_parents {
        records.push(generalization(
            &format!("model.gen.O-G{i:03}"),
            "model.O",
            &format!("model.G{i:03}"),
        ));
    }
    records.push(field_member("model.O.z", "model.O", "model.G000"));
    records.push(redefinition(
        "model.redef.z",
        "model.O",
        "model.O.z",
        "model.G000.x",
    ));
    Bundle::new(
        ModelSelection::fixture(format!("bundle.wide-contested-{n_parents}")),
        records,
    )
}

/// A wide (128+ direct generalizations) but genuinely *contested* ancestry
/// must resolve by dominance exactly as a
/// narrow one would, not refuse `conformance-depth` — the breadth-vs-depth
/// defect the `edges.len() < 2` guard alone left unfixed for `c >= 2`
/// groups.
///
/// Revert probe: reverting phase 4's dominance lookup from
/// `owner_ancestor_sets` (derived from phase 3's own `type_paths`) back to
/// `ensure_owner_closures`/`ancestor_closure`'s own bounded walk reproduces
/// the original `conformance-depth` refusal at both widths — confirmed by
/// hand: reintroducing that walk locally makes this test fail at both 128
/// and 200, restoring the fix returns it to green.
#[trace("TC-195", "TC-196", "FR-151-AC-2")]
#[test]
fn n06_wide_ancestry_with_two_contesting_redefiners_completes() {
    for n_parents in [128usize, 200usize] {
        let bundle = fixture_wide_ancestry_contested_redefiners(n_parents);
        let start = std::time::Instant::now();
        let outcome = normalize(&bundle, ModelNormalizationLimits::UNLIMITED);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "normalize took {elapsed:?} for {n_parents} direct generalizations \
             with two contesting redefiners"
        );
        let view = match outcome {
            NormalizeOutcome::Completed(view) => view,
            other => panic!(
                "expected Completed for {n_parents} direct generalizations, got {other:?} \
                 (a genuinely contested wide ancestry must resolve by \
                 dominance, not refuse conformance-depth)"
            ),
        };

        if n_parents == 128 {
            assert_eq!(
                view.declarations.len(),
                134,
                "129 type declarations (G000..G127, O) plus 5 member \
                 declarations (G000.x and G000.w at G000's own view; O's own \
                 inherited x and w plus its direct z)"
            );
        }

        let owner_o = view
            .declarations
            .iter()
            .find(|entry| {
                entry.preimage.owner_effective_type.is_none()
                    && entry.preimage.original.identity == "model.O"
            })
            .unwrap_or_else(|| panic!("no type declaration for model.O in {n_parents}-parent view"))
            .effective_id
            .clone();

        let winner = find_member(&view, &owner_o, "model.O.z");
        assert!(
            winner.visible,
            "O properly dominates G000, so O.z must win over G000.w"
        );
        let loser = find_member(&view, &owner_o, "model.G000.w");
        assert!(
            !loser.visible,
            "G000.w loses to the more-derived O.z at O's own effective view"
        );
    }
}

/// An operation-member redefinition record is charged by
/// `normalize.redefinition-check` exactly like a field one, and
/// its owner's effective-member count `m` includes operation members —
/// `apply_redefinitions` itself still skips operation-member redefinition
/// for conflict *resolution* (see the module docs; `crate::model::conformance`
/// resolves that directly), so this checks the charge alone.
fn fixture_operation_redefinition() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.op-redef"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            operation_member("model.A.op", "model.A"),
            operation_member("model.B.op2", "model.B"),
            redefinition("model.redef.op", "model.B", "model.B.op2", "model.A.op"),
        ],
    )
}

/// Cross-checked against the crate by running it directly: `B`'s own
/// effective members are `A.op` (inherited) and `B.op2` (direct), so
/// `m(B) = 2`; this is the only redefinition record in the bundle, so
/// `r = 0`, and `normalize.redefinition-check` charges exactly `2`.
///
/// Revert probe: reverting the `m` computation to count only
/// `member_counts_by_owner`'s pre-fix (field-only) tally, with no
/// operation-member contribution, makes both assertions below fail: the
/// total drops from `18` to `16` (the redefinition-check charge drops from
/// `2` to `0`), and `work_units = 10` no longer denies at
/// `normalize.redefinition-check` (`next_charge` drops to `0`) — confirmed
/// by hand: removing the operation-member contribution locally reproduces
/// both failures, restoring it returns this test to green.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_redefinition_is_charged_like_a_field_redefinition() {
    let bundle = fixture_operation_redefinition();

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        1,
        "the operation redefinition record is charged even though \
         apply_redefinitions itself never resolves it"
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        0,
        "operation-member redefinition resolution stays out of scope for \
         apply_redefinitions (see the module docs); only the charge changed"
    );
    assert_eq!(
        meter.consumed(LimitKind::WorkUnits),
        18,
        "6 normalize.record + 2 normalize.fact (A/B qualify) + 1 \
         normalize.cycle-check + 1 normalize.fact (B's own inherit-A path) \
         + 1 normalize.redefinition-check (m + r = 2 + 0) + 2 \
         normalize.declaration + 2 normalize.hash (per type declaration) + \
         2 normalize.hash (universe/view) = 18; no member declarations at \
         all, since operation members never enter member_preimages"
    );

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 10;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 10);
            assert_eq!(
                incomplete.next_charge, 2,
                "m(B) = 2 (A.op inherited, B.op2 direct), r = 0"
            );
            assert_eq!(
                incomplete.charge_point,
                ChargePoint::NormalizeRedefinitionCheck
            );
        }
        other => panic!("expected Incomplete at normalize.redefinition-check, got {other:?}"),
    }
}

/// QSL #145: `B` declares two operation members, `B.op2` and `B.op3`, both
/// redefining the identical inherited
/// operation `A.op` — the operation-member analog of `fixture_n06_conflict`'s
/// field contest, except `B` is the only owner (nothing dominates anything;
/// this fixture is only about the `c >= 2` *charge*, never resolution).
fn fixture_operation_redefinition_conflict() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.op-redef-conflict"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            operation_member("model.A.op", "model.A"),
            operation_member("model.B.op2", "model.B"),
            operation_member("model.B.op3", "model.B"),
            redefinition("model.redef.op2", "model.B", "model.B.op2", "model.A.op"),
            redefinition("model.redef.op3", "model.B", "model.B.op3", "model.A.op"),
        ],
    )
}

/// `value-accounting.md:456` prices every `(effective type, redefined
/// member)` reached by `c >= 2` redefinition records, and an operation
/// member is a redefined member same as a field one — `apply_redefinitions`
/// now charges a contested operation-redefinition group exactly like a
/// contested field group, even though it still never resolves which
/// operation redefiner wins (that stays `crate::model::conformance`'s job;
/// see the module docs, and QSL #173 for the still-open detection gap).
///
/// Cross-checked by hand: `f(A) = 1` (`A`'s own qualify fact only), `f(B) =
/// 2` (qualify plus its own inherit-`A` fact) — both types are otherwise
/// identical to `fixture_operation_redefinition`'s. The contested group at
/// `A.op` has `c = 2` edges, both owned by `B`, so `Σ (c − 1) × f(o) = (2 −
/// 1) × (f(B) + f(B)) = 1 × (2 + 2) = 4`.
///
/// Revert-probe: removing the operation-group charging loop in
/// `apply_redefinitions` (which never touches `member_preimages`/`hidden`)
/// drops the `NormalizeConflictCheck`
/// count to `0` and the `work_units` floor below no longer denies at that
/// charge point — confirmed by hand: removing the loop locally reproduces
/// both failures, restoring it returns this test to green.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_redefinition_group_with_two_or_more_redefiners_is_charged_a_conflict_check() {
    let bundle = fixture_operation_redefinition_conflict();

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        1,
        "the contested operation-redefinition group is charged once, even \
         though apply_redefinitions never resolves which operation \
         redefiner wins"
    );
    assert_eq!(
        meter.consumed(LimitKind::WorkUnits),
        29,
        "8 normalize.record + 2 normalize.fact (A/B qualify) + 1 \
         normalize.cycle-check + 1 normalize.fact (B's own inherit-A path) \
         + 2 normalize.redefinition-check (m + r = 3 + 0, then 3 + 1) + 1 \
         normalize.conflict-check ((c-1) * (f(B)+f(B)) = 1 * 4 = 4) + 2 \
         normalize.declaration + 2 normalize.hash (per type declaration) + \
         2 normalize.hash (universe/view) = 29; no member declarations at \
         all, since operation members never enter member_preimages"
    );

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 19;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 19);
            assert_eq!(
                incomplete.next_charge, 4,
                "(c-1) * (f(B)+f(B)) = 1 * (2+2) = 4"
            );
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// `value-accounting.md:456` prices every contested `(effective type,
/// redefined member)` in one ascending pass "by effective member key" --
/// field and operation targets
/// interleaved by that one key, never field targets charged as a block
/// before operation targets as a block. `C <= B <= A` (a two-step
/// generalization chain, so `f(A) = 1`, `f(B) = 2`, `f(C) = 3`):
///
/// - Field `A.z` is redefined by `B.z1` (owner `B`) and `C.z2` (owner `C`).
///   `C` is a proper descendant of `B`, so this contest resolves (`C.z2`
///   wins) without a refusal, but a resolved contest still owes its charge
///   (`c >= 2` is the only condition, `value-accounting.md:456`). `c = 2`,
///   `Σ (c − 1) × f(o) = (2 − 1) × (f(B) + f(C)) = 1 × (2 + 3) = 5`.
/// - Operation `A.a` is redefined by `C.a2` and `C.a3`, both owned by `C`.
///   `apply_redefinitions` never resolves an operation contest (see the
///   module docs), but still charges it: `c = 2`,
///   `Σ (c − 1) × f(o) = (2 − 1) × (f(C) + f(C)) = 1 × (3 + 3) = 6`.
///
/// `model.A.a` sorts before `model.A.z` (identity bytes: `a` < `z`), so the
/// merged-and-sorted order charges the operation group (`6`) before the
/// field group (`5`) -- the reverse of the pre-fix order, which charged
/// every field group (here, just the one `5`) before any operation group
/// (`6`), regardless of which target key sorts first.
///
/// Cross-checked by running the crate directly: with every other limit
/// unlimited, this bundle completes at exactly `work_units = 95` (`89`
/// before QSL #169: the field redefinition group alone produces six phase-4
/// redefine facts, `+6` -- two at `B` for `redef.z1` reaching only `B`, and
/// four at `C` for `redef.z1`/`redef.z2` both reaching `C`; the operation
/// group's own two records contribute no facts at all, since operation
/// members never enter `member_preimages` and get no `Fact` here, only a
/// `normalize.conflict-check` charge -- see the module docs), and
/// `work_units = 64` (`58 + 6`) is exactly enough to admit every charge up
/// to and including these six phase-4 `normalize.fact` charges, denying at
/// the first `normalize.conflict-check` -- the operation group's `6`, not
/// the field group's `5`.
///
/// Revert probe: reverting `apply_redefinitions` back to each loop pushing
/// its own charge straight to `conflict_check_work` (the pre-fix shape)
/// makes the `work_units = 64` assertion fail -- `next_charge` becomes `5`
/// (the field group, charged first again) instead of `6` -- confirmed by hand:
/// reverting the two loops to push directly, locally, reproduces the
/// failure; restoring the collect-sort-push shape returns this test to
/// green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn conflict_check_charges_interleave_field_and_operation_groups_by_target_key() {
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.field-op-order"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            generalization("model.gen.C-B", "model.C", "model.B"),
            field_member("model.A.z", "model.A", "model.A"),
            field_member("model.B.z1", "model.B", "model.A"),
            field_member("model.C.z2", "model.C", "model.A"),
            redefinition("model.redef.z1", "model.B", "model.B.z1", "model.A.z"),
            redefinition("model.redef.z2", "model.C", "model.C.z2", "model.A.z"),
            operation_member("model.A.a", "model.A"),
            operation_member("model.C.a2", "model.C"),
            operation_member("model.C.a3", "model.C"),
            redefinition("model.redef.a2", "model.C", "model.C.a2", "model.A.a"),
            redefinition("model.redef.a3", "model.C", "model.C.a3", "model.A.a"),
        ],
    );

    let (outcome, meter) = normalize_with_meter(&bundle, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        2,
        "one contested group for A.z (field) and one for A.a (operation)"
    );
    // Includes the field redefinition group's own six phase-4 redefine
    // facts' `normalize.fact` charges (operation members derive no facts of
    // their own -- see the module docs).
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 95);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 64;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 64);
            assert_eq!(incomplete.consumed, 64);
            assert_eq!(
                incomplete.next_charge, 6,
                "model.A.a sorts before model.A.z, so the operation group's \
                 charge (6) is denied before the field group's (5)"
            );
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// A phase-3 refusal (`specialization-cycle`) wins over a phase-4 refusal
/// (`derivation-conflict`) when a bundle has
/// both, matching FR-150's "each normalization phase ... reports every
/// refusal it exposes in charge order; a phase that reports a refusal ends
/// checking." `fixture_n06_conflict` (`B`/`C`, two undominated redefiners
/// of `A.x`, with no `D` to resolve them — phase 4's own defect) plus an
/// unrelated `Y <-> Z` generalization cycle (phase 3's own defect, TC-196
/// R01) exercises this precedence directly: phase 3 runs, and refuses,
/// before phase 4 ever gets a turn.
fn fixture_n06_conflict_with_unrelated_cycle() -> Bundle {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("model.Y"));
    records.push(object_type("model.Z"));
    records.push(generalization("model.gen.Y-Z", "model.Y", "model.Z"));
    records.push(generalization("model.gen.Z-Y", "model.Z", "model.Y"));
    Bundle::new(ModelSelection::fixture("bundle.n06-cycle"), records)
}

#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn phase3_specialization_cycle_refusal_wins_over_phase4_derivation_conflict() {
    match normalize(
        &fixture_n06_conflict_with_unrelated_cycle(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                quire_spec_language::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: ProducerKey::fixture("model.Y"),
                    via: ProducerKey::fixture("model.gen.Z-Y"),
                },
                "phase 3's own refusal must win over phase 4's undominated-\
                 redefiner derivation-conflict, matching FR-150's \
                 charge-order precedence"
            );
            assert!(
                refusal.detail.contains("[model.Y, model.Z]"),
                "detail must name the unrelated cycle, got: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// A phase-4 refusal (`derivation-conflict`/`redefinition-target`) does not
/// short-circuit `build()` before `charge_all` replays phase 4's own
/// charges. `value-accounting.md:481`'s "checking is exhaustive within a
/// stage" means every `normalize.redefinition-check`, every phase-4
/// `normalize.fact` and every `normalize.conflict-check` for
/// `fixture_n06_conflict` (`B.x2`/`C.x3`, undominated redefiners of `A.x`
/// reaching `D`) must be admitted before the `derivation-conflict` refusal
/// they expose is reported; a tighter limit that runs out first reports
/// that `Incomplete` instead, never the refusal.
///
/// `fixture_n06_conflict`'s exact phase-4 shape: `redef.B` reaches only `B`
/// (2 facts), `redef.C` reaches only `C` (2 facts), and both
/// `redef.B`/`redef.C` reach `D` (4 facts) -- eight phase-4 facts total. A
/// completed (`UNLIMITED`) run charges 27 `derivation_facts` (19 through
/// phase 2/3 plus these 8) and 57 `work_units` (40 through phase 3, `+5`
/// for the two `normalize.redefinition-check` charges, `+8` for the
/// phase-4 facts, `+4` for the one `normalize.conflict-check` group) before
/// reporting the refusal.
///
/// Revert probe: hand-reverting `build()` to return the phase-4 refusal
/// directly makes every case below fail -- `derivation_facts = 19` reports
/// `Refused` instead of `Incomplete`, and every `work_units` case in
/// `16..=56` reports `Refused` instead of `Incomplete` -- confirmed by
/// hand: reverting `build`/`charge_all` locally reproduces every failure,
/// restoring them returns this test to green.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_conflict_refusal_waits_for_every_phase4_charge_to_admit() {
    let bundle = fixture_n06_conflict();

    // A `derivation_facts` budget that exhausts exactly at the 19th
    // phase-2/3 fact -- one short of the first phase-4 fact -- reports
    // `Incomplete` at that first phase-4 `normalize.fact`, never the
    // `derivation-conflict` refusal `D`'s own resolution would otherwise
    // expose.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.derivation_facts = 19;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::DerivationFacts,
                    limit: 19,
                    consumed: 19,
                    next_charge: 20,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at the first phase-4 normalize.fact, got {other:?}"),
    }

    // `work_units` in `45..=52` land inside the eight phase-4
    // `normalize.fact` charges, after both `normalize.redefinition-check`
    // charges and before the one `normalize.conflict-check` charge.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 48;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 48,
                    consumed: 48,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at a phase-4 normalize.fact, got {other:?}"),
    }

    // `work_units` in `16..=39` land inside phase 2/3's own facts, well
    // before phase 4 starts.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 30;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 30,
                    consumed: 30,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at a phase 2/3 normalize.fact, got {other:?}"),
    }

    // `work_units = 56` is one short of the one `normalize.conflict-check`
    // charge's own `work_units` price (`4`): 53 are already consumed (40
    // through phase 3, `+5` redefinition-check, `+8` phase-4 facts), so the
    // attempted 4-unit conflict-check charge would reach 57, one over the
    // limit -- `consumed` reports that pre-charge total, not the limit
    // itself.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 56;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 56,
                    consumed: 53,
                    next_charge: 4,
                    charge_point: ChargePoint::NormalizeConflictCheck,
                }
            );
        }
        other => {
            panic!("expected Incomplete at the normalize.conflict-check charge, got {other:?}")
        }
    }

    // `work_units = 57` is exactly enough to admit every phase-4 charge
    // (40 through phase 3, `+5` redefinition-check, `+8` phase-4 facts,
    // `+4` conflict-check), so the refusal these charges expose is finally
    // reported.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 57;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::DerivationConflict {
                        type_: ProducerKey::fixture("model.D"),
                        member: ProducerKey::fixture("model.A.x"),
                        redefiners: vec![
                            ProducerKey::fixture("model.B.x2"),
                            ProducerKey::fixture("model.C.x3"),
                        ],
                    },
                    detail: "type model.D has 2 undominated redefinitions of model.A.x: \
                              [model.gen.D-B, model.redef.B, model.A.x] and \
                              [model.gen.D-C, model.redef.C, model.A.x]"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(derivation-conflict) at work_units=57, got {other:?}"),
    }
}

/// The deferred-refusal treatment covers `RedefinitionUnreachable` -- a
/// redefinition record whose own target, or whose redefining member, is
/// not an effective member of its owner -- exactly like the dominance
/// refusals in `n06_conflict_refusal_waits_for_every_phase4_charge_to_admit`
/// above: `apply_redefinitions` holds it in `accounting.refusal` and moves
/// on to the next target group, rather than returning it directly out of
/// `build()`. `fixture_n06_conflict_with_unreachable_redefiner` adds type
/// `E` (no generalization) with `redef.E` (`E`, `E.y` redefines `A.x`):
/// since `E` does not inherit `A`, `A.x` is not an effective member of
/// `E`, so this redefinition's own target is unreachable, alongside `D`'s
/// own dominance conflict over the same `A.x` (from `fixture_n06_conflict`).
///
/// `E`'s own `RedefinitionUnreachable` wins over `D`'s `derivation-conflict`
/// under `UNLIMITED`: `record_phase4_refusal` ranks a
/// `normalize.redefinition-check`-stage refusal (`value-accounting.md:455`)
/// ahead of a `normalize.conflict-check`-stage refusal (`:456`), since every
/// redefinition-check charge precedes every phase-4 fact charge, which in
/// turn precedes every conflict-check charge -- `E`'s target check fails at
/// its own redefinition-check charge, long before `D`'s ambiguity is even
/// checked at its own later conflict-check charge, regardless of `D`
/// sorting before `E` in `type_keys`' ascending identity order.
///
/// Revert probe: hand-reverting the two `apply_redefinitions` sites that
/// hold this refusal back to an eager `return Err(...)` makes
/// `work_units = 30` report `Refused(RedefinitionUnreachable)` with zero
/// charges replayed instead of `Incomplete` -- confirmed by hand, then
/// restored.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_unreachable_redefinition_target_also_waits_for_every_phase4_charge() {
    let bundle = fixture_n06_conflict_with_unreachable_redefiner();

    // A `work_units` budget that runs out inside phase 2/3's own facts
    // reports `Incomplete` there, never `E`'s own `RedefinitionUnreachable`
    // refusal (which the pre-fix code would have returned immediately,
    // with zero charges replayed).
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 30;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 30,
                    consumed: 30,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at a phase 2/3 normalize.fact, got {other:?}"),
    }

    // Under `UNLIMITED`, every phase-4 charge is admitted, and `E`'s own
    // `RedefinitionUnreachable` -- ranked ahead of `D`'s conflict, since it
    // belongs to the earlier-charged redefinition-check stage -- is the
    // refusal reported (see the doc comment above).
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::DanglingReference,
                    cause: ModelRefusalCause::RedefinitionUnreachable {
                        member: ProducerKey::fixture("model.A.x"),
                        owner: ProducerKey::fixture("model.E"),
                    },
                    detail: "redefinition target model.A.x is not an effective member of model.E"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(RedefinitionUnreachable) for E, got {other:?}"),
    }

    // The exact `work_units` bound between the last-admitted phase-4 charge
    // and the refusal it exposes: `64` is one short of the one
    // `normalize.conflict-check` charge's own price (`4`, added to `61`
    // already consumed through phase 3, redefinition-check and every
    // phase-4 fact); `65` admits it and reports the refusal -- `E`'s
    // `RedefinitionUnreachable`, ranked ahead of `D`'s conflict (see the doc
    // comment above).
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 64;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 64,
                    consumed: 61,
                    next_charge: 4,
                    charge_point: ChargePoint::NormalizeConflictCheck,
                }
            );
        }
        other => {
            panic!("expected Incomplete at the normalize.conflict-check charge, got {other:?}")
        }
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 65;
    match normalize(&bundle, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::DanglingReference,
                    cause: ModelRefusalCause::RedefinitionUnreachable {
                        member: ProducerKey::fixture("model.A.x"),
                        owner: ProducerKey::fixture("model.E"),
                    },
                    detail: "redefinition target model.A.x is not an effective member of model.E"
                        .to_string(),
                }
            );
        }
        other => {
            panic!("expected Refused(RedefinitionUnreachable) at work_units=65, got {other:?}")
        }
    }
}

/// The conflict-check owner (`J`) sorts before the redefinition-check owner
/// (`K`) in `type_keys`' ascending order, yet `K`'s own
/// `RedefinitionUnreachable` -- a `normalize.redefinition-check`-stage
/// refusal (`value-accounting.md:455`) -- ranks ahead of `J`'s own
/// `derivation-conflict` -- a `normalize.conflict-check`-stage refusal
/// (`:456`) -- and is the refusal `build` reports, exactly as
/// `record_phase4_refusal` ranks them.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_redefinition_check_refusal_outranks_earlier_processed_conflict_check_refusal() {
    let bundle = fixture_conflict_check_owner_sorts_before_redefinition_check_owner();
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::DanglingReference,
                    cause: ModelRefusalCause::RedefinitionUnreachable {
                        member: ProducerKey::fixture("model.G.g"),
                        owner: ProducerKey::fixture("model.K"),
                    },
                    detail: "redefinition target model.G.g is not an effective member of model.K"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(RedefinitionUnreachable) for K, got {other:?}"),
    }
}

/// Two independent conflict-check-stage refusals: `B9`'s own dominance
/// conflict over `M.w` and `D`'s own dominance conflict over `A.x`
/// (`fixture_n06_conflict`). `normalize.conflict-check` charges per type,
/// in `type_keys` order, and only then by target within that type
/// (`value-accounting.md:456`; `model-complete.md:160` puts the owning
/// type first in the effective member key), and `B9` sorts before `D`, so
/// `B9`'s own conflict-check is charged first and `record_phase4_refusal`
/// ranks its refusal ahead of `D`'s.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_conflict_check_refusal_ranks_by_type_before_target() {
    let bundle = fixture_n06_conflict_with_a_second_diamond_sorting_first();
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::DerivationConflict {
                        type_: ProducerKey::fixture("model.B9"),
                        member: ProducerKey::fixture("model.M.w"),
                        redefiners: vec![
                            ProducerKey::fixture("model.M1.w1"),
                            ProducerKey::fixture("model.M2.w2"),
                        ],
                    },
                    detail: "type model.B9 has 2 undominated redefinitions of model.M.w: \
                              [model.gen.B9-M1, model.redef.M1, model.M.w] and \
                              [model.gen.B9-M2, model.redef.M2, model.M.w]"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(derivation-conflict) for B9, got {other:?}"),
    }
}

/// `E`'s own resolution and `Da`'s own resolution (`Da <- E`) both fail the
/// identical "target not an effective member" check for the identical
/// record, so both rank identically -- `Da` sorts before `E`, but the
/// refusal still names `E`, the record's own owning type, since the check
/// always ranks and names the record's own owner, never the resolving
/// `type_key`.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_unreachable_target_refusal_names_the_records_owning_type_not_a_tied_descendant() {
    let bundle = fixture_unreachable_target_reached_by_owner_and_an_earlier_sorted_descendant();
    match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: quire_spec_language::diagnostic::Code::DanglingReference,
                    cause: ModelRefusalCause::RedefinitionUnreachable {
                        member: ProducerKey::fixture("model.A.x"),
                        owner: ProducerKey::fixture("model.E"),
                    },
                    detail: "redefinition target model.A.x is not an effective member of model.E"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(RedefinitionUnreachable) naming E, got {other:?}"),
    }
}
