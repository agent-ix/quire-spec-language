// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-189 collection kind algebra over the real `value` boundary (FR-144).

use ix_trace_rs::trace;
use quire_exact::EffectiveId;
use quire_exact::NodeKey;
use quire_exact::{
    admit_text, BoundViolation, IeeeWidth, IllTyped, IllTypedCause, Presence, TextPayload,
    TextProfile, TextType,
};
use quire_exact::{
    construct_collection, form_collection, CollectionType, Deferred, FieldValue, OptionValue,
    Value, ValueType,
};
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Incomplete, Integer, LimitKind, Meter, ObjectId,
    ObjectReference, Outcome, Refusal, ScalarLimits, UniverseId,
};
use quire_spec_language::family::FamilyOutcome;
use quire_spec_language::value::declaration::{
    CompositeDeclaration, CompositeShape, EqualityOperand, EqualityOperator, FieldDeclaration,
    ObjectTypeDeclaration, TypeEnvironment,
};
use quire_spec_language::value::enumeration::EnumMemberIndex;
use sha2::{Digest, Sha256};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn key(label: &str) -> NodeKey {
    NodeKey::from_digest(Sha256::digest(label.as_bytes()).into())
}

/// A model object type's effective-declaration identity (ADR-013 O-05): the
/// identity `Reference<T>` and `ObjectTypeDeclaration` carry.
fn object_type(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn collection_type(
    kind: CollectionKind,
    element: ValueType,
    minimum: u64,
    maximum: u64,
) -> CollectionType {
    CollectionType::new(
        kind,
        element,
        CardinalityBound::new(minimum, maximum).unwrap(),
    )
}

fn integers(kind: CollectionKind, maximum: u64) -> CollectionType {
    collection_type(kind, ValueType::Integer, 0, maximum)
}

/// Evaluate the constructor `kind[values]` against `collection_type`.
fn construct(
    collection_type: &CollectionType,
    values: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    let elements: Vec<Deferred<'_>> = values
        .into_iter()
        .map(|value| -> Deferred<'_> { Box::new(move |_: &mut Meter| Outcome::Completed(value)) })
        .collect();
    construct_collection(collection_type, elements, meter)
}

fn completed(collection_type: &CollectionType, values: Vec<Value>) -> Value {
    match construct(collection_type, values, &mut Meter::new(UNLIMITED)) {
        Outcome::Completed(value) => value,
        other => panic!("construction completes, not {other:?}"),
    }
}

fn elements(value: &Value) -> &[Value] {
    match value {
        Value::Collection(collection) => collection.elements(),
        other => panic!("a collection, not {other:?}"),
    }
}

fn values(values: &[i64]) -> Vec<Value> {
    values.iter().copied().map(int).collect()
}

fn equal(env: &TypeEnvironment, value_type: &ValueType, left: &Value, right: &Value) -> bool {
    let checked = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::typed(value_type.clone()),
            EqualityOperand::typed(value_type.clone()),
            &EnumMemberIndex::default(),
        )
        .unwrap();
    match checked.evaluate(left, right, &mut Meter::new(UNLIMITED)) {
        Outcome::Completed(result) => result,
        other => panic!("equality completes, not {other:?}"),
    }
}

fn out_of_bound(
    violation: BoundViolation,
    collection_type: &CollectionType,
    count: u64,
) -> Refusal {
    Refusal::CardinalityOutOfBound {
        violation,
        kind: collection_type.kind(),
        bound: collection_type.bound(),
        count,
    }
}

fn incomplete(
    limit_kind: LimitKind,
    limit: u64,
    consumed: u64,
    next_charge: i64,
    charge_point: ChargePoint,
) -> Outcome<Value> {
    Outcome::Incomplete(Incomplete {
        limit_kind,
        limit,
        consumed,
        next_charge: Integer::from(next_charge),
        charge_point,
    })
}

/// `Debug` comparison: completed values carry no `PartialEq`.
fn assert_outcome(actual: &Outcome<Value>, expected: &Outcome<Value>) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

#[trace("TC-189", "FR-144-AC-1")]
#[test]
fn c01_permutation_changes_only_sequence_and_ordered_set_equality() {
    let env = TypeEnvironment::default();
    for (kind, maximum, left, right, expected) in [
        (CollectionKind::Set, 2, vec![1, 2], vec![2, 1], true),
        (CollectionKind::Bag, 3, vec![1, 1, 2], vec![2, 1, 1], true),
        (CollectionKind::Sequence, 2, vec![1, 2], vec![2, 1], false),
        (CollectionKind::OrderedSet, 2, vec![1, 2], vec![2, 1], false),
    ] {
        let declared = integers(kind, maximum);
        let value_type = ValueType::collection(declared.clone());
        let left = completed(&declared, values(&left));
        let right = completed(&declared, values(&right));
        assert_eq!(
            equal(&env, &value_type, &left, &right),
            expected,
            "{kind:?}"
        );
    }
}

#[trace("TC-189", "FR-144-AC-2")]
#[test]
fn c02_duplicates_are_kept_by_sequence_and_bag_and_coalesced_by_sets() {
    let represented = |kind, occurrences: &[i64]| {
        let value = completed(&integers(kind, 3), values(occurrences));
        format!("{:?}", elements(&value))
    };
    let expect = |list: &[i64]| format!("{:?}", values(list));
    assert_eq!(
        represented(CollectionKind::Sequence, &[1, 1, 2]),
        expect(&[1, 1, 2])
    );
    assert_eq!(
        represented(CollectionKind::Set, &[1, 1, 2]),
        expect(&[1, 2])
    );
    // A bag lists each occurrence: `1` twice, `2` once.
    assert_eq!(
        represented(CollectionKind::Bag, &[1, 1, 2]),
        expect(&[1, 1, 2])
    );
    assert_eq!(
        represented(CollectionKind::OrderedSet, &[2, 1, 2]),
        expect(&[2, 1])
    );
}

#[trace("TC-189", "FR-144-AC-3")]
#[trace("TC-189", "FR-144-AC-5")]
#[test]
fn c03_bound_violations_refuse_after_the_bound_charge() {
    let set = integers(CollectionKind::Set, 2);
    assert_eq!(elements(&completed(&set, values(&[1, 1, 2]))).len(), 2);

    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&set, values(&[1, 2, 3]), &mut meter),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &set, 3)),
    );
    assert_eq!(
        meter.admitted_charges().last(),
        Some(&ChargePoint::CollectionBound)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::CollectionResultRetain));
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);

    let bag = integers(CollectionKind::Bag, 2);
    assert_outcome(
        &construct(&bag, values(&[1, 1, 2]), &mut Meter::new(UNLIMITED)),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &bag, 3)),
    );
    let sequence = collection_type(CollectionKind::Sequence, ValueType::Integer, 1, 3);
    let refusal = out_of_bound(BoundViolation::BelowMinimum, &sequence, 0);
    assert_outcome(
        &construct(&sequence, vec![], &mut Meter::new(UNLIMITED)),
        &Outcome::Refused(refusal),
    );
    assert_eq!(refusal.code(), Some("cardinality_out_of_bound"));
    assert_eq!(BoundViolation::BelowMinimum.as_str(), "below-minimum");
    assert_eq!(BoundViolation::AboveMaximum.as_str(), "above-maximum");
    assert!(CardinalityBound::new(3, 1).is_err());
}

