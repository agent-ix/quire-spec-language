// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-440 (ADR-014 §4 "IR's predicate", FR-097-AC-6): QSL's extent
//! classification and IR's `requires-bound` agree over the v2 wire QSL
//! emits.
//!
//! Each fixture record is checked, emitted, read by IR's own v2 reader at
//! the pinned revision (`quire-contract-model`, this crate's `Cargo.toml`) and lowered by IR with `require_bounds` set. QSL classifies
//! the same record type with `family::classify_extent`. IR must report
//! `RequiresBound` exactly when QSL's extent is `Unbounded`, and IR's first
//! unbounded node must be the form QSL named: a collection with no bound, or
//! an `Integer` with no range.
//!
//! IR's `requires-bound` is position-sensitive: a `bounded_domain` over the
//! shared `integer` scalar node does not bound an unranged `Integer` field
//! beside it (`Mixed{n: Integer, k: Int[0, 9]}`), and the `integer` node a
//! bounded collection's `collection_bounds` literals are typed at is not a
//! position (`Flags{xs: Sequence<Boolean>[0, 3]}`). A type in a recursion
//! group requires a bound, so the recursive `RangedTree`, which ADR-014 §4
//! classifies as unbounded by depth, agrees too.
//!
//! A quantity record (`Measure{len: metre}`) is emitted with the `metre`
//! unit and `Length` dimension nodes lowering builds (FR-094), and IR reads
//! and lowers it. QSL classifies it unbounded and unboundable (ADR-014 §4);
//! IR's `requires-bound` treats a unit type as needing no bound, so the
//! agreement test over it is ignored until IR requires one.
//!
//! **Operation-application records** (FR-097-AC-6). QSL's requirement
//! record is the authority for an operation-application claim's extent.
//! IR's predicate on the application node agrees with it for each record
//! whose roots are all reachable from the node through its operands. A
//! record rooted through a `let`'s bound value, or read only by a guard, is
//! outside that agreement: only its own extent is asserted.

use quire_contract_model::{
    read_checked_package, CheckedNodeId, CheckedNodeTag, CheckedPackageDispatchResult,
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CompleteLoweringProfileV2,
    CompleteLoweringRecordV2,
};
use serde_json::{json, Value};

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::{classify_extent, ClaimExtent, DomainKind};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, Integer, IntegerInterval, NodeKey, Presence,
    ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};

use super::tests::source_metre_units;
use super::tests::{nodes, source, whole_unit, wire};
use super::{emit_checked, emit_package, CheckedPackage, Emission, OmittedNode};

const LIMIT: u64 = 1_000;

fn int_0_9() -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap())
}

fn sequence(element: ValueType, bound: Option<(u64, u64)>) -> ValueType {
    ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        element,
        bound.map(|(minimum, maximum)| CardinalityBound::new(minimum, maximum).unwrap()),
    ))
}

fn key(fill: u8) -> NodeKey {
    NodeKey::from_digest([fill; 32])
}

fn record(fill: u8, name: &str, fields: Vec<(&str, ValueType)>) -> CompositeDeclaration {
    CompositeDeclaration::new(
        key(fill),
        name,
        CompositeShape::Record(
            fields
                .into_iter()
                .map(|(field, value_type)| {
                    FieldDeclaration::new(field, value_type, Presence::Required)
                })
                .collect(),
        ),
    )
}

