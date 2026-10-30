// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1024 and TC-195 N12/N13 (FR-081-AC-11, NFR-012): the bytes admitted to
//! identity hashing, the retained facts and the work units of normalization
//! follow closed formulas that are linear in effective types and direct
//! supertype edges, however many ancestor paths the shape has.
//!
//! The fact and work-unit figures are computed here independently of the
//! encoder under test; byte totals are checked only for linear growth.

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::{LimitKind, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::normalize::{normalize_with_meter, NormalizeOutcome};
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
