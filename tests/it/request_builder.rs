// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-449 (FR-075-AC-8): `route`'s request builder writes one requested
//! item per requirement record of a checked package, with no
//! caller-supplied item list.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound};
use qsl_foundation::digest::{ByteDigest, WireNodeId};
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_route::request::{
    items_from_requirements, ExtentClassification, RequestWriter, RequirementItem,
};
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, ManifestDigest, Mode, ProviderOrigin, Registry,
};
use qsl_semantics::check::{
    ApplicationTerm, BodyTerm, Capability, CheckedGraph, LeafTerm, MemberTerm, PackageDeclarations,
};
use quire_exact::{Integer, IntegerInterval, NodeKey, ValueType};
use quire_semantic_value::checking::CheckingLimits;

pub(super) const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// FR-062's RR-5.
const SQ: &str = "function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }";

/// FR-062's RR-6.
const BIG: &str = "function big using v(n: Integer): Integer pure { n + 1 }";

/// TC-984's opposite-guard occurrences of one application node.
const GUARDED: &str =
    "function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }";

/// TC-984's unguarded record.
const UNGUARDED: &str = "function g using v(x: Int[0, 9]): Integer pure { x + 1 }";

/// QSL-710's `inc` example: the `+` is narrowed to the declared result.
const INC: &str = "function inc using v(a: Int[0, 9]): Int[0, 10] pure { a + 1 }";

/// FR-062's RR-16.
const TWO_CALLEES: &str = "function g using v(p: Int[0, 10]): Boolean pure { true }\n\
    function h using v(q: Int[0, 20]): Boolean pure { true }\n\
    function f using v(x: Int[0, 9]): Boolean pure { g(x + 1) and h(x + 1) }";

/// `declarations` checked through S1, S2 and S3.
/// Trace: FR-003-AC-9
pub(super) fn check(declarations: &str) -> CheckedGraph {
    let text = format!("{HEADER}{declarations}\n");
    crate::support::front_end::emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        &BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("each checked request-builder fixture retains its formatted package identity");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// `declarations` checked and linked into an S4 package.
pub(super) fn package(declarations: &str) -> CheckedPackage {
    CheckedPackage::link(check(declarations))
}

fn backend(id: &str, advertises: (Capability, Mode)) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new(id)),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(id.as_bytes()).as_bytes()),
        [advertises],
        Some(BTreeSet::from([DomainKind::Collection])),
    )
    .expect("a well-formed descriptor")
}

/// A registry holding the one backend `kani`, advertising `kind` bounded.
pub(super) fn registry(kind: Capability) -> (Registry, Candidate) {
    let descriptor = backend("kani", (kind, Mode::Bounded));
    let candidate = descriptor.candidate().clone();
    let mut registry = Registry::new();
    registry.register(descriptor).expect("one registration");
    (registry, candidate)
}

