// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-187 dimensions, units, compound units and quantities over the real
//! `value` boundary.
//!
//! The dimension/unit node fixtures, their invalid mutations, the
//! compound-unit fixtures and U01–U23 are self-authored preimages, checked
//! for content-addressing against an independent JCS canonicalizer (never
//! the code under test) and for admission against the real `value` boundary.

use ix_trace_rs::trace;
use num_bigint::{BigInt, BigUint};
use num_traits::Pow;
use quire_exact::{Integer, IntegerInterval};
use quire_spec_language::value::{
    compare_quantity, convert_quantity, evaluate_quantity, ChargePoint, ComparisonOperator,
    CompoundUnitCause, CompoundUnitPreimage, ConvertedValue, Decimal, DecimalType, Dimension,
    DimensionPreimage, IllTyped, IllTypedCause, Incomplete, InjectedDenial, InvalidCompoundUnit,
    InvalidSemanticGraph, LimitKind, Meter, NodeKey, NodeOwner, Outcome, OwnerSelection,
    OwnerSubject, Quantity, QuantityOperation, QuantityTarget, QuantityUnit, Rational, Refusal,
    RoundingMode, ScalarLimits, SemanticGraphCause, Undefined, UnitGraph, UnitPreimage,
};
use serde_json::{json, Value};
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

/// Independent RFC 8785 JCS for the string-only preimages.
fn jcs(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => value.to_string(),
        Value::Number(_) => panic!("preimages carry no JSON numbers"),
        Value::Array(items) => format!("[{}]", items.iter().map(jcs).collect::<Vec<_>>().join(",")),
        Value::Object(members) => {
            let mut entries: Vec<_> = members.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            let body: Vec<_> = entries
                .into_iter()
                .map(|(key, item)| format!("{}:{}", Value::String(key.clone()), jcs(item)))
                .collect();
            format!("{{{}}}", body.join(","))
        }
    }
}

fn digest_hex(preimage: &Value) -> String {
    Sha256::digest(jcs(preimage).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fixture_key(preimage: &Value) -> NodeKey {
    NodeKey::from_hex(&digest_hex(preimage)).unwrap()
}

// ---- graph fixtures --------------------------------------------------------

fn owner_json(identity: &str) -> Value {
    json!({"kind": "definition", "authority": "agent-ix", "identity": identity})
}

fn example_owner(identity: &str) -> NodeOwner {
    NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".into(),
        identity: identity.into(),
    })
}

fn owners() -> OwnerSelection {
    OwnerSelection::new([example_owner("example-model"), example_owner("other-model")])
}

fn node_id(key: NodeKey) -> Value {
    json!({"domain": "quire.checked-semantic-node/v1", "digest": key.to_string()})
}

fn base_dimension(identity: &str, name: &str) -> Value {
    json!({
        "version": "quire.dimension-node/v1",
        "owner": owner_json(identity),
        "qualified_declaration": ["Example", name],
        "terms": [],
    })
}

fn rational_json((numerator, denominator): (&str, &str)) -> Value {
    json!({"numerator": numerator, "denominator": denominator})
}

fn unit_json(
    name: &str,
    dimension: NodeKey,
    target: Option<NodeKey>,
    scale: (&str, &str),
    offset: (&str, &str),
) -> Value {
    json!({
        "version": "quire.unit-node/v1",
        "owner": owner_json("example-model"),
        "qualified_declaration": ["Example", name],
        "dimension_node_id": node_id(dimension),
        "target_unit_node_id": target.map_or(Value::Null, node_id),
        "scale": rational_json(scale),
        "offset": rational_json(offset),
    })
}

fn root_json(name: &str, dimension: NodeKey) -> Value {
    unit_json(name, dimension, None, ("1", "1"), ("0", "1"))
}

/// A graph input: preimages paired with their honestly recomputed keys.
#[derive(Clone, Default)]
struct Nodes {
    dimensions: Vec<(Value, NodeKey)>,
    units: Vec<(Value, NodeKey)>,
}

impl Nodes {
    fn dimension(&mut self, preimage: Value) -> NodeKey {
        let key = fixture_key(&preimage);
        self.dimensions.push((preimage, key));
        key
    }

    fn unit(&mut self, preimage: Value) -> NodeKey {
        let key = fixture_key(&preimage);
        self.units.push((preimage, key));
        key
    }

    fn admit(&self) -> Result<UnitGraph, InvalidSemanticGraph> {
        let dimensions: Result<Vec<_>, _> = self
            .dimensions
            .iter()
            .map(|(preimage, key)| Ok((DimensionPreimage::from_json(preimage.clone())?, *key)))
            .collect();
        let units: Result<Vec<_>, _> = self
            .units
            .iter()
            .map(|(preimage, key)| Ok((UnitPreimage::from_json(preimage.clone())?, *key)))
            .collect();
        UnitGraph::admit(dimensions?, units?, &owners())
    }
}

/// The TC-187 fixture: `L`, `T`, `Theta`; `m`, `s`, `K`, `cm`, `in`, `degC`,
/// `degF`.
struct Fixture {
    nodes: Nodes,
    graph: UnitGraph,
    length: NodeKey,
    time: NodeKey,
    m: NodeKey,
    s: NodeKey,
    kelvin: NodeKey,
    cm: NodeKey,
    inch: NodeKey,
    deg_c: NodeKey,
    deg_f: NodeKey,
}

fn fixture() -> Fixture {
    let mut nodes = Nodes::default();
    let length = nodes.dimension(base_dimension("example-model", "Length"));
    let time = nodes.dimension(base_dimension("example-model", "Time"));
    let theta = nodes.dimension(base_dimension("example-model", "Temperature"));
    let m = nodes.unit(root_json("metre", length));
    let s = nodes.unit(root_json("second", time));
    let kelvin = nodes.unit(root_json("kelvin", theta));
    let cm = nodes.unit(unit_json(
        "centimetre",
        length,
        Some(m),
        ("1", "100"),
        ("0", "1"),
    ));
    let inch = nodes.unit(unit_json(
        "inch",
        length,
        Some(m),
        ("127", "5000"),
        ("0", "1"),
    ));
    let deg_c = nodes.unit(unit_json(
        "degree_Celsius",
        theta,
        Some(kelvin),
        ("1", "1"),
        ("5463", "20"),
    ));
    let deg_f = nodes.unit(unit_json(
        "degree_Fahrenheit",
        theta,
        Some(kelvin),
        ("5", "9"),
        ("45967", "180"),
    ));
    let graph = nodes.admit().unwrap();
    Fixture {
        nodes,
        graph,
        length,
        time,
        m,
        s,
        kelvin,
        cm,
        inch,
        deg_c,
        deg_f,
    }
}

impl Fixture {
    fn unit(&self, key: NodeKey) -> QuantityUnit {
        QuantityUnit::Declared(Box::new(self.graph.unit(key).unwrap().clone()))
    }

    fn quantity(&self, value: Rational, key: NodeKey) -> Quantity {
        Quantity::new(value, self.unit(key))
    }
}

fn int(value: i64) -> Integer {
    Integer::from(value)
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(int(numerator), int(denominator)).unwrap()
}

fn whole(value: i64) -> Rational {
    Rational::from_integer(int(value))
}

fn evaluated(operation: QuantityOperation<'_>, meter: &mut Meter) -> Outcome<Quantity> {
    evaluate_quantity(operation, meter).expect("well-typed operation")
}

fn typed<T>(cause: IllTypedCause) -> Result<T, IllTyped> {
    Err(IllTyped { cause })
}

fn exact(outcome: Outcome<Quantity>) -> Quantity {
    match outcome {
        Outcome::Completed(quantity) => quantity,
        other => panic!("expected a completed quantity, got {other:?}"),
    }
}

fn converted(
    source: &Quantity,
    unit: &QuantityUnit,
    target: &QuantityTarget,
    meter: &mut Meter,
) -> Outcome<ConvertedValue> {
    match convert_quantity(source, unit, target, meter).expect("well-typed conversion") {
        Outcome::Completed(conversion) => Outcome::Completed(conversion.value().clone()),
        Outcome::Undefined(reason) => Outcome::Undefined(reason),
        Outcome::Refused(reason) => Outcome::Refused(reason),
        Outcome::Incomplete(record) => Outcome::Incomplete(record),
    }
}

fn unlimited() -> Meter {
    Meter::new(UNLIMITED)
}

fn compound(terms: &[(NodeKey, &str)]) -> CompoundUnitPreimage {
    let terms: Vec<Value> = terms
        .iter()
        .map(|(key, exponent)| json!({"unit_node_id": node_id(*key), "exponent": exponent}))
        .collect();
    CompoundUnitPreimage::from_json(
        json!({"version": "quire.value.compound-unit/v1", "terms": terms}),
    )
    .unwrap()
}

fn compound_unit(fixture: &Fixture, terms: &[(NodeKey, &str)]) -> QuantityUnit {
    QuantityUnit::Compound(fixture.graph.compound_unit(&compound(terms)).unwrap())
}

