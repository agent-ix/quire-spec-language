// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1024 and TC-195 N12/N13 (FR-081-AC-11, NFR-012): the bytes admitted to
//! identity hashing, the retained facts and the work units of normalization
//! follow closed formulas that are linear in effective types and direct
//! supertype edges, however many ancestor paths the shape has.
//!
//! The fact and work-unit figures are computed here independently of the
//! encoder under test; byte totals are checked only for linear growth.

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::{
    ChargePoint, Incomplete, LimitKind, ModelNormalizationLimits,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::normalize::{normalize, normalize_with_meter, NormalizeOutcome};
use quire_checked::key::DeclarationKey;

/// One object type and its direct supertypes, by node name.
struct Node {
    name: String,
    parents: Vec<String>,
}

fn node_key(name: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{name}"))
}

fn package(nodes: &[Node]) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("n01"),
        nodes
            .iter()
            .map(|node| {
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: node_key(&node.name),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: node.parents.iter().map(|parent| node_key(parent)).collect(),
                })
            })
            .collect(),
    )
}

/// `T0` to `T<edges>`, each generalizing the one before.
fn chain(edges: usize, width: usize) -> Vec<Node> {
    (0..=edges)
        .map(|at| Node {
            name: format!("T{at:0width$}"),
            parents: if at == 0 {
                Vec::new()
            } else {
                vec![format!("T{:0width$}", at - 1)]
            },
        })
        .collect()
}

/// A width-2 ladder: layer `i` is `T<i>a`, `T<i>b`, each generalizing both
/// types of layer `i - 1`.
fn ladder(layers: usize, width: usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    for layer in 0..layers {
        for side in ["a", "b"] {
            nodes.push(Node {
                name: format!("T{layer:0width$}{side}"),
                parents: if layer == 0 {
                    Vec::new()
                } else {
                    ["a", "b"]
                        .iter()
                        .map(|parent| format!("T{:0width$}{parent}", layer - 1))
                        .collect()
                },
            });
        }
    }
    nodes
}

/// `count` types, each generalizing the previous `fan` (or fewer).
fn dense(count: usize, fan: usize, width: usize) -> Vec<Node> {
    (0..count)
        .map(|at| Node {
            name: format!("T{at:0width$}"),
            parents: (at.saturating_sub(fan)..at)
                .map(|parent| format!("T{parent:0width$}"))
                .collect(),
        })
        .collect()
}

fn edges(nodes: &[Node]) -> u64 {
    nodes
        .iter()
        .map(|node| u64::try_from(node.parents.len()).expect("count fits"))
        .sum()
}

struct Counters {
    hashed_bytes: u64,
    facts: u64,
    work_units: u64,
    declarations: u64,
}

fn counters(nodes: &[Node]) -> Counters {
    let (outcome, meter) =
        normalize_with_meter(&package(nodes), ModelNormalizationLimits::default());
    match outcome {
        NormalizeOutcome::Completed(_) => {}
        other => panic!("the shape completes at the defaults, got {other:?}"),
    }
    Counters {
        hashed_bytes: meter.consumed(LimitKind::HashedBytes),
        facts: meter.consumed(LimitKind::DerivationFacts),
        work_units: meter.consumed(LimitKind::WorkUnits),
        declarations: meter.consumed(LimitKind::EffectiveDeclarations),
    }
}

fn assert_closed_formulas(nodes: &[Node]) {
    let types = u64::try_from(nodes.len()).expect("count fits");
    let edges = edges(nodes);
    let counters = counters(nodes);
    assert_eq!(counters.declarations, types);
    assert_eq!(counters.facts, types + edges, "types plus edges");
    assert_eq!(
        counters.work_units,
        4 * types + 2 * edges + 2,
        "4V + 2E + 2"
    );
}

