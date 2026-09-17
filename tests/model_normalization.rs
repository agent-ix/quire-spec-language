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
    ObjectTypeRecord,
};
use quire_spec_language::model::key::ProducerKey;
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

#[trace("TC-195", "FR-150-AC-4")]
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

#[trace("TC-195", "FR-150-AC-2")]
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