/// The fixture records, each with its environment key.
pub(super) fn records() -> Vec<CompositeDeclaration> {
    vec![
        // §10 scenario 2: a collection with no bound.
        record(1, "Bag", vec![("xs", sequence(int_0_9(), None))]),
        // An `Integer` with no range (ADR-014 §9, C-22).
        record(2, "Counter", vec![("n", ValueType::Integer)]),
        // Every position bounded.
        record(
            3,
            "Box",
            vec![("xs", sequence(int_0_9(), Some((0, 3)))), ("k", int_0_9())],
        ),
        // Unbounded through a record it names.
        record(4, "Outer", vec![("bag", ValueType::Composite(key(1)))]),
        // Bounded through a record it names.
        record(5, "Holder", vec![("b", ValueType::Composite(key(3)))]),
        // A bounded collection of unranged integers.
        record(
            6,
            "Ints",
            vec![("xs", sequence(ValueType::Integer, Some((0, 3))))],
        ),
        // A bounded collection of booleans: its bound literals' `integer`
        // node is not a position.
        record(
            7,
            "Flags",
            vec![("xs", sequence(ValueType::Boolean, Some((0, 3))))],
        ),
        // Recursive (QSpec FR-143). The `Int[0, 9]` field covers
        // the bound literals' `integer` node, so only the recursion differs.
        record(
            8,
            "RangedTree",
            vec![
                ("kids", sequence(ValueType::Composite(key(8)), Some((0, 3)))),
                ("k", int_0_9()),
            ],
        ),
        // An unranged integer beside a ranged one.
        record(
            9,
            "Mixed",
            vec![("n", ValueType::Integer), ("k", int_0_9())],
        ),
    ]
}

/// The checked, emitted package holding [`records`], read back by IR's own
/// v2 reader.
fn emitted() -> (TypeEnvironment, Value, Box<CheckedPackageV2>) {
    let types = TypeEnvironment::new(records(), []).expect("FR-143 admits the fixtures");
    let (wire, admitted, omitted) = emit_and_read(types.clone());
    assert_eq!(omitted, []);
    (types, wire, admitted)
}

/// Check and emit a package declaring `types`, and read it with IR's v2
/// reader: the wire, IR's package and the emitter's omissions.
fn emit_and_read(types: TypeEnvironment) -> (Value, Box<CheckedPackageV2>, Vec<OmittedNode>) {
    let package = package_declaring(types);
    let emission = emit_package(&package, whole_unit).expect("the package emits");
    let (wire, admitted) = read_emission(&emission);
    (wire, admitted, emission.omitted)
}

/// The checked, linked package declaring `types`.
pub(super) fn package_declaring(types: TypeEnvironment) -> CheckedPackage {
    CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("the fixture records check"),
    )
}

/// IR's v2 reading of `emission`, with its wire.
fn read_emission(emission: &Emission) -> (Value, Box<CheckedPackageV2>) {
    let wire = wire(emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageDispatchResult::AdmittedV2(admitted) = read_checked_package(
        emission.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("IR admits the emitted package");
    };
    (wire, admitted)
}

/// The written node declaring `name`, as IR's node id and QSL's wire id.
fn declared(wire: &Value, name: &str) -> (CheckedNodeId, WireNodeId) {
    let node = nodes(wire)
        .iter()
        .find(|node| node["declaration"]["qualified_name"] == json!([name]))
        .unwrap_or_else(|| panic!("{name} is written"));
    let digest = node["node_id"]["digest"].as_str().unwrap();
    (
        CheckedNodeId {
            domain: node["node_id"]["domain"].as_str().unwrap().into(),
            digest: digest.into(),
        },
        WireNodeId::from_hex(digest).expect("a wire node id"),
    )
}

/// The written node with id `id`.
fn node_by_id<'w>(wire: &'w Value, id: &CheckedNodeId) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == json!(id.digest.as_ref()))
        .unwrap_or_else(|| panic!("{} is written", id.digest))
}

/// IR's lowering of `id` with every family supported and bounds required.
fn ir_lowering(package: &CheckedPackageV2, id: &CheckedNodeId) -> CompleteLoweringRecordV2 {
    let profile = CompleteLoweringProfileV2 {
        supported_tags: CheckedNodeTag::ALL.iter().copied().collect(),
        require_bounds: true,
        work_limit: u64::MAX,
    };
    package
        .lower(std::slice::from_ref(id), &profile)
        .records
        .remove(0)
}