fn candidates(item: &RequirementItem) -> &[Candidate] {
    match item.candidates() {
        CandidateOutcome::Candidates(set) => set.candidates(),
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

/// TC-449 step 1 (FR-075-AC-8), and TC-160 step 10's S4 read: RR-5's three
/// records are three items, in occurrence-key order with request indices
/// 0, 1 and 2; the two `+` items share one node and have distinct keys;
/// each is `value-validity`, `bounded`, with no unbounded domain, its
/// record's result bound and the one backend as its candidate set. The
/// records `CheckedPackage::graph()` reaches are S3's.
#[trace("TC-449", "FR-075-AC-8", "TC-160", "FR-062-AC-13")]
#[test]
fn tc_449_one_item_per_record_in_key_order() {
    let package = package(SQ);
    let records = package.graph().requirements();
    assert_eq!(records, check(SQ).requirements());
    let (registry, kani) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    assert_eq!(items.len(), 3);
    for ((position, item), (key, record)) in items.iter().enumerate().zip(records) {
        assert_eq!(item.index().get(), position);
        assert_eq!(item.occurrence(), key);
        assert_eq!(item.node(), key.node());
        assert_eq!(item.kind(), Capability::ValueValidity);
        assert_eq!(item.extent(), ExtentClassification::Bounded);
        assert_eq!(item.unbounded(), []);
        assert_eq!(item.result_bound(), record.result_bound().node());
        assert_eq!(candidates(item), std::slice::from_ref(&kani));
    }
    let keys: Vec<_> = items.iter().map(RequirementItem::occurrence).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");
    let add: Vec<&RequirementItem> = items
        .iter()
        .filter(|item| {
            items
                .iter()
                .filter(|other| other.node() == item.node())
                .count()
                == 2
        })
        .collect();
    let [first, second] = add.as_slice() else {
        panic!("two items at the `+` node: {items:#?}");
    };
    assert_ne!(first.occurrence(), second.occurrence());
}

/// TC-449 step 2 (FR-075-AC-8): RR-6's one item is `unbounded` with a
/// finite bound available and one `Integer` domain at `n`'s parameter
/// node. Its bounded follow-up is a new request of one item, index 0,
/// `bounded`, carrying the original item's occurrence key.
#[trace("TC-449", "FR-075-AC-8")]
#[test]
fn tc_449_an_unbounded_item_and_its_bounded_follow_up() {
    let package = package(BIG);
    let graph = package.graph();
    let records = graph.requirements();
    let (registry, _) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    let [item] = items.as_slice() else {
        panic!("one item: {items:#?}");
    };
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );
    let n = graph
        .semantic_graph()
        .node(graph.function_identity("big").expect("big is declared"))
        .and_then(|function| function.function_parameters())
        .expect("big's function node")[0];
    let n = DomainKey::Node {
        node: WireNodeId::from_digest(*n.as_bytes()),
        path: Vec::new(),
    };
    assert_eq!(item.unbounded(), [(n.clone(), DomainKind::Integer)]);

    let record = &records[item.occurrence()];
    let mut follow_up = RequestWriter::new();
    let index = follow_up
        .bounded_item(
            item.occurrence().clone(),
            record.requirements(),
            BTreeMap::from([(
                n,
                FiniteBound::integer_range(Integer::from(0_i64), Integer::from(100_i64))
                    .expect("a nonempty range"),
            )]),
        )
        .expect("one bound of the domain's kind");
    let written = follow_up.finish();
    assert_eq!(index.get(), 0);
    let [bounded] = written.as_slice() else {
        panic!("one follow-up item: {written:#?}");
    };
    assert_eq!(bounded.index().get(), 0);
    assert_eq!(bounded.extent(), ExtentClassification::Bounded);
    assert_eq!(bounded.occurrence(), item.occurrence());
    assert_eq!(
        written
            .iter()
            .filter(|written| written.occurrence() == item.occurrence())
            .count(),
        1
    );
}

/// TC-449 steps 3 and 4 (FR-075-AC-8): with no backend advertising
/// `value-validity`, every item is still written, each with an empty
/// candidate set; with a named backend the registry does not hold, every
/// item carries the unknown-backend marker naming it.
#[trace("TC-449", "FR-075-AC-8")]
#[test]
fn tc_449_every_record_is_an_item_whatever_its_candidates() {
    let package = package(SQ);
    let records = package.graph().requirements();
    let (contracts, _) = registry(Capability::OperationContract);
    let items = items_from_requirements(records, &contracts, None);
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|item| candidates(item).is_empty()));

    let (registry, _) = registry(Capability::ValueValidity);
    let missing = BackendId::new("missing");
    let items = items_from_requirements(records, &registry, Some(&missing));
    assert_eq!(items.len(), 3);
    for item in &items {
        assert_eq!(
            item.candidates(),
            &CandidateOutcome::UnknownBackend(missing.clone())
        );
    }
}

/// TC-449 step 5 (FR-075-AC-8): a unit whose only body is `b and c` has no
/// record, so no item.
#[trace("TC-449", "FR-075-AC-8")]
#[test]
fn tc_449_a_package_with_no_records_gives_no_items() {
    let package =
        package("function both using v(b: Boolean, c: Boolean): Boolean pure { b and c }");
    let (registry, _) = registry(Capability::ValueValidity);
    assert_eq!(
        items_from_requirements(package.graph().requirements(), &registry, None),
        []
    );
}

