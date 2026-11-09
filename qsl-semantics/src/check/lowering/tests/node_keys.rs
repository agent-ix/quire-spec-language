// SPDX-License-Identifier: AGPL-3.0-or-later
//! Producer-retained expression identities read by execution backends.

use super::*;
use crate::check::ir::CheckedNode;

fn source(text: &str) -> CheckedGraph {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{text}"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("node-keys", "unit"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("the source forms build");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the source assembles")
    .check(CheckingLimits::default())
    .expect("the source checks")
}

fn graph_body(graph: &CheckedGraph, name: &str) -> NodeKey {
    graph
        .semantic_graph()
        .node(graph.function_identity(name).expect("declared function"))
        .expect("the graph retains the function")
        .function_body()
        .expect("the body is a graph reference")
}

fn children(node: CheckedNode<'_>) -> [CheckedNode<'_>; 2] {
    node.children().try_into().expect("a binary operation")
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn checked_operation_keys_are_the_graphs_shared_structural_identities() {
    let graph = source(
        "function w using v(a: Integer, b: Integer): Integer pure { (a + b) * (a + b) }\n\
         function other using v(a: Integer, b: Integer): Integer pure { a * b }",
    );
    let root = graph.function_state(0).expect("w").body;
    let [left, right] = children(root);
    let root_key = graph_body(&graph, "w");
    assert_eq!(root.node_key(), Some(&root_key));
    let application = graph
        .semantic_graph()
        .node(root_key.clone())
        .expect("the multiplication exists")
        .body()
        .application_term()
        .expect("the body is an application");
    let targets: Vec<_> = application
        .arguments
        .iter()
        .map(|argument| {
            let MemberTerm::Leaf(leaf) = argument else {
                panic!("an arithmetic operand is a leaf")
            };
            leaf.reference_target()
                .and_then(NodeRef::admitted)
                .expect("the addition is a graph node")
        })
        .collect();
    assert_eq!(targets.len(), 2);
    assert_eq!(left.node_key(), Some(targets[0]));
    assert_eq!(right.node_key(), Some(targets[1]));
    assert_eq!(
        left.node_key(),
        right.node_key(),
        "equal additions share a node"
    );
    assert_ne!(
        left.location(),
        right.location(),
        "source sites stay distinct"
    );
    assert_ne!(root.node_key(), left.node_key());
    let other = graph_body(&graph, "other");
    assert_eq!(
        graph.function_state(1).expect("other").body.node_key(),
        Some(&other)
    );
    assert_ne!(other, root_key);
    assert_ne!(Some(&other), left.node_key());

    let mut relocated = root.body().clone();
    assert_eq!(
        &relocated,
        root.body(),
        "key retention preserves body equality"
    );
    let location = root.location().clone();
    relocated.relocate(|_| Some(location.clone()));
    let moved = relocated.root();
    let [moved_left, moved_right] = children(moved);
    assert_eq!(moved.location(), moved_left.location());
    assert_eq!(moved_left.location(), moved_right.location());
    assert_eq!(moved.node_key(), root.node_key());
    assert_eq!(moved_left.node_key(), left.node_key());
    assert_eq!(moved_right.node_key(), right.node_key());
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn checked_operation_keys_resolve_recursive_call_drafts() {
    let graph = check(vec![recursive("f", "f")]).expect("the recursive function checks");
    let root = graph.function_state(0).expect("f").body;
    assert_eq!(root.node_key(), Some(&graph_body(&graph, "f")));
    let call = root
        .descendants()
        .into_iter()
        .find(|node| matches!(node.kind(), NodeKind::Call { .. }))
        .expect("the recursive branch calls f");
    let call_key = call
        .node_key()
        .expect("settle retains the recursive call key");
    let application = graph
        .semantic_graph()
        .node(call_key.clone())
        .expect("the recursive call exists in the admitted graph")
        .body()
        .application_term()
        .expect("the recursive call is an application");
    assert_eq!(application.operator, Operator::Call);
    let MemberTerm::Leaf(callee) = &application.arguments[0] else {
        panic!("the first call argument names the callee")
    };
    assert_eq!(callee.key(), graph.function_identity("f"));
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn inline_literals_have_no_expression_key_and_locals_name_their_parameter() {
    let graph = source("function f using v(x: Integer): Integer pure { x + 1 }");
    let root = graph.function_state(0).expect("f").body;
    let [local, literal] = children(root);
    assert!(
        literal.node_key().is_none(),
        "an inline literal is not its type node"
    );
    let parameter = graph
        .semantic_graph()
        .node(graph.function_identity("f").expect("f identity"))
        .expect("f node")
        .function_parameters()
        .expect("f parameters");
    assert_eq!(local.node_key(), Some(&parameter[0]));
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn a_cloned_body_cannot_silently_reuse_keys_from_another_package() {
    let graph = source("function f using v(): Boolean pure { true and false }");
    let body = graph.function_state(0).expect("f").body.body().clone();
    let original = body
        .root()
        .node_key()
        .expect("the admitted body has a key")
        .clone();
    let scope = empty_scope();
    let owner = SourceOwner::new("different", "owner").expect("valid source owner");
    let mut occurrences = OccurrenceMap::default();
    let mut meter = Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new_with_source_owners(
        &scope,
        &owner,
        &std::collections::BTreeMap::new(),
        &[],
        &[],
        scope.types().units().clone(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        "different-package",
    );
    let changed = lowering
        .expression(body.root(), &mut Binders::new(&[], &[]))
        .expect("the closed Boolean expression lowers under the other owner");
    assert_ne!(
        changed.key().as_ref(),
        Some(&original),
        "the second package really assigns a different authoritative key"
    );
    let Err(refusal) = lowering.finish(body.root().location()) else {
        panic!("the body already belongs to its original graph")
    };
    assert!(
        matches!(refusal.cause, CheckCause::InternalFault(ref cause) if matches!(**cause, KeyFault::ExpressionIdentityConflict)),
        "{refusal:?}"
    );
    assert_eq!(
        body.root().node_key(),
        Some(&original),
        "refusal cannot overwrite the admitted identity"
    );
}

/// Count resolution work at the same private mapper used by settlement.
/// The inputs are independent, genuinely checked function bodies; a pending
/// control adds one producer-local placeholder per new body.
fn resolution_visits(functions: usize, pending: bool) -> usize {
    let declarations = (0..functions)
        .map(|index| {
            format!("function f{index} using v(): Boolean pure {{ true and false }}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let graph = source(&declarations);
    let scope = empty_scope();
    let owner = SourceOwner::new("node-keys", "unit").expect("valid source owner");
    let mut occurrences = OccurrenceMap::default();
    let mut meter = Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new_with_source_owners(
        &scope,
        &owner,
        &std::collections::BTreeMap::new(),
        &[],
        &[],
        scope.types().units().clone(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        "fixture",
    );
    let mut visits = 0;
    for index in 0..functions {
        let root = graph.function_state(index).expect("the function exists").body;
        lowering
            .expression(root, &mut Binders::new(&[], &[]))
            .expect("the independent closed expression lowers");
        if pending {
            let placeholder = lowering.placeholder();
            lowering.retain_checked_node(root, &LeafTerm::reference_node(placeholder));
        }
        lowering
            .resolve_checked_node_keys(|reference| {
                visits += 1;
                Ok(match reference {
                    NodeRef::Pending(_) => NodeRef::Admitted(
                        root.node_key().expect("the source supplied its real key").clone(),
                    ),
                    admitted @ NodeRef::Admitted(_) => admitted.clone(),
                })
            })
            .expect("new pending keys resolve");
        let after_new = visits;
        lowering
            .resolve_checked_node_keys(|reference| {
                visits += 1;
                Ok(reference.clone())
            })
            .expect("an empty delta resolves without work");
        assert_eq!(visits, after_new, "a settlement never revisits its old delta");
    }
    assert_eq!(
        lowering.checked_body_keys.values().map(|body| body.nodes.len()).sum::<usize>(),
        functions * if pending { 4 } else { 3 },
        "the control really retains each function's expression references"
    );
    lowering.finish(&generated_location()).expect("resolved keys publish");
    visits
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn settlement_does_not_revisit_admitted_expression_keys() {
    assert_eq!(resolution_visits(16, false), 0);
    assert_eq!(resolution_visits(32, false), 0);
}

#[trace("TC-415", "FR-093-AC-62")]
#[test]
fn settlement_resolves_each_new_pending_expression_reference_once() {
    assert_eq!(resolution_visits(16, true), 16);
    assert_eq!(resolution_visits(32, true), 32);
}
