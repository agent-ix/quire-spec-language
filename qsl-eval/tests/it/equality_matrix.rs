// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-194 complete equality matrix over the real `value` boundary (FR-149).
//!
//! Every equality is type-checked from the operands' declared types by
//! `TypeEnvironment::check_equality` and then evaluated over completed values.
//! Unit and enum node keys come from an independent RFC 8785 canonicalizer;
//! composite and object-type keys are opaque producer-assigned fixture keys.
//! E20's source-order row and E26's `let`-bound rows run through the FR-146
//! expression checker and evaluator, as do the scalar operator rows.

use std::cell::Cell;
use std::sync::OnceLock;

use ix_trace_rs::trace;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, Evaluation, LocatedLoss, ValueLoss};
use qsl_forms::{BinaryOperator, Expression, FieldInitializer, FunctionDeclaration};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckCause, CheckMode, CheckRefusal, CheckedExpression, CheckingLimits, Obligation,
    PackageDeclarations,
};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::declaration::{
    CheckedEquality, Component, CompositeDeclaration, CompositeShape, ConstructionCause,
    ConstructionRefusal, EqualityOperand, EqualityOperator, FieldDeclaration, FieldExpression,
    ObjectTypeDeclaration, TypeEnvironment,
};
use qsl_semantics::value::enumeration::{
    EnumDeclaration, EnumDeclarationPreimage, EnumMemberIndex, EnumMemberPreimage,
};
use qsl_semantics::value::quantity::UnitTable;
use qsl_semantics::value::{
    AdmittedIeeeProfile, CatalogRole, DefinitionLock, DefinitionReference, DefinitionRevision,
    DimensionPreimage, NodeOwner, OwnerSelection, OwnerSubject, UnitGraph, UnitPreimage,
};
use quire_exact::EffectiveId;
use quire_exact::EnumMember;
use quire_exact::NodeKey;
use quire_exact::{
    admit_text, compare_ieee, convert_ieee_width, Decimal, DecimalType, IeeeComparison,
    IeeeExactLoss, IeeeFlag, IeeeValue, IeeeWidth, IllTyped, IllTypedCause, ObjectId,
    ObjectReference, Outcome, Presence, Quantity, Rational, Refusal, RoundingMode, Text,
    TextPayload, TextProfile, TextType, Undefined, UnitDomain, UnitId, UniverseId,
};
use quire_exact::{
    form_collection, plan_equality, CollectionType, FieldValue, OptionValue, RationalDomain, Value,
    ValueType,
};
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Incomplete, InjectedDenial, Integer,
    IntegerInterval, LimitKind, Meter, ScalarLimits,
};
use serde_json::json;
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

const ZERO: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

// ---- fixtures --------------------------------------------------------------

/// Independent RFC 8785 JCS for the string/Boolean-only preimages.
fn jcs(value: &serde_json::Value) -> String {
    use serde_json::Value as Json;
    match value {
        Json::Null | Json::Bool(_) | Json::String(_) => value.to_string(),
        Json::Number(_) => panic!("preimages carry no JSON numbers"),
        Json::Array(items) => format!("[{}]", items.iter().map(jcs).collect::<Vec<_>>().join(",")),
        Json::Object(members) => {
            let mut entries: Vec<_> = members.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            let body: Vec<_> = entries
                .into_iter()
                .map(|(key, item)| format!("{}:{}", Json::String(key.clone()), jcs(item)))
                .collect();
            format!("{{{}}}", body.join(","))
        }
    }
}

fn hex(bytes: &[u8]) -> NodeKey {
    NodeKey::from_digest(Sha256::digest(bytes).into())
}

fn fixture_key(preimage: &serde_json::Value) -> NodeKey {
    hex(jcs(preimage).as_bytes())
}

/// An opaque producer-assigned declaration key.
fn key(label: &str) -> NodeKey {
    hex(label.as_bytes())
}

/// A model object type's effective-declaration identity (ADR-013 O-05): the
/// identity `Reference<T>` and `ObjectTypeDeclaration` carry.
fn object_type(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn owners() -> OwnerSelection {
    OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".into(),
        identity: "example-model".into(),
    })])
}

fn owner_json() -> serde_json::Value {
    json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"})
}

fn node_id(key: NodeKey) -> serde_json::Value {
    json!({"domain": "quire.checked-semantic-node/v1", "digest": key.to_string()})
}

/// Three declared units by their kernel ids, and the table that resolves
/// them.
struct Units {
    m: UnitId,
    cm: UnitId,
    s: UnitId,
    table: UnitTable,
}

fn units() -> Units {
    let dimension = |name: &str| {
        json!({
            "version": "quire.dimension-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "terms": [],
        })
    };
    let unit = |name: &str, dimension: NodeKey, target: Option<NodeKey>, scale: &str| {
        json!({
            "version": "quire.unit-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_id(dimension),
            "target_unit_node_id": target.map_or(serde_json::Value::Null, node_id),
            "scale": {"numerator": "1", "denominator": scale},
            "offset": {"numerator": "0", "denominator": "1"},
        })
    };
    let length = dimension("Length");
    let time = dimension("Time");
    let (length_key, time_key) = (fixture_key(&length), fixture_key(&time));
    let m = unit("metre", length_key, None, "1");
    let m_key = fixture_key(&m);
    let s = unit("second", time_key, None, "1");
    let cm = unit("centimetre", length_key, Some(m_key), "100");
    let keys = [fixture_key(&m), fixture_key(&cm), fixture_key(&s)];
    let graph = UnitGraph::admit(
        [length, time].into_iter().map(|preimage| {
            let key = fixture_key(&preimage);
            (DimensionPreimage::from_json(preimage).unwrap(), key)
        }),
        [m, cm, s].into_iter().map(|preimage| {
            let key = fixture_key(&preimage);
            (UnitPreimage::from_json(preimage).unwrap(), key)
        }),
        &owners(),
    )
    .unwrap();
    let declared = |key| graph.declared_unit_id(key).unwrap();
    Units {
        m: declared(keys[0]),
        cm: declared(keys[1]),
        s: declared(keys[2]),
        table: UnitTable::declared(&graph),
    }
}

fn enum_declaration(name: &str) -> EnumDeclaration {
    let preimage = json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": owner_json(),
        "qualified_declaration": ["Example", name],
        "ordered": false,
        "members": ["DONE", "READY"],
    });
    let key = fixture_key(&preimage);
    EnumDeclaration::admit(
        EnumDeclarationPreimage::from_json(preimage).unwrap(),
        key,
        &owners(),
    )
    .unwrap()
}

fn enum_value(
    declaration: &EnumDeclaration,
    case: &str,
) -> qsl_semantics::value::enumeration::EnumValue {
    let preimage = json!({
        "version": "quire.enum-member-node/v1",
        "declaration_node_id": node_id(declaration.key()),
        "case": case,
    });
    let key = fixture_key(&preimage);
    declaration
        .admit_member(&EnumMemberPreimage::from_json(preimage).unwrap(), key)
        .unwrap()
}

/// ADR-013 O-14/OQ-D: a bare kernel `Value::Enum` (`VariantId` and rank),
/// which `equal`'s `check`-then-`evaluate` path resolves back to its full
/// [`enum_value`] through the `Enum` schedule's own captured index
/// (`check_with_enums`).
fn member(declaration: &EnumDeclaration, case: &str) -> Value {
    let member = enum_value(declaration, case);
    Value::Enum(EnumMember::new(
        member.variant(),
        u32::try_from(member.position()).unwrap(),
    ))
}

fn integer(value: i64) -> Integer {
    Integer::from(value)
}

fn int(value: i64) -> Value {
    Value::Integer(integer(value))
}

fn interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(integer(lower), integer(upper)).unwrap()
}

fn int_type(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(interval(lower, upper))
}

fn rational(numerator: i64, denominator: i64) -> Value {
    Value::Rational(Rational::new(integer(numerator), integer(denominator)).unwrap())
}

fn rational_type(n1: i64, n2: i64, d1: i64, d2: i64) -> ValueType {
    ValueType::Rational(RationalDomain::new(interval(n1, n2), interval(d1, d2)).unwrap())
}

fn decimal(coefficient: i64, scale: u32) -> Value {
    Value::Decimal(Decimal::new(integer(coefficient), scale))
}

fn decimal_type(lower: i64, upper: i64, min_scale: u64, max_scale: u64) -> ValueType {
    decimal_type_mode(lower, upper, min_scale, max_scale, RoundingMode::Exact)
}

fn decimal_type_mode(
    lower: i64,
    upper: i64,
    min_scale: u64,
    max_scale: u64,
    mode: RoundingMode,
) -> ValueType {
    ValueType::Decimal(
        DecimalType::new(integer(lower), integer(upper), min_scale, max_scale, mode).unwrap(),
    )
}

fn text_type(profile: TextProfile) -> TextType {
    TextType::new(0, 16, profile).unwrap()
}

fn text(payload: &str, profile: TextProfile) -> Value {
    let text: Text = admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).unwrap(),
        &text_type(profile),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .unwrap();
    Value::Text(text)
}

fn bound(minimum: u64, maximum: u64) -> CardinalityBound {
    CardinalityBound::new(minimum, maximum).unwrap()
}

fn collection_type(kind: CollectionKind, element: ValueType, maximum: u64) -> CollectionType {
    CollectionType::new(kind, element, bound(0, maximum))
}

/// A parameter value of `collection_type` holding `occurrences`.
fn collection(collection_type: &CollectionType, occurrences: Vec<Value>) -> Value {
    form_collection(collection_type, occurrences, &mut Meter::new(UNLIMITED))
        .unwrap()
        .completed()
        .unwrap()
}