/// Where IR's first unbounded node must lie for it to agree with the domain
/// QSL names.
#[derive(Clone, Copy)]
enum FirstUnbounded {
    /// A node of this `semantic_form`, the form QSL names as the domain.
    Form(&'static str),
    /// Any member of the recursion group of the node QSL names
    /// `Recursive`. IR reports the reachable group member with the lowest
    /// node id, which depends on digest order, so only membership is
    /// asserted.
    InRecursionGroup,
}

/// TC-440 (ADR-014 §4; §10 scenarios 2 and 3): over the comparable
/// fixtures, IR's `requires-bound` fires exactly when QSL's extent is
/// unbounded, and IR's first unbounded node is a form QSL named as a domain
/// or, for a recursive type, a member of the recursion group QSL named.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_qsl_extent_agrees_with_ir_requires_bound() {
    use FirstUnbounded::{Form, InRecursionGroup};
    let expectations = [
        ("Bag", Some((Form("sequence"), DomainKind::Collection))),
        ("Counter", Some((Form("integer"), DomainKind::Integer))),
        ("Box", None),
        ("Outer", Some((Form("sequence"), DomainKind::Collection))),
        ("Holder", None),
        ("Ints", Some((Form("integer"), DomainKind::Integer))),
        ("Flags", None),
        (
            "RangedTree",
            Some((InRecursionGroup, DomainKind::Recursive)),
        ),
        ("Mixed", Some((Form("integer"), DomainKind::Integer))),
    ];
    for (name, expected) in expectations {
        let (extent, lowering, wire) = both_sides(name);
        match (expected, &extent, &lowering) {
            (None, ClaimExtent::Bounded, CompleteLoweringRecordV2::Lowered { .. }) => {}
            (
                Some((first, kind)),
                ClaimExtent::Unbounded(domains),
                CompleteLoweringRecordV2::RequiresBound { unbounded_type, .. },
            ) => {
                let named = domains
                    .iter()
                    .find(|(_, domain)| *domain == kind)
                    .map(|(key, _)| match key {
                        qsl_foundation::bound::DomainKey::Node { node, .. } => *node,
                        other => panic!("{name}: a {kind:?} domain is a node key: {other:?}"),
                    })
                    .unwrap_or_else(|| panic!("{name}: QSL names a {kind:?} domain: {domains:?}"));
                let ir_node = node_by_id(&wire, unbounded_type);
                match first {
                    Form(form) => assert_eq!(
                        ir_node["semantic_form"], form,
                        "{name}: IR's first unbounded node is the {form} QSL named"
                    ),
                    InRecursionGroup => {
                        let group = &wire_node(&wire, named)["recursion_group"];
                        assert!(
                            group.is_string(),
                            "{name}: QSL's Recursive root is in a recursion group"
                        );
                        assert_eq!(
                            &ir_node["recursion_group"], group,
                            "{name}: IR's first unbounded node is in the recursion group QSL named"
                        );
                    }
                }
            }
            _ => panic!("{name}: QSL {extent:?} disagrees with IR {lowering:?}"),
        }
    }
}

/// The written node whose wire id is `node`.
fn wire_node(wire: &Value, node: WireNodeId) -> &Value {
    let digest = node.to_string();
    nodes(wire)
        .iter()
        .find(|written| written["node_id"]["digest"] == json!(digest))
        .unwrap_or_else(|| panic!("{digest} is written"))
}

/// QSL's extent of the record declared `name`, and IR's lowering of it.
fn both_sides(name: &str) -> (ClaimExtent, CompleteLoweringRecordV2, Value) {
    let (types, wire, package) = emitted();
    let (ir_id, wire_id) = declared(&wire, name);
    let declaration = records()
        .into_iter()
        .find(|declaration| declaration.name() == name)
        .unwrap();
    let checked_type = ValueType::Composite(declaration.key());
    let extent = classify_extent(&[(wire_id, &checked_type)], &types, LIMIT).unwrap();
    (extent, ir_lowering(&package, &ir_id), wire)
}