/// TC-449 step 6 (FR-075-AC-8): RR-16's two `+` items at one node carry
/// distinct occurrence keys and their own records' result bounds,
/// `Int[0, 10]` then `Int[0, 20]` in key order.
#[trace("TC-449", "FR-075-AC-8", "FR-075-AC-15")]
#[test]
fn tc_449_two_items_at_one_node_carry_their_own_result_bounds() {
    let package = package(TWO_CALLEES);
    let records = package.graph().requirements();
    let (registry, _) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    let [first, second] = items.as_slice() else {
        panic!("two items: {items:#?}");
    };
    assert_eq!(first.node(), second.node());
    assert_ne!(first.occurrence(), second.occurrence());
    let bounds: Vec<&ValueType> = records
        .values()
        .map(|record| record.result_bound().value_type())
        .collect();
    assert_eq!(bounds, [&int(0, 10), &int(0, 20)]);
    let nodes: Vec<WireNodeId> = records
        .values()
        .map(|record| record.result_bound().node())
        .collect();
    assert_eq!([first.result_bound(), second.result_bound()], nodes[..]);
    assert_ne!(first.result_bound(), second.result_bound());
    let contexts: Vec<_> = records
        .values()
        .map(|record| record.conversion_context().cloned())
        .collect();
    assert_eq!(contexts.len(), 2);
    assert!(contexts.iter().all(Option::is_some));
    assert_eq!(first.conversion_context(), contexts[0].as_ref());
    assert_eq!(second.conversion_context(), contexts[1].as_ref());
    assert_ne!(contexts[0], contexts[1]);
}

/// FR-062-AC-13: the `inc` example's request item carries the enclosing
/// narrow's identity as its typed conversion context: a node other than the
/// add, whose operation is the narrow and whose only operand is the add.
#[trace("TC-449", "FR-075-AC-8", "FR-062-AC-13")]
#[test]
fn tc_449_inc_item_carries_its_enclosing_narrow_as_conversion_context() {
    let package = package(INC);
    let records = package.graph().requirements();
    let (registry, _) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    let [item] = items.as_slice() else {
        panic!("one item for the add: {items:#?}");
    };
    let context = item
        .conversion_context()
        .expect("the narrowed add carries a conversion context");
    assert_ne!(context.node(), item.node());
    let graph = package.graph().semantic_graph();
    let narrow = graph
        .node(NodeKey::from_digest(*context.node().as_bytes()))
        .expect("the context names a graph node");
    let BodyTerm::Application(ApplicationTerm {
        operation,
        arguments,
        ..
    }) = narrow.body()
    else {
        panic!("the context is an application: {narrow:#?}");
    };
    assert_eq!(operation.identity(), "quire.op.numeric.narrow");
    let [MemberTerm::Leaf(LeafTerm::Reference { target })] = arguments.as_slice() else {
        panic!("the narrow has the add as its one operand: {arguments:#?}");
    };
    assert_eq!(*target.0.as_bytes(), *item.node().as_bytes());
}

/// TC-984 / FR-075-AC-9: each request item preserves its record's canonical
/// guard sequence, including opposite outcomes for the shared branch guard.
#[trace("TC-984", "FR-075-AC-9")]
#[test]
fn tc_984_items_preserve_occurrence_path_conditions() {
    let checked = package(GUARDED);
    let records = checked.graph().requirements();
    let (registry, _) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    let selected: Vec<_> = items
        .iter()
        .filter(|item| {
            items
                .iter()
                .filter(|other| other.node() == item.node())
                .count()
                == 2
        })
        .collect();
    let [first, second] = selected.as_slice() else {
        panic!("two items at the shared `+` node: {items:#?}");
    };
    assert_eq!(first.node(), second.node());
    assert_ne!(first.occurrence(), second.occurrence());
    assert!(first.occurrence() < second.occurrence());

    for item in [*first, *second] {
        let record = &records[item.occurrence()];
        assert_eq!(item.path_condition(), record.path_condition());
        assert_eq!(item.path_condition().len(), 1);
    }
    let [then_guard] = first.path_condition() else {
        panic!("then occurrence has one guard: {first:#?}");
    };
    let [else_guard] = second.path_condition() else {
        panic!("else occurrence has one guard: {second:#?}");
    };
    assert_eq!(then_guard.occurrence(), else_guard.occurrence());
    assert!(then_guard.holds());
    assert!(!else_guard.holds());

    let unguarded = package(UNGUARDED);
    let records = unguarded.graph().requirements();
    let items = items_from_requirements(records, &registry, None);
    let [item] = items.as_slice() else {
        panic!("one unguarded item: {items:#?}");
    };
    assert_eq!(
        item.path_condition(),
        records[item.occurrence()].path_condition()
    );
    assert!(item.path_condition().is_empty());
    assert_eq!(item.conversion_context(), None);
}