fn integers(values: &[i64]) -> Vec<Value> {
    values.iter().copied().map(int).collect()
}

fn typed(value_type: &ValueType) -> EqualityOperand {
    EqualityOperand::typed(value_type.clone())
}

/// No test using this helper (or `equal`, built over it) exercises an `Enum`
/// schedule, so an empty index is correct for every one of them; `e07_...`
/// below builds its own real index with [`check_with_enums`] instead.
fn check(
    env: &TypeEnvironment,
    left: &ValueType,
    right: &ValueType,
) -> Result<CheckedEquality, IllTyped> {
    check_with_enums(env, left, right, &EnumMemberIndex::default())
}

fn check_with_enums(
    env: &TypeEnvironment,
    left: &ValueType,
    right: &ValueType,
    enum_members: &EnumMemberIndex,
) -> Result<CheckedEquality, IllTyped> {
    env.check_equality(
        EqualityOperator::Equal,
        typed(left),
        typed(right),
        enum_members,
    )
}

/// `left = right` for parameters of the declared types, under unlimited limits.
fn equal(
    env: &TypeEnvironment,
    (left_type, left): (&ValueType, &Value),
    (right_type, right): (&ValueType, &Value),
) -> Result<Outcome<bool>, IllTyped> {
    check(env, left_type, right_type)
        .map(|checked| checked.evaluate(left, right, &mut Meter::new(UNLIMITED)))
}

fn is(expected: bool) -> Result<Outcome<bool>, IllTyped> {
    Ok(Outcome::Completed(expected))
}

fn ill_typed(cause: IllTypedCause) -> Result<Outcome<bool>, IllTyped> {
    Err(IllTyped { cause })
}

fn pairs(left: &Value, right: &Value) -> u64 {
    plan_equality(left, right)
        .unwrap()
        .pair_events()
        .to_u64()
        .unwrap()
}

fn incomplete(
    limit_kind: LimitKind,
    limit: u64,
    consumed: u64,
    next_charge: i64,
    charge_point: ChargePoint,
) -> Outcome<bool> {
    Outcome::Incomplete(Incomplete {
        limit_kind,
        limit,
        consumed,
        next_charge: integer(next_charge),
        charge_point,
    })
}

/// Every value-kind row also compares its left operand with a disjoint kind.
fn assert_disjoint(env: &TypeEnvironment, left: &ValueType) {
    let disjoint = match left {
        ValueType::Boolean => ValueType::Integer,
        _ => ValueType::Boolean,
    };
    let mismatch = Err(IllTyped {
        cause: IllTypedCause::TypeMismatch,
    });
    assert_eq!(check(env, left, &disjoint), mismatch, "{left:?}");
    assert_eq!(check(env, &disjoint, left), mismatch, "{left:?}");
}

const PLAN_SCHEDULE: [ChargePoint; 4] = [
    ChargePoint::EqualityPlanForm,
    ChargePoint::EqualityPlan,
    ChargePoint::EqualityPair,
    ChargePoint::EqualityResultRetain,
];

// ---- scalar rows -------------------------------------------------------------

#[trace("TC-194", "FR-149-AC-1")]
#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e01_boolean_identical_truth_value() {
    let env = TypeEnvironment::default();
    let b = ValueType::Boolean;
    let t = Value::Boolean(true);
    assert_eq!(equal(&env, (&b, &t), (&b, &Value::Boolean(true))), is(true));
    assert_eq!(
        equal(&env, (&b, &t), (&b, &Value::Boolean(false))),
        is(false)
    );
    assert_disjoint(&env, &b);
}

#[trace("TC-194", "FR-149-AC-3")]
#[trace("TC-194", "FR-149-AC-5")]
#[test]
fn e02_integers_and_admitted_rational_conversion() {
    let env = TypeEnvironment::default();
    let i = ValueType::Integer;
    assert_eq!(equal(&env, (&i, &int(1)), (&i, &int(1))), is(true));
    assert_eq!(equal(&env, (&i, &int(1)), (&i, &int(2))), is(false));
    let (small, ratio) = (int_type(0, 2), rational_type(0, 2, 1, 1));
    let source = int(1);
    let converted = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(small.clone(), ratio.clone()),
            typed(&ratio),
            &EnumMemberIndex::default(),
        )
        .unwrap();
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        converted.evaluate(&source, &rational(1, 1), &mut meter),
        Outcome::Completed(true)
    );
    assert_eq!(meter.admitted_charges(), PLAN_SCHEDULE);
    assert!(matches!(&source, Value::Integer(value) if *value == integer(1)));
    assert_eq!(
        equal(&env, (&small, &source), (&ratio, &rational(1, 1))),
        ill_typed(IllTypedCause::TypeMismatch)
    );
    assert_disjoint(&env, &i);
    assert_disjoint(&env, &ratio);
}

#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-5")]
#[test]
fn e03_decimals_compare_mathematically_and_convert_without_mutation() {
    let env = TypeEnvironment::default();
    let d = decimal_type(0, 1000, 0, 2);
    let left = decimal(10, 1);
    assert_eq!(equal(&env, (&d, &left), (&d, &decimal(100, 2))), is(true));
    assert_eq!(equal(&env, (&d, &left), (&d, &decimal(11, 1))), is(false));
    let (source_type, target) = (decimal_type(0, 100, 0, 2), rational_type(0, 100, 1, 100));
    let converted = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(source_type, target.clone()),
            typed(&target),
            &EnumMemberIndex::default(),
        )
        .unwrap();
    let one = rational(1, 1);
    assert_eq!(
        converted.evaluate(&left, &one, &mut Meter::new(UNLIMITED)),
        Outcome::Completed(true)
    );
    let Value::Decimal(source) = &left else {
        panic!("the source stays a decimal");
    };
    assert_eq!(source.representation().coefficient(), &integer(10));
    assert_eq!(source.representation().scale(), 1);
    assert!(matches!(&one, Value::Rational(_)));
    assert_disjoint(&env, &d);
}

#[trace("TC-194", "FR-149-AC-3")]
#[trace("TC-194", "FR-149-AC-10")]
#[test]
fn e04_rational_with_denominator_above_one_has_no_decimal_equality_conversion() {
    let env = TypeEnvironment::default();
    let target = decimal_type_mode(0, 100, 2, 2, RoundingMode::NearestEven);
    assert_eq!(
        env.check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(rational_type(0, 1, 1, 3), target.clone()),
            typed(&target),
            &EnumMemberIndex::default(),
        ),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
}

#[trace("TC-194", "FR-149-AC-3")]
#[trace("TC-194", "FR-149-AC-5")]
#[test]
fn e05_quantities_after_explicit_canonical_unit_conversion() {
    let units = units();
    let env = TypeEnvironment::default().with_units(units.table.clone());
    let whole = |value: i64| Rational::from_integer(integer(value));
    let (metres, centimetres, seconds) = (
        ValueType::Quantity(units.m),
        ValueType::Quantity(units.cm),
        ValueType::Quantity(units.s),
    );
    let source = Quantity::new(whole(100), units.cm);
    let snapshot = source.clone();
    let one_metre = Value::Quantity(Quantity::new(whole(1), units.m));
    let converted = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(centimetres.clone(), metres.clone()),
            typed(&metres),
            &EnumMemberIndex::default(),
        )
        .unwrap();
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        converted.evaluate(&Value::Quantity(source.clone()), &one_metre, &mut meter),
        Outcome::Completed(true)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::EqualityPlan));
    assert_eq!(source, snapshot);
    assert_eq!(
        equal(
            &env,
            (&centimetres, &Value::Quantity(source)),
            (&metres, &one_metre)
        ),
        ill_typed(IllTypedCause::DistinctUnits)
    );
    let one_second = Value::Quantity(Quantity::new(whole(1), units.s));
    assert_eq!(
        equal(&env, (&metres, &one_metre), (&seconds, &one_second)),
        ill_typed(IllTypedCause::IncompatibleDimensions)
    );
    assert_disjoint(&env, &metres);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e06_text_under_one_pinned_profile() {
    let env = TypeEnvironment::default();
    let nfc = ValueType::Text(text_type(TextProfile::Nfc));
    let binary = ValueType::Text(text_type(TextProfile::BinaryUtf8));
    let composed = text("\u{e9}", TextProfile::Nfc);
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        check(&env, &nfc, &nfc).unwrap().evaluate(
            &composed,
            &text("e\u{301}", TextProfile::Nfc),
            &mut meter
        ),
        Outcome::Completed(true)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::EqualityPlanForm));
    assert_eq!(
        equal(
            &env,
            (&nfc, &text("a", TextProfile::Nfc)),
            (&nfc, &text("b", TextProfile::Nfc))
        ),
        is(false)
    );
    assert_eq!(
        equal(
            &env,
            (&nfc, &composed),
            (&binary, &text("\u{e9}", TextProfile::BinaryUtf8))
        ),
        ill_typed(IllTypedCause::DistinctTextProfiles)
    );
    assert_disjoint(&env, &nfc);
}

/// ADR-013 O-14: a bare kernel `Value::Enum` carries no declaration of its
/// own, so unlike every other `equal` call in this file, an enum comparison
/// needs a real `EnumMemberIndex` -- built here from a declaration's
/// admitted members into a shared index, exactly as
/// `check::check::enum_member_index` builds one from `scope.enums` in
/// production.
fn enum_shape(
    declaration: &EnumDeclaration,
    cases: &[&str],
    index: &mut EnumMemberIndex,
) -> ValueType {
    let variants: Vec<_> = cases
        .iter()
        .map(|case| {
            let member = enum_value(declaration, case);
            let variant = member.variant();
            index.record(member);
            variant
        })
        .collect();
    ValueType::Enum(quire_exact::EnumShape::new(
        declaration.preimage().is_ordered(),
        variants,
    ))
}