// ---- node identity fixtures ---------------------------------------------------

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn dimension_and_unit_node_preimages_are_content_addressed_and_admit() {
    let mut nodes = Nodes::default();
    let length = nodes.dimension(base_dimension("example-model", "Length"));
    let time = nodes.dimension(base_dimension("example-model", "Time"));
    let theta = nodes.dimension(base_dimension("example-model", "Temperature"));
    let velocity_preimage = derived_dimension("Velocity", &[(time, "-1"), (length, "1")]);
    let velocity = nodes.dimension(velocity_preimage.clone());
    let metre_preimage = root_json("metre", length);
    let metre = nodes.unit(metre_preimage.clone());
    let kelvin_preimage = root_json("kelvin", theta);
    let kelvin = nodes.unit(kelvin_preimage.clone());
    let cm_preimage = unit_json("centimetre", length, Some(metre), ("1", "100"), ("0", "1"));
    let cm = nodes.unit(cm_preimage.clone());
    let mm_preimage = unit_json("millimetre", length, Some(cm), ("1", "10"), ("0", "1"));
    let mm = nodes.unit(mm_preimage.clone());
    let huge_scale: Integer = "340282366920938463463374607431768211457".parse().unwrap();
    let huge_preimage = unit_json(
        "huge_exact_scale",
        length,
        Some(metre),
        (&huge_scale.to_string(), "1"),
        ("0", "1"),
    );
    let huge = nodes.unit(huge_preimage.clone());
    let celsius_preimage = unit_json(
        "degree_Celsius",
        theta,
        Some(kelvin),
        ("1", "1"),
        ("5463", "20"),
    );
    let celsius = nodes.unit(celsius_preimage.clone());

    // Every preimage's independently recomputed content digest matches the
    // crate's own node key, for both the dimension and unit preimage shapes.
    let dimensions = [(velocity_preimage, velocity)];
    let units = [
        (metre_preimage, metre),
        (kelvin_preimage, kelvin),
        (cm_preimage, cm),
        (mm_preimage, mm),
        (huge_preimage, huge),
        (celsius_preimage, celsius),
    ];
    for (preimage, key) in &dimensions {
        assert_eq!(NodeKey::from_hex(&digest_hex(preimage)).unwrap(), *key);
        assert_eq!(
            DimensionPreimage::from_json(preimage.clone())
                .unwrap()
                .node_key()
                .unwrap(),
            *key
        );
    }
    for (preimage, key) in &units {
        assert_eq!(NodeKey::from_hex(&digest_hex(preimage)).unwrap(), *key);
        assert_eq!(
            UnitPreimage::from_json(preimage.clone())
                .unwrap()
                .node_key()
                .unwrap(),
            *key
        );
    }

    let graph = nodes.admit().unwrap();
    let velocity_node = graph.dimension(velocity).unwrap();
    let terms: Vec<_> = velocity_node
        .exponents()
        .map(|(base, exponent)| (base, exponent.clone()))
        .collect();
    assert_eq!(terms, [(time, int(-1)), (length, int(1))]);

    let mm_node = graph.unit(mm).unwrap();
    assert_eq!(mm_node.root(), metre);
    assert_eq!(mm_node.path().len(), 2);
    assert_eq!(mm_node.canonical().scale(), &ratio(1, 1000));
    assert!(!mm_node.is_affine());
    let huge_node = graph.unit(huge).unwrap();
    assert_eq!(
        huge_node.canonical().scale(),
        &Rational::from_integer(huge_scale)
    );
    let celsius_node = graph.unit(celsius).unwrap();
    assert!(celsius_node.is_affine());
    assert_eq!(celsius_node.root(), kelvin);
    assert_eq!(celsius_node.canonical().offset(), &ratio(5463, 20));
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn dimension_and_unit_semantic_mutations_refuse_by_their_named_cause() {
    let mut base = Nodes::default();
    let length = base.dimension(base_dimension("example-model", "Length"));
    let time = base.dimension(base_dimension("example-model", "Time"));
    let metre = base.unit(root_json("metre", length));

    let mut zero_exponent = base.clone();
    zero_exponent.dimension(derived_dimension("ZeroTerm", &[(length, "0")]));
    assert_eq!(
        zero_exponent.admit().unwrap_err().cause,
        SemanticGraphCause::ZeroExponent
    );

    let mut duplicate_term = base.clone();
    duplicate_term.dimension(derived_dimension(
        "DupTerm",
        &[(length, "1"), (length, "2")],
    ));
    assert_eq!(
        duplicate_term.admit().unwrap_err().cause,
        SemanticGraphCause::DuplicateTerm
    );

    // `derived_dimension` sorts its own terms, so an unsorted preimage is
    // built directly: the higher key must come first.
    let (first, second) = if length < time {
        (time, length)
    } else {
        (length, time)
    };
    let mut unsorted_terms = base.clone();
    unsorted_terms.dimension(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner_json("example-model"),
        "qualified_declaration": ["Example", "Unsorted"],
        "terms": [
            {"dimension_node_id": node_id(first), "exponent": "1"},
            {"dimension_node_id": node_id(second), "exponent": "1"},
        ],
    }));
    assert_eq!(
        unsorted_terms.admit().unwrap_err().cause,
        SemanticGraphCause::UnsortedTerms
    );

    // A root unit (no target) whose scale is not the identity.
    let mut nonidentity_root = base.clone();
    nonidentity_root.unit(unit_json("bad_root", length, None, ("2", "1"), ("0", "1")));
    assert_eq!(
        nonidentity_root.admit().unwrap_err().cause,
        SemanticGraphCause::NonIdentityRoot
    );

    // A scale rational not in lowest terms.
    let mut unreduced = base.clone();
    unreduced.unit(unit_json(
        "unreduced",
        length,
        Some(metre),
        ("2", "4"),
        ("0", "1"),
    ));
    assert_eq!(
        unreduced.admit().unwrap_err().cause,
        SemanticGraphCause::UnreducedRational
    );
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn stale_keys_and_foreign_owners_refuse_admission() {
    let base = fixture();
    let mut stale = base.nodes.clone();
    stale.units[3].0["qualified_declaration"] = json!(["Example", "renamed"]);
    assert_eq!(
        stale.admit().unwrap_err().cause,
        SemanticGraphCause::StaleKey
    );
    let mut stale = base.nodes.clone();
    stale.dimensions[0].0["qualified_declaration"] = json!(["Example", "Distance"]);
    assert_eq!(
        stale.admit().unwrap_err().cause,
        SemanticGraphCause::StaleKey
    );
    let mut foreign = base.nodes.clone();
    foreign.units[0].0["owner"]["identity"] = json!("absent-model");
    assert_eq!(
        foreign.admit().unwrap_err().cause,
        SemanticGraphCause::OwnerNotSelected
    );
    let mut repeated = base.nodes.clone();
    repeated.units.push(repeated.units[0].clone());
    assert_eq!(
        repeated.admit().unwrap_err().cause,
        SemanticGraphCause::DuplicateNode
    );
    // A derived dimension over an unknown or derived term.
    let mut derived = base.nodes.clone();
    let velocity = json!({
        "version": "quire.dimension-node/v1",
        "owner": owner_json("example-model"),
        "qualified_declaration": ["Example", "Velocity"],
        "terms": [{"dimension_node_id": node_id(base.length), "exponent": "1"}],
    });
    let velocity_key = derived.dimension(velocity);
    let mut nested = derived.clone();
    nested.dimension(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner_json("example-model"),
        "qualified_declaration": ["Example", "Nested"],
        "terms": [{"dimension_node_id": node_id(velocity_key), "exponent": "1"}],
    }));
    assert!(derived.admit().is_ok());
    assert_eq!(
        nested.admit().unwrap_err().cause,
        SemanticGraphCause::NonBaseDimensionTerm
    );
    let mut unknown = base.nodes.clone();
    unknown.units.push((
        root_json("orphan", velocity_key),
        fixture_key(&root_json("orphan", velocity_key)),
    ));
    assert_eq!(
        unknown.admit().unwrap_err().cause,
        SemanticGraphCause::UnknownDimension
    );
}

// ---- compound-unit fixtures ---------------------------------------------------

/// A compound-unit preimage over `terms`, sorted by key as the canonical form
/// requires, kept as raw JSON so its digest can be independently recomputed.
fn compound_preimage(terms: &[(NodeKey, &str)]) -> Value {
    let mut terms = terms.to_vec();
    terms.sort_by_key(|(key, _)| *key);
    let terms: Vec<Value> = terms
        .iter()
        .map(|(key, exponent)| json!({"unit_node_id": node_id(*key), "exponent": exponent}))
        .collect();
    json!({"version": "quire.value.compound-unit/v1", "terms": terms})
}

#[trace("TC-187", "FR-142-AC-6")]
#[test]
fn compound_unit_preimages_are_content_addressed_and_mutations_refuse() {
    let mut nodes = Nodes::default();
    let length = nodes.dimension(base_dimension("example-model", "Length"));
    let time = nodes.dimension(base_dimension("example-model", "Time"));
    let metre = nodes.unit(root_json("metre", length));
    let second = nodes.unit(root_json("second", time));
    let cm = nodes.unit(unit_json(
        "centimetre",
        length,
        Some(metre),
        ("1", "100"),
        ("0", "1"),
    ));
    let graph = nodes.admit().unwrap();

    let dimensionless_preimage = compound_preimage(&[]);
    let per_time_preimage = compound_preimage(&[(metre, "1"), (second, "-1")]);
    for (preimage, is_dimensionless) in [
        (dimensionless_preimage.clone(), true),
        (per_time_preimage.clone(), false),
    ] {
        let expected_digest = digest_hex(&preimage);
        let parsed = CompoundUnitPreimage::from_json(preimage).unwrap();
        assert_eq!(parsed.identity().to_string(), expected_digest);
        let unit = graph.compound_unit(&parsed).unwrap();
        assert_eq!(unit.identity().to_string(), expected_digest);
        assert_eq!(unit.dimension().is_dimensionless(), is_dimensionless);
    }
    assert_ne!(
        digest_hex(&dimensionless_preimage),
        digest_hex(&per_time_preimage),
        "distinct preimages have distinct content-addressed identities"
    );

    assert_eq!(
        graph.compound_unit(&compound(&[(metre, "0")])),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::ZeroExponent
        })
    );
    assert_eq!(
        graph.compound_unit(&compound(&[(metre, "1"), (metre, "1")])),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::DuplicateTerm
        })
    );
    // Deliberately out of ascending-key order.
    let (first, second_term) = if metre < second {
        (second, metre)
    } else {
        (metre, second)
    };
    assert_eq!(
        graph.compound_unit(&compound(&[(first, "1"), (second_term, "1")])),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::UnsortedTerms
        })
    );

    // A term must name an admitted canonical root unit.
    assert_eq!(
        graph.compound_unit(&compound(&[(cm, "1")])),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::NotRootUnit
        })
    );

    let noncanonical = json!({"version": "quire.value.compound-unit/v1", "terms": [], "extra": 1});
    assert_eq!(
        CompoundUnitPreimage::from_json(noncanonical),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::NonCanonicalPreimage
        })
    );
}

