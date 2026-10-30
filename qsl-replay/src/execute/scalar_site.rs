// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: where a scalar node sits in a recompiled package. It reads the
//! checked nodes' preimages as JSON values to find the operation a node
//! applies and the nodes a function's body reaches; it encodes nothing and
//! computes no digest.

use std::borrow::Borrow;

use qsl_semantics::check::{
    ApplicationTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag, SemanticGraph, SemanticNode,
};
use quire_exact::NodeKey;

use quire_semantic_value::catalog::InternalFault;

use super::operator_parity::ScalarIdentityMismatch;
use super::ReplayRefusal;
use crate::scalar::{OperandIdentity, ScalarOperand, ScalarOperation, ScalarOperator};
use quire_semantic_value::declaration::EqualityOperator;

/// Whether `target` is `function`'s node or one its body reaches. The walk
/// follows every node a body names through its leaves, and stops at another
/// function's node: a call to another function is not that function's body.
pub(crate) fn in_body(
    graph: &SemanticGraph,
    function: &NodeKey,
    target: impl Borrow<NodeKey>,
) -> bool {
    let target = target.borrow();
    let mut seen = std::collections::BTreeSet::from([function.clone()]);
    let mut pending = vec![function.clone()];
    while let Some(key) = pending.pop() {
        if &key == target {
            return true;
        }
        let Some(node) = graph.node(key.clone()) else {
            continue;
        };
        if &key != function && node.node_tag() == NodeTag::Function {
            continue;
        }
        for referenced in node.body().leaves().into_iter().filter_map(LeafTerm::key) {
            if seen.insert(referenced.clone()) {
                pending.push(referenced);
            }
        }
    }
    false
}

/// Whether `node` is an application of `operator`: its body's operation
/// identity is the operator's catalog identity.
pub(crate) fn applies(node: &SemanticNode, operator: ScalarOperator) -> bool {
    node.body()
        .application_term()
        .is_some_and(|term| term.operation.identity() == operator.catalog_identity())
}

/// [`check_operands`] over `node`'s own body, as the refusal the replay
/// settles.
pub(super) fn check_node_operands(
    node: &SemanticNode,
    application: &NodeKey,
    operation: &ScalarOperation,
) -> Result<(), ReplayRefusal> {
    let term = node.body().application_term().ok_or_else(|| {
        ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "a-checked-application-node-has-an-application-body",
        ))
    })?;
    check_operands(term, application, operation)
        .map_err(|mismatch| ReplayRefusal::ScalarIdentity(Box::new(mismatch)))
}

/// The leaf `argument` is, when it is a bare leaf.
fn leaf_of(argument: &MemberTerm) -> Option<&LeafTerm> {
    match argument {
        MemberTerm::Leaf(leaf) => Some(leaf),
        MemberTerm::Group(_) | MemberTerm::Binding(_) => None,
    }
}

/// The node a leaf references, when it is a reference.
fn referenced(leaf: &LeafTerm) -> Option<NodeKey> {
    match leaf {
        LeafTerm::Reference { target } => target.admitted().cloned(),
        LeafTerm::Literal { .. } | LeafTerm::DependencyReference { .. } => None,
    }
}

/// The integer an inline integer literal leaf holds.
fn integer_literal(leaf: &LeafTerm) -> Option<String> {
    match leaf {
        LeafTerm::Literal {
            value: LiteralValue::Integer(value),
            ..
        } => Some(value.to_string()),
        _ => None,
    }
}

/// Whether `term`, an application node's body, carries the operands the
/// claim names, position by position (FR-357-AC-17): the same number of
/// arguments; for a `GraphChild` the argument is a reference to that node;
/// for an `InlineLiteral` the argument is an inline integer literal, whose
/// value is the operand's value and whose singleton range is the operand's.
pub(crate) fn check_operands(
    term: &ApplicationTerm,
    application: &NodeKey,
    operation: &ScalarOperation,
) -> Result<(), ScalarIdentityMismatch> {
    let (first, second) = operation.operands();
    let claimed: Vec<&ScalarOperand> = std::iter::once(first).chain(second).collect();
    let arguments = &term.arguments;
    if arguments.len() != claimed.len() {
        return Err(ScalarIdentityMismatch::OperandCount {
            node: application.clone(),
            claimed: claimed.len(),
            found: arguments.len(),
        });
    }
    for (position, (operand, argument)) in claimed.into_iter().zip(arguments).enumerate() {
        let leaf = leaf_of(argument);
        match &operand.identity {
            OperandIdentity::GraphChild(node) => {
                let found = leaf.and_then(referenced);
                if found.as_ref() != Some(node) {
                    return Err(ScalarIdentityMismatch::OperandChild {
                        node: application.clone(),
                        position,
                        claimed: node.clone(),
                        found,
                    });
                }
            }
            OperandIdentity::InlineLiteral => {
                let Some(found) = leaf.and_then(integer_literal) else {
                    return Err(ScalarIdentityMismatch::NotInlineLiteral {
                        node: application.clone(),
                        position,
                    });
                };
                if found != operand.value.to_string()
                    || operand.range != ScalarOperand::literal(operand.value.clone()).range
                {
                    return Err(ScalarIdentityMismatch::LiteralValue {
                        node: application.clone(),
                        position,
                        found,
                    });
                }
            }
        }
    }
    Ok(())
}

/// The equality operator `node` applies, when its body's operation is a
/// catalog `quire.op.<family>.eq` or `quire.op.<family>.ne` (QSpec FR-149's
/// `=` and `!=`).
pub(crate) fn equality_operator(node: &SemanticNode) -> Option<EqualityOperator> {
    let identity = node.body().application_term()?.operation.identity();
    let (_, suffix) = identity.strip_prefix("quire.op.")?.rsplit_once('.')?;
    match suffix {
        "eq" => Some(EqualityOperator::Equal),
        "ne" => Some(EqualityOperator::NotEqual),
        _ => None,
    }
}

/// FR-358: the identity of each argument of the application `node`, in
/// position order: a reference is a `GraphChild`, an inline integer literal
/// an `InlineLiteral`. The composite arm names exactly two operands.
pub(super) fn operand_identities(
    node: &SemanticNode,
    application: &NodeKey,
) -> Result<Vec<OperandIdentity>, ScalarIdentityMismatch> {
    let fault = || ScalarIdentityMismatch::OperandCount {
        node: application.clone(),
        claimed: 2,
        found: 0,
    };
    let term = node.body().application_term().ok_or_else(fault)?;
    if term.arguments.len() != 2 {
        return Err(ScalarIdentityMismatch::OperandCount {
            node: application.clone(),
            claimed: 2,
            found: term.arguments.len(),
        });
    }
    term.arguments
        .iter()
        .enumerate()
        .map(|(position, argument)| {
            let leaf = leaf_of(argument);
            if let Some(target) = leaf.and_then(referenced) {
                Ok(OperandIdentity::GraphChild(target))
            } else if leaf.and_then(integer_literal).is_some() {
                Ok(OperandIdentity::InlineLiteral)
            } else {
                Err(ScalarIdentityMismatch::NotInlineLiteral {
                    node: application.clone(),
                    position,
                })
            }
        })
        .collect()
}