#[trace("TC-194", "FR-149-AC-3")]
#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e07_enumerations_by_declaration_node_and_case() {
    let env = TypeEnvironment::default();
    let (a, b) = (enum_declaration("EnumA"), enum_declaration("EnumB"));
    let mut index = EnumMemberIndex::default();
    let a_type = enum_shape(&a, &["DONE", "READY"], &mut index);
    let b_type = enum_shape(&b, &["DONE", "READY"], &mut index);
    let equal_enum =
        |left_type: &ValueType, left: &Value, right_type: &ValueType, right: &Value| {
            check_with_enums(&env, left_type, right_type, &index)
                .map(|checked| checked.evaluate(left, right, &mut Meter::new(UNLIMITED)))
        };
    let ready = member(&a, "READY");
    assert_eq!(
        equal_enum(&a_type, &ready, &a_type, &member(&a, "READY")),
        is(true)
    );
    assert_eq!(
        equal_enum(&a_type, &ready, &a_type, &member(&a, "DONE")),
        is(false)
    );
    assert_eq!(
        equal_enum(&a_type, &ready, &b_type, &member(&b, "READY")),
        ill_typed(IllTypedCause::DistinctEnumDeclarations)
    );
    assert_disjoint(&env, &a_type);
}

// ---- presence, composite and collection rows -------------------------------

#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-6")]
#[test]
fn e08_options_compare_state_then_payload() {
    let env = TypeEnvironment::default();
    let option = ValueType::option(ValueType::Integer);
    let none = || OptionValue::none(ValueType::Integer);
    let present = |value| OptionValue::present(ValueType::Integer, int(value)).unwrap();
    let eq = |left: &Value, right: &Value| equal(&env, (&option, left), (&option, right));
    assert_eq!(eq(&none(), &none()), is(true));
    assert_eq!(eq(&present(1), &present(1)), is(true));
    assert_eq!(eq(&none(), &present(1)), is(false));
    let rational_option = ValueType::option(ValueType::Rational(
        RationalDomain::new(interval(0, 2), interval(1, 1)).unwrap(),
    ));
    assert_eq!(
        check(&env, &option, &rational_option),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
    assert_disjoint(&env, &option);
}

fn holder_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Holder"),
            "Holder",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "value",
                ValueType::Integer,
                Presence::Optional,
            )]),
        )],
        [],
    )
    .unwrap()
}

#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-6")]
#[test]
fn e09_absence_and_null_stay_distinct() {
    let env = holder_environment();
    let holder_type = ValueType::Composite(key("Holder"));
    let holder = |state| env.record(key("Holder"), vec![("value", state)]).unwrap();
    let eq = |left: &Value, right: &Value| equal(&env, (&holder_type, left), (&holder_type, right));
    let null = holder(FieldValue::Null);
    assert_eq!(eq(&null, &holder(FieldValue::Null)), is(true));
    assert_eq!(
        eq(&holder(FieldValue::Absent), &holder(FieldValue::Absent)),
        is(true)
    );
    assert_eq!(eq(&holder(FieldValue::Absent), &null), is(false));
    assert_eq!(eq(&holder(FieldValue::Present(int(1))), &null), is(false));
    assert_eq!(
        eq(
            &holder(FieldValue::Present(int(1))),
            &holder(FieldValue::Present(int(1)))
        ),
        is(true)
    );
    assert_eq!(pairs(&null, &holder(FieldValue::Absent)), 2);
    assert_disjoint(&env, &holder_type);
}

fn record_environment() -> TypeEnvironment {
    let inner = CompositeShape::Record(vec![FieldDeclaration::new(
        "v",
        ValueType::Integer,
        Presence::Required,
    )]);
    let outer = || {
        CompositeShape::Record(vec![
            FieldDeclaration::new("id", ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                "inner",
                ValueType::Composite(key("Inner")),
                Presence::Required,
            ),
        ])
    };
    let pair = || CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Boolean]);
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(key("Inner"), "Inner", inner),
            CompositeDeclaration::new(key("record-A"), "RecordA", outer()),
            CompositeDeclaration::new(key("record-B"), "RecordB", outer()),
            CompositeDeclaration::new(key("tuple-A"), "TupleA", pair()),
            CompositeDeclaration::new(key("tuple-B"), "TupleB", pair()),
        ],
        [],
    )
    .unwrap()
}

fn nested(env: &TypeEnvironment, declaration: &str, id: i64, v: i64) -> Value {
    let inner = env
        .record(key("Inner"), vec![("v", FieldValue::Present(int(v)))])
        .unwrap();
    env.record(
        key(declaration),
        vec![
            ("id", FieldValue::Present(int(id))),
            ("inner", FieldValue::Present(inner)),
        ],
    )
    .unwrap()
}

#[trace("TC-194", "FR-149-AC-2")]
#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e10_records_compare_structurally_within_one_declaration() {
    let env = record_environment();
    let (a, b) = (
        ValueType::Composite(key("record-A")),
        ValueType::Composite(key("record-B")),
    );
    let left = nested(&env, "record-A", 1, 2);
    assert_eq!(
        equal(&env, (&a, &left), (&a, &nested(&env, "record-A", 1, 2))),
        is(true)
    );
    assert_eq!(
        equal(&env, (&a, &left), (&a, &nested(&env, "record-A", 1, 3))),
        is(false)
    );
    assert_eq!(
        equal(&env, (&a, &left), (&b, &nested(&env, "record-B", 1, 2))),
        ill_typed(IllTypedCause::TypeMismatch)
    );
    // `$`, `$.id`, `$.inner`, `$.inner.v`.
    assert_eq!(pairs(&left, &nested(&env, "record-A", 9, 9)), 4);
    assert_disjoint(&env, &a);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e11_tuples_compare_positionally_within_one_declaration() {
    let env = record_environment();
    let (a, b) = (
        ValueType::Composite(key("tuple-A")),
        ValueType::Composite(key("tuple-B")),
    );
    let tuple = |declaration: &str, first, second| {
        env.tuple(key(declaration), vec![int(first), Value::Boolean(second)])
            .unwrap()
    };
    let left = tuple("tuple-A", 1, true);
    assert_eq!(
        equal(&env, (&a, &left), (&a, &tuple("tuple-A", 1, true))),
        is(true)
    );
    assert_eq!(
        equal(&env, (&a, &left), (&a, &tuple("tuple-A", 1, false))),
        is(false)
    );
    assert_eq!(
        equal(&env, (&a, &left), (&b, &tuple("tuple-B", 1, true))),
        ill_typed(IllTypedCause::TypeMismatch)
    );
    assert_disjoint(&env, &a);
}

/// Equal, unequal and wrong-kind mutations of one collection row.
fn collection_row(kind: CollectionKind, left: &[i64], equal_to: &[&[i64]], unequal_to: &[&[i64]]) {
    let env = TypeEnvironment::default();
    let declared = collection_type(kind, ValueType::Integer, 3);
    let value_type = ValueType::collection(declared.clone());
    let left = collection(&declared, integers(left));
    for (rights, expected) in [(equal_to, true), (unequal_to, false)] {
        for right in rights {
            let right = collection(&declared, integers(right));
            assert_eq!(
                equal(&env, (&value_type, &left), (&value_type, &right)),
                is(expected),
                "{kind:?} {right:?}"
            );
        }
    }
    for other in CollectionKind::ALL
        .into_iter()
        .filter(|other| *other != kind)
    {
        let other = ValueType::collection(collection_type(other, ValueType::Integer, 3));
        assert_eq!(
            check(&env, &value_type, &other),
            Err(IllTyped {
                cause: IllTypedCause::TypeMismatch
            })
        );
    }
    let rationals = ValueType::collection(collection_type(kind, rational_type(0, 2, 1, 1), 3));
    assert_eq!(
        check(&env, &value_type, &rationals),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
    assert_disjoint(&env, &value_type);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e12_sequences_compare_by_index() {
    collection_row(
        CollectionKind::Sequence,
        &[1, 2],
        &[&[1, 2]],
        &[&[2, 1], &[1, 2, 2]],
    );
}

#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-8")]
#[test]
fn e13_sets_ignore_insertion_order() {
    collection_row(
        CollectionKind::Set,
        &[1, 2],
        &[&[2, 1], &[2, 1, 2]],
        &[&[1, 3]],
    );
}

#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-8")]
#[test]
fn e14_bags_compare_multiplicity() {
    collection_row(
        CollectionKind::Bag,
        &[1, 1, 2],
        &[&[2, 1, 1]],
        &[&[1, 2, 2], &[1, 2]],
    );
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e15_ordered_sets_normalize_to_first_occurrence() {
    collection_row(
        CollectionKind::OrderedSet,
        &[1, 2, 1],
        &[&[1, 2]],
        &[&[2, 1]],
    );
}

fn object_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Holder"),
            "Holder",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "r",
                ValueType::Reference(object_type("M::Obj")),
                Presence::Required,
            )]),
        )],
        [ObjectTypeDeclaration::new(
            object_type("M::Obj"),
            "Obj",
            vec![FieldDeclaration::new(
                "balance",
                ValueType::Integer,
                Presence::Required,
            )],
        )],
    )
    .unwrap()
}

fn universe_id(tag: &str) -> UniverseId {
    UniverseId::from_digest(Sha256::digest(tag.as_bytes()).into())
}

fn reference(universe: &str, identity: &str) -> ObjectReference {
    ObjectReference::new(
        universe_id(universe),
        object_type("M::Obj"),
        ObjectId::new(identity).unwrap(),
    )
}