// ---- U01–U13 -----------------------------------------------------------------

#[trace("TC-187", "FR-142-AC-1", "FR-142-AC-5")]
#[test]
fn u01_exact_conversion_then_arithmetic() {
    let f = fixture();
    let source = f.quantity(whole(250), f.cm);
    let mut meter = unlimited();
    let Outcome::Completed(conversion) =
        convert_quantity(&source, &f.unit(f.m), &QuantityTarget::Exact, &mut meter)
            .expect("well-typed conversion")
    else {
        panic!("conversion did not complete");
    };
    assert_eq!(conversion.source(), &source);
    assert_eq!(conversion.canonical(), &ratio(5, 2));
    assert_eq!(conversion.value(), &ConvertedValue::Exact(ratio(5, 2)));
    assert_eq!(conversion.unit(), &f.unit(f.m));
    let sum = exact(evaluated(
        QuantityOperation::Add(&f.quantity(ratio(5, 2), f.m), &f.quantity(whole(1), f.m)),
        &mut unlimited(),
    ));
    assert_eq!(sum.value(), &ratio(7, 2));
    assert_eq!(sum.unit(), &f.unit(f.m));
    let dimension: Vec<_> = sum
        .unit()
        .dimension()
        .exponents()
        .map(|(key, exponent)| (key, exponent.clone()))
        .collect();
    assert_eq!(dimension, [(f.length, int(1))]);
    // Conversion never occurs implicitly.
    assert_eq!(
        evaluate_quantity(
            QuantityOperation::Add(&source, &f.quantity(whole(1), f.m)),
            &mut unlimited()
        ),
        typed(IllTypedCause::DistinctUnits)
    );
}

#[trace("TC-187", "FR-142-AC-4")]
#[test]
fn u02_dimension_maps_normalize() {
    let f = fixture();
    let length = f.graph.dimension(f.length).unwrap();
    let time = f.graph.dimension(f.time).unwrap();
    assert_eq!(&length.multiply(time).divide(time), length);
    let squared = length.multiply(length);
    assert_eq!(&squared.divide(length), length);
    assert_eq!(length.power(&int(0)), Dimension::dimensionless());
    assert!(length.power(&int(0)).is_dimensionless());
    assert_eq!(length.divide(length).exponents().count(), 0);
    assert_eq!(length.multiply(time).exponents().count(), 2);
}