/// The same three occurrences `[1, 1, 2]` count as three for a sequence and a
/// bag and as two for a set and an ordered set, so one `[3, 3]` bound admits
/// only the occurrence-counting kinds and one `[2, 2]` bound only the
/// member-counting kinds.
#[trace("TC-189", "FR-144-AC-5")]
#[test]
fn c03b_bound_counting_is_occurrences_or_unique_members_per_kind() {
    let occurrences = values(&[1, 1, 2]);
    for kind in CollectionKind::ALL {
        let counted = if kind.is_unique() { 2 } else { 3 };
        let admitting = collection_type(kind, ValueType::Integer, counted, counted);
        assert_eq!(
            elements(&completed(&admitting, occurrences.clone())).len(),
            usize::try_from(counted).unwrap()
        );

        // The other kind's count is out of this kind's bound, in the direction
        // unique-member coalescing moves it.
        let other = if kind.is_unique() { 3 } else { 2 };
        let refusing = collection_type(kind, ValueType::Integer, other, other);
        let violation = if kind.is_unique() {
            BoundViolation::BelowMinimum
        } else {
            BoundViolation::AboveMaximum
        };
        assert_outcome(
            &construct(&refusing, occurrences.clone(), &mut Meter::new(UNLIMITED)),
            &Outcome::Refused(out_of_bound(violation, &refusing, counted)),
        );
    }
}