#[trace("TC-194", "FR-149-AC-2")]
#[trace("TC-194", "FR-149-AC-4")]
#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e16_references_compare_identity_triple_only() {
    let env = object_environment();
    let state = |balance| vec![("balance", FieldValue::Present(int(balance)))];
    let a = reference("u1", "a");
    let before = ObjectEnvironment::new(&env, [(a.clone(), state(1))]).unwrap();
    let after = ObjectEnvironment::new(
        &env,
        [(a.clone(), state(2)), (reference("u1", "b"), state(1))],
    )
    .unwrap();
    let balance = |objects: &ObjectEnvironment| match objects.attribute(&env, &a, "balance") {
        Some(FieldValue::Present(Value::Integer(value))) => value.clone(),
        other => panic!("balance is a present integer, not {other:?}"),
    };
    assert_ne!(balance(&before), balance(&after));
    let r = ValueType::Reference(object_type("M::Obj"));
    let left = Value::Reference(a.clone());
    assert_eq!(
        equal(&env, (&r, &left), (&r, &Value::Reference(a))),
        is(true)
    );
    assert_eq!(
        equal(
            &env,
            (&r, &left),
            (&r, &Value::Reference(reference("u1", "b")))
        ),
        is(false)
    );
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        check(&env, &r, &r).unwrap().evaluate(
            &left,
            &Value::Reference(reference("u2", "a")),
            &mut meter
        ),
        Outcome::Refused(Refusal::ForeignReference)
    );
    assert_eq!(Refusal::ForeignReference.code(), Some("foreign_reference"));
    assert_eq!(meter.admitted_charges(), [ChargePoint::EqualityPlanForm]);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);
    assert_disjoint(&env, &r);
}

// ---- IEEE rows -------------------------------------------------------------

fn profile() -> &'static AdmittedIeeeProfile {
    static PROFILE: OnceLock<AdmittedIeeeProfile> = OnceLock::new();
    PROFILE.get_or_init(|| {
        let lock = DefinitionLock::pinned();
        let entry = lock.entry(CatalogRole::IeeeProfile).unwrap();
        let reference = DefinitionReference {
            authority: entry.authority.to_owned(),
            identity: entry.identity.to_owned(),
            revision: DefinitionRevision {
                namespace: entry.revision_namespace.to_owned(),
                value: entry.revision_value.to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: "0".repeat(64),
        };
        lock.admit_ieee_profile(&[reference], &[]).unwrap()
    })
}

/// Numeric equality, total-order equivalence (both directions) and bit
/// identity.
fn ieee_relations(left: IeeeValue, right: IeeeValue) -> [bool; 3] {
    let compare = |comparison, a, b| {
        compare_ieee(comparison, a, b, &mut Meter::new(UNLIMITED))
            .unwrap()
            .completed()
            .unwrap()
    };
    [
        compare(IeeeComparison::NumericEqual, left, right),
        compare(IeeeComparison::TotalOrder, left, right)
            && compare(IeeeComparison::TotalOrder, right, left),
        compare(IeeeComparison::BitIdentical, left, right),
    ]
}

#[trace("TC-194", "FR-149-AC-1")]
#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e17_signed_zeros_under_each_selected_ieee_relation() {
    let (positive, negative) = (IeeeValue::binary32(0), IeeeValue::binary32(0x8000_0000));
    assert_eq!(ieee_relations(positive, negative), [true, false, false]);
}

#[trace("TC-194", "FR-149-AC-1")]
#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e18_quiet_nan_under_each_selected_ieee_relation() {
    let nan = IeeeValue::binary32(0x7fc0_0001);
    assert_eq!(ieee_relations(nan, nan), [false, true, true]);
}

#[trace("TC-194", "FR-149-AC-3")]
#[trace("TC-194", "FR-149-AC-5")]
#[test]
fn e19_ieee_widths_need_explicit_conversion() {
    let narrow = IeeeValue::binary32(0x3f80_0000);
    let wide = IeeeValue::binary64(0x3ff0_0000_0000_0000);
    assert_eq!(
        compare_ieee(
            IeeeComparison::NumericEqual,
            narrow,
            wide,
            &mut Meter::new(UNLIMITED)
        ),
        Err(IllTyped {
            cause: IllTypedCause::DistinctIeeeWidths
        })
    );
    let converted = convert_ieee_width(
        narrow,
        IeeeWidth::Binary64,
        RoundingMode::Exact,
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .unwrap()
    .value();
    assert!(ieee_relations(converted, wide)[0]);
    assert_eq!(narrow.width(), IeeeWidth::Binary32);
    assert_eq!(wide.width(), IeeeWidth::Binary64);
}

// ---- dispositions and accounting -------------------------------------------

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e20_nested_construction_dispositions_propagate_in_declaration_order() {
    let env = record_environment();
    let stopped: [Outcome<Value>; 3] = [
        Outcome::Undefined(Undefined::DivisionByZero),
        Outcome::Refused(Refusal::InexactDecimal),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 0,
            consumed: 0,
            next_charge: integer(1),
            charge_point: ChargePoint::DecimalArithmetic,
        }),
    ];
    for nested in stopped {
        let mut meter = Meter::new(UNLIMITED);
        let inner_ran = Cell::new(false);
        let outer = env
            .evaluate_record(
                key("record-A"),
                vec![
                    (
                        "inner",
                        FieldExpression::Evaluate(Box::new(|meter: &mut Meter| {
                            inner_ran.set(true);
                            env.evaluate_record(
                                key("Inner"),
                                vec![(
                                    "v",
                                    FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                                        nested.clone()
                                    })),
                                )],
                                meter,
                            )
                            .unwrap()
                        })),
                    ),
                    (
                        "id",
                        FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                            Outcome::Refused(Refusal::CheckedInvariant)
                        })),
                    ),
                ],
                &mut meter,
            )
            .unwrap();
        // `id` is declared first, so its refusal wins and `inner` never runs.
        assert!(matches!(outer, Outcome::Refused(Refusal::CheckedInvariant)));
        assert!(!inner_ran.get());
        let only_inner = env
            .evaluate_record(
                key("record-A"),
                vec![
                    (
                        "inner",
                        FieldExpression::Evaluate(Box::new(|meter: &mut Meter| {
                            env.evaluate_record(
                                key("Inner"),
                                vec![(
                                    "v",
                                    FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                                        nested.clone()
                                    })),
                                )],
                                meter,
                            )
                            .unwrap()
                        })),
                    ),
                    (
                        "id",
                        FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                            Outcome::Completed(int(1))
                        })),
                    ),
                ],
                &mut meter,
            )
            .unwrap();
        assert_eq!(format!("{only_inner:?}"), format!("{nested:?}"));
        assert!(meter.admitted_charges().is_empty());
    }
}

fn list_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("List"),
            "List",
            CompositeShape::Record(vec![
                FieldDeclaration::new("head", ValueType::Integer, Presence::Required),
                FieldDeclaration::new(
                    "tail",
                    ValueType::Composite(key("List")),
                    Presence::Optional,
                ),
            ]),
        )],
        [],
    )
    .unwrap()
}

/// `[from..=8]` ending in `tail`, or an absent tail.
fn list_onto(env: &TypeEnvironment, from: i64, tail: Option<Value>) -> Value {
    (from..=8)
        .rev()
        .fold(tail, |tail, head| {
            let tail = tail.map_or(FieldValue::Absent, FieldValue::Present);
            Some(
                env.record(
                    key("List"),
                    vec![("head", FieldValue::Present(int(head))), ("tail", tail)],
                )
                .unwrap(),
            )
        })
        .unwrap()
}

const E21: ScalarLimits = ScalarLimits {
    integer_bits: 4,
    value_occurrences: 17,
    work_units: 51,
    result_units: 1,
    ..ZERO
};

fn e21_operands(env: &TypeEnvironment) -> [(Value, Value); 2] {
    let duplicated = (list_onto(env, 1, None), list_onto(env, 1, None));
    let shared_tail = list_onto(env, 5, None);
    let prefix = |tail: &Value| {
        (1..=4).rev().fold(tail.clone(), |tail, head| {
            env.record(
                key("List"),
                vec![
                    ("head", FieldValue::Present(int(head))),
                    ("tail", FieldValue::Present(tail)),
                ],
            )
            .unwrap()
        })
    };
    [duplicated, (prefix(&shared_tail), prefix(&shared_tail))]
}

#[trace("TC-194", "FR-149-AC-7")]
#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e21_duplicated_and_shared_lists_charge_seventeen_pairs() {
    let env = list_environment();
    let list = ValueType::Composite(key("List"));
    let checked = check(&env, &list, &list).unwrap();
    let mut expected_points = vec![ChargePoint::EqualityPlanForm, ChargePoint::EqualityPlan];
    expected_points.extend([ChargePoint::EqualityPair; 17]);
    expected_points.push(ChargePoint::EqualityResultRetain);
    for (left, right) in e21_operands(&env) {
        assert_eq!(left.occ(), integer(16));
        assert_eq!(pairs(&left, &right), 17);
        let mut meter = Meter::new(E21);
        assert_eq!(
            checked.evaluate(&left, &right, &mut meter),
            Outcome::Completed(true)
        );
        assert_eq!(meter.admitted_charges(), expected_points.as_slice());
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 51);
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);
        assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 17);

        let denied = |point, occurrence| {
            Meter::new(E21).with_injected_denial(InjectedDenial { point, occurrence })
        };
        assert_eq!(
            checked.evaluate(&left, &right, &mut denied(ChargePoint::EqualityPair, 17)),
            incomplete(LimitKind::WorkUnits, 49, 49, 1, ChargePoint::EqualityPair)
        );
        assert_eq!(
            checked.evaluate(
                &left,
                &right,
                &mut denied(ChargePoint::EqualityResultRetain, 1)
            ),
            incomplete(
                LimitKind::WorkUnits,
                50,
                50,
                1,
                ChargePoint::EqualityResultRetain
            )
        );
    }
}

