// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-440 (ADR-014 §4 "IR's predicate", FR-097-AC-6): QSL's extent
//! classification and IR's `requires-bound` agree over the v2 wire QSL
//! emits.
//!
//! Each fixture record is checked, emitted, read by IR's own v2 reader at
//! the pinned revision (`quire-contract-model` `1d7884c`, this crate's
//! `Cargo.toml`) and lowered by IR with `require_bounds` set. QSL classifies
//! the same record type with `family::classify_extent`. IR must report
//! `RequiresBound` exactly when QSL's extent is `Unbounded`, and IR's first
//! unbounded node must be the form QSL named: a collection with no bound, or
//! an `Integer` with no range.
//!
//! Two cases disagree at the pinned revision and are asserted as measured,
//! so each assertion fails when IR changes and the case can join the
//! agreeing ones:
//!
//! - **A bounded collection of a non-integer element** (`Flags`). QSL emits
//!   the `collection_bounds` domain's `min` and `max` as integer literals
//!   typed at the `integer` scalar node. IR `1d7884c`'s `requires_bound`
//!   walks every reachable node, finds that `integer` node with no
//!   `integer_range` over it, and reports `RequiresBound` naming it. The
//!   bound's own literals are not a value domain, so QSL classifies the
//!   record `Bounded`. A record that also has an `Int[a,b]` field hides the
//!   difference, because its `integer_range` covers the same `integer` node.
//! - **A recursive record** (`RangedTree`). ADR-014 §4 makes a recursive
//!   value type (QSpec FR-143) an unbounded domain, boundable by `Depth`.
//!   IR `1d7884c`'s `requires_bound` has no recursion rule, so it lowers the
//!   record.

use quire_contract_ir::{
    read_checked_package, CheckedNodeId, CheckedNodeTag, CheckedPackageDispatchResult,
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CompleteLoweringProfileV2,
    CompleteLoweringRecordV2,
};
use serde_json::{json, Value};

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{CheckingLimits, PackageDeclarations};
use qsl_semantics::family::{classify_extent, ClaimExtent, DomainKind};
use qsl_semantics::value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, Integer, IntegerInterval, NodeKey, Presence,
    ValueType,
};

use super::tests::{locked_artifacts, nodes, source, whole_unit, wire};
use super::{emit_package, CheckedPackage};

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
fn records() -> Vec<CompositeDeclaration> {
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
        // Measured divergence: a bounded collection of booleans.
        record(
            7,
            "Flags",
            vec![("xs", sequence(ValueType::Boolean, Some((0, 3))))],
        ),
        // Measured divergence: recursive (QSpec FR-143). The `Int[0, 9]`
        // field covers the bound literals' `integer` node, so only the
        // recursion differs.
        record(
            8,
            "RangedTree",
            vec![
                ("kids", sequence(ValueType::Composite(key(8)), Some((0, 3)))),
                ("k", int_0_9()),
            ],
        ),
    ]
}

/// The checked, emitted package holding [`records`], read back by IR's own
/// v2 reader.
fn emitted() -> (TypeEnvironment, Value, Box<CheckedPackageV2>) {
    let types = TypeEnvironment::new(records(), []).expect("FR-143 admits the fixtures");
    let package = CheckedPackage::link(
        PackageDeclarations {
            types: types.clone(),
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("the fixture records check"),
    );
    let emission = emit_package(&package, whole_unit).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    locked_artifacts(&wire["lock"], &mut evidence);
    locked_artifacts(&wire["diagnostics"], &mut evidence);
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
    (types, wire, admitted)
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

/// TC-440 (ADR-014 §4; §10 scenarios 2 and 3): over the agreeing fixtures,
/// IR's `requires-bound` fires exactly when QSL's extent is unbounded, and
/// IR's first unbounded node is a form QSL named as a domain.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_qsl_extent_agrees_with_ir_requires_bound() {
    let expectations = [
        ("Bag", Some(("sequence", DomainKind::Collection))),
        ("Counter", Some(("integer", DomainKind::Integer))),
        ("Box", None),
        ("Outer", Some(("sequence", DomainKind::Collection))),
        ("Holder", None),
        ("Ints", Some(("integer", DomainKind::Integer))),
    ];
    for (name, expected) in expectations {
        let (extent, lowering, wire) = both_sides(name);
        match (expected, &extent, &lowering) {
            (None, ClaimExtent::Bounded, CompleteLoweringRecordV2::Lowered { .. }) => {}
            (
                Some((form, kind)),
                ClaimExtent::Unbounded(domains),
                CompleteLoweringRecordV2::RequiresBound { unbounded_type, .. },
            ) => {
                assert!(
                    domains.iter().any(|(_, domain)| domain == kind),
                    "{name}: QSL names a {kind:?} domain: {domains:?}"
                );
                assert_eq!(
                    node_by_id(&wire, unbounded_type)["semantic_form"],
                    form,
                    "{name}: IR's first unbounded node is the {form} QSL named"
                );
            }
            _ => panic!("{name}: QSL {extent:?} disagrees with IR {lowering:?}"),
        }
    }
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

/// TC-440: the measured divergences at IR `1d7884c` (this module's doc).
/// `Flags` is bounded in QSL, and IR requires a bound for the `integer`
/// node its bound literals are typed at; `RangedTree` is unbounded by depth
/// in QSL, and IR lowers it.
#[trace("TC-440", "FR-097-AC-6")]
#[test]
fn tc_440_measured_divergences_from_ir_at_the_pin() {
    let (extent, lowering, wire) = both_sides("Flags");
    assert_eq!(extent, ClaimExtent::Bounded);
    let CompleteLoweringRecordV2::RequiresBound { unbounded_type, .. } = &lowering else {
        panic!("IR now lowers Flags: move it into the agreeing cases: {lowering:?}");
    };
    assert_eq!(
        node_by_id(&wire, unbounded_type)["semantic_form"],
        "integer"
    );

    let (extent, lowering, _) = both_sides("RangedTree");
    let ClaimExtent::Unbounded(domains) = extent else {
        panic!("a recursive record is unbounded (ADR-014 §4)");
    };
    assert_eq!(
        domains.iter().map(|(_, kind)| kind).collect::<Vec<_>>(),
        [DomainKind::Recursive]
    );
    assert!(
        matches!(lowering, CompleteLoweringRecordV2::Lowered { .. }),
        "IR now classifies the recursive record: move it into the agreeing cases: {lowering:?}"
    );
}