#[trace("TC-189", "FR-144-AC-4")]
#[test]
fn c04_canonical_order_is_key_order() {
    let set = integers(CollectionKind::Set, 3);
    for source in [[3, 1, 2], [2, 3, 1]] {
        assert_eq!(
            format!("{:?}", elements(&completed(&set, values(&source)))),
            format!("{:?}", values(&[1, 2, 3]))
        );
    }

    let options = collection_type(
        CollectionKind::Set,
        ValueType::option(ValueType::Integer),
        0,
        2,
    );
    let none = OptionValue::none(ValueType::Integer);
    let one = OptionValue::present(ValueType::Integer, int(1)).unwrap();
    assert_eq!(
        format!(
            "{:?}",
            elements(&completed(&options, vec![one.clone(), none.clone()]))
        ),
        format!("{:?}", [none, one])
    );

    let text_type = TextType::new(0, 4, TextProfile::BinaryUtf8).unwrap();
    let text = |payload: &str| match admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).unwrap(),
        &text_type,
        &mut Meter::new(UNLIMITED),
    ) {
        Outcome::Completed(text) => Value::Text(text),
        other => panic!("text admits, not {other:?}"),
    };
    let texts = collection_type(CollectionKind::Set, ValueType::Text(text_type), 0, 2);
    assert_eq!(
        format!(
            "{:?}",
            elements(&completed(&texts, vec![text("b"), text("a")]))
        ),
        format!("{:?}", [text("a"), text("b")])
    );
}

fn holder_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                key("Holder"),
                "Holder",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "r",
                    ValueType::Reference(object_type("M::Obj")),
                    Presence::Required,
                )]),
            ),
            CompositeDeclaration::new(
                key("F"),
                "F",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "x",
                    ValueType::Float(IeeeWidth::Binary32),
                    Presence::Required,
                )]),
            ),
        ],
        [ObjectTypeDeclaration::new(
            object_type("M::Obj"),
            "Obj",
            vec![],
        )],
    )
    .unwrap()
}

fn universe_id(tag: &str) -> UniverseId {
    UniverseId::from_digest(Sha256::digest(tag.as_bytes()).into())
}

fn holder(env: &TypeEnvironment, universe: &str, identity: &str) -> Value {
    let reference = ObjectReference::new(
        universe_id(universe),
        object_type("M::Obj"),
        ObjectId::new(identity).unwrap(),
    );
    env.record(
        key("Holder"),
        vec![("r", FieldValue::Present(Value::Reference(reference)))],
    )
    .unwrap()
}