#[trace("TC-194", "FR-149-AC-8")]
#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e22_sets_and_bags_match_reference_holders_by_key_rank() {
    let env = object_environment();
    let holder_type = ValueType::Composite(key("Holder"));
    let holder = |name: &str| {
        env.record(
            key("Holder"),
            vec![(
                "r",
                FieldValue::Present(Value::Reference(reference("u1", name))),
            )],
        )
        .unwrap()
    };
    let (h1, h2, h3) = (holder("h1"), holder("h2"), holder("h3"));
    let run = |value_type: &ValueType, left: &Value, right: &Value| {
        let mut meter = Meter::new(UNLIMITED);
        let outcome = check(&env, value_type, value_type)
            .unwrap()
            .evaluate(left, right, &mut meter);
        (
            outcome,
            pairs(left, right),
            meter.consumed(LimitKind::WorkUnits),
        )
    };
    let set_type = collection_type(CollectionKind::Set, holder_type.clone(), 2);
    let set_value_type = ValueType::collection(set_type.clone());
    let set =
        |members: &[&Value]| collection(&set_type, members.iter().copied().cloned().collect());
    let left = set(&[&h1, &h2]);
    for (right, expected) in [
        (set(&[&h2, &h1]), true),
        (set(&[&h1, &h2]), true),
        (set(&[&h1, &h3]), false),
    ] {
        assert_eq!(
            run(&set_value_type, &left, &right),
            (Outcome::Completed(expected), 5, 17)
        );
        assert_eq!(
            run(&set_value_type, &right, &left),
            (Outcome::Completed(expected), 5, 17)
        );
    }
    assert_eq!(
        run(&set_value_type, &left, &set(&[&h1])),
        (Outcome::Completed(false), 1, 11)
    );
    let bag_type = collection_type(CollectionKind::Bag, holder_type, 3);
    let bag_value_type = ValueType::collection(bag_type.clone());
    let bag =
        |members: &[&Value]| collection(&bag_type, members.iter().copied().cloned().collect());
    let left = bag(&[&h1, &h1, &h2]);
    for (right, expected) in [
        (bag(&[&h2, &h1, &h1]), true),
        (bag(&[&h1, &h2, &h1]), true),
        (bag(&[&h1, &h2, &h2]), false),
        (bag(&[&h2, &h2, &h2]), false),
    ] {
        assert_eq!(
            run(&bag_value_type, &left, &right),
            (Outcome::Completed(expected), 7, 23)
        );
        assert_eq!(
            run(&bag_value_type, &right, &left),
            (Outcome::Completed(expected), 7, 23)
        );
    }
}

#[trace("TC-194", "FR-149-AC-7")]
#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e23_plan_reservation_is_unavailable_after_plan_formation() {
    let env = list_environment();
    let list = ValueType::Composite(key("List"));
    let [(left, right), _] = e21_operands(&env);
    let mut meter = Meter::new(ScalarLimits {
        work_units: 50,
        ..E21
    });
    assert_eq!(
        check(&env, &list, &list)
            .unwrap()
            .evaluate(&left, &right, &mut meter),
        incomplete(LimitKind::WorkUnits, 50, 32, 19, ChargePoint::EqualityPlan)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::EqualityPlanForm]);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
}

#[trace("TC-194", "FR-149-AC-7")]
#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e24_quantity_leaf_charges_only_its_pair() {
    let units = units();
    let env = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Length"),
            "Length",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "d",
                ValueType::Quantity(units.cm),
                Presence::Required,
            )]),
        )],
        [],
    )
    .unwrap();
    let length = ValueType::Composite(key("Length"));
    let record = || {
        let d = Quantity::new(Rational::from_integer(integer(1)), units.cm);
        env.record(
            key("Length"),
            vec![("d", FieldValue::Present(Value::Quantity(d)))],
        )
        .unwrap()
    };
    let checked = check(&env, &length, &length).unwrap();
    let limits = ScalarLimits {
        value_occurrences: 2,
        work_units: 8,
        result_units: 1,
        ..ZERO
    };
    let mut meter = Meter::new(limits);
    assert_eq!(
        checked.evaluate(&record(), &record(), &mut meter),
        Outcome::Completed(true)
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::EqualityPlanForm,
            ChargePoint::EqualityPlan,
            ChargePoint::EqualityPair,
            ChargePoint::EqualityPair,
            ChargePoint::EqualityResultRetain,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 8);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);
    assert_eq!(meter.consumed(LimitKind::UnitEdges), 0);
    for (work, expected) in [
        (
            7,
            incomplete(LimitKind::WorkUnits, 7, 4, 4, ChargePoint::EqualityPlan),
        ),
        (
            3,
            incomplete(LimitKind::WorkUnits, 3, 0, 4, ChargePoint::EqualityPlanForm),
        ),
    ] {
        let mut short = Meter::new(ScalarLimits {
            work_units: work,
            ..limits
        });
        assert_eq!(checked.evaluate(&record(), &record(), &mut short), expected);
    }
}

#[trace("TC-194", "FR-149-AC-9")]
#[test]
fn e25_top_level_scalar_and_reference_equality_is_a_one_pair_plan() {
    let env = object_environment();
    let limits = ScalarLimits {
        value_occurrences: 1,
        work_units: 5,
        result_units: 1,
        ..ZERO
    };
    let decimal_type = decimal_type(0, 100, 1, 2);
    let reference_type = ValueType::Reference(object_type("M::Obj"));
    let a = Value::Reference(reference("u1", "a"));
    let rows: [(EqualityOperator, &ValueType, Value, Value, bool); 4] = [
        (
            EqualityOperator::Equal,
            &ValueType::Boolean,
            Value::Boolean(true),
            Value::Boolean(true),
            true,
        ),
        (
            EqualityOperator::NotEqual,
            &ValueType::Integer,
            int(7),
            int(7),
            false,
        ),
        (
            EqualityOperator::Equal,
            &decimal_type,
            decimal(10, 1),
            decimal(100, 2),
            true,
        ),
        (EqualityOperator::Equal, &reference_type, a.clone(), a, true),
    ];
    for (operator, value_type, left, right, expected) in rows {
        let checked = env
            .check_equality(
                operator,
                typed(value_type),
                typed(value_type),
                &EnumMemberIndex::default(),
            )
            .unwrap();
        let mut meter = Meter::new(limits);
        assert_eq!(
            checked.evaluate(&left, &right, &mut meter),
            Outcome::Completed(expected)
        );
        assert_eq!(meter.admitted_charges(), PLAN_SCHEDULE);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 5);
    }
    let boolean = check(&env, &ValueType::Boolean, &ValueType::Boolean).unwrap();
    for (work, expected) in [
        (
            4,
            incomplete(LimitKind::WorkUnits, 4, 2, 3, ChargePoint::EqualityPlan),
        ),
        (
            1,
            incomplete(LimitKind::WorkUnits, 1, 0, 2, ChargePoint::EqualityPlanForm),
        ),
    ] {
        let mut meter = Meter::new(ScalarLimits {
            work_units: work,
            ..limits
        });
        assert_eq!(
            boolean.evaluate(&Value::Boolean(true), &Value::Boolean(true), &mut meter),
            expected
        );
    }
}

#[trace("TC-194", "FR-149-AC-10")]
#[test]
fn e26_exactly_the_tabled_equality_conversions_are_admitted() {
    let env = TypeEnvironment::default();
    let d_200 = decimal_type(0, 200, 2, 2);
    let rows: [(ValueType, ValueType, Value, Value, Option<bool>); 8] = [
        (
            int_type(0, 2),
            d_200.clone(),
            int(1),
            decimal(100, 2),
            Some(true),
        ),
        (
            decimal_type(0, 9, 0, 0),
            int_type(0, 9),
            decimal(3, 0),
            int(3),
            Some(true),
        ),
        (
            rational_type(0, 5, 1, 1),
            int_type(0, 5),
            rational(4, 1),
            int(4),
            Some(true),
        ),
        (
            decimal_type(0, 9, 0, 1),
            int_type(0, 9),
            decimal(1, 0),
            int(1),
            None,
        ),
        (int_type(0, 300), d_200, int(1), decimal(100, 2), None),
        (
            ValueType::Integer,
            rational_type(0, 2, 1, 1),
            int(1),
            rational(1, 1),
            None,
        ),
        (
            decimal_type(0, 100, 1, 2),
            decimal_type(0, 100, 0, 2),
            decimal(10, 1),
            decimal(1, 0),
            Some(true),
        ),
        (
            decimal_type(0, 100, 0, 2),
            decimal_type(0, 100, 1, 2),
            decimal(1, 0),
            decimal(10, 1),
            None,
        ),
    ];
    for (row, (source, target, left, right, expected)) in rows.into_iter().enumerate() {
        let checked = env.check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(source, target.clone()),
            typed(&target),
            &EnumMemberIndex::default(),
        );
        match expected {
            Some(expected) => assert_eq!(
                checked
                    .unwrap()
                    .evaluate(&left, &right, &mut Meter::new(UNLIMITED)),
                Outcome::Completed(expected),
                "row {row}"
            ),
            None => assert_eq!(
                checked,
                Err(IllTyped {
                    cause: IllTypedCause::TypeMismatch
                }),
                "row {row}"
            ),
        }
    }
}

