// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-195: model normalization provenance (FR-150).
//!
//! Fixtures F1/F2 and expected identities are taken verbatim from
//! `resources/complete-value/quire-specification/spec/test-cases/TC-195-model-normalization-provenance.md`
//! and its ground-truth vectors,
//! `resources/complete-value/quire-specification/proposals/checked-package-v2/model-effective-declaration-vectors.json`.

use ix_trace_rs::trace;
use quire_spec_language::model::accounting::{ChargePoint, LimitKind, ModelNormalizationLimits};
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord, RedefinitionRecord,
};
use quire_spec_language::model::key::{EffectiveId, ProducerKey, RULE_REDEFINE};
use quire_spec_language::model::normalize::{normalize, normalize_with_meter, NormalizeOutcome};

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
            assert_eq!(refusal.cause, "digest-domain-mismatch");
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
            assert_eq!(refusal.cause, "unknown-owner");
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
            assert_eq!(refusal.cause, "unknown-specific");
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
            assert_eq!(refusal.cause, "unknown-general");
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
            assert_eq!(refusal.cause, "unsupported-wire");
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
            assert_eq!(refusal.cause, "derivation-conflict");
            assert!(refusal.detail.contains("model.gen.D-B"));
            assert!(refusal.detail.contains("model.redef.B"));
            assert!(refusal.detail.contains("model.gen.D-C"));
            assert!(refusal.detail.contains("model.redef.C"));
            assert!(refusal.detail.contains("model.A.x"));
        }
        other => panic!("expected Refused, got {other:?}"),
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
            assert_eq!(refusal.cause, "unknown-member");
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
            assert_eq!(refusal.cause, "wrong-model-selection");
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
        assert_eq!(refusal.cause, "unsupplied-producer-record");
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
    assert_eq!(cause, "unsorted-derivation");
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
    assert_eq!(cause, "duplicate-path");
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
    assert_eq!(refusal.cause, "unsorted-view");
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
            assert_eq!(refusal.cause, "specialization-cycle");
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
            assert_eq!(refusal.cause, "specialization-cycle");
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
