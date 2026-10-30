// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-319 and FR-320 over real complete-V1 source: the union type node, the
//! construction node and the `case` node that `PackageDeclarations::check`
//! lowers and keys, and the member identities it seals each union with.
//! Every id compared here is read from the checked graph the public `check`
//! returns; none is minted by the test.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_exact::{NodeKey, VariantId};

use super::*;
use crate::check::{CheckedGraph, SemanticNode};

const SHAPE_DECL: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n";

/// FR-318-AC-1's `area`, its arms in member order.
const AREA: &str = "function area using v(s: Shape): Integer pure \
     { case s { Empty: 0; Circle(r): r; Rect(w, h): w * h; } }\n";

/// A function returning `Shape::Rect(2, 3)`.
const RECT: &str = "function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

/// `declarations` through S1, S2, the assembler and S3.
fn graph(declarations: &str) -> CheckedGraph {
    unit(declarations)
        .package
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"))
}

/// Every id FR-319-AC-1 holds fixed across compiles of one unit.
#[derive(Debug, Eq, PartialEq)]
struct Ids {
    /// The union's node id.
    union: NodeKey,
    /// Each member's name and `VariantId`, in declared order.
    variants: Vec<(String, VariantId)>,
    /// The `case` node ids.
    cases: Vec<NodeKey>,
    /// The `union_value` node ids.
    values: Vec<NodeKey>,
}

/// The declaration handle of the union `name` in `graph`'s scope.
fn handle(graph: &CheckedGraph, name: &str) -> NodeKey {
    graph
        .scope()
        .types()
        .composites()
        .find(|declaration| declaration.name() == name)
        .unwrap_or_else(|| panic!("the unit declares {name}"))
        .key()
}

/// The nodes of `graph` with `semantic_form`, ascending by id.
fn forms<'g>(graph: &'g CheckedGraph, semantic_form: &str) -> Vec<&'g SemanticNode> {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == semantic_form)
        .collect()
}

/// The ids of `graph`'s union `Shape`, each member's `VariantId` read from
/// the sealed type environment.
fn ids(graph: &CheckedGraph) -> Ids {
    let types = graph.scope().types();
    let handle = handle(graph, "Shape");
    let union = types
        .union_key(handle.clone())
        .expect("check sealed the union under its node id");
    let CompositeShape::Union(members) = types
        .composite(handle)
        .expect("the declaration is retained")
        .shape()
    else {
        panic!("Shape is a union")
    };
    let variants = members
        .iter()
        .map(|member| {
            (
                member.identifier().to_owned(),
                member
                    .member()
                    .expect("check bound every member to its FR-441 key")
                    .variant()
                    .clone(),
            )
        })
        .collect();
    Ids {
        union,
        variants,
        cases: forms(graph, "case").iter().map(|n| n.key()).collect(),
        values: forms(graph, "union_value")
            .iter()
            .map(|n| n.key())
            .collect(),
    }
}

/// The keys `node` names: its semantic type, the nodes its body references
/// and its application's result type.
fn names(node: &SemanticNode) -> Vec<NodeKey> {
    let result = node
        .body()
        .application_term()
        .and_then(|application| application.result_type.admitted().cloned());
    node.semantic_type()
        .into_iter()
        .chain(result)
        .chain(
            node.body()
                .leaves()
                .into_iter()
                .filter_map(|leaf| leaf.key()),
        )
        .collect()
}

/// The keys of every node of `graph` that names `target`, directly or
/// through the nodes that name it.
fn naming(graph: &CheckedGraph, target: NodeKey) -> BTreeSet<NodeKey> {
    let mut named = BTreeSet::from([target.clone()]);
    loop {
        let before = named.len();
        for node in graph.semantic_graph().nodes() {
            if names(node).iter().any(|key| named.contains(key)) {
                named.insert(node.key());
            }
        }
        if named.len() == before {
            named.remove(&target);
            return named;
        }
    }
}

#[trace("FR-319-AC-1", "TC-826")]
#[test]
fn compiling_twice_and_changing_only_trivia_and_arm_order_keeps_every_id() {
    let base = ids(&graph(&format!("{SHAPE_DECL}{AREA}{RECT}")));
    assert_eq!(base.cases.len(), 1);
    assert_eq!(base.values.len(), 1);
    assert_eq!(
        ids(&graph(&format!("{SHAPE_DECL}{AREA}{RECT}"))),
        base,
        "a second compile"
    );
    let changed = "// shapes\n\
         union   Shape {\n    Empty,\n    Circle( Integer ),   // round\n    \
         Rect( Integer,  Integer ),\n}\n\n\
         function area using v(s: Shape): Integer pure {\n    case s {\n        \
         Rect(w, h): w * h;   // two sides\n        Empty: 0;\n        \
         Circle(r): r;\n    }\n}\n\n\
         // a rectangle\n\
         function rect using v(): Shape pure { Shape::Rect(2,   3) }\n";
    assert_eq!(
        ids(&graph(changed)),
        base,
        "trivia changed and the arms reordered"
    );
}