#[trace("TC-194", "FR-149-AC-11")]
#[test]
fn e27_structural_mismatch_and_cardinality_short_circuit_pair_counts() {
    let env = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("R"),
            "R",
            CompositeShape::Record(vec![
                FieldDeclaration::new("a", ValueType::Integer, Presence::Required),
                FieldDeclaration::new("b", ValueType::Integer, Presence::Optional),
            ]),
        )],
        [],
    )
    .unwrap();
    let run = |value_type: &ValueType, left: &Value, right: &Value| {
        let mut meter = Meter::new(UNLIMITED);
        let outcome = check(&env, value_type, value_type)
            .unwrap()
            .evaluate(left, right, &mut meter);
        (
            outcome.completed().unwrap(),
            pairs(left, right),
            meter.consumed(LimitKind::WorkUnits),
        )
    };
    let option = ValueType::option(ValueType::Integer);
    let none = || OptionValue::none(ValueType::Integer);
    let present = |value| OptionValue::present(ValueType::Integer, int(value)).unwrap();
    assert_eq!(run(&option, &none(), &none()), (true, 1, 5));
    assert_eq!(run(&option, &present(1), &present(2)), (false, 2, 8));
    assert_eq!(run(&option, &none(), &present(1)), (false, 1, 6));

    let r_type = ValueType::Composite(key("R"));
    let r = |a, b| {
        env.record(key("R"), vec![("a", FieldValue::Present(int(a))), ("b", b)])
            .unwrap()
    };
    let absent = || FieldValue::Absent;
    assert_eq!(run(&r_type, &r(1, absent()), &r(1, absent())), (true, 3, 9));
    assert_eq!(
        run(&r_type, &r(1, FieldValue::Null), &r(1, absent())),
        (false, 3, 9)
    );
    assert_eq!(
        run(
            &r_type,
            &r(1, FieldValue::Present(int(2))),
            &r(2, FieldValue::Present(int(2)))
        ),
        (false, 3, 11)
    );

    let of = |kind| {
        let declared = collection_type(kind, ValueType::Integer, 3);
        let value_type = ValueType::collection(declared.clone());
        (value_type, move |values: &[i64]| {
            collection(&declared, integers(values))
        })
    };
    let (sequence, seq) = of(CollectionKind::Sequence);
    assert_eq!(
        run(&sequence, &seq(&[1, 2]), &seq(&[1, 2, 3])),
        (false, 1, 10)
    );
    assert_eq!(run(&sequence, &seq(&[1, 2]), &seq(&[1, 3])), (false, 3, 11));
    let (set_type, set) = of(CollectionKind::Set);
    assert_eq!(run(&set_type, &set(&[1, 2]), &set(&[2, 1])), (true, 3, 11));
    assert_eq!(
        run(&set_type, &set(&[1, 2]), &set(&[1, 2, 3])),
        (false, 1, 10)
    );
    let (bag_type, bag) = of(CollectionKind::Bag);
    assert_eq!(
        run(&bag_type, &bag(&[1, 1, 2]), &bag(&[1, 2, 2])),
        (false, 4, 14)
    );
}

#[trace("TC-194", "FR-149-AC-9")]
#[test]
fn e28_equality_on_ieee_bearing_types_is_operator_ineligible() {
    let env = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("F"),
            "F",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "x",
                ValueType::Float(IeeeWidth::Binary32),
                Presence::Required,
            )]),
        )],
        [],
    )
    .unwrap();
    let ineligible = IllTyped {
        cause: IllTypedCause::OperatorIneligible,
    };
    for value_type in [
        ValueType::Float(IeeeWidth::Binary32),
        ValueType::Composite(key("F")),
    ] {
        assert_eq!(check(&env, &value_type, &value_type), Err(ineligible));
    }
    assert_eq!(ineligible.cause.tag(), Some("operator-ineligible"));
    let set = ValueType::collection(collection_type(
        CollectionKind::Set,
        ValueType::Float(IeeeWidth::Binary64),
        2,
    ));
    assert_eq!(env.check_type(&set), Err(ineligible));
}

#[trace("TC-194", "FR-149-AC-10")]
#[test]
fn e29_integer_to_decimal_conversion_charges_its_decimal_schedule() {
    let env = TypeEnvironment::default();
    let target = decimal_type(0, 200, 2, 2);
    let checked = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::converted(int_type(0, 2), target.clone()),
            typed(&target),
            &EnumMemberIndex::default(),
        )
        .unwrap();
    let limits = ScalarLimits {
        integer_bits: 8,
        decimal_digits: 3,
        scale_expansion: 2,
        value_occurrences: 1,
        work_units: 9,
        result_units: 2,
        ..ZERO
    };
    let mut meter = Meter::new(limits);
    assert_eq!(
        checked.evaluate(&int(1), &decimal(100, 2), &mut meter),
        Outcome::Completed(true)
    );
    let mut expected = vec![
        ChargePoint::DecimalOperands,
        ChargePoint::DecimalScaleExpansion,
        ChargePoint::DecimalArithmetic,
        ChargePoint::DecimalResultRetain,
    ];
    expected.extend(PLAN_SCHEDULE);
    assert_eq!(meter.admitted_charges(), expected.as_slice());
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 9);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 2);
    // `sbits(1,2) = bits(1) + bits(100) = 8`, `sdigits(1,2) = 3`.
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 8);
    assert_eq!(meter.consumed(LimitKind::DecimalDigits), 3);
    assert_eq!(meter.consumed(LimitKind::ScaleExpansion), 2);
    let mut short = Meter::new(ScalarLimits {
        work_units: 8,
        ..limits
    });
    assert_eq!(
        checked.evaluate(&int(1), &decimal(100, 2), &mut short),
        incomplete(LimitKind::WorkUnits, 8, 6, 3, ChargePoint::EqualityPlan)
    );
}

#[trace("TC-194", "FR-149-AC-5")]
#[test]
fn e_construction_refusals_are_located() {
    let env = record_environment();
    assert_eq!(
        env.record(
            key("Inner"),
            vec![("v", FieldValue::Present(Value::Boolean(true)))]
        )
        .unwrap_err(),
        ConstructionRefusal {
            component: Component::Field("v".into()),
            cause: ConstructionCause::TypeMismatch,
        }
    );
}

// ---- scalar operators through the expression checker and evaluator ----------

fn expression_package(types: TypeEnvironment, ieee: bool) -> CheckedPackage {
    let graph = PackageDeclarations {
        types,
        ieee_profile: ieee.then(|| profile().clone()),
        ..PackageDeclarations::new(qsl_semantics::check::fixture_owner())
    }
    .check(CheckingLimits::default())
    .unwrap();
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import.
    CheckedPackage::link(graph)
}

fn plain_package() -> CheckedPackage {
    expression_package(TypeEnvironment::default(), false)
}

fn operand(spelling: &str) -> Expression {
    Expression::Name(spelling.to_owned())
}

fn operation(operator: BinaryOperator, left: &str, right: &str) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(operand(left)),
        right: Box::new(operand(right)),
    }
}

fn negation(spelling: &str) -> Expression {
    Expression::Negate(Box::new(operand(spelling)))
}

fn check_in(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    expected: Option<&ValueType>,
    mode: CheckMode,
) -> Result<CheckedExpression, CheckRefusal> {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package.graph().check_expression(
        parameters,
        expression,
        expected,
        mode,
        CheckingLimits::default(),
    )
}

/// The cause of a checking refusal.
fn refused(result: Result<CheckedExpression, CheckRefusal>) -> CheckCause {
    match result {
        Err(refusal) => refusal.cause,
        Ok(_) => panic!("a checking refusal"),
    }
}

