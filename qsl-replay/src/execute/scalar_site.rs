// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: where a scalar node sits in a recompiled package. It reads the
//! checked nodes' preimages as JSON values to find the operation a node
//! applies and the nodes a function's body reaches; it encodes nothing and
//! computes no digest.

use qsl_semantics::check::{NodeTag, SemanticGraph, SemanticNode};
use quire_exact::NodeKey;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::WireNodeId;

use super::operator_parity::ScalarIdentityMismatch;
use super::ReplayRefusal;
use crate::scalar::{OperandIdentity, ScalarOperand, ScalarOperation, ScalarOperator};
use quire_semantic_value::declaration::EqualityOperator;

/// Whether `target` is `function`'s node or one its body reaches. The walk
/// follows every node reference of a node's preimage, and stops at another
/// function's node: a call to another function is not that function's body.
#[qsl_attrs::string_edge]
pub(super) fn in_body(graph: &SemanticGraph, function: NodeKey, target: NodeKey) -> bool {
    let keys: std::collections::BTreeMap<String, NodeKey> = graph
        .nodes()
        .map(SemanticNode::key)
        .map(|key| (key.to_string(), key))
        .collect();
    let mut seen = std::collections::BTreeSet::from([function]);
    let mut pending = vec![function];
    while let Some(key) = pending.pop() {
        if key == target {
            return true;
        }
        let Some(node) = graph.node(key) else {
            continue;
        };
        if key != function && node.node_tag() == NodeTag::Function {
            continue;
        }
        let Ok(preimage) = serde_json::from_slice::<serde_json::Value>(node.preimage()) else {
            continue;
        };
        let mut digests = Vec::new();
        node_digests(&preimage, &mut digests);
        for digest in digests {
            let referenced = keys.get(&digest).copied();
            if let Some(referenced) = referenced {
                if seen.insert(referenced) {
                    pending.push(referenced);
                }
            }
        }
    }
    false
}

/// Every `digest` string of a preimage's node ids (`{domain, digest}`).
#[qsl_attrs::string_edge]
fn node_digests(value: &serde_json::Value, out: &mut Vec<String>) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(digest)) = map.get("digest") {
                    out.push(digest.clone());
                }
                pending.extend(map.values());
            }
            serde_json::Value::Array(items) => pending.extend(items),
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }
}

/// Whether `node` is an application of `operator`: its body's operation
/// identity is the operator's catalog identity.
#[qsl_attrs::string_edge]
pub(super) fn applies(node: &SemanticNode, operator: ScalarOperator) -> bool {
    serde_json::from_slice::<serde_json::Value>(node.preimage())
        .ok()
        .is_some_and(|preimage| {
            preimage["body"]["operation"]["identity"] == operator.catalog_identity()
        })
}

/// [`check_operands`] over `node`'s own preimage, as the refusal the replay
/// settles.
pub(super) fn check_node_operands(
    node: &SemanticNode,
    application: WireNodeId,
    operation: &ScalarOperation,
) -> Result<(), ReplayRefusal> {
    let preimage = serde_json::from_slice::<serde_json::Value>(node.preimage()).map_err(|_| {
        ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "a-checked-node-preimage-is-json",
        ))
    })?;
    check_operands(&preimage, application, operation)
        .map_err(|mismatch| ReplayRefusal::ScalarIdentity(Box::new(mismatch)))
}

/// Whether `preimage`, an application node's preimage, carries the operands
/// the claim names, position by position (FR-357-AC-17): the same number of
/// arguments; for a `GraphChild` the argument is a reference to that node;
/// for an `InlineLiteral` the argument is an inline integer literal, whose
/// value is the operand's value and whose singleton range is the operand's.
#[qsl_attrs::string_edge]
pub(super) fn check_operands(
    preimage: &serde_json::Value,
    application: WireNodeId,
    operation: &ScalarOperation,
) -> Result<(), ScalarIdentityMismatch> {
    let (first, second) = operation.operands();
    let claimed: Vec<&ScalarOperand> = std::iter::once(first).chain(second).collect();
    let arguments = preimage["body"]["arguments"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    if arguments.len() != claimed.len() {
        return Err(ScalarIdentityMismatch::OperandCount {
            node: application,
            claimed: claimed.len(),
            found: arguments.len(),
        });
    }
    for (position, (operand, argument)) in claimed.into_iter().zip(arguments).enumerate() {
        match operand.identity {
            OperandIdentity::GraphChild(node) => {
                let found = (argument["term"] == "reference")
                    .then(|| argument["target"]["digest"].as_str())
                    .flatten()
                    .and_then(WireNodeId::from_hex);
                if found != Some(node) {
                    return Err(ScalarIdentityMismatch::OperandChild {
                        node: application,
                        position,
                        claimed: node,
                        found,
                    });
                }
            }
            OperandIdentity::InlineLiteral => {
                if argument["term"] != "literal" || argument["value_kind"] != "integer" {
                    return Err(ScalarIdentityMismatch::NotInlineLiteral {
                        node: application,
                        position,
                    });
                }
                let found = argument["value"].as_str().unwrap_or_default();
                if found != operand.value.to_string()
                    || operand.range != ScalarOperand::literal(operand.value).range
                {
                    return Err(ScalarIdentityMismatch::LiteralValue {
                        node: application,
                        position,
                        found: found.to_owned(),
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
#[qsl_attrs::string_edge]
pub(super) fn equality_operator(node: &SemanticNode) -> Option<EqualityOperator> {
    let preimage = serde_json::from_slice::<serde_json::Value>(node.preimage()).ok()?;
    let identity = preimage["body"]["operation"]["identity"].as_str()?;
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
#[qsl_attrs::string_edge]
pub(super) fn operand_identities(
    node: &SemanticNode,
    application: WireNodeId,
) -> Result<Vec<OperandIdentity>, ScalarIdentityMismatch> {
    let fault = || ScalarIdentityMismatch::OperandCount {
        node: application,
        claimed: 2,
        found: 0,
    };
    let preimage =
        serde_json::from_slice::<serde_json::Value>(node.preimage()).map_err(|_| fault())?;
    let arguments = preimage["body"]["arguments"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    if arguments.len() != 2 {
        return Err(ScalarIdentityMismatch::OperandCount {
            node: application,
            claimed: 2,
            found: arguments.len(),
        });
    }
    arguments
        .iter()
        .enumerate()
        .map(|(position, argument)| {
            if argument["term"] == "reference" {
                argument["target"]["digest"]
                    .as_str()
                    .and_then(WireNodeId::from_hex)
                    .map(OperandIdentity::GraphChild)
                    .ok_or_else(fault)
            } else if argument["term"] == "literal" && argument["value_kind"] == "integer" {
                Ok(OperandIdentity::InlineLiteral)
            } else {
                Err(ScalarIdentityMismatch::NotInlineLiteral {
                    node: application,
                    position,
                })
            }
        })
        .collect()
}