/// `Measure{len: metre}`, whose `metre` unit the fixture unit's own source
/// declares, checked, emitted and read by IR's v2 reader: the environment,
/// the wire, IR's package and `Measure`'s ids.
fn emitted_measure() -> (
    TypeEnvironment,
    CompositeDeclaration,
    Value,
    Box<CheckedPackageV2>,
    quire_exact::UnitId,
) {
    let (units, metre) = source_metre_units();
    let measure = record(10, "Measure", vec![("len", ValueType::Quantity(metre))]);
    let types = TypeEnvironment::new([measure.clone()], [])
        .expect("FR-143 admits Measure")
        .with_units(units);
    let (wire, package, omitted) = emit_and_read(types.clone());
    assert!(omitted.is_empty(), "{omitted:?}");
    (types, measure, wire, package, metre)
}

/// TC-440 (FR-097-AC-6, FR-094 "Quantity type nodes"): a record with a
/// quantity field reaches IR. Lowering builds the `metre` unit node and its
/// `Length` dimension node, so the emission omits nothing: it writes
/// `Measure`, the unit node with its `quire.unit-node/v1` preimage, typed
/// by and depending on the dimension node, and IR's v2 reader admits the
/// package and lowers `Measure`.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_a_quantity_record_is_emitted_and_reaches_ir() {
    let (_, _, wire, package, metre) = emitted_measure();
    let metre_id = metre
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let unit = nodes(&wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == json!(metre_id))
        .expect("the metre unit node is written");
    assert_eq!(unit["node_tag"], json!("scalar_type"));
    assert_eq!(unit["semantic_form"], json!("unit"));
    let preimage = &unit["nominal_identity_preimage"];
    assert_eq!(preimage["version"], json!("quire.unit-node/v1"));
    assert_eq!(
        preimage["qualified_declaration"],
        json!(["Example", "metre"])
    );
    let dimension = &preimage["dimension_node_id"];
    assert_eq!(&unit["semantic_type"], dimension);
    assert_eq!(unit["dependencies"], json!([dimension]));
    let length = nodes(&wire)
        .iter()
        .find(|node| node["node_id"] == *dimension)
        .expect("the Length dimension node is written");
    assert_eq!(length["semantic_form"], json!("dimension"));
    assert_eq!(
        length["nominal_identity_preimage"]["version"],
        json!("quire.dimension-node/v1")
    );
    let (ir_id, _) = declared(&wire, "Measure");
    let lowering = ir_lowering(&package, &ir_id);
    assert!(
        matches!(
            lowering,
            CompleteLoweringRecordV2::Lowered { .. }
                | CompleteLoweringRecordV2::RequiresBound { .. }
        ),
        "{lowering:?}"
    );
}