/// Like [`run_in_losses`], discarding `location`/`losses` for the many
/// fixtures in this file that do not assert on either.
fn run_in(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    expected: Option<&ValueType>,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> (quire_exact::Outcome<Value>, Meter) {
    let (outcome, _losses, meter) =
        run_in_losses(package, parameters, expression, expected, arguments, limits);
    (outcome, meter)
}

/// FR-090: `CheckedPackage::evaluate` returns `Evaluation { outcome:
/// FamilyOutcome, losses, .. }` (FR-090-OQ-3, ruled option A); `Evaluated`
/// carries the kernel `quire_exact::Outcome<Value>` unchanged (ADR-013
/// O-16). Every fixture in this file completes, refuses, is undefined or
/// incomplete through the kernel path -- none hits a family-owned
/// evaluation-time result.
fn run_in_losses(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    expected: Option<&ValueType>,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> (quire_exact::Outcome<Value>, Vec<LocatedLoss>, Meter) {
    let checked = check_in(package, parameters, expression, expected, CheckMode::Kernel).unwrap();
    let mut meter = Meter::new(limits);
    let evaluation: Evaluation = package
        .evaluate(
            &checked,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap();
    let outcome = match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected FamilyOutcome::Evaluated(_), got {other:?}"),
    };
    (outcome, evaluation.losses, meter)
}

fn completed_as(outcome: &quire_exact::Outcome<Value>, expected: &Value) {
    assert_eq!(
        format!("{outcome:?}"),
        format!("{:?}", Outcome::Completed(expected.clone()))
    );
}

#[trace("TC-191", "FR-146-AC-6")]
#[test]
fn x01_rational_arithmetic_evaluates_and_its_range_and_divisor_are_obligations() {
    let package = plain_package();
    let parameters = [
        ("a", rational_type(0, 3, 1, 2)),
        ("b", rational_type(-2, 2, 1, 3)),
    ];
    let arguments = || vec![rational(3, 2), rational(-1, 3)];
    let rows = [
        (operation(BinaryOperator::Add, "a", "b"), rational(7, 6)),
        (
            operation(BinaryOperator::Subtract, "a", "b"),
            rational(11, 6),
        ),
        (
            operation(BinaryOperator::Multiply, "a", "b"),
            rational(-1, 2),
        ),
        (negation("a"), rational(-3, 2)),
    ];
    for (expression, expected) in &rows {
        let (evaluation, meter) = run_in(
            &package,
            &parameters,
            expression,
            None,
            arguments(),
            UNLIMITED,
        );
        completed_as(&evaluation, expected);
        assert_eq!(
            meter.admitted_charges().first(),
            Some(&ChargePoint::RationalArithmeticOperands)
        );
        check_in(&package, &parameters, expression, None, CheckMode::Linked).unwrap();
    }

    // `a + b` has numerator `[0,9] + [-4,4]` over `[1,6]`, outside `[0,3]/[1,6]`.
    let narrow = rational_type(0, 3, 1, 6);
    let sum = operation(BinaryOperator::Add, "a", "b");
    assert_eq!(
        refused(check_in(
            &package,
            &parameters,
            &sum,
            Some(&narrow),
            CheckMode::Linked
        )),
        CheckCause::Unproved(Obligation::RationalRange)
    );
    let (evaluation, _) = run_in(
        &package,
        &parameters,
        &sum,
        Some(&narrow),
        arguments(),
        UNLIMITED,
    );
    assert!(matches!(
        evaluation,
        quire_exact::Outcome::Refused(quire_exact::Refusal::RationalOutOfDomain)
    ));
    assert_eq!(
        refused(check_in(
            &package,
            &parameters,
            &operation(BinaryOperator::Divide, "a", "b"),
            Some(&rational_type(-100, 100, 1, 100)),
            CheckMode::Linked
        )),
        CheckCause::Unproved(Obligation::Nonzero)
    );
}

#[trace("TC-185", "FR-140-AC-3")]
#[test]
fn x02_decimal_arithmetic_takes_its_target_and_retains_the_loss() {
    let package = plain_package();
    let parameters = [
        ("p", decimal_type(0, 100, 0, 0)),
        ("q", decimal_type(1, 9, 0, 0)),
    ];
    let target = decimal_type_mode(0, 10_000, 2, 2, RoundingMode::NearestEven);
    let quotient = operation(BinaryOperator::Divide, "p", "q");
    let (evaluation, losses, _) = run_in_losses(
        &package,
        &parameters,
        &quotient,
        Some(&target),
        vec![decimal(1, 0), decimal(3, 0)],
        UNLIMITED,
    );
    completed_as(&evaluation, &decimal(33, 2));
    // FR-090-OQ-3 (ruled option A): `Evaluation.losses` carries the division's
    // own rounding loss.
    assert_eq!(losses.len(), 1);
    assert!(matches!(losses[0].loss, ValueLoss::Decimal(_)));
    assert_eq!(losses[0].location.path, Vec::<usize>::new());
    check_in(
        &package,
        &parameters,
        &quotient,
        Some(&target),
        CheckMode::Linked,
    )
    .unwrap();

    let (exact, exact_losses, _) = run_in_losses(
        &package,
        &parameters,
        &operation(BinaryOperator::Add, "p", "q"),
        Some(&target),
        vec![decimal(1, 0), decimal(3, 0)],
        UNLIMITED,
    );
    completed_as(&exact, &decimal(400, 2));
    assert!(exact_losses.is_empty());

    let zero_divisor = [
        ("p", decimal_type(0, 100, 0, 0)),
        ("q", decimal_type(0, 9, 0, 0)),
    ];
    assert_eq!(
        refused(check_in(
            &package,
            &zero_divisor,
            &quotient,
            Some(&target),
            CheckMode::Linked
        )),
        CheckCause::Unproved(Obligation::Nonzero)
    );
    assert_eq!(
        refused(check_in(
            &package,
            &parameters,
            &quotient,
            None,
            CheckMode::Kernel
        )),
        CheckCause::IllTyped(IllTypedCause::AmbiguousLiteral)
    );
}

#[trace("TC-186", "FR-141-AC-4")]
#[test]
fn x03_text_orders_lexicographically_within_one_profile() {
    let package = plain_package();
    let nfc = ValueType::Text(text_type(TextProfile::Nfc));
    let binary = ValueType::Text(text_type(TextProfile::BinaryUtf8));
    let less = operation(BinaryOperator::Less, "s", "t");
    let (evaluation, meter) = run_in(
        &package,
        &[("s", nfc.clone()), ("t", nfc.clone())],
        &less,
        None,
        vec![text("a", TextProfile::Nfc), text("b", TextProfile::Nfc)],
        UNLIMITED,
    );
    completed_as(&evaluation, &Value::Boolean(true));
    assert!(!meter.admitted_charges().is_empty());
    assert_eq!(
        refused(check_in(
            &package,
            &[("s", nfc), ("t", binary)],
            &less,
            None,
            CheckMode::Kernel
        )),
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

#[trace("TC-187", "FR-142-AC-9")]
#[trace("TC-187", "FR-142-AC-2")]
#[test]
fn x04_quantities_order_and_add_only_in_one_unit() {
    let units = units();
    let package = expression_package(
        TypeEnvironment::default().with_units(units.table.clone()),
        false,
    );
    let whole = |value: i64| Rational::from_integer(integer(value));
    let metres = ValueType::Quantity(units.m);
    let metre = |value| Value::Quantity(Quantity::new(whole(value), units.m));
    let parameters = [
        ("x", metres.clone()),
        ("y", metres.clone()),
        ("c", ValueType::Quantity(units.cm)),
        ("s", ValueType::Quantity(units.s)),
    ];
    let arguments = || {
        vec![
            metre(1),
            metre(2),
            Value::Quantity(Quantity::new(whole(1), units.cm)),
            Value::Quantity(Quantity::new(whole(1), units.s)),
        ]
    };
    let (ordered, _) = run_in(
        &package,
        &parameters,
        &operation(BinaryOperator::Less, "x", "y"),
        None,
        arguments(),
        UNLIMITED,
    );
    completed_as(&ordered, &Value::Boolean(true));
    let (added, _) = run_in(
        &package,
        &parameters,
        &operation(BinaryOperator::Add, "x", "y"),
        None,
        arguments(),
        UNLIMITED,
    );
    completed_as(&added, &metre(3));

    let cause = |expression: &Expression| {
        refused(check_in(
            &package,
            &parameters,
            expression,
            None,
            CheckMode::Kernel,
        ))
    };
    // Checking reports the kernel's distinct-units and incompatible-dimensions
    // causes under their FR-272 tag, `type-mismatch`.
    for operator in [BinaryOperator::Less, BinaryOperator::Add] {
        for other in ["c", "s"] {
            assert_eq!(
                cause(&operation(operator, "x", other)),
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            );
        }
    }
    assert_eq!(
        cause(&negation("x")),
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

/// A product or quotient's unit is a compound unit no package declares:
/// checking forms it as the node's static type, and evaluation forms it again
/// when the value exists, so a further operation reads it by its `UnitId`.
#[trace("TC-187", "FR-142-AC-6")]
#[test]
fn x04_compound_results_feed_further_quantity_operations() {
    let units = units();
    let package = expression_package(
        TypeEnvironment::default().with_units(units.table.clone()),
        false,
    );
    let whole = |value: i64| Rational::from_integer(integer(value));
    let parameters = [
        ("x", ValueType::Quantity(units.m)),
        ("y", ValueType::Quantity(units.m)),
        ("t", ValueType::Quantity(units.s)),
    ];
    let arguments = || {
        vec![
            Value::Quantity(Quantity::new(whole(2), units.m)),
            Value::Quantity(Quantity::new(whole(3), units.m)),
            Value::Quantity(Quantity::new(whole(4), units.s)),
        ]
    };
    let binary = |operator, left, right| Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    };
    let times_t = |name: &str| binary(BinaryOperator::Multiply, operand(name), operand("t"));

    let (quotient, _) = run_in(
        &package,
        &parameters,
        &binary(BinaryOperator::Divide, times_t("x"), operand("t")),
        None,
        arguments(),
        UNLIMITED,
    );
    let quire_exact::Outcome::Completed(Value::Quantity(quotient)) = quotient else {
        panic!("expected a completed quantity, got {quotient:?}");
    };
    assert_eq!(quotient.magnitude(), &whole(2));
    // `m*s/s` is the compound unit `m^1`, never the declared unit `m`.
    assert_eq!(quotient.unit().domain(), UnitDomain::Compound);
    assert_ne!(quotient.unit(), units.m);

    // Ordering and FR-149 equality over two compound operands: checking
    // resolves the formed unit, and the checked equality evaluates with it.
    for (operator, left, right, expected) in [
        (BinaryOperator::Less, "x", "y", true),
        (BinaryOperator::Equal, "x", "y", false),
        (BinaryOperator::Equal, "x", "x", true),
        (BinaryOperator::NotEqual, "x", "y", true),
        (BinaryOperator::NotEqual, "x", "x", false),
    ] {
        let (compared, _) = run_in(
            &package,
            &parameters,
            &binary(operator, times_t(left), times_t(right)),
            None,
            arguments(),
            UNLIMITED,
        );
        completed_as(&compared, &Value::Boolean(expected));
    }
}

/// An S6a invariant break, not a refusal: a quantity expression checked
/// against one package's units and evaluated in a package whose table lacks
/// them passes admission (the argument's unit id matches the parameter's) and
/// reaches an operation whose operand unit nothing resolves.
#[trace("TC-384", "FR-090-AC-3")]
#[test]
fn x04_an_unresolved_unit_past_admission_is_an_internal_fault() {
    let units = units();
    let with_units = expression_package(
        TypeEnvironment::default().with_units(units.table.clone()),
        false,
    );
    let parameters = [("x", ValueType::Quantity(units.m))];
    let one_metre = Value::Quantity(Quantity::new(Rational::from_integer(integer(1)), units.m));
    for operator in [BinaryOperator::Add, BinaryOperator::Less] {
        let checked = check_in(
            &with_units,
            &parameters,
            &operation(operator, "x", "x"),
            None,
            CheckMode::Kernel,
        )
        .unwrap();
        let mut meter = Meter::new(UNLIMITED);
        let failure = plain_package()
            .evaluate(
                &checked,
                vec![one_metre.clone()],
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .expect_err("an unresolved unit must fault, never evaluate");
        let CallFailure::Fault(fault) = failure else {
            panic!("expected an internal fault, got {failure:?}");
        };
        assert_eq!(fault.stage(), "S6a");
        assert_eq!(fault.invariant(), "quantity-unit-unresolved-past-admission");
    }
}

#[trace("TC-193", "FR-148-AC-8")]
#[trace("TC-193", "FR-148-AC-6")]
#[test]
fn x05_ieee_arithmetic_records_flags_and_grammar_ordering_is_ineligible() {
    let package = expression_package(TypeEnvironment::default(), true);
    let double = ValueType::Float(IeeeWidth::Binary64);
    let parameters = [
        ("f", double.clone()),
        ("g", double.clone()),
        ("h", ValueType::Float(IeeeWidth::Binary32)),
        ("i", ValueType::Integer),
    ];
    let one = IeeeValue::binary64(0x3ff0_0000_0000_0000);
    let three = IeeeValue::binary64(0x4008_0000_0000_0000);
    let arguments = |left, right| {
        vec![
            Value::Float(left),
            Value::Float(right),
            Value::Float(IeeeValue::binary32(0)),
            int(0),
        ]
    };
    let divide = operation(BinaryOperator::Divide, "f", "g");
    // An omitted rounding spelling is strict `exact`: `1 / 3` refuses.
    let (inexact, inexact_losses, _) = run_in_losses(
        &package,
        &parameters,
        &divide,
        None,
        arguments(one, three),
        UNLIMITED,
    );
    assert!(matches!(
        inexact,
        quire_exact::Outcome::Refused(quire_exact::Refusal::IeeeNotExact { .. })
    ));
    assert!(inexact_losses.is_empty());
    let (infinite, infinite_losses, _) = run_in_losses(
        &package,
        &parameters,
        &divide,
        None,
        arguments(one, IeeeValue::binary64(0)),
        UNLIMITED,
    );
    completed_as(
        &infinite,
        &Value::Float(IeeeValue::binary64(0x7ff0_0000_0000_0000)),
    );
    // FR-090-OQ-3 (ruled option A): `Evaluation.losses` carries the
    // `DivideByZero` flag this division records.
    match infinite_losses.as_slice() {
        [LocatedLoss {
            loss: ValueLoss::IeeeFlags(flags),
            ..
        }] => assert!(flags.contains(IeeeFlag::DivideByZero)),
        other => panic!("one flag record, not {other:?}"),
    }

    let negative_zero = IeeeValue::binary64(0x8000_0000_0000_0000);
    let (exact, exact_losses, _) = run_in_losses(
        &package,
        &parameters,
        &Expression::Convert {
            target: crate::support::type_form::type_form(&rational_type(0, 0, 1, 1)),
            operand: Box::new(operand("f")),
        },
        None,
        arguments(negative_zero, three),
        UNLIMITED,
    );
    completed_as(&exact, &rational(0, 1));
    // FR-090-OQ-3 (ruled option A): `Evaluation.losses` carries the
    // `NegativeZeroSign` loss this conversion records.
    assert!(matches!(
        exact_losses.as_slice(),
        [LocatedLoss {
            loss: ValueLoss::IeeeExact(IeeeExactLoss::NegativeZeroSign),
            ..
        }]
    ));

    let cause = |package: &CheckedPackage, expression: &Expression| {
        refused(check_in(
            package,
            &parameters,
            expression,
            None,
            CheckMode::Kernel,
        ))
    };
    let ill = |cause| CheckCause::IllTyped(cause);
    assert_eq!(
        cause(&package, &operation(BinaryOperator::Less, "f", "g")),
        ill(IllTypedCause::OperatorIneligible)
    );
    assert_eq!(
        cause(&package, &negation("f")),
        ill(IllTypedCause::OperatorIneligible)
    );
    assert_eq!(
        cause(&package, &operation(BinaryOperator::Add, "f", "h")),
        ill(IllTypedCause::TypeMismatch)
    );
    for operator in [BinaryOperator::Add, BinaryOperator::Less] {
        assert_eq!(
            cause(&package, &operation(operator, "f", "i")),
            ill(IllTypedCause::TypeMismatch)
        );
    }
    assert_eq!(
        cause(&plain_package(), &operation(BinaryOperator::Add, "f", "g")),
        CheckCause::IeeeProfileNotAdmitted
    );
}

fn record(name: &str, fields: Vec<(&str, Expression)>) -> Expression {
    Expression::Record {
        name: name.to_owned(),
        fields: fields
            .into_iter()
            .map(|(field, value)| (field.to_owned(), FieldInitializer::Value(value)))
            .collect(),
    }
}

fn plus_one(spelling: &str) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::Add,
        left: Box::new(operand(spelling)),
        right: Box::new(Expression::Integer(integer(1))),
    }
}

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e20_source_order_row_evaluates_fields_in_declaration_order() {
    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Two"),
            "Two",
            CompositeShape::Record(vec![
                FieldDeclaration::new("a", ValueType::Integer, Presence::Required),
                FieldDeclaration::new("b", ValueType::Integer, Presence::Required),
            ]),
        )],
        [],
    )
    .unwrap();
    let graph = PackageDeclarations {
        types,
        functions: vec![FunctionDeclaration::new(
            "pick".to_owned(),
            vec![(
                "n".to_owned(),
                crate::support::type_form::type_form(&int_type(0, 1)),
            )],
            crate::support::type_form::type_form(&ValueType::Integer),
            None,
            operand("n"),
        )],
        ..PackageDeclarations::new(qsl_semantics::check::fixture_owner())
    }
    .check(CheckingLimits::default())
    .unwrap();
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import.
    let package = CheckedPackage::link(graph);
    let parameters = [("p", ValueType::Integer)];
    // `eA = pick(p)` refuses its `Int[0,1]` argument before `function.call`;
    // `eB = p + 1` charges and is incomplete under the zero tuple.
    let e_a = || Expression::Call {
        name: "pick".to_owned(),
        arguments: vec![operand("p")],
    };
    let right = || record("Two", vec![("a", plus_one("p")), ("b", plus_one("p"))]);
    let compare = |left: Expression| Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(left),
        right: Box::new(right()),
    };

    let (refused, meter) = run_in(
        &package,
        &parameters,
        &compare(record("Two", vec![("b", plus_one("p")), ("a", e_a())])),
        None,
        vec![int(5)],
        ZERO,
    );
    assert!(matches!(
        refused,
        quire_exact::Outcome::Refused(quire_exact::Refusal::IntegerOutOfDomain)
    ));
    assert!(meter.admitted_charges().is_empty());

    let (incomplete, _) = run_in(
        &package,
        &parameters,
        &compare(record(
            "Two",
            vec![("b", plus_one("p")), ("a", operand("p"))],
        )),
        None,
        vec![int(5)],
        ZERO,
    );
    assert!(matches!(
        incomplete,
        quire_exact::Outcome::Incomplete(Incomplete {
            charge_point: ChargePoint::IntegerArithmeticOperands,
            ..
        })
    ));
}

