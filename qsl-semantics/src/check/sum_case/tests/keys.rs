// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-319 and FR-320 over real complete-V1 source: the union type node, the
//! construction node and the `case` node that `PackageDeclarations::check`
//! lowers and keys, and the member identities it seals each union with.
//! Every id compared here is read from the checked graph the public `check`
//! returns; none is minted by the test.

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

#[trace("FR-319-AC-2", "TC-826")]
#[test]
fn a_checked_member_variant_id_is_its_member_over_the_union_node() {
    let graph = graph(&format!("{SHAPE_DECL}{AREA}{RECT}"));
    let ids = ids(&graph);
    let names: Vec<&str> = ids
        .variants
        .iter()
        .map(|(member, _)| member.as_str())
        .collect();
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