/// A source-declared unit graph past roots over base dimensions: the base
/// dimensions `Length` and `Time`, the derived dimension `Velocity = Length
/// / Time`, the root units `metre`, `second` and `mps` (of `Velocity`), and
/// `km = 1000 × metre`, a non-root unit. Each node is owned by the fixture
/// unit's own source. Returns the table and the keys by name.
fn velocity_units() -> (
    quire_semantic_value::quantity::UnitTable,
    std::collections::BTreeMap<&'static str, NodeKey>,
) {
    use qsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeIdentityPreimage, NodeOwner, OwnerSelection,
        UnitPreimage,
    };
    let owner = json!({"kind": "source", "authority": "a", "identity": "u"});
    let id =
        |key: NodeKey| json!({"domain": quire_exact::NODE_KEY_DOMAIN, "digest": key.to_string()});
    let mut keys = std::collections::BTreeMap::new();
    let dimension = |name: &str, terms: Value| {
        let preimage = DimensionPreimage::from_json(json!({
            "version": "quire.dimension-node/v1",
            "owner": owner,
            "qualified_declaration": [name],
            "terms": terms,
        }))
        .expect("a dimension");
        let key = NodeKey::from_digest(preimage.digest().expect("the dimension digests"));
        (preimage, key)
    };
    let length = dimension("Length", json!([]));
    let time = dimension("Time", json!([]));
    let mut velocity_terms = vec![
        json!({"dimension_node_id": id(length.1), "exponent": "1"}),
        json!({"dimension_node_id": id(time.1), "exponent": "-1"}),
    ];
    velocity_terms.sort_by_key(|term| term["dimension_node_id"]["digest"].to_string());
    let velocity = dimension("Velocity", Value::Array(velocity_terms));
    let unit = |name: &str, dimension: NodeKey, target: Option<NodeKey>, scale: &str| {
        let preimage = UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner,
            "qualified_declaration": [name],
            "dimension_node_id": id(dimension),
            "target_unit_node_id": target.map(id),
            "scale": {"numerator": scale, "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a unit");
        let key = NodeKey::from_digest(preimage.digest().expect("the unit digests"));
        (preimage, key)
    };
    let metre = unit("metre", length.1, None, "1");
    let km = unit("km", length.1, Some(metre.1), "1000");
    let second = unit("second", time.1, None, "1");
    let mps = unit("mps", velocity.1, None, "1");
    for (name, key) in [
        ("Length", length.1),
        ("Time", time.1),
        ("Velocity", velocity.1),
        ("metre", metre.1),
        ("km", km.1),
        ("second", second.1),
        ("mps", mps.1),
    ] {
        keys.insert(name, key);
    }
    let selection: NodeOwner = serde_json::from_value(owner.clone()).expect("an owner");
    let graph = admit_unit_graph(
        [length, time, velocity],
        [metre, km, second, mps],
        &OwnerSelection::new([selection]),
    )
    .expect("the unit graph admits");
    (
        quire_semantic_value::quantity::UnitTable::declared(&graph),
        keys,
    )
}

/// FR-094-AC-8 (TC-419 step 5): `Trip{d: km, v: mps}`, over a non-root
/// unit and a unit of a derived dimension, is emitted with nothing omitted.
/// Lowering's walk follows `km`'s target to `metre` and `Velocity`'s terms
/// to `Length` and `Time`; the emitter lists those as the nodes'
/// dependencies and writes each node's own preimage; IR's v2 reader admits
/// the package (the emit-and-read path TC-440 uses) and lowers `Trip`.
#[trace("FR-094-AC-8", "TC-419")]
#[test]
fn a_non_root_unit_and_a_derived_dimension_are_emitted_and_read_by_ir() {
    let (units, keys) = velocity_units();
    let quantity = |name: &str| ValueType::Quantity(quire_exact::UnitId::declared(keys[name]));
    let trip = record(
        12,
        "Trip",
        vec![("d", quantity("km")), ("v", quantity("mps"))],
    );
    let types = TypeEnvironment::new([trip], [])
        .expect("FR-143 admits Trip")
        .with_units(units);
    let (wire, package, omitted) = emit_and_read(types);
    assert!(omitted.is_empty(), "{omitted:?}");
    let node = |name: &str| {
        let digest = keys[name].to_string();
        nodes(&wire)
            .iter()
            .find(|node| node["node_id"]["digest"] == json!(digest))
            .unwrap_or_else(|| panic!("the {name} node is written"))
            .clone()
    };
    let ids = |names: &[&str]| {
        let mut ids: Vec<Value> = names
            .iter()
            .map(|name| json!({"domain": quire_exact::NODE_KEY_DOMAIN, "digest": keys[name].to_string()}))
            .collect();
        ids.sort_by_key(|id| id["digest"].to_string());
        Value::Array(ids)
    };
    let km = node("km");
    assert_eq!(km["semantic_form"], json!("unit"));
    assert_eq!(km["dependencies"], ids(&["Length", "metre"]));
    let km_preimage = &km["nominal_identity_preimage"];
    assert_eq!(
        km_preimage["target_unit_node_id"]["digest"],
        json!(keys["metre"].to_string())
    );
    assert_eq!(
        km_preimage["scale"],
        json!({"numerator": "1000", "denominator": "1"})
    );
    assert_eq!(node("metre")["dependencies"], ids(&["Length"]));
    let velocity = node("Velocity");
    assert_eq!(velocity["semantic_form"], json!("dimension"));
    assert_eq!(velocity["dependencies"], ids(&["Length", "Time"]));
    let terms = velocity["nominal_identity_preimage"]["terms"]
        .as_array()
        .expect("Velocity's terms");
    assert_eq!(terms.len(), 2);
    assert_eq!(
        node("mps")["semantic_type"]["digest"],
        json!(keys["Velocity"].to_string())
    );
    for base in ["Length", "Time"] {
        assert_eq!(node(base)["dependencies"], json!([]));
    }
    let (ir_id, _) = declared(&wire, "Trip");
    let lowering = ir_lowering(&package, &ir_id);
    assert!(
        matches!(
            lowering,
            CompleteLoweringRecordV2::Lowered { .. }
                | CompleteLoweringRecordV2::RequiresBound { .. }
        ),
        "{lowering:?}"
    );
}