#[trace("TC-194", "FR-149-AC-10")]
#[test]
fn e26_let_bound_conversions_are_ordinary_conversions() {
    let package = plain_package();
    let let_equal = |target: ValueType| Expression::Let {
        name: "x".to_owned(),
        value: Box::new(Expression::Convert {
            target: crate::support::type_form::type_form(&target),
            operand: Box::new(operand("e")),
        }),
        body: Box::new(operation(BinaryOperator::Equal, "x", "d")),
    };

    let whole = [("e", decimal_type(0, 9, 0, 0)), ("d", int_type(0, 9))];
    let (evaluation, evaluation_losses, _) = run_in_losses(
        &package,
        &whole,
        &let_equal(int_type(0, 9)),
        None,
        vec![decimal(3, 0), int(3)],
        UNLIMITED,
    );
    completed_as(&evaluation, &Value::Boolean(true));
    assert!(evaluation_losses.is_empty());

    // E04's `r` and `d`: no equality conversion, but an ordinary FR-140 one.
    let target = decimal_type_mode(0, 100, 2, 2, RoundingMode::NearestEven);
    let thirds = [("e", rational_type(0, 1, 1, 3)), ("d", target.clone())];
    let (evaluation, evaluation_losses, _) = run_in_losses(
        &package,
        &thirds,
        &let_equal(target.clone()),
        None,
        vec![rational(1, 3), decimal(33, 2)],
        UNLIMITED,
    );
    completed_as(&evaluation, &Value::Boolean(true));
    // FR-090-OQ-3 (ruled option A): `Evaluation.losses` carries this
    // conversion's own decimal loss.
    match evaluation_losses.as_slice() {
        [LocatedLoss {
            location,
            loss: ValueLoss::Decimal(_),
        }] => assert_eq!(location.path, vec![0]),
        other => panic!("one decimal loss, not {other:?}"),
    }
    let direct = Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(Expression::Convert {
            target: crate::support::type_form::type_form(&target),
            operand: Box::new(operand("e")),
        }),
        right: Box::new(operand("d")),
    };
    assert_eq!(
        refused(check_in(
            &package,
            &thirds,
            &direct,
            None,
            CheckMode::Kernel
        )),
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}