#[trace("TC-189", "FR-144-AC-4")]
#[trace("TC-189", "FR-144-AC-6")]
#[test]
fn c06_reference_holders_are_keyed_and_ieee_elements_are_ineligible() {
    let env = holder_environment();
    let holders = collection_type(
        CollectionKind::Set,
        ValueType::Composite(key("Holder")),
        0,
        2,
    );
    let (h1, h2) = (holder(&env, "u1", "h1"), holder(&env, "u1", "h2"));
    let hs = completed(&holders, vec![h2.clone(), h1.clone()]);
    assert_eq!(format!("{:?}", elements(&hs)), format!("{:?}", [h1, h2]));
    let hs_type = ValueType::collection(holders);
    assert!(env.check_type(&hs_type).is_ok());
    assert!(equal(&env, &hs_type, &hs, &hs));

    let ineligible = Err(IllTyped {
        cause: IllTypedCause::OperatorIneligible,
    });
    let floats = collection_type(
        CollectionKind::Set,
        ValueType::Float(IeeeWidth::Binary64),
        0,
        2,
    );
    let records = collection_type(CollectionKind::Bag, ValueType::Composite(key("F")), 0, 2);
    for declared in [floats, records] {
        assert_eq!(env.check_type(&ValueType::collection(declared)), ineligible);
    }
}

#[trace("TC-189", "FR-144-AC-7")]
#[trace("TC-189", "FR-144-AC-8")]
#[test]
fn c07_set_construction_charges_membership_comparisons_in_retention_order() {
    let set = integers(CollectionKind::Set, 3);
    let mut meter = Meter::new(UNLIMITED);
    let outcome = construct(&set, values(&[1, 2, 1]), &mut meter);
    let Outcome::Completed(value) = outcome else {
        panic!("construction completes");
    };
    assert_eq!(
        format!("{:?}", elements(&value)),
        format!("{:?}", values(&[1, 2]))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::CollectionElement,
            ChargePoint::CollectionElement,
            ChargePoint::CollectionElement,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionBound,
            ChargePoint::CollectionResultRetain,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 11);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 3);
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 3);
}

#[trace("TC-189", "FR-144-AC-7")]
#[trace("TC-189", "FR-144-AC-8")]
#[test]
fn c08_denied_construction_charges_expose_no_collection() {
    let set = integers(CollectionKind::Set, 3);
    for (limits, expected) in [
        (
            ScalarLimits {
                work_units: 4,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::WorkUnits,
                4,
                3,
                2,
                ChargePoint::CollectionMemberWalk,
            ),
        ),
        (
            ScalarLimits {
                work_units: 10,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::WorkUnits,
                10,
                10,
                1,
                ChargePoint::CollectionResultRetain,
            ),
        ),
        (
            ScalarLimits {
                value_occurrences: 2,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::ValueOccurrences,
                2,
                2,
                3,
                ChargePoint::CollectionResultRetain,
            ),
        ),
    ] {
        assert_outcome(
            &construct(&set, values(&[1, 2, 1]), &mut Meter::new(limits)),
            &expected,
        );
    }
}

#[trace("TC-189", "FR-144-AC-7")]
#[trace("TC-189", "FR-144-AC-8")]
#[test]
fn c09_sequence_and_rejected_set_charge_exact_work() {
    let sequence = integers(CollectionKind::Sequence, 3);
    let mut meter = Meter::new(UNLIMITED);
    assert!(matches!(
        construct(&sequence, values(&[1, 1]), &mut meter),
        Outcome::Completed(_)
    ));
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 3);

    let set = integers(CollectionKind::Set, 1);
    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&set, values(&[1, 2]), &mut meter),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &set, 2)),
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 6);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
}

#[trace("TC-189", "FR-144-AC-6")]
#[trace("TC-189", "FR-144-AC-7")]
#[test]
fn c10_reference_holder_sets_charge_pairs_and_refuse_foreign_universes() {
    let env = holder_environment();
    let holders = collection_type(
        CollectionKind::Set,
        ValueType::Composite(key("Holder")),
        0,
        3,
    );
    let (h1, h2) = (holder(&env, "u1", "h1"), holder(&env, "u1", "h2"));
    let mut meter = Meter::new(UNLIMITED);
    let Outcome::Completed(value) = construct(
        &holders,
        vec![h1.clone(), h2.clone(), h1.clone()],
        &mut meter,
    ) else {
        panic!("construction completes");
    };
    assert_eq!(
        format!("{:?}", elements(&value)),
        format!("{:?}", [&h1, &h2])
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 17);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 5);

    let hx = holder(&env, "u2", "hx");
    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&holders, vec![h1, hx], &mut meter),
        &Outcome::Refused(Refusal::ForeignReference),
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 6);
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::CollectionMemberTest));
}