#[trace("FR-319-AC-1", "TC-826")]
#[test]
fn renaming_adding_or_retyping_a_member_changes_the_union_and_every_node_naming_it() {
    let base = graph(&format!("{SHAPE_DECL}{AREA}{RECT}"));
    let base_ids = ids(&base);
    let naming_shape = naming(&base, base_ids.union.clone());
    // The parameter, the `case`, `area`, the construction and `rect` name it.
    assert!(naming_shape.len() >= 5, "{naming_shape:?}");
    assert!(base_ids
        .cases
        .iter()
        .all(|case| naming_shape.contains(case)));
    assert!(base_ids
        .values
        .iter()
        .all(|value| naming_shape.contains(value)));

    let area = |arms: &str| {
        format!("function area using v(s: Shape): Integer pure {{ case s {{ {arms} }} }}\n")
    };
    for (label, union, area) in [
        (
            "renamed member",
            "union Shape { Nothing, Circle(Integer), Rect(Integer, Integer) }\n",
            area("Nothing: 0; Circle(r): r; Rect(w, h): w * h;"),
        ),
        (
            "added member",
            "union Shape { Empty, Circle(Integer), Rect(Integer, Integer), Line(Integer) }\n",
            area("Empty: 0; Circle(r): r; Rect(w, h): w * h; Line(n): n;"),
        ),
        (
            "changed payload type",
            "union Shape { Empty, Circle(Boolean), Rect(Integer, Integer) }\n",
            area("Empty: 0; Circle(r): 1; Rect(w, h): w * h;"),
        ),
    ] {
        let changed = graph(&format!("{union}{area}{RECT}"));
        let changed_ids = ids(&changed);
        assert_ne!(changed_ids.union, base_ids.union, "{label}: union id");
        assert_ne!(changed_ids.cases, base_ids.cases, "{label}: case id");
        let kept: BTreeSet<NodeKey> = changed
            .semantic_graph()
            .nodes()
            .map(SemanticNode::key)
            .collect();
        for node in &naming_shape {
            assert!(
                !kept.contains(node),
                "{label}: {node} names the union and survived"
            );
        }
    }
}

#[trace("FR-319-AC-2", "TC-826")]
#[test]
fn a_checked_member_variant_id_is_its_member_over_the_union_node() {
    let graph = graph(&format!("{SHAPE_DECL}{AREA}{RECT}"));
    let ids = ids(&graph);
    let names: Vec<&str> = ids.variants.iter().map(|(member, _)| member.as_str()).collect();
    assert_eq!(names, ["Empty", "Circle", "Rect"]);
    // Each checked `VariantId` is the member's id over the union's node id,
    // and the index the check built resolves it to its position.
    let types = graph.scope().types();
    let handle = handle(&graph, "Shape");
    for (position, (member, variant)) in ids.variants.iter().enumerate() {
        assert_eq!(
            variant,
            &mint_union_variant_id(ids.union.clone(), member).expect("a member name"),
            "{member}"
        );
        let (found, _) = types
            .union_member(handle.clone(), variant.clone())
            .expect("the sealed union resolves its member");
        assert_eq!(found, position);
    }
}

#[trace("FR-320-AC-1", "TC-827")]
#[test]
fn a_recursive_union_is_one_recursion_group_and_seals() {
    let graph = graph(
        "union List { Cons(Integer, Option<List>), Nil }\n\
         function same using v(a: List, b: List): Boolean pure { a = b }\n\
         function nil using v(): List pure { List::Nil }\n",
    );
    let [list] = forms(&graph, "union")[..] else {
        panic!("one union node")
    };
    assert!(list.recursion().is_some(), "List reaches itself");
    let types = graph.scope().types();
    assert_eq!(
        types.union_key(handle(&graph, "List")),
        Some(list.key()),
        "sealed under the node id of its group member"
    );
    // `quire.op.structural.eq` over a union that reaches no text has no leaves.
    let equality: Vec<usize> = graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| node.body().application_term())
        .filter(|application| application.operation.identity() == "quire.op.structural.eq")
        .map(|application| application.operation.leaves().len())
        .collect();
    assert_eq!(equality, [0]);
}
