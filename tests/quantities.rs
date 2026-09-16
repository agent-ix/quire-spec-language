// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-187 dimensions, units, compound units and quantities over the real
//! `value` boundary.
//!
//! The dimension/unit node vectors, their invalid mutations, the compound-unit
//! vectors and U01–U13 are read from or transcribed from the vendored
//! quire-specification files pinned by `tests/complete_value_lock.rs`. Digests
//! are reproduced by an independent JCS canonicalizer, never authored here.

use std::path::Path;

use ix_trace_rs::trace;
use quire_spec_language::value::{
    convert_quantity, evaluate_quantity, ChargePoint, CompoundUnitCause, CompoundUnitPreimage,
    ConvertedValue, Decimal, DecimalType, Dimension, DimensionPreimage, Incomplete, InjectedDenial,
    Integer, InvalidCompoundUnit, InvalidSemanticGraph, LimitKind, Meter, NodeKey, NodeOwner,
    Outcome, OwnerSelection, OwnerSubject, Quantity, QuantityOperation, QuantityTarget,
    QuantityUnit, Rational, Refusal, RoundingMode, ScalarLimits, SemanticGraphCause, Undefined,
    UnitGraph, UnitPreimage,
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

// ---- vendored files --------------------------------------------------------

const SPEC: &str = "resources/complete-value/quire-specification/proposals";

fn read_json(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(SPEC)
        .join(relative);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn compile(schema: &Value) -> jsonschema::JSONSchema {
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(schema)
        .unwrap()
}

fn node_vectors() -> Value {
    read_json("checked-package-v2/node-identity-vectors.json")
}

fn node_schema() -> jsonschema::JSONSchema {
    compile(&read_json(
        "checked-package-v2/node-identity-preimage.schema.json",
    ))
}

fn compound_vectors() -> Value {
    read_json("quire-v1/definitions/value-compound-unit-vectors.json")
}

fn compound_schema() -> jsonschema::JSONSchema {
    compile(&read_json(
        "quire-v1/definitions/value-compound-unit.schema.json",
    ))
}

fn entry<'a>(vectors: &'a Value, name: &str) -> &'a Value {
    vectors["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap()
}

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

fn apply_patch(mut target: Value, patch: &Value) -> Value {
    for operation in patch.as_array().unwrap() {
        let path = operation["path"].as_str().unwrap();
        match operation["op"].as_str().unwrap() {
            "replace" => *target.pointer_mut(path).unwrap() = operation["value"].clone(),
            "move" => {
                let from = operation["from"].as_str().unwrap();
                let (from_parent, from_index) = from.rsplit_once('/').unwrap();
                let (to_parent, to_index) = path.rsplit_once('/').unwrap();
                assert_eq!(from_parent, to_parent, "array move within one parent");
                let items = target
                    .pointer_mut(from_parent)
                    .unwrap()
                    .as_array_mut()
                    .unwrap();
                let moved = items.remove(from_index.parse().unwrap());
                items.insert(to_index.parse().unwrap(), moved);
            }
            other => panic!("unsupported patch op {other}"),
        }
    }
    target
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
    match convert_quantity(source, unit, target, meter) {
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

// ---- node identity vectors ---------------------------------------------------

const DIMENSION_VECTORS: [&str; 4] = [
    "dimension-length",
    "dimension-time",
    "dimension-temperature",
    "dimension-velocity",
];
const UNIT_VECTORS: [&str; 7] = [
    "unit-metre",
    "unit-second",
    "unit-kelvin",
    "unit-degree-celsius",
    "unit-centimetre",
    "unit-millimetre",
    "unit-huge-exact-scale",
];

fn vector_nodes(vectors: &Value) -> Nodes {
    let pairs = |names: &[&str]| {
        names
            .iter()
            .map(|name| {
                let entry = entry(vectors, name);
                (
                    entry["preimage"].clone(),
                    NodeKey::from_hex(entry["sha256"].as_str().unwrap()).unwrap(),
                )
            })
            .collect()
    };
    Nodes {
        dimensions: pairs(&DIMENSION_VECTORS),
        units: pairs(&UNIT_VECTORS),
    }
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn dimension_and_unit_node_vectors_reproduce_and_admit() {
    let vectors = node_vectors();
    let schema = node_schema();
    for name in DIMENSION_VECTORS.iter().chain(&UNIT_VECTORS) {
        let entry = entry(&vectors, name);
        let (preimage, digest) = (&entry["preimage"], entry["sha256"].as_str().unwrap());
        assert!(schema.is_valid(preimage), "{name}");
        assert_eq!(digest_hex(preimage), digest, "{name}");
        let key = if name.starts_with("dimension-") {
            DimensionPreimage::from_json(preimage.clone())
                .unwrap()
                .node_key()
        } else {
            UnitPreimage::from_json(preimage.clone())
                .unwrap()
                .node_key()
        };
        assert_eq!(key.unwrap().to_string(), digest, "{name}");
    }

    let nodes = vector_nodes(&vectors);
    let graph = nodes.admit().unwrap();
    let key = |name| NodeKey::from_hex(entry(&vectors, name)["sha256"].as_str().unwrap()).unwrap();
    let (length, time) = (key("dimension-length"), key("dimension-time"));
    let velocity = graph.dimension(key("dimension-velocity")).unwrap();
    let terms: Vec<_> = velocity
        .exponents()
        .map(|(base, exponent)| (base, exponent.clone()))
        .collect();
    assert_eq!(terms, [(time, int(-1)), (length, int(1))]);

    let metre = key("unit-metre");
    let mm = graph.unit(key("unit-millimetre")).unwrap();
    assert_eq!(mm.root(), metre);
    assert_eq!(mm.path().len(), 2);
    assert_eq!(mm.canonical().scale(), &ratio(1, 1000));
    assert!(!mm.is_affine());
    let huge = graph.unit(key("unit-huge-exact-scale")).unwrap();
    let huge_scale: Integer = "340282366920938463463374607431768211457".parse().unwrap();
    assert_eq!(
        huge.canonical().scale(),
        &Rational::from_integer(huge_scale)
    );
    let celsius = graph.unit(key("unit-degree-celsius")).unwrap();
    assert!(celsius.is_affine());
    assert_eq!(celsius.root(), key("unit-kelvin"));
    assert_eq!(celsius.canonical().offset(), &ratio(5463, 20));
}

fn mutation_cause(name: &str) -> SemanticGraphCause {
    match name {
        "dimension-zero-exponent" => SemanticGraphCause::ZeroExponent,
        "dimension-duplicate-term" => SemanticGraphCause::DuplicateTerm,
        "dimension-unsorted-terms" => SemanticGraphCause::UnsortedTerms,
        "root-unit-nonidentity-scale" | "nonroot-unit-missing-target" => {
            SemanticGraphCause::NonIdentityRoot
        }
        "unit-unreduced-rational" => SemanticGraphCause::UnreducedRational,
        other => panic!("unclassified semantic mutation {other}"),
    }
}

#[trace("TC-187", "FR-142-AC-5")]
#[test]
fn dimension_and_unit_invalid_mutations_refuse_by_their_named_check() {
    let vectors = node_vectors();
    let schema = node_schema();
    let mut checked = 0;
    for mutation in vectors["invalid_mutations"].as_array().unwrap() {
        let base = mutation["base"].as_str().unwrap();
        if !(base.starts_with("dimension-") || base.starts_with("unit-")) {
            continue;
        }
        let name = mutation["name"].as_str().unwrap();
        assert_eq!(mutation["expected_code"], InvalidSemanticGraph::CODE);
        let retained = NodeKey::from_hex(mutation["retained_sha256"].as_str().unwrap()).unwrap();
        let preimage = apply_patch(
            entry(&vectors, base)["preimage"].clone(),
            &mutation["patch"],
        );
        let is_dimension = base.starts_with("dimension-");
        let parsed = |preimage: Value| {
            if is_dimension {
                DimensionPreimage::from_json(preimage).map(|_| ())
            } else {
                UnitPreimage::from_json(preimage).map(|_| ())
            }
        };
        match mutation["refused_by"].as_str().unwrap() {
            "schema" => {
                assert!(!schema.is_valid(&preimage), "{name}");
                assert_eq!(
                    parsed(preimage),
                    Err(InvalidSemanticGraph {
                        cause: SemanticGraphCause::NonCanonicalPreimage
                    }),
                    "{name}"
                );
            }
            "semantic" => {
                assert!(schema.is_valid(&preimage), "{name}");
                let mut nodes = vector_nodes(&vectors);
                let slot = if is_dimension {
                    &mut nodes.dimensions
                } else {
                    &mut nodes.units
                };
                let replaced = slot.iter_mut().find(|(_, key)| *key == retained).unwrap();
                *replaced = (preimage, retained);
                assert_eq!(
                    nodes.admit().unwrap_err().cause,
                    mutation_cause(name),
                    "{name}"
                );
            }
            other => panic!("unknown refusal stage {other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 9);
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

// ---- compound-unit vectors ---------------------------------------------------

fn compound_mutation_cause(name: &str) -> CompoundUnitCause {
    match name {
        "zero-exponent" => CompoundUnitCause::ZeroExponent,
        "duplicate-term" => CompoundUnitCause::DuplicateTerm,
        "unsorted-terms" => CompoundUnitCause::UnsortedTerms,
        other => panic!("unclassified compound mutation {other}"),
    }
}

#[trace("TC-187", "FR-142-AC-6")]
#[test]
fn compound_unit_vectors_reproduce_and_mutations_refuse() {
    let vectors = compound_vectors();
    assert_eq!(vectors["version"], "quire.value.compound-unit-vectors/v1");
    let schema = compound_schema();
    let nodes = vector_nodes(&node_vectors());
    let graph = nodes.admit().unwrap();
    let names: Vec<_> = vectors["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["dimensionless", "metre-per-second"]);
    for name in names {
        let entry = entry(&vectors, name);
        let (preimage, digest) = (&entry["preimage"], entry["sha256"].as_str().unwrap());
        assert!(schema.is_valid(preimage), "{name}");
        assert_eq!(digest_hex(preimage), digest, "{name}");
        let parsed = CompoundUnitPreimage::from_json(preimage.clone()).unwrap();
        assert_eq!(parsed.identity().to_string(), digest, "{name}");
        let unit = graph.compound_unit(&parsed).unwrap();
        assert_eq!(unit.identity().to_string(), digest, "{name}");
        assert_eq!(unit.dimension().is_dimensionless(), name == "dimensionless");
    }

    let mut checked = 0;
    for mutation in vectors["invalid_mutations"].as_array().unwrap() {
        let name = mutation["name"].as_str().unwrap();
        let base = mutation["base"].as_str().unwrap();
        let retained = mutation["retained_sha256"].as_str().unwrap();
        let preimage = apply_patch(
            entry(&vectors, base)["preimage"].clone(),
            &mutation["patch"],
        );
        match (
            mutation["kind"].as_str().unwrap(),
            mutation["refused_by"].as_str(),
        ) {
            ("stale_key", None) => {
                assert!(schema.is_valid(&preimage), "{name}");
                let parsed = CompoundUnitPreimage::from_json(preimage.clone()).unwrap();
                let identity = graph.compound_unit(&parsed).unwrap().identity();
                assert_ne!(identity.to_string(), retained, "{name}");
                assert_eq!(identity.to_string(), digest_hex(&preimage), "{name}");
            }
            ("semantic", Some("schema")) => {
                assert!(!schema.is_valid(&preimage), "{name}");
                assert_eq!(
                    CompoundUnitPreimage::from_json(preimage),
                    Err(InvalidCompoundUnit {
                        cause: CompoundUnitCause::NonCanonicalPreimage
                    }),
                    "{name}"
                );
            }
            ("semantic", Some("semantic")) => {
                assert!(schema.is_valid(&preimage), "{name}");
                let parsed = CompoundUnitPreimage::from_json(preimage).unwrap();
                assert_eq!(
                    graph.compound_unit(&parsed),
                    Err(InvalidCompoundUnit {
                        cause: compound_mutation_cause(name)
                    }),
                    "{name}"
                );
            }
            other => panic!("unknown mutation stage {other:?}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 5);

    // A term must name an admitted canonical root unit.
    let centimetre = NodeKey::from_hex(
        entry(&node_vectors(), "unit-centimetre")["sha256"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        graph.compound_unit(&compound(&[(centimetre, "1")])),
        Err(InvalidCompoundUnit {
            cause: CompoundUnitCause::NotRootUnit
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
    else {
        panic!("conversion did not complete");
    };
    assert_eq!(conversion.source(), &source);
    assert_eq!(conversion.canonical(), &ratio(5, 2));
    assert_eq!(conversion.value(), &ConvertedValue::Exact(ratio(5, 2)));
    assert_eq!(conversion.unit(), &f.unit(f.m));
    let sum = exact(evaluate_quantity(
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
        Outcome::Refused(Refusal::DistinctUnits)
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
            Outcome::Refused(Refusal::IncompatibleDimensions)
        );
        assert_eq!(
            meter.admitted_charges(),
            [ChargePoint::UnitIdentityRead, ChargePoint::UnitIdentityRead]
        );
    }
    assert_eq!(
        convert_quantity(
            &metre,
            &f.unit(f.s),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Refused(Refusal::IncompatibleDimensions)
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
        ) else {
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
                Outcome::Refused(Refusal::AffineUnitArithmetic),
                "{operation:?}"
            );
            assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
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
    ) else {
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
        (loss.exact_numerator(), loss.exact_denominator()),
        (&int(127), &int(5000))
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
        Outcome::Refused(Refusal::IncompatibleDimensions)
    );
    assert_eq!(
        converted(
            &other,
            &declared(f.m),
            &QuantityTarget::Exact,
            &mut unlimited()
        ),
        Outcome::Refused(Refusal::IncompatibleDimensions)
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
        Outcome::Refused(Refusal::DistinctUnits)
    );
}

const U10: ScalarLimits = ScalarLimits {
    integer_bits: 7,
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
    assert_eq!(consumed, [7, 0, 0, 0, 0, 0, 1, 1, 6, 1]);

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
    let vectors = compound_vectors();
    for (operation, value, unit) in cases {
        let result = exact(evaluate_quantity(operation, &mut unlimited()));
        assert_eq!(result.value(), &whole(value), "{operation:?}");
        assert_eq!(result.unit(), &unit, "{operation:?}");
    }
    let identity = |name| entry(&vectors, name)["sha256"].as_str().unwrap().to_owned();
    let QuantityUnit::Compound(per_second) = exact(evaluate_quantity(
        QuantityOperation::Divide(&six_m, &two_s),
        &mut unlimited(),
    ))
    .unit()
    .clone() else {
        panic!("expected a compound unit");
    };
    // The vectors' metre and second keys are the fixture's own.
    assert_eq!(
        per_second.identity().to_string(),
        identity("metre-per-second")
    );
    let QuantityUnit::Compound(dimensionless) = exact(evaluate_quantity(
        QuantityOperation::Divide(&five_m, &five_m),
        &mut unlimited(),
    ))
    .unit()
    .clone() else {
        panic!("expected a compound unit");
    };
    assert_eq!(
        dimensionless.identity().to_string(),
        identity("dimensionless")
    );
    assert!(dimensionless.dimension().is_dimensionless());

    // A non-root operand converts to its root first: `150 cm * 2 s = 3 m*s`.
    let mut meter = unlimited();
    let product = exact(evaluate_quantity(
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
    let back = exact(evaluate_quantity(
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
        evaluate_quantity(
            QuantityOperation::Divide(&two_m, &q(0, f.s)),
            &mut unlimited()
        ),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(
        evaluate_quantity(
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
    let next: Integer = "18446744073709551617".parse().unwrap();
    assert_eq!(
        evaluate_quantity(
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
                ) else {
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
                    assert_eq!(sum, Outcome::Refused(Refusal::AffineUnitArithmetic));
                    continue;
                }
                let sum = exact(sum);
                assert_eq!(
                    sum.value(),
                    &as_rational(reduce(value.0 + 3 * value.1, value.1))
                );
                let product = exact(evaluate_quantity(
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