#[trace("TC-187", "FR-142-AC-2")]
#[test]
fn u03_incompatible_dimensions_refuse_before_arithmetic() {
    let f = fixture();
    let (metre, second) = (f.quantity(whole(1), f.m), f.quantity(whole(1), f.s));
    for operation in [
        QuantityOperation::Add(&metre, &second),
        QuantityOperation::Subtract(&metre, &second),
    ] {
        let mut meter = unlimited();
        assert_eq!(
            evaluate_quantity(operation, &mut meter),
            typed(IllTypedCause::IncompatibleDimensions)
        );
        assert!(meter.admitted_charges().is_empty());
    }
    assert_eq!(
        convert_quantity(
            &metre,
            &f.unit(f.s),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        typed(IllTypedCause::IncompatibleDimensions)
    );
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn u04_affine_conversions_compose_through_the_root() {
    let f = fixture();
    for (value, unit) in [(whole(0), f.deg_c), (whole(32), f.deg_f)] {
        let Outcome::Completed(conversion) = convert_quantity(
            &f.quantity(value, unit),
            &f.unit(f.kelvin),
            &QuantityTarget::Exact,
            &mut unlimited(),
        )
        .expect("well-typed conversion") else {
            panic!("conversion did not complete");
        };
        assert_eq!(conversion.canonical(), &ratio(5463, 20));
        assert_eq!(conversion.value(), &ConvertedValue::Exact(ratio(5463, 20)));
    }
    // Back through the inverse target mapping.
    assert_eq!(
        converted(
            &f.quantity(whole(0), f.deg_c),
            &f.unit(f.deg_f),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Completed(ConvertedValue::Exact(whole(32)))
    );
}

#[trace("TC-187", "FR-142-AC-2", "FR-142-AC-5")]
#[test]
fn u05_affine_arithmetic_refuses() {
    let f = fixture();
    let two = int(2);
    for affine in [f.deg_c, f.deg_f] {
        let point = f.quantity(whole(1), affine);
        let kelvin = f.quantity(whole(1), f.kelvin);
        let metre = f.quantity(whole(1), f.m);
        for operation in [
            QuantityOperation::Add(&point, &point),
            QuantityOperation::Subtract(&point, &point),
            QuantityOperation::Add(&kelvin, &point),
            QuantityOperation::Multiply(&point, &metre),
            QuantityOperation::Multiply(&metre, &point),
            QuantityOperation::Divide(&point, &point),
            QuantityOperation::Power(&point, &two),
        ] {
            let mut meter = unlimited();
            assert_eq!(
                evaluate_quantity(operation, &mut meter),
                typed(IllTypedCause::AffineUnitArithmetic),
                "{operation:?}"
            );
            assert!(meter.admitted_charges().is_empty());
        }
    }
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn u06_zero_scale_refuses_admission() {
    let f = fixture();
    let mut nodes = f.nodes.clone();
    nodes.unit(unit_json(
        "nothing",
        f.length,
        Some(f.m),
        ("0", "1"),
        ("0", "1"),
    ));
    assert_eq!(
        nodes.admit(),
        Err(InvalidSemanticGraph {
            cause: SemanticGraphCause::ZeroScale
        })
    );
}

fn decimal_type(lower: i64, upper: i64, scale: u64, mode: RoundingMode) -> QuantityTarget {
    QuantityTarget::Decimal(DecimalType::new(int(lower), int(upper), scale, scale, mode).unwrap())
}

#[trace("TC-187", "FR-142-AC-3")]
#[test]
fn u07_lossy_decimal_conversion_reports_or_refuses() {
    let f = fixture();
    let inch = f.quantity(whole(1), f.inch);
    let mut meter = unlimited();
    assert_eq!(
        converted(
            &inch,
            &f.unit(f.m),
            &decimal_type(-1000, 1000, 2, RoundingMode::Exact),
            &mut meter
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
    let Outcome::Completed(conversion) = convert_quantity(
        &inch,
        &f.unit(f.m),
        &decimal_type(-1000, 1000, 2, RoundingMode::NearestEven),
        &mut unlimited(),
    )
    .expect("well-typed conversion") else {
        panic!("conversion did not complete");
    };
    assert_eq!(conversion.canonical(), &ratio(127, 5000));
    let ConvertedValue::Decimal(result) = conversion.value() else {
        panic!("expected a decimal");
    };
    assert_eq!(result.value().representation().coefficient(), &int(3));
    assert_eq!(result.value().representation().scale(), 2);
    let loss = result.loss().unwrap();
    assert_eq!(
        (loss.exact_numerator().clone(), loss.exact_denominator()),
        (int(127), int(5000))
    );
    assert_eq!(
        (
            loss.rounded_coefficient(),
            loss.rounded_scale(),
            loss.mode()
        ),
        (&int(3), 2, RoundingMode::NearestEven)
    );
}

#[trace("TC-187", "FR-142-AC-3")]
#[test]
fn u08_target_domain_endpoints_admit_and_one_over_refuses() {
    let f = fixture();
    let target = decimal_type(-2, 2, 0, RoundingMode::Exact);
    for value in [-2, 2] {
        assert_eq!(value_of_decimal(&f, value, &target), value);
    }
    for value in [-3, 3] {
        let mut meter = unlimited();
        assert_eq!(
            converted(
                &f.quantity(whole(value), f.m),
                &f.unit(f.m),
                &target,
                &mut meter
            ),
            Outcome::Refused(Refusal::DecimalOutOfDomain)
        );
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
        assert!(!meter
            .admitted_charges()
            .contains(&ChargePoint::UnitResultRetain));
    }
}

fn value_of(value: &Decimal) -> i64 {
    value
        .representation()
        .coefficient()
        .to_string()
        .parse()
        .unwrap()
}

fn value_of_decimal(f: &Fixture, value: i64, target: &QuantityTarget) -> i64 {
    match converted(
        &f.quantity(whole(value), f.m),
        &f.unit(f.m),
        target,
        &mut unlimited(),
    ) {
        Outcome::Completed(ConvertedValue::Decimal(result)) => value_of(result.value()),
        other => panic!("expected a decimal, got {other:?}"),
    }
}

#[trace("TC-187", "FR-142-AC-2")]
#[test]
fn u09_u09b_identity_is_the_node_key_not_the_shape() {
    let f = fixture();
    let mut nodes = f.nodes.clone();
    let other_length = nodes.dimension(base_dimension("other-model", "Length"));
    let other_metre = nodes.unit(root_json("metre", other_length));
    let alias = nodes.unit(unit_json(
        "metre_alias",
        f.length,
        Some(f.m),
        ("1", "1"),
        ("0", "1"),
    ));
    let graph = nodes.admit().unwrap();
    assert_ne!(other_length, f.length);
    let declared = |key| QuantityUnit::Declared(Box::new(graph.unit(key).unwrap().clone()));
    let metre = Quantity::new(whole(1), declared(f.m));
    let other = Quantity::new(whole(1), declared(other_metre));
    assert_eq!(
        evaluate_quantity(QuantityOperation::Add(&metre, &other), &mut unlimited()),
        typed(IllTypedCause::IncompatibleDimensions)
    );
    assert_eq!(
        convert_quantity(
            &other,
            &declared(f.m),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        typed(IllTypedCause::IncompatibleDimensions)
    );

    let aliased = Quantity::new(whole(7), declared(alias));
    assert_eq!(
        converted(
            &aliased,
            &declared(f.m),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Completed(ConvertedValue::Exact(whole(7)))
    );
    assert_ne!(declared(alias), declared(f.m));
    assert_eq!(
        evaluate_quantity(QuantityOperation::Add(&aliased, &metre), &mut unlimited()),
        typed(IllTypedCause::DistinctUnits)
    );
}

const U10: ScalarLimits = ScalarLimits {
    integer_bits: 8,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 1,
    value_occurrences: 1,
    work_units: 6,
    result_units: 1,
};

#[trace("TC-187", "FR-142-AC-7")]
#[test]
fn u10_exact_bound_accounting_and_named_denials() {
    let f = fixture();
    let source = f.quantity(whole(100), f.cm);
    let run = |meter: &mut Meter| converted(&source, &f.unit(f.m), &QuantityTarget::Exact, meter);
    let mut meter = Meter::new(U10);
    assert_eq!(
        run(&mut meter),
        Outcome::Completed(ConvertedValue::Exact(whole(1)))
    );
    let charges = [
        ChargePoint::UnitIdentityRead,
        ChargePoint::UnitEdge,
        ChargePoint::UnitRationalArithmetic,
        ChargePoint::UnitRationalArithmetic,
        ChargePoint::UnitTargetDomain,
        ChargePoint::UnitResultRetain,
    ];
    assert_eq!(meter.admitted_charges(), charges);
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    // `max(bits(100) + bits(1), bits(1) + bits(100)) = 8` at the multiply event.
    assert_eq!(consumed, [8, 0, 0, 0, 0, 0, 1, 1, 6, 1]);

    assert_eq!(
        run(&mut Meter::new(ScalarLimits {
            work_units: 5,
            ..U10
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 5,
            consumed: 5,
            next_charge: int(1),
            charge_point: ChargePoint::UnitResultRetain,
        })
    );
    for (limit_kind, limit, next, point) in [
        (LimitKind::IntegerBits, 6, 7, ChargePoint::UnitIdentityRead),
        (LimitKind::UnitEdges, 0, 1, ChargePoint::UnitEdge),
        (
            LimitKind::ValueOccurrences,
            0,
            1,
            ChargePoint::UnitIdentityRead,
        ),
        (LimitKind::ResultUnits, 0, 1, ChargePoint::UnitResultRetain),
    ] {
        let mut limits = U10;
        match limit_kind {
            LimitKind::IntegerBits => limits.integer_bits = limit,
            LimitKind::UnitEdges => limits.unit_edges = limit,
            LimitKind::ValueOccurrences => limits.value_occurrences = limit,
            _ => limits.result_units = limit,
        }
        assert_eq!(
            run(&mut Meter::new(limits)),
            Outcome::Incomplete(Incomplete {
                limit_kind,
                limit,
                consumed: 0,
                next_charge: int(next),
                charge_point: point,
            })
        );
    }

    let mut occurrences: Vec<(ChargePoint, u64)> = Vec::new();
    for (work, point) in (0_u64..).zip(charges) {
        let occurrence = match occurrences.iter_mut().find(|(seen, _)| *seen == point) {
            Some((_, count)) => {
                *count += 1;
                *count
            }
            None => {
                occurrences.push((point, 1));
                1
            }
        };
        let mut denied = Meter::new(U10).with_injected_denial(InjectedDenial { point, occurrence });
        assert_eq!(
            run(&mut denied),
            Outcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: work,
                consumed: work,
                next_charge: int(1),
                charge_point: point,
            })
        );
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
    // An independently metered sibling is unaffected.
    assert_eq!(
        run(&mut Meter::new(U10)),
        Outcome::Completed(ConvertedValue::Exact(whole(1)))
    );
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn u11_invalid_topologies_refuse_admission() {
    let f = fixture();
    let refuses = |nodes: &Nodes, cause| {
        assert_eq!(nodes.admit(), Err(InvalidSemanticGraph { cause }));
    };

    let mut two_roots = f.nodes.clone();
    two_roots.unit(root_json("second_metre", f.length));
    refuses(&two_roots, SemanticGraphCause::DuplicateRoot);

    // `L` without its root: `cm` and `in` target a unit that is absent.
    let mut unknown = f.nodes.clone();
    unknown.units.retain(|(_, key)| *key != f.m);
    refuses(&unknown, SemanticGraphCause::UnknownTarget);

    // A retained target cycle with no root. Keys are retained references, so
    // topology is refused before key recomputation.
    let cycle = |root: bool| {
        let mut nodes = f.nodes.clone();
        nodes.units.retain(|(_, key)| ![f.cm, f.inch].contains(key));
        if !root {
            nodes.units.retain(|(_, key)| *key != f.m);
        }
        let first = unit_json("first", f.length, Some(f.inch), ("2", "1"), ("0", "1"));
        let second = unit_json("second", f.length, Some(f.cm), ("3", "1"), ("0", "1"));
        nodes.units.push((first, f.cm));
        nodes.units.push((second, f.inch));
        nodes
    };
    refuses(&cycle(false), SemanticGraphCause::MissingRoot);
    refuses(&cycle(true), SemanticGraphCause::TargetCycle);

    let mut cross = f.nodes.clone();
    cross.unit(unit_json(
        "minute_length",
        f.length,
        Some(f.s),
        ("60", "1"),
        ("0", "1"),
    ));
    refuses(&cross, SemanticGraphCause::CrossDimensionTarget);
}

#[trace("TC-187", "FR-142-AC-6", "FR-142-AC-4")]
#[test]
fn u12_multiplication_division_and_power_use_compound_units() {
    let f = fixture();
    let q = |value, key| f.quantity(whole(value), key);
    let (two_m, three_s, six_m, two_s, five_m) =
        (q(2, f.m), q(3, f.s), q(6, f.m), q(2, f.s), q(5, f.m));
    let two = int(2);
    let cases = [
        (
            QuantityOperation::Multiply(&two_m, &three_s),
            6,
            compound_unit(&f, &[(f.m, "1"), (f.s, "1")]),
        ),
        (
            QuantityOperation::Divide(&six_m, &two_s),
            3,
            compound_unit(&f, &[(f.m, "1"), (f.s, "-1")]),
        ),
        (
            QuantityOperation::Power(&two_m, &two),
            4,
            compound_unit(&f, &[(f.m, "2")]),
        ),
        (
            QuantityOperation::Divide(&five_m, &five_m),
            1,
            compound_unit(&f, &[]),
        ),
    ];
    for (operation, value, unit) in cases {
        let result = exact(evaluated(operation, &mut unlimited()));
        assert_eq!(result.value(), &whole(value), "{operation:?}");
        assert_eq!(result.unit(), &unit, "{operation:?}");
    }
    let QuantityUnit::Compound(per_second) = exact(evaluated(
        QuantityOperation::Divide(&six_m, &two_s),
        &mut unlimited(),
    ))
    .unit()
    .clone() else {
        panic!("expected a compound unit");
    };
    // The evaluator's own compound-unit identity matches an independently
    // recomputed content digest over the same terms.
    assert_eq!(
        per_second.identity().to_string(),
        digest_hex(&compound_preimage(&[(f.m, "1"), (f.s, "-1")]))
    );
    let QuantityUnit::Compound(dimensionless) = exact(evaluated(
        QuantityOperation::Divide(&five_m, &five_m),
        &mut unlimited(),
    ))
    .unit()
    .clone() else {
        panic!("expected a compound unit");
    };
    assert_eq!(
        dimensionless.identity().to_string(),
        digest_hex(&compound_preimage(&[]))
    );
    assert!(dimensionless.dimension().is_dimensionless());

    // A non-root operand converts to its root first: `150 cm * 2 s = 3 m*s`.
    let mut meter = unlimited();
    let product = exact(evaluated(
        QuantityOperation::Multiply(&f.quantity(whole(150), f.cm), &two_s),
        &mut meter,
    ));
    assert_eq!(product.value(), &whole(3));
    assert_eq!(
        product.unit(),
        &compound_unit(&f, &[(f.m, "1"), (f.s, "1")])
    );
    assert_eq!(meter.consumed(LimitKind::UnitEdges), 1);
    // Multiplication and division are inverse on normalized units.
    let back = exact(evaluated(
        QuantityOperation::Divide(&product, &two_s),
        &mut unlimited(),
    ));
    assert_eq!(back.value(), &ratio(3, 2));
    assert_eq!(back.unit(), &compound_unit(&f, &[(f.m, "1")]));
    assert_eq!(back.unit().dimension(), f.unit(f.m).dimension());
    assert_eq!(
        converted(
            &back,
            &f.unit(f.cm),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Completed(ConvertedValue::Exact(whole(150)))
    );
    // Division by a zero quantity is undefined.
    assert_eq!(
        evaluated(
            QuantityOperation::Divide(&two_m, &q(0, f.s)),
            &mut unlimited()
        ),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(
        evaluated(
            QuantityOperation::Power(&q(0, f.m), &int(-1)),
            &mut unlimited()
        ),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
}

#[trace("TC-187", "FR-142-AC-7")]
#[test]
fn u13_huge_power_is_incomplete_before_computing() {
    let f = fixture();
    let limits = ScalarLimits {
        integer_bits: 64,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 1,
        work_units: 5,
        result_units: 1,
    };
    let exponent: Integer = "18446744073709551616".parse().unwrap();
    // `abs(n) × maxparts(2) = 2^64 × 2`.
    let next: Integer = "36893488147419103232".parse().unwrap();
    assert_eq!(
        evaluated(
            QuantityOperation::Power(&f.quantity(whole(2), f.m), &exponent),
            &mut Meter::new(limits)
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 64,
            consumed: 2,
            next_charge: next,
            charge_point: ChargePoint::UnitRationalArithmetic,
        })
    );
}

// ---- conversion sizing, nominal dimensions, power and comparison ------------

fn wide_decimal(scale: u64, mode: RoundingMode) -> QuantityTarget {
    let bound: Integer = format!("1{}", "0".repeat(200)).parse().unwrap();
    let lower: Integer = format!("-{bound}").parse().unwrap();
    QuantityTarget::Decimal(DecimalType::new(lower, bound, scale, scale, mode).unwrap())
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn huge_target_scale_is_decided_before_materializing_the_coefficient() {
    let f = fixture();
    let scale = u64::from(u32::MAX);
    let limits = ScalarLimits {
        integer_bits: 64,
        ..UNLIMITED
    };
    let third = f.quantity(ratio(1, 3), f.m);
    let quarter = f.quantity(ratio(-1, 4), f.m);
    for (source, mode) in [
        (&third, RoundingMode::TowardZero),
        (&third, RoundingMode::NearestEven),
        (&quarter, RoundingMode::Exact),
    ] {
        let mut meter = Meter::new(limits);
        let Outcome::Incomplete(record) =
            converted(source, &f.unit(f.m), &wide_decimal(scale, mode), &mut meter)
        else {
            panic!("expected incomplete for {mode:?}");
        };
        assert_eq!(record.charge_point, ChargePoint::UnitTargetDomain);
        assert_eq!(record.limit_kind, LimitKind::IntegerBits);
        assert_eq!(record.limit, 64);
        // `v × 10^T` has more than `3 × T` bits.
        assert!(record.next_charge > Integer::from(3 * scale));
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
    }
    let mut meter = Meter::new(limits);
    assert_eq!(
        converted(
            &third,
            &f.unit(f.m),
            &wide_decimal(scale, RoundingMode::Exact),
            &mut meter
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::UnitTargetDomain));
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn rounded_target_sizes_bound_the_materialized_coefficient() {
    let f = fixture();
    let fractions = [
        (1, 3),
        (2, 3),
        (-7, 9),
        (99, 7),
        (-1, 7),
        (12_345, 11),
        (1, 6),
        (-5, 12),
        (999_999, 1_000_001),
        (1_000_001, 999_999),
        (1, 1_048_575),
        (-3, 1_024),
    ];
    for (numerator, denominator) in fractions {
        let source = f.quantity(ratio(numerator, denominator), f.m);
        let input_bits = source.value().max_part_bits();
        let magnitude = BigInt::from(numerator).magnitude().clone();
        let numerator_digits = u64::try_from(magnitude.to_string().len()).unwrap();
        for scale in 0..64_u64 {
            // `sbits(a,T)` and `sdigits(a,T)` of the reduced numerator `a`.
            let bits = if scale == 0 {
                magnitude.bits()
            } else {
                magnitude.bits()
                    + BigUint::from(10_u8)
                        .pow(u32::try_from(scale).unwrap())
                        .bits()
            };
            let digits = numerator_digits + scale;
            for mode in RoundingMode::ALL.into_iter().skip(1) {
                let target = wide_decimal(scale, mode);
                let run = |meter: &mut Meter| converted(&source, &f.unit(f.m), &target, meter);
                let Outcome::Completed(ConvertedValue::Decimal(result)) = run(&mut unlimited())
                else {
                    panic!("{numerator}/{denominator} at {scale} {mode:?} did not complete");
                };
                let coefficient = result.value().representation().coefficient();
                let context = format!("{numerator}/{denominator} at {scale} {mode:?}");
                assert!(coefficient.magnitude_bits() <= bits, "{context}");
                assert!(coefficient.decimal_digits() <= digits, "{context}");
                if bits > input_bits {
                    assert_eq!(
                        run(&mut Meter::new(ScalarLimits {
                            integer_bits: bits - 1,
                            ..UNLIMITED
                        })),
                        Outcome::Incomplete(Incomplete {
                            limit_kind: LimitKind::IntegerBits,
                            limit: bits - 1,
                            consumed: input_bits,
                            next_charge: Integer::from(bits),
                            charge_point: ChargePoint::UnitTargetDomain,
                        }),
                        "{context}"
                    );
                }
                assert_eq!(
                    run(&mut Meter::new(ScalarLimits {
                        decimal_digits: digits - 1,
                        ..UNLIMITED
                    })),
                    Outcome::Incomplete(Incomplete {
                        limit_kind: LimitKind::DecimalDigits,
                        limit: digits - 1,
                        consumed: 0,
                        next_charge: Integer::from(digits),
                        charge_point: ChargePoint::UnitTargetDomain,
                    }),
                    "{context}"
                );
                let mut exact_bound = Meter::new(ScalarLimits {
                    integer_bits: bits.max(input_bits),
                    decimal_digits: digits,
                    ..UNLIMITED
                });
                assert!(
                    matches!(run(&mut exact_bound), Outcome::Completed(_)),
                    "{context}"
                );
            }
        }
    }
}

fn derived_dimension(name: &str, terms: &[(NodeKey, &str)]) -> Value {
    let mut terms = terms.to_vec();
    terms.sort_by_key(|(key, _)| *key);
    let terms: Vec<Value> = terms
        .iter()
        .map(|(key, exponent)| json!({"dimension_node_id": node_id(*key), "exponent": exponent}))
        .collect();
    json!({
        "version": "quire.dimension-node/v1",
        "owner": owner_json("example-model"),
        "qualified_declaration": ["Example", name],
        "terms": terms,
    })
}

/// `L(i,d,e,o,w,r)` from TC-187.
fn limits(
    integer_bits: u64,
    decimal_digits: u64,
    unit_edges: u64,
    value_occurrences: u64,
    work_units: u64,
    result_units: u64,
) -> Meter {
    Meter::new(ScalarLimits {
        integer_bits,
        decimal_digits,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges,
        value_occurrences,
        work_units,
        result_units,
    })
}

fn work_denied(limit: u64, point: ChargePoint) -> Incomplete {
    Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit,
        consumed: limit,
        next_charge: int(1),
        charge_point: point,
    }
}

/// The TC-187 fixture extended with `M`/`kg`, `Torque`/`N_m`, `Energy`/`J`,
/// `Area`/`m2`, `u1`, `u2`, `u3`, `rev` and `cm2`.
struct Extended {
    base: Fixture,
    graph: UnitGraph,
    kg: NodeKey,
    n_m: NodeKey,
    joule: NodeKey,
    m2: NodeKey,
    u1: NodeKey,
    u2: NodeKey,
    u3: NodeKey,
    rev: NodeKey,
    cm2: NodeKey,
}

impl Extended {
    fn unit(&self, key: NodeKey) -> QuantityUnit {
        QuantityUnit::Declared(Box::new(self.graph.unit(key).unwrap().clone()))
    }

    fn quantity(&self, value: Rational, key: NodeKey) -> Quantity {
        Quantity::new(value, self.unit(key))
    }
}

fn extended() -> Extended {
    let base = fixture();
    let mut nodes = base.nodes.clone();
    let mass = nodes.dimension(base_dimension("example-model", "Mass"));
    let kg = nodes.unit(root_json("kilogram", mass));
    let work_terms = [(base.length, "2"), (mass, "1"), (base.time, "-2")];
    let torque = nodes.dimension(derived_dimension("Torque", &work_terms));
    let energy = nodes.dimension(derived_dimension("Energy", &work_terms));
    let area = nodes.dimension(derived_dimension("Area", &[(base.length, "2")]));
    let n_m = nodes.unit(root_json("newton_metre", torque));
    let joule = nodes.unit(root_json("joule", energy));
    let m2 = nodes.unit(root_json("square_metre", area));
    let theta = base.graph.unit(base.kelvin).unwrap().dimension_node();
    let u1 = nodes.unit(unit_json(
        "u1",
        theta,
        Some(base.kelvin),
        ("1", "1"),
        ("10", "1"),
    ));
    let u2 = nodes.unit(unit_json("u2", theta, Some(u1), ("1", "1"), ("-10", "1")));
    let u3 = nodes.unit(unit_json(
        "u3",
        theta,
        Some(base.deg_c),
        ("1", "1"),
        ("0", "1"),
    ));
    let rev = nodes.unit(unit_json(
        "rev",
        base.length,
        Some(base.m),
        ("-1", "1"),
        ("0", "1"),
    ));
    let cm2 = nodes.unit(unit_json("cm2", area, Some(m2), ("1", "10000"), ("0", "1")));
    let graph = nodes.admit().unwrap();
    assert_eq!(graph.dimension(torque), graph.dimension(energy));
    Extended {
        base,
        graph,
        kg,
        n_m,
        joule,
        m2,
        u1,
        u2,
        u3,
        rev,
        cm2,
    }
}

#[trace("TC-187", "FR-142-AC-2", "FR-142-AC-7")]
#[test]
fn u14_static_refusals_precede_every_charge() {
    let f = fixture();
    let q = |value, key| f.quantity(whole(value), key);
    let (metre, second, centimetre) = (q(1, f.m), q(1, f.s), q(1, f.cm));
    let (celsius, zero_celsius) = (q(1, f.deg_c), q(0, f.deg_c));
    // The static order among the causes is informative; this evaluator checks
    // incompatible dimensions, then affine arithmetic, then distinct units.
    let cases = [
        (
            QuantityOperation::Add(&metre, &second),
            IllTypedCause::IncompatibleDimensions,
        ),
        (
            QuantityOperation::Add(&celsius, &second),
            IllTypedCause::IncompatibleDimensions,
        ),
        (
            QuantityOperation::Add(&celsius, &celsius),
            IllTypedCause::AffineUnitArithmetic,
        ),
        // Refused statically as affine, never reaching the zero divisor.
        (
            QuantityOperation::Divide(&celsius, &zero_celsius),
            IllTypedCause::AffineUnitArithmetic,
        ),
        (
            QuantityOperation::Add(&centimetre, &metre),
            IllTypedCause::DistinctUnits,
        ),
    ];
    for (operation, cause) in cases {
        let mut meter = limits(0, 0, 0, 0, 0, 0);
        assert_eq!(
            evaluate_quantity(operation, &mut meter),
            typed(cause),
            "{operation:?}"
        );
        assert!(meter.admitted_charges().is_empty());
    }
    let mut meter = limits(0, 0, 0, 0, 0, 0);
    assert_eq!(
        compare_quantity(ComparisonOperator::Equal, &metre, &centimetre, &mut meter),
        typed(IllTypedCause::DistinctUnits)
    );
    assert_eq!(
        compare_quantity(ComparisonOperator::Less, &metre, &second, &mut meter),
        typed(IllTypedCause::IncompatibleDimensions)
    );
    assert!(meter.admitted_charges().is_empty());
}

#[trace("TC-187", "FR-142-AC-5", "FR-142-AC-7")]
#[test]
fn u15_every_edge_is_charged_before_the_first_rational_event() {
    let f = fixture();
    let inch = f.quantity(whole(1), f.inch);
    let run = |meter: &mut Meter| converted(&inch, &f.unit(f.cm), &QuantityTarget::Exact, meter);
    let mut meter = limits(15, 0, 2, 1, 9, 1);
    assert_eq!(
        run(&mut meter),
        Outcome::Completed(ConvertedValue::Exact(ratio(127, 50)))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitEdge,
            ChargePoint::UnitEdge,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    // Event amounts 14, 15, 15 and 14: the offset add to `127/5000` is
    // `max(bits(127) + bits(1), bits(0) + bits(5000)) + 1 = 15`.
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 15);
    assert_eq!(
        run(&mut limits(14, 0, 2, 1, 9, 1)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 14,
            consumed: 14,
            next_charge: int(15),
            charge_point: ChargePoint::UnitRationalArithmetic,
        })
    );
    assert_eq!(
        run(&mut limits(7, 0, 1, 1, 9, 1)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::UnitEdges,
            limit: 1,
            consumed: 1,
            next_charge: int(2),
            charge_point: ChargePoint::UnitEdge,
        })
    );
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn u16_strict_rounding_refuses_before_the_target_domain() {
    let f = fixture();
    let inch = f.quantity(whole(1), f.inch);
    let mut meter = limits(15, 5, 1, 1, 6, 1);
    assert_eq!(
        converted(
            &inch,
            &f.unit(f.m),
            &decimal_type(-1000, 1000, 2, RoundingMode::Exact),
            &mut meter
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::UnitTargetDomain));
    assert_eq!(
        converted(
            &inch,
            &f.unit(f.m),
            &decimal_type(-1000, 1000, 2, RoundingMode::Exact),
            &mut limits(15, 5, 1, 1, 4, 1)
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    let nearest = decimal_type(-1000, 1000, 2, RoundingMode::NearestEven);
    let mut meter = limits(15, 5, 1, 1, 6, 1);
    let Outcome::Completed(ConvertedValue::Decimal(result)) =
        converted(&inch, &f.unit(f.m), &nearest, &mut meter)
    else {
        panic!("nearest-even did not complete");
    };
    assert_eq!(value_of(result.value()), 3);
    // `unit.target-domain`: `sbits(127,2) = 14`, `sdigits(127,2) = 5`.
    assert_eq!(meter.consumed(LimitKind::DecimalDigits), 5);

    assert_eq!(
        converted(
            &inch,
            &f.unit(f.m),
            &nearest,
            &mut limits(15, 5, 1, 1, 4, 1)
        ),
        Outcome::Incomplete(work_denied(4, ChargePoint::UnitTargetDomain))
    );
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn u17_membership_refuses_after_the_target_domain() {
    let f = fixture();
    let three = f.quantity(whole(3), f.m);
    let target = decimal_type(-2, 2, 0, RoundingMode::Exact);
    let mut meter = limits(2, 1, 0, 1, 2, 0);
    assert_eq!(
        converted(&three, &f.unit(f.m), &target, &mut meter),
        Outcome::Refused(Refusal::DecimalOutOfDomain)
    );
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::UnitIdentityRead, ChargePoint::UnitTargetDomain]
    );
    assert_eq!(
        converted(&three, &f.unit(f.m), &target, &mut limits(2, 1, 0, 1, 1, 0)),
        Outcome::Incomplete(work_denied(1, ChargePoint::UnitTargetDomain))
    );
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn u18_huge_target_scale_is_sized_analytically() {
    let f = fixture();
    let target = QuantityTarget::Decimal(
        DecimalType::new(int(0), int(1), 0, u64::from(u32::MAX), RoundingMode::Exact).unwrap(),
    );
    assert_eq!(
        converted(
            &f.quantity(whole(1), f.m),
            &f.unit(f.m),
            &target,
            &mut limits(u64::MAX, 64, 0, 1, 2, 1)
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::DecimalDigits,
            limit: 64,
            consumed: 0,
            next_charge: Integer::from(4_294_967_296_u64),
            charge_point: ChargePoint::UnitTargetDomain,
        })
    );
}

#[trace("TC-187", "FR-142-AC-1", "FR-142-AC-7")]
#[test]
fn u19_rational_target_charges_one_work_unit_and_no_size() {
    let f = fixture();
    let (two, three) = (f.quantity(whole(2), f.m), f.quantity(whole(3), f.m));
    let operation = QuantityOperation::Add(&two, &three);
    let mut meter = limits(4, 0, 0, 2, 5, 1);
    assert_eq!(
        evaluated(operation, &mut meter),
        Outcome::Completed(f.quantity(whole(5), f.m))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    assert_eq!(
        evaluated(operation, &mut limits(4, 0, 0, 2, 4, 1)),
        Outcome::Incomplete(work_denied(4, ChargePoint::UnitResultRetain))
    );
}

#[trace("TC-187", "FR-142-AC-2", "FR-142-AC-5")]
#[test]
fn u20_affinity_is_the_composed_offset() {
    let x = extended();
    let q = |value, key| x.quantity(whole(value), key);
    let mut meter = limits(0, 0, 0, 0, 0, 0);
    assert_eq!(
        evaluate_quantity(QuantityOperation::Add(&q(1, x.u1), &q(2, x.u1)), &mut meter),
        typed(IllTypedCause::AffineUnitArithmetic)
    );
    assert_eq!(
        evaluate_quantity(
            QuantityOperation::Multiply(&q(1, x.u3), &q(1, x.base.m)),
            &mut meter
        ),
        typed(IllTypedCause::AffineUnitArithmetic)
    );
    assert!(meter.admitted_charges().is_empty());
    let mut meter = limits(4, 0, 0, 2, 5, 1);
    assert_eq!(
        evaluated(QuantityOperation::Add(&q(1, x.u2), &q(2, x.u2)), &mut meter),
        Outcome::Completed(q(3, x.u2))
    );
    assert!(!meter.admitted_charges().contains(&ChargePoint::UnitEdge));
}

#[trace("TC-187", "FR-142-AC-6", "FR-142-AC-7")]
#[test]
fn u21_zero_base_power_is_one_at_zero_and_undefined_below() {
    let f = fixture();
    let zero = f.quantity(whole(0), f.m);
    let mut meter = limits(1, 0, 0, 1, 4, 1);
    let result = exact(evaluated(
        QuantityOperation::Power(&zero, &int(0)),
        &mut meter,
    ));
    assert_eq!(result.value(), &whole(1));
    assert_eq!(result.unit(), &compound_unit(&f, &[]));
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    let negative = int(-1);
    let mut meter = limits(1, 0, 0, 1, 1, 0);
    assert_eq!(
        evaluated(QuantityOperation::Power(&zero, &negative), &mut meter),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::UnitIdentityRead]);
    assert_eq!(
        evaluated(
            QuantityOperation::Power(&zero, &negative),
            &mut limits(1, 0, 0, 1, 0, 0)
        ),
        Outcome::Incomplete(work_denied(0, ChargePoint::UnitIdentityRead))
    );
}

#[trace("TC-187", "FR-142-AC-2", "FR-142-AC-4", "FR-142-AC-10")]
#[test]
fn u22_declared_conversion_requires_one_dimension_node() {
    let x = extended();
    let one_n_m = x.quantity(whole(1), x.n_m);
    let mut meter = limits(0, 0, 0, 0, 0, 0);
    assert_eq!(
        convert_quantity(
            &one_n_m,
            &x.unit(x.joule),
            &QuantityTarget::Exact,
            &mut meter
        ),
        typed(IllTypedCause::IncompatibleDimensions)
    );
    assert!(meter.admitted_charges().is_empty());
    assert_eq!(
        evaluate_quantity(
            QuantityOperation::Add(&one_n_m, &x.quantity(whole(1), x.joule)),
            &mut unlimited()
        ),
        // The nominal rule guards only direct conversion; addition compares
        // base-dimension maps, then requires the identical unit.
        typed(IllTypedCause::DistinctUnits)
    );
    let square = exact(evaluated(
        QuantityOperation::Power(&x.quantity(whole(2), x.base.m), &int(2)),
        &mut unlimited(),
    ));
    let mut meter = limits(3, 0, 0, 1, 3, 1);
    assert_eq!(
        converted(&square, &x.unit(x.m2), &QuantityTarget::Exact, &mut meter),
        Outcome::Completed(ConvertedValue::Exact(whole(4)))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    let to_cm2 =
        |meter: &mut Meter| converted(&square, &x.unit(x.cm2), &QuantityTarget::Exact, meter);
    let mut meter = limits(17, 0, 1, 1, 6, 1);
    assert_eq!(
        to_cm2(&mut meter),
        Outcome::Completed(ConvertedValue::Exact(whole(40_000)))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitEdge,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    assert_eq!(
        // The divide event's `bits(4) + bits(10000) = 17` after the subtract
        // event's 5.
        to_cm2(&mut limits(16, 0, 1, 1, 6, 1)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 16,
            consumed: 5,
            next_charge: int(17),
            charge_point: ChargePoint::UnitRationalArithmetic,
        })
    );
    // Nominal identity guards only direct declared-to-declared conversion: a
    // pivot through the coherent compound unit is admitted.
    let mut terms = [(x.base.m, "2"), (x.kg, "1"), (x.base.s, "-2")];
    terms.sort_by_key(|(key, _)| *key);
    let compound = QuantityUnit::Compound(x.graph.compound_unit(&compound(&terms)).unwrap());
    let Outcome::Completed(ConvertedValue::Exact(pivot)) = converted(
        &x.quantity(whole(5), x.n_m),
        &compound,
        &QuantityTarget::Exact,
        &mut unlimited(),
    ) else {
        panic!("declared to compound did not complete");
    };
    assert_eq!(pivot, whole(5));
    assert_eq!(
        converted(
            &Quantity::new(pivot, compound),
            &x.unit(x.joule),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Completed(ConvertedValue::Exact(whole(5)))
    );
}

#[trace("TC-187", "FR-142-AC-5", "FR-142-AC-7", "FR-142-AC-9")]
#[test]
fn u23_comparison_uses_root_values_of_the_identical_unit() {
    let x = extended();
    let q = |value, key| x.quantity(whole(value), key);
    let (one, two) = (q(1, x.base.deg_c), q(2, x.base.deg_c));
    let mut meter = limits(15, 0, 2, 2, 9, 1);
    assert_eq!(
        compare_quantity(ComparisonOperator::Less, &one, &two, &mut meter),
        Ok(Outcome::Completed(true))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitEdge,
            ChargePoint::UnitEdge,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitResultRetain,
        ]
    );
    assert_eq!(
        compare_quantity(
            ComparisonOperator::Less,
            &one,
            &two,
            &mut limits(15, 0, 2, 2, 8, 1)
        ),
        Ok(Outcome::Incomplete(work_denied(
            8,
            ChargePoint::UnitResultRetain
        )))
    );
    let (one_rev, two_rev) = (q(1, x.rev), q(2, x.rev));
    assert_eq!(
        compare_quantity(
            ComparisonOperator::Less,
            &one_rev,
            &two_rev,
            &mut unlimited()
        ),
        Ok(Outcome::Completed(false))
    );
    for (operator, expected) in ComparisonOperator::ALL
        .into_iter()
        .zip([false, true, false, false, true, true])
    {
        assert_eq!(
            compare_quantity(operator, &one_rev, &two_rev, &mut unlimited()),
            Ok(Outcome::Completed(expected)),
            "{operator:?}"
        );
    }
    assert_eq!(
        compare_quantity(ComparisonOperator::Equal, &one, &one, &mut unlimited()),
        Ok(Outcome::Completed(true))
    );
    let (zero_c, thirty_two_f) = (q(0, x.base.deg_c), q(32, x.base.deg_f));
    let mut meter = limits(0, 0, 0, 0, 0, 0);
    for operator in [ComparisonOperator::Less, ComparisonOperator::Equal] {
        assert_eq!(
            compare_quantity(operator, &zero_c, &thirty_two_f, &mut meter),
            typed(IllTypedCause::DistinctUnits)
        );
    }
    assert!(meter.admitted_charges().is_empty());
}

#[trace("TC-187", "FR-142-AC-1", "FR-142-AC-7")]
#[test]
fn u24_identical_unit_addition_traverses_no_edge() {
    let f = fixture();
    let mut meter = limits(4, 0, 0, 2, 5, 1);
    assert_eq!(
        evaluated(
            QuantityOperation::Add(&f.quantity(whole(1), f.cm), &f.quantity(whole(2), f.cm)),
            &mut meter
        ),
        Outcome::Completed(f.quantity(whole(3), f.cm))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
}

#[trace("TC-187", "FR-142-AC-2", "FR-142-AC-4", "FR-142-AC-10")]
#[test]
fn u25_compound_pivot_between_nominal_dimensions() {
    let x = extended();
    let mut terms = [(x.base.m, "2"), (x.kg, "1"), (x.base.s, "-2")];
    terms.sort_by_key(|(key, _)| *key);
    let compound = QuantityUnit::Compound(x.graph.compound_unit(&compound(&terms)).unwrap());
    let no_edge = [
        ChargePoint::UnitIdentityRead,
        ChargePoint::UnitTargetDomain,
        ChargePoint::UnitResultRetain,
    ];
    let mut meter = limits(1, 0, 0, 1, 3, 1);
    let Outcome::Completed(ConvertedValue::Exact(pivot)) = converted(
        &x.quantity(whole(1), x.n_m),
        &compound,
        &QuantityTarget::Exact,
        &mut meter,
    ) else {
        panic!("N_m to the compound unit did not complete");
    };
    assert_eq!(meter.admitted_charges(), no_edge);
    let mut meter = limits(1, 0, 0, 1, 3, 1);
    assert_eq!(
        converted(
            &Quantity::new(pivot, compound),
            &x.unit(x.joule),
            &QuantityTarget::Exact,
            &mut meter
        ),
        Outcome::Completed(ConvertedValue::Exact(whole(1)))
    );
    assert_eq!(meter.admitted_charges(), no_edge);
}

#[trace("TC-187", "FR-142-AC-5", "FR-142-AC-7")]
#[test]
fn u26_conversion_has_no_common_ancestor_shortcut() {
    let x = extended();
    let one = x.quantity(whole(1), x.u2);
    let run = |meter: &mut Meter| converted(&one, &x.unit(x.u1), &QuantityTarget::Exact, meter);
    let mut meter = limits(6, 0, 3, 1, 12, 1);
    assert_eq!(
        run(&mut meter),
        Outcome::Completed(ConvertedValue::Exact(whole(-9)))
    );
    let mut expected = vec![ChargePoint::UnitIdentityRead];
    expected.extend([ChargePoint::UnitEdge; 3]);
    expected.extend([ChargePoint::UnitRationalArithmetic; 6]);
    expected.extend([ChargePoint::UnitTargetDomain, ChargePoint::UnitResultRetain]);
    assert_eq!(meter.admitted_charges(), expected);
    assert_eq!(
        run(&mut limits(6, 0, 2, 1, 12, 1)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::UnitEdges,
            limit: 2,
            consumed: 2,
            next_charge: int(3),
            charge_point: ChargePoint::UnitEdge,
        })
    );
}

#[trace("TC-187", "FR-142-AC-6", "FR-142-AC-7")]
#[test]
fn u27_zero_divisor_is_undefined_after_the_reads() {
    let f = fixture();
    let (one, zero) = (f.quantity(whole(1), f.cm), f.quantity(whole(0), f.cm));
    let operation = QuantityOperation::Divide(&one, &zero);
    let mut meter = limits(1, 0, 0, 2, 2, 0);
    assert_eq!(
        evaluated(operation, &mut meter),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::UnitIdentityRead, ChargePoint::UnitIdentityRead]
    );
    assert_eq!(
        evaluated(operation, &mut limits(1, 0, 0, 2, 1, 0)),
        Outcome::Incomplete(work_denied(1, ChargePoint::UnitIdentityRead))
    );
}

#[trace("TC-187", "FR-142-AC-5", "FR-142-AC-7")]
#[test]
fn u28_multiplication_charges_left_then_right_then_the_product() {
    let f = fixture();
    let (cm, inch) = (f.quantity(whole(1), f.cm), f.quantity(whole(1), f.inch));
    let operation = QuantityOperation::Multiply(&cm, &inch);
    let mut meter = limits(20, 0, 2, 2, 11, 1);
    let product = exact(evaluated(operation, &mut meter));
    assert_eq!(product.value(), &ratio(127, 500_000));
    assert_eq!(product.unit(), &compound_unit(&f, &[(f.m, "2")]));
    let mut expected = vec![ChargePoint::UnitIdentityRead; 2];
    expected.extend([ChargePoint::UnitEdge; 2]);
    expected.extend([ChargePoint::UnitRationalArithmetic; 5]);
    expected.extend([ChargePoint::UnitTargetDomain, ChargePoint::UnitResultRetain]);
    assert_eq!(meter.admitted_charges(), expected);
    let bits_denied = |limit: u64, consumed: u64| {
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit,
            consumed,
            next_charge: Integer::from(limit + 1),
            charge_point: ChargePoint::UnitRationalArithmetic,
        })
    };
    assert_eq!(
        evaluated(operation, &mut limits(13, 0, 2, 2, 11, 1)),
        bits_denied(13, 9)
    );
    assert_eq!(
        evaluated(operation, &mut limits(19, 0, 2, 2, 11, 1)),
        bits_denied(19, 15)
    );
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn u29_integer_target_charges_integer_bits_only() {
    let f = fixture();
    let inch = f.quantity(whole(1), f.inch);
    let run = |source: &Quantity, rounding, meter: &mut Meter| {
        converted(
            source,
            &f.unit(f.m),
            &integer_target(-2, 2, rounding),
            meter,
        )
    };
    let mut meter = limits(15, 0, 1, 1, 4, 1);
    assert_eq!(
        run(&inch, RoundingMode::Exact, &mut meter),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitEdge,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
        ]
    );
    assert_eq!(
        run(
            &inch,
            RoundingMode::NearestEven,
            &mut limits(15, 0, 1, 1, 4, 1)
        ),
        Outcome::Incomplete(work_denied(4, ChargePoint::UnitTargetDomain))
    );
    let Outcome::Completed(ConvertedValue::Integer { value, .. }) = run(
        &inch,
        RoundingMode::NearestEven,
        &mut limits(15, 0, 1, 1, 6, 1),
    ) else {
        panic!("nearest-even did not complete");
    };
    assert_eq!(value.value(), &int(0));
    let three = f.quantity(whole(3), f.m);
    let mut meter = limits(2, 0, 0, 1, 2, 0);
    assert_eq!(
        run(&three, RoundingMode::Exact, &mut meter),
        Outcome::Refused(Refusal::IntegerOutOfDomain)
    );
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::UnitIdentityRead, ChargePoint::UnitTargetDomain]
    );
    assert_eq!(
        run(&three, RoundingMode::Exact, &mut limits(2, 0, 0, 1, 1, 0)),
        Outcome::Incomplete(work_denied(1, ChargePoint::UnitTargetDomain))
    );
}

fn integer_target(lower: i64, upper: i64, rounding: RoundingMode) -> QuantityTarget {
    QuantityTarget::Integer {
        domain: IntegerInterval::new(int(lower), int(upper)).unwrap(),
        rounding,
    }
}

#[trace("TC-187", "FR-142-AC-3", "FR-142-AC-7")]
#[test]
fn integer_target_places_at_scale_zero_then_admits_the_integer_domain() {
    let f = fixture();
    let centimetres = f.quantity(whole(250), f.cm);
    let run = |target: &QuantityTarget, meter: &mut Meter| {
        converted(&centimetres, &f.unit(f.m), target, meter)
    };
    let mut meter = unlimited();
    assert_eq!(
        run(&integer_target(-10, 10, RoundingMode::Exact), &mut meter),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::UnitTargetDomain));

    let mut meter = unlimited();
    let Outcome::Completed(ConvertedValue::Integer { value, loss }) = run(
        &integer_target(-10, 10, RoundingMode::NearestEven),
        &mut meter,
    ) else {
        panic!("integer conversion did not complete");
    };
    assert_eq!(value.value(), &int(2));
    let loss = loss.unwrap();
    assert_eq!(
        (loss.exact_numerator().clone(), loss.exact_denominator()),
        (int(5), int(2))
    );
    assert_eq!(
        (loss.rounded_coefficient(), loss.rounded_scale()),
        (&int(2), 0)
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitEdge,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitRationalArithmetic,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::DecimalDigits), 0);

    let exact_three = f.quantity(whole(300), f.cm);
    let mut meter = unlimited();
    assert_eq!(
        converted(
            &exact_three,
            &f.unit(f.m),
            &integer_target(-2, 2, RoundingMode::Exact),
            &mut meter
        ),
        Outcome::Refused(Refusal::IntegerOutOfDomain)
    );
    assert_eq!(
        meter.admitted_charges().last(),
        Some(&ChargePoint::UnitTargetDomain)
    );
    assert_eq!(
        converted(
            &exact_three,
            &f.unit(f.m),
            &integer_target(-3, 3, RoundingMode::Exact),
            &mut unlimited()
        ),
        Outcome::Completed(ConvertedValue::Integer {
            value: IntegerInterval::new(int(-3), int(3))
                .unwrap()
                .admit(int(3))
                .unwrap(),
            loss: None,
        })
    );
}

// ---- generated graphs --------------------------------------------------------

/// A deterministic SplitMix64 stream.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn signed(&mut self, magnitude: u64) -> i64 {
        i64::try_from(self.below(2 * magnitude + 1)).unwrap() - i64::try_from(magnitude).unwrap()
    }
}

/// An independent oracle fraction `(numerator, denominator)` in `i128`.
type Fraction = (i128, i128);

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

fn reduce(numerator: i128, denominator: i128) -> Fraction {
    let sign = if denominator < 0 { -1 } else { 1 };
    let divisor = gcd(numerator, denominator).max(1);
    (sign * numerator / divisor, sign * denominator / divisor)
}

fn as_rational((numerator, denominator): Fraction) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

#[trace("TC-187", "FR-142-AC-1", "FR-142-AC-5", "FR-142-AC-7")]
#[test]
fn generated_unit_graphs_match_the_affine_oracle_and_every_denial() {
    let mut stream = Stream(187);
    for graph_index in 0..24 {
        let mut nodes = Nodes::default();
        let length = nodes.dimension(base_dimension("example-model", "Length"));
        let root = nodes.unit(root_json("root", length));
        // Each generated unit: key, target index, scale, offset, composed map.
        let mut maps: Vec<(NodeKey, Fraction, Fraction)> = vec![(root, (1, 1), (0, 1))];
        let count = 1 + stream.below(5);
        for index in 0..count {
            let target = usize::try_from(stream.below(u64::try_from(maps.len()).unwrap())).unwrap();
            let scale = reduce(
                i128::from(stream.signed(6)).max(1) * if stream.below(2) == 0 { 1 } else { -1 },
                i128::from(1 + stream.below(6)),
            );
            let offset = if graph_index % 3 == 0 {
                reduce(
                    i128::from(stream.signed(9)),
                    i128::from(1 + stream.below(4)),
                )
            } else {
                (0, 1)
            };
            let spell = |(n, d): Fraction| (n.to_string(), d.to_string());
            let (scale_text, offset_text) = (spell(scale), spell(offset));
            let (target_key, target_scale, target_offset) = maps[target];
            let key = nodes.unit(unit_json(
                &format!("u{index}"),
                length,
                Some(target_key),
                (&scale_text.0, &scale_text.1),
                (&offset_text.0, &offset_text.1),
            ));
            // canonical(target(v)) = ts * (s*v + o) + to
            let composed_scale = reduce(target_scale.0 * scale.0, target_scale.1 * scale.1);
            let shifted = reduce(target_scale.0 * offset.0, target_scale.1 * offset.1);
            let composed_offset = reduce(
                shifted.0 * target_offset.1 + target_offset.0 * shifted.1,
                shifted.1 * target_offset.1,
            );
            maps.push((key, composed_scale, composed_offset));
        }
        let graph = nodes.admit().unwrap();
        for (key, scale, offset) in &maps {
            let unit = graph.unit(*key).unwrap();
            assert_eq!(unit.canonical().scale(), &as_rational(*scale));
            assert_eq!(unit.canonical().offset(), &as_rational(*offset));
            assert_eq!(unit.root(), root);
            assert_eq!(unit.is_affine(), offset.0 != 0);
        }
        let declared = |key| QuantityUnit::Declared(Box::new(graph.unit(key).unwrap().clone()));
        for (source, source_scale, source_offset) in &maps {
            for (target, target_scale, target_offset) in &maps {
                let value = reduce(
                    i128::from(stream.signed(20)),
                    i128::from(1 + stream.below(3)),
                );
                // (s*v + o - to) / ts
                let canonical = reduce(
                    source_scale.0 * value.0 * source_offset.1
                        + source_offset.0 * source_scale.1 * value.1,
                    source_scale.1 * value.1 * source_offset.1,
                );
                let lifted = reduce(
                    canonical.0 * target_offset.1 - target_offset.0 * canonical.1,
                    canonical.1 * target_offset.1,
                );
                let expected = reduce(lifted.0 * target_scale.1, lifted.1 * target_scale.0);
                let quantity = Quantity::new(as_rational(value), declared(*source));
                let mut meter = unlimited();
                let Outcome::Completed(conversion) = convert_quantity(
                    &quantity,
                    &declared(*target),
                    &QuantityTarget::Exact,
                    &mut meter,
                )
                .expect("well-typed conversion") else {
                    panic!("generated conversion did not complete");
                };
                assert_eq!(conversion.canonical(), &as_rational(canonical));
                assert_eq!(
                    conversion.value(),
                    &ConvertedValue::Exact(as_rational(expected))
                );
                let edges = graph.unit(*source).unwrap().path().len()
                    + graph.unit(*target).unwrap().path().len();
                assert_eq!(
                    meter.consumed(LimitKind::UnitEdges),
                    u64::try_from(edges).unwrap()
                );
                assert_eq!(
                    meter.consumed(LimitKind::WorkUnits),
                    u64::try_from(3 + 3 * edges).unwrap()
                );

                // Every admitted charge denies atomically with the exact record.
                let admitted = meter.admitted_charges().to_vec();
                let mut seen: Vec<ChargePoint> = Vec::new();
                for (work, point) in (0_u64..).zip(&admitted) {
                    seen.push(*point);
                    let occurrence =
                        u64::try_from(seen.iter().filter(|p| *p == point).count()).unwrap();
                    let mut denied = unlimited().with_injected_denial(InjectedDenial {
                        point: *point,
                        occurrence,
                    });
                    assert_eq!(
                        converted(
                            &quantity,
                            &declared(*target),
                            &QuantityTarget::Exact,
                            &mut denied
                        ),
                        Outcome::Incomplete(Incomplete {
                            limit_kind: LimitKind::WorkUnits,
                            limit: work,
                            consumed: work,
                            next_charge: int(1),
                            charge_point: *point,
                        })
                    );
                    assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
                }

                // Arithmetic in the source unit.
                let other = Quantity::new(whole(3), declared(*source));
                let sum =
                    evaluate_quantity(QuantityOperation::Add(&quantity, &other), &mut unlimited());
                if source_offset.0 != 0 {
                    assert_eq!(sum, typed(IllTypedCause::AffineUnitArithmetic));
                    continue;
                }
                let sum = exact(sum.expect("well-typed operation"));
                assert_eq!(
                    sum.value(),
                    &as_rational(reduce(value.0 + 3 * value.1, value.1))
                );
                let product = exact(evaluated(
                    QuantityOperation::Multiply(&quantity, &other),
                    &mut unlimited(),
                ));
                // Both operands convert to the root: canonical(v)*canonical(3).
                let three = reduce(3 * source_scale.0, source_scale.1);
                assert_eq!(
                    product.value(),
                    &as_rational(reduce(canonical.0 * three.0, canonical.1 * three.1))
                );
                assert_eq!(
                    product.unit(),
                    &QuantityUnit::Compound(
                        graph.compound_unit(&compound(&[(root, "2")])).unwrap()
                    )
                );
            }
        }

        // Independent topology mutations of the generated graph.
        let mut two_roots = nodes.clone();
        two_roots.unit(root_json("extra_root", length));
        assert_eq!(
            two_roots.admit().unwrap_err().cause,
            SemanticGraphCause::DuplicateRoot
        );
        let mut rootless = nodes.clone();
        rootless.units.retain(|(_, key)| *key != root);
        assert_eq!(
            rootless.admit().unwrap_err().cause,
            SemanticGraphCause::UnknownTarget
        );
        let mut zero = nodes.clone();
        zero.units[1].0["scale"] = json!({"numerator": "0", "denominator": "1"});
        assert_eq!(
            zero.admit().unwrap_err().cause,
            SemanticGraphCause::ZeroScale
        );
        let mut stale = nodes.clone();
        stale.units[1].0["offset"] = json!({"numerator": "1", "denominator": "7"});
        assert_eq!(
            stale.admit().unwrap_err().cause,
            SemanticGraphCause::StaleKey
        );
    }
}