/// TC-440: agreement over the quantity record `Measure{len: metre}`. Its
/// magnitude is an unbounded `Rational`, so QSL classifies it unbounded and
/// unboundable (ADR-014 §4), and IR must require a bound for it.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_quantity_extent_agrees_with_ir_requires_bound() {
    let (types, measure, wire, package, _) = emitted_measure();
    let (ir_id, wire_id) = declared(&wire, "Measure");
    let checked_type = ValueType::Composite(measure.key());
    let extent = classify_extent(&[(wire_id, &checked_type)], &types, LIMIT).unwrap();
    let lowering = ir_lowering(&package, &ir_id);
    if let Some(disagreement) =
        disagreement("Measure", Some(DomainKind::Quantity), &extent, &lowering)
    {
        panic!("{disagreement}");
    }
}

/// Whether QSL's `extent` and IR's `lowering` of `name` agree, given QSL's
/// `expected` domain kind (`None` for bounded); the disagreement if not.
fn disagreement(
    name: &str,
    expected: Option<DomainKind>,
    extent: &ClaimExtent,
    lowering: &CompleteLoweringRecordV2,
) -> Option<String> {
    match (expected, extent, lowering) {
        (None, ClaimExtent::Bounded, CompleteLoweringRecordV2::Lowered { .. }) => None,
        (
            Some(kind),
            ClaimExtent::Unbounded(domains),
            CompleteLoweringRecordV2::RequiresBound { .. },
        ) if domains.iter().any(|(_, domain)| domain == kind) => None,
        _ => Some(format!("{name}: QSL {extent:?}; IR {lowering:?}")),
    }
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// `declarations` checked from source text through S1 to S4, emitted at
/// its form spans and read back by IR's v2 reader.
fn emit_text(declarations: &str) -> (CheckedPackage, Value, Box<CheckedPackageV2>) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            Vec::new(),
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}")),
    );
    let emission = emit_checked(&package).expect("the package emits");
    let (wire, admitted) = read_emission(&emission);
    (package, wire, admitted)
}

/// The written node whose id is `node`, as IR's node id.
fn written(wire: &Value, node: WireNodeId) -> CheckedNodeId {
    let written = wire_node(wire, node);
    CheckedNodeId {
        domain: written["node_id"]["domain"].as_str().unwrap().into(),
        digest: node.to_string().into(),
    }
}

/// Each requirement record of `package` whose application applies
/// `operation`, in key order: its node and its extent.
fn requirement_records(
    package: &CheckedPackage,
    operation: &str,
) -> Vec<(WireNodeId, ClaimExtent)> {
    let graph = package.graph();
    graph
        .requirements()
        .iter()
        .filter(|(key, _)| {
            let node = graph
                .semantic_graph()
                .node(NodeKey::from_digest(*key.node().as_bytes()))
                .expect("the record's node is lowered");
            matches!(
                node.body(),
                qsl_semantics::check::BodyTerm::Application(
                    qsl_semantics::check::ApplicationTerm { operation: applied, .. }
                ) if applied.identity() == operation
            )
        })
        .map(|(key, record)| (key.node(), record.requirements().extent().clone()))
        .collect()
}