/// The published N13 shapes, by their literal fact and work-unit counters.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-11")]
#[test]
fn n13_counters_equal_the_published_figures() {
    let one = counters(&chain(99, 2));
    assert_eq!((one.facts, one.work_units), (199, 600), "chain of 99 edges");
    let two = counters(&ladder(50, 2));
    assert_eq!(
        (two.facts, two.work_units),
        (296, 794),
        "50-layer width-2 ladder, 196 edges"
    );
    let three = counters(&dense(30, 5, 2));
    assert_eq!(
        (three.facts, three.work_units),
        (165, 392),
        "thirty types, each generalizing the previous five"
    );
}

/// TC-1024 steps 1 to 4: chains, a 500-layer ladder whose paths are
/// exponential, and 1,000 types with ten parents each.
#[trace("QSpec-TC-1024", "QSpec-FR-081-AC-11")]
#[test]
fn cost_follows_closed_formulas() {
    for edges in [10, 100, 1000] {
        assert_closed_formulas(&chain(edges, 1));
    }
    assert_closed_formulas(&ladder(500, 1));
    assert_closed_formulas(&dense(1000, 10, 1));
}

/// Hashed bytes grow linearly along a chain: every added edge adds the same
/// number of bytes, so the cost has no hidden quadratic term.
#[trace("QSpec-TC-1024", "QSpec-NFR-012")]
#[test]
fn hashed_bytes_grow_linearly_along_a_chain() {
    let bytes = |edges: usize| counters(&chain(edges, 4)).hashed_bytes;
    let (a, b, c) = (bytes(100), bytes(200), bytes(300));
    assert_eq!(b - a, c - b, "equal added edges add equal bytes");
}

/// N12 as TC-1024 step 5 states it: 13 effective declarations, 24 retained
/// facts and 76 work units (8 records, 24 facts, 4 cycle-checks, 13
/// declarations, 15 hashes, 7 of redefinition-check and 5 of conflict-check).
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-11")]
#[test]
fn n12_phase_four_charges_follow_value_accounting() {
    use qsl_semantics::model::domain_package::{FieldMemberRecord, Multiplicity, ValueTypeRef};
    let field = |name: &str, owner: &str, redefines: Option<&str>| {
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: node_key(name),
            owner: node_key(owner),
            value_type: ValueTypeRef::Package(node_key("A")),
            multiplicity: Multiplicity {
                lower: 0,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            presence: quire_exact::Presence::Required,
            subsets: Vec::new(),
            redefines: redefines.map(node_key),
            position: 0,
        })
    };
    let object = |name: &str, parents: &[&str]| {
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: node_key(name),
            interface_features: None,
            abstract_type: false,
            supertypes: parents.iter().map(|parent| node_key(parent)).collect(),
        })
    };
    let package = DomainPackage::new(
        DomainPackageRef::fixture("n02"),
        vec![
            object("A", &[]),
            object("B", &["A"]),
            object("C", &["A"]),
            object("D", &["B", "C"]),
            field("A/x", "A", None),
            field("B/x2", "B", Some("A/x")),
            field("C/x3", "C", None),
            field("D/x4", "D", Some("A/x")),
        ],
    );
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    assert_eq!(meter.consumed(LimitKind::EffectiveDeclarations), 13);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 24);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 76);
}

/// A shape where every type after the first 50 has 50 parents, over 10,001
/// types, needs about 313 million hashed bytes and is refused at
/// `hashed_bytes` before the denied preimage is hashed.
#[trace("QSpec-TC-1024", "QSpec-NFR-012")]
#[test]
fn fifty_parents_per_type_is_refused_at_hashed_bytes() {
    let nodes = dense(10_001, 50, 5);
    match normalize(&package(&nodes), ModelNormalizationLimits::default()) {
        NormalizeOutcome::Incomplete(Incomplete {
            limit_kind,
            limit,
            charge_point,
            ..
        }) => {
            assert_eq!(limit_kind, LimitKind::HashedBytes);
            assert_eq!(limit, 268_435_456);
            assert_eq!(charge_point, ChargePoint::NormalizeHash);
        }
        other => panic!("expected incomplete at hashed_bytes, got {other:?}"),
    }
}