#[trace("TC-189", "FR-144-AC-3")]
#[test]
fn c11_the_bound_is_part_of_the_collection_type() {
    let env = TypeEnvironment::default();
    let x = ValueType::collection(integers(CollectionKind::Set, 2));
    let s = ValueType::collection(integers(CollectionKind::Set, 3));
    assert_eq!(
        env.check_equality(
            EqualityOperator::Equal,
            EqualityOperand::typed(x),
            EqualityOperand::typed(s),
            &EnumMemberIndex::default(),
        ),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
}

#[trace("TC-189", "FR-144-AC-2")]
#[test]
fn formed_occurrences_outside_the_element_type_refuse_at_their_index() {
    let set = integers(CollectionKind::Set, 3);
    let refused = form_collection(
        &set,
        vec![int(1), Value::Boolean(true)],
        &mut Meter::new(UNLIMITED),
    )
    .unwrap_err();
    // `form_collection` is `quire_exact`'s own function now (QSL-131 V5), so
    // its refusal is the kernel's own `Component`, not QSL's name-keyed one.
    assert_eq!(refused.component, quire_exact::Component::Element(1));
}

/// Rows that need the FR-145/FR-146 checker and evaluator boundary.
mod checked {
    use super::*;
    use qsl_forms::{BinaryOperator, BinderQuery, Expression, TypeForm};
    use quire_exact::NODE_KEY_DOMAIN;
    use quire_spec_language::check::{
        CheckCause, CheckMode, CheckRefusal, CheckedExpression, CheckingLimits, EnumBinding,
        PackageDeclarations,
    };
    use quire_spec_language::model::object_environment::ObjectEnvironment;
    use quire_spec_language::value::enumeration::{
        EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
    };
    use quire_spec_language::value::{
        CheckedPackage, CheckedPackageEvaluation, NodeOwner, OwnerSelection, OwnerSubject,
        SemanticGraphCause,
    };
    use serde_json::json;

    fn name(spelling: &str) -> Expression {
        Expression::Name(spelling.to_owned())
    }

    fn literal(value: i64) -> Expression {
        Expression::Integer(Integer::from(value))
    }

    fn equal(left: Expression, right: Expression) -> Expression {
        Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn literal_collection(kind: CollectionKind, values: &[i64]) -> Expression {
        Expression::Collection {
            kind,
            elements: values.iter().copied().map(literal).collect(),
        }
    }

    fn exists(source: &str, body: Expression) -> Expression {
        Expression::Query {
            query: BinderQuery::Exists,
            binder: "x".to_owned(),
            source: Box::new(name(source)),
            body: Box::new(body),
        }
    }

    fn package(declarations: PackageDeclarations) -> CheckedPackage {
        let graph = declarations.check(CheckingLimits::default()).unwrap();
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        CheckedPackage::link(graph)
    }

    fn check(
        package: &CheckedPackage,
        parameters: &[(&str, ValueType)],
        expression: &Expression,
    ) -> Result<CheckedExpression, CheckRefusal> {
        package.graph().check_expression(
            parameters
                .iter()
                .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
                .collect(),
            expression,
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
    }

    fn ill_typed(
        package: &CheckedPackage,
        parameters: &[(&str, ValueType)],
        expression: &Expression,
    ) -> IllTypedCause {
        match check(package, parameters, expression).map(|_| ()) {
            Err(CheckRefusal {
                cause: CheckCause::IllTyped(cause),
                ..
            }) => cause,
            other => panic!("an ill_typed refusal, not {other:?}"),
        }
    }

    /// The completed value and the admitted charges of one evaluation.
    fn evaluate(
        package: &CheckedPackage,
        parameters: &[(&str, ValueType)],
        expression: &Expression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
    ) -> (Value, Vec<ChargePoint>) {
        let checked = check(package, parameters, expression).unwrap();
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = package
            .evaluate(&checked, arguments, objects, &mut meter)
            .unwrap();
        match evaluation.outcome {
            FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)) => {
                (value, meter.admitted_charges().to_vec())
            }
            other => panic!("a completed value, not {other:?}"),
        }
    }

    fn visits(charges: &[ChargePoint]) -> usize {
        charges
            .iter()
            .filter(|charge| **charge == ChargePoint::CollectionVisit)
            .count()
    }

    fn jcs(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Array(items) => {
                format!("[{}]", items.iter().map(jcs).collect::<Vec<_>>().join(","))
            }
            serde_json::Value::Object(members) => {
                let mut entries: Vec<_> = members.iter().collect();
                entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
                let body: Vec<_> = entries
                    .into_iter()
                    .map(|(key, item)| {
                        format!("{}:{}", serde_json::Value::from(key.as_str()), jcs(item))
                    })
                    .collect();
                format!("{{{}}}", body.join(","))
            }
            other => other.to_string(),
        }
    }

    fn preimage_key(preimage: &serde_json::Value) -> NodeKey {
        key_of_bytes(jcs(preimage).as_bytes())
    }

    fn key_of_bytes(bytes: &[u8]) -> NodeKey {
        NodeKey::from_digest(Sha256::digest(bytes).into())
    }

    fn owners() -> OwnerSelection {
        OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".into(),
            identity: "example-model".into(),
        })])
    }

    fn declaration_preimage(label: &str, ordered: bool, members: &[&str]) -> serde_json::Value {
        json!({
            "version": "quire.enum-declaration-node/v1",
            "owner": {"kind": "definition", "authority": "agent-ix", "identity": "example-model"},
            "qualified_declaration": ["Example", label],
            "ordered": ordered,
            "members": members,
        })
    }

    fn admit(label: &str, ordered: bool, members: &[&str]) -> EnumBinding {
        let preimage = declaration_preimage(label, ordered, members);
        let declaration = EnumDeclaration::admit(
            EnumDeclarationPreimage::from_json(preimage.clone()).unwrap(),
            preimage_key(&preimage),
            &owners(),
        )
        .unwrap();
        let members = members
            .iter()
            .map(|case| {
                let member = json!({
                    "version": "quire.enum-member-node/v1",
                    "declaration_node_id": {
                        "domain": NODE_KEY_DOMAIN,
                        "digest": declaration.key().to_string(),
                    },
                    "case": case,
                });
                declaration
                    .admit_member(
                        &EnumMemberPreimage::from_json(member.clone()).unwrap(),
                        preimage_key(&member),
                    )
                    .unwrap()
            })
            .collect();
        EnumBinding {
            name: label.to_owned(),
            declaration,
            members,
        }
    }

    fn member(binding: &EnumBinding, case: &str) -> Value {
        let member = binding
            .members
            .iter()
            .find(|member| member.case() == case)
            .unwrap();
        Value::Enum(quire_exact::EnumMember::new(
            member.variant(),
            u32::try_from(member.position()).unwrap(),
        ))
    }

    fn formed(value_type: &CollectionType, values: Vec<Value>) -> Value {
        quire_exact::form_collection(value_type, values, &mut Meter::new(UNLIMITED))
            .unwrap()
            .completed()
            .unwrap()
    }

    #[trace("TC-189", "FR-144-AC-7")]
    #[trace("TC-189", "FR-144-AC-9")]
    // Also TC-409 steps 3-4 (FR-088-AC-11): a real kernel Set formed from
    // admitted enum values visits an unordered enum's members in
    // case-identifier byte order and an ordered enum's in declaration order.
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn c05_enum_keys_order_unordered_members_by_identifier_bytes() {
        let color = admit("Color", false, &["blue", "green", "red"]);
        let level = admit("Level", true, &["high", "low"]);
        let colors = collection_type(CollectionKind::Set, ValueType::Enum(color.shape()), 0, 3);
        let levels = collection_type(CollectionKind::Set, ValueType::Enum(level.shape()), 0, 2);
        let package = package(PackageDeclarations {
            enums: vec![color.clone(), level.clone()],
            ..PackageDeclarations::default()
        });
        let c = formed(
            &colors,
            vec![
                member(&color, "red"),
                member(&color, "blue"),
                member(&color, "green"),
            ],
        );
        let l = formed(&levels, vec![member(&level, "low"), member(&level, "high")]);
        let parameters = [
            ("c", ValueType::collection(colors)),
            ("l", ValueType::collection(levels)),
        ];
        for (source, case, expected_visits) in [
            ("c", "Color::blue", 1),
            ("c", "Color::red", 3),
            ("l", "Level::high", 1),
        ] {
            let (value, charges) = evaluate(
                &package,
                &parameters,
                &exists(source, equal(name("x"), name(case))),
                vec![c.clone(), l.clone()],
                &ObjectEnvironment::default(),
            );
            assert_eq!(format!("{value:?}"), format!("{:?}", Value::Boolean(true)));
            assert_eq!(visits(&charges), expected_visits, "{case}");
        }

        // Reordering the source cases leaves one admitted declaration: an
        // unordered declaration node retains its members sorted, so a
        // declaration position never reaches the key.
        let reordered = declaration_preimage("Color", false, &["green", "red", "blue"]);
        assert_eq!(
            EnumDeclaration::admit(
                EnumDeclarationPreimage::from_json(reordered.clone()).unwrap(),
                preimage_key(&reordered),
                &owners(),
            )
            .unwrap_err()
            .cause,
            SemanticGraphCause::UnsortedUnorderedMembers
        );

        assert_eq!(
            ill_typed(
                &package,
                &[],
                &Expression::Binary {
                    operator: BinaryOperator::Less,
                    left: Box::new(name("Color::red")),
                    right: Box::new(name("Color::blue")),
                },
            ),
            IllTypedCause::OperatorIneligible
        );
    }

    #[trace("TC-189", "FR-144-AC-6")]
    #[test]
    fn c06_reference_keyed_holder_sets_answer_every_query_form() {
        let types = holder_environment();
        let package = package(PackageDeclarations {
            types: types.clone(),
            ..PackageDeclarations::default()
        });
        let holder_type = ValueType::Composite(key("Holder"));
        let holders = collection_type(CollectionKind::Set, holder_type.clone(), 0, 2);
        let (h1, h2) = (holder(&types, "u1", "h1"), holder(&types, "u1", "h2"));
        let hs = formed(&holders, vec![h2.clone(), h1.clone()]);
        let objects = ObjectEnvironment::new(
            &types,
            ["h1", "h2"].map(|identity| {
                (
                    ObjectReference::new(
                        universe_id("u1"),
                        object_type("M::Obj"),
                        ObjectId::new(identity).unwrap(),
                    ),
                    Vec::new(),
                )
            }),
        )
        .unwrap();
        let parameters = [("hs", ValueType::collection(holders)), ("h1", holder_type)];
        let run = |expression: Expression| {
            evaluate(
                &package,
                &parameters,
                &expression,
                vec![hs.clone(), h1.clone()],
                &objects,
            )
            .0
        };
        let ordered = format!("{:?}", [h1.clone(), h2.clone()]);
        let mapped = run(Expression::Query {
            query: BinderQuery::Map,
            binder: "x".to_owned(),
            source: Box::new(name("hs")),
            body: Box::new(name("x")),
        });
        assert_eq!(format!("{:?}", elements(&mapped)), ordered);
        let converted = run(Expression::Convert {
            target: TypeForm::collection(CollectionKind::Sequence, crate::support::type_form::SPAN)
                .with_arguments(vec![crate::support::type_form::named_type_form("Holder")])
                .with_bounds(vec!["0".to_owned(), "2".to_owned()]),
            operand: Box::new(name("hs")),
        });
        let Value::Collection(sequence) = &converted else {
            panic!("a collection");
        };
        assert_eq!(sequence.collection_type().kind(), CollectionKind::Sequence);
        assert_eq!(format!("{:?}", sequence.elements()), ordered);
        assert_eq!(
            format!("{:?}", run(Expression::Size(Box::new(name("hs"))))),
            format!("{:?}", int(2))
        );
        for expression in [
            Expression::Contains {
                collection: Box::new(name("hs")),
                item: Box::new(name("h1")),
            },
            equal(name("hs"), name("hs")),
        ] {
            assert_eq!(
                format!("{:?}", run(expression)),
                format!("{:?}", Value::Boolean(true))
            );
        }
    }

    #[trace("TC-189", "FR-144-AC-9")]
    #[test]
    fn c11_converting_to_the_other_bound_admits_equality_without_loss() {
        let package = package(PackageDeclarations::default());
        let x_type = collection_type(CollectionKind::Set, ValueType::Integer, 0, 2);
        let s_type = collection_type(CollectionKind::Set, ValueType::Integer, 0, 3);
        let parameters = [
            ("x", ValueType::collection(x_type.clone())),
            ("s", ValueType::collection(s_type.clone())),
        ];
        assert_eq!(
            ill_typed(&package, &parameters, &equal(name("x"), name("s"))),
            IllTypedCause::TypeMismatch
        );
        let widened = Expression::Let {
            name: "y".to_owned(),
            value: Box::new(Expression::Convert {
                target: crate::support::type_form::type_form(&ValueType::collection(
                    s_type.clone(),
                )),
                operand: Box::new(name("x")),
            }),
            body: Box::new(equal(name("y"), name("s"))),
        };
        let checked = check(&package, &parameters, &widened).unwrap();
        let losses = checked.losses();
        assert_eq!(losses.len(), 1);
        assert!(losses[0].discarded.is_empty());
        let (value, _) = evaluate(
            &package,
            &parameters,
            &widened,
            vec![
                formed(&x_type, values(&[1, 2])),
                formed(&s_type, values(&[1, 2])),
            ],
            &ObjectEnvironment::default(),
        );
        assert_eq!(format!("{value:?}"), format!("{:?}", Value::Boolean(true)));
    }

    #[trace("TC-189", "FR-144-AC-10")]
    #[test]
    fn c12_collection_literals_take_their_unique_expected_type() {
        let package = package(PackageDeclarations::default());
        let set = || literal_collection(CollectionKind::Set, &[2, 1]);
        assert_eq!(
            ill_typed(
                &package,
                &[],
                &equal(literal_collection(CollectionKind::Set, &[1, 2]), set()),
            ),
            IllTypedCause::AmbiguousLiteral
        );
        assert_eq!(
            ill_typed(
                &package,
                &[],
                &Expression::Let {
                    name: "z".to_owned(),
                    value: Box::new(literal_collection(CollectionKind::Set, &[1])),
                    body: Box::new(Expression::Size(Box::new(name("z")))),
                },
            ),
            IllTypedCause::AmbiguousLiteral
        );

        let s_type = collection_type(CollectionKind::Set, ValueType::Integer, 0, 2);
        let parameters = [("s", ValueType::collection(s_type.clone()))];
        let (value, _) = evaluate(
            &package,
            &parameters,
            &equal(name("s"), set()),
            vec![formed(&s_type, values(&[1, 2]))],
            &ObjectEnvironment::default(),
        );
        assert_eq!(format!("{value:?}"), format!("{:?}", Value::Boolean(true)));
        assert_eq!(
            ill_typed(
                &package,
                &parameters,
                &equal(
                    name("s"),
                    literal_collection(CollectionKind::Sequence, &[1, 2])
                ),
            ),
            IllTypedCause::TypeMismatch
        );
    }
}
