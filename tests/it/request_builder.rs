// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-449 (FR-075-AC-8): `route`'s request builder writes one requested
//! item per requirement record of a checked package, with no
//! caller-supplied item list.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::{build_unit, FormsLimits};
use qsl_foundation::bound::{DomainKey, FiniteBound};
use qsl_foundation::digest::{ByteDigest, WireNodeId};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_route::request::{
    items_from_requirements, ExtentClassification, RequestWriter, RequirementItem,
};
use qsl_route::{
    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, Mode, Registry,
    ToolIdentity,
};
use qsl_semantics::check::{Capability, CheckedGraph, CheckingLimits, PackageDeclarations};
use qsl_semantics::family::DomainKind;
use quire_exact::{Integer, IntegerInterval, ValueType};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// FR-062's RR-5.
const SQ: &str = "function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }";

/// FR-062's RR-6.
const BIG: &str = "function big using v(n: Integer): Integer pure { n + 1 }";

/// FR-062's RR-16.
const TWO_CALLEES: &str = "function g using v(p: Int[0, 10]): Boolean pure { true }\n\
    function h using v(q: Int[0, 20]): Boolean pure { true }\n\
    function f using v(x: Int[0, 9]): Boolean pure { g(x + 1) and h(x + 1) }";

/// `declarations` checked through S1, S2 and S3.
fn check(declarations: &str) -> CheckedGraph {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
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
fn package(declarations: &str) -> CheckedPackage {
    CheckedPackage::link(check(declarations))
}

fn backend(id: &str, advertises: (Capability, Mode)) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(
            BackendId::new(id),
            ManifestDigest::from_digest(ByteDigest::of(id.as_bytes()).as_bytes()),
        ),
        ToolIdentity::new(format!("tool-for-{id}")),
        [advertises],
    )
}

/// A registry holding the one backend `kani`, advertising `kind` bounded.
fn registry(kind: Capability) -> (Registry, Candidate) {
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
    let n = DomainKey::new(WireNodeId::from_digest(*n.as_bytes()), Vec::new());
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
#[trace("TC-449", "FR-075-AC-8")]
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
}