/// `function`'s parameter at `index`, as the unbounded `Integer` domain of
/// an extent rooted there.
fn integer_at(package: &CheckedPackage, function: &str, index: usize) -> ClaimExtent {
    let graph = package.graph();
    let parameter = graph
        .semantic_graph()
        .node(graph.function_identity(function).expect("declared"))
        .and_then(|node| node.function_parameters())
        .expect("a function node")[index];
    ClaimExtent::from_domains(std::collections::BTreeMap::from([(
        qsl_foundation::bound::DomainKey::Node {
            node: WireNodeId::from_digest(*parameter.as_bytes()),
            path: Vec::new(),
        },
        DomainKind::Integer,
    )]))
}

/// `(x + 1) + n`'s two `+` records, checked and emitted: the inner and the
/// outer application's node and extent, and IR's lowering of each node.
fn inner_and_outer() -> [(ClaimExtent, CompleteLoweringRecordV2); 2] {
    let (package, wire, admitted) =
        emit_text("function f using v(x: Int[0, 9], n: Integer): Integer pure { (x + 1) + n }");
    let outer = integer_at(&package, "f", 1);
    let mut found: Vec<(ClaimExtent, CompleteLoweringRecordV2)> =
        requirement_records(&package, "quire.op.integer.add")
            .into_iter()
            .map(|(node, extent)| {
                let lowering = ir_lowering(&admitted, &written(&wire, node));
                (extent, lowering)
            })
            .collect();
    // The inner `+` first: its extent is the bounded one.
    found.sort_by_key(|(extent, _)| *extent != ClaimExtent::Bounded);
    let [inner, outer_found] = <[_; 2]>::try_from(found).expect("two `+` records");
    assert_eq!(inner.0, ClaimExtent::Bounded);
    assert_eq!(outer_found.0, outer, "the outer `+` is unbounded at `n`");
    [inner, outer_found]
}

/// TC-440 step 4 (FR-097-AC-6): over `(x + 1) + n`, the inner `+` record
/// is `Bounded` and IR lowers its node; the outer `+` record is
/// `Unbounded` at `n`. RR-7's `*` (rooted through `t`'s bound value) and
/// `k`'s `+` (rooted at `n` through its guard) are outside the agreement;
/// their records' extents are asserted, `Bounded` and `Unbounded` at `n`.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_operation_application_records_agree_with_ir_per_node() {
    let [(_, inner), _] = inner_and_outer();
    assert!(
        matches!(inner, CompleteLoweringRecordV2::Lowered { .. }),
        "IR lowers the bounded inner `+`: {inner:?}"
    );

    let (package, _, _) =
        emit_text("function lt using v(x: Int[0, 9]): Integer pure { let t = x + 1 in t * 2 }");
    let multiply: Vec<ClaimExtent> = requirement_records(&package, "quire.op.integer.mul")
        .into_iter()
        .map(|(_, extent)| extent)
        .collect();
    assert_eq!(multiply, [ClaimExtent::Bounded]);

    let (package, _, _) = emit_text(
        "function k using v(x: Int[0, 9], n: Integer): Integer pure { if n = 0 then x + 1 else 0 }",
    );
    let add: Vec<ClaimExtent> = requirement_records(&package, "quire.op.integer.add")
        .into_iter()
        .map(|(_, extent)| extent)
        .collect();
    assert_eq!(add, [integer_at(&package, "k", 1)]);
}

/// TC-440 step 4 (FR-097-AC-6): IR requires a bound for the outer `+` of
/// `(x + 1) + n`, whose record is `Unbounded` at `n`. `x: Int[0, 9]`'s
/// `bounded_domain` over the shared `integer` node does not bound `n`'s
/// position.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_an_unbounded_application_record_requires_a_bound_in_ir() {
    let [_, (_, outer)] = inner_and_outer();
    assert!(
        matches!(outer, CompleteLoweringRecordV2::RequiresBound { .. }),
        "IR requires a bound for the unbounded outer `+`: {outer:?}"
    );
}
