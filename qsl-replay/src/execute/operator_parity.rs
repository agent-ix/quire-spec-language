// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: [`replay_operator_parity`], the replay facade's entry for an
//! operator-level scalar-parity claim.
//!
//! The request carries the original proving context: the package's
//! `package_id`, sources and byte provision, as the call site of the
//! enclosing function (which may be non-`Boolean`) named them. The replay
//! recompiles that source by FR-098's rules, requires its `package_id`, and
//! requires the claim's scalar node to lie in the enclosing function's body
//! and to be an application of the claim's operator. It never selects a function: no synthetic predicate or
//! function obligation stands in for the operator.

use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{NodeTag, SemanticGraph, SemanticNode};
use qsl_semantics::library::PackageId;
use quire_exact::NodeKey;

use super::{lookup, recompile, ReplayRefusal};
use crate::identity::{ObligationIdentity, QualifiedName};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{
    compare, OperatorClaim, OperatorParityReport, OperatorParityResult, ScalarOperator,
};

/// FR-357's `stale_dependency`/`revision-mismatch`: a claim identity that is
/// not the recompiled package's.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ScalarIdentityMismatch {
    /// The claim's scalar node is no node of the recompiled package.
    #[error("the claim names scalar node {node} but package {} holds no such node", .package.hex())]
    Node {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The recompiled package.
        package: PackageId,
    },
    /// The claim's scalar node is no node of the body of the function the
    /// request selects as the enclosing function.
    #[error("scalar node {node} is not in the body of function {function} of package {}", .package.hex())]
    Function {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The request's enclosing function.
        function: QualifiedName,
        /// The recompiled package.
        package: PackageId,
    },
    /// The claim's scalar node is not an application of the claim's
    /// operator.
    #[error("scalar node {node} of package {} is no {operator:?} application", .package.hex())]
    Operator {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The claim's operator.
        operator: ScalarOperator,
        /// The recompiled package.
        package: PackageId,
    },
}

/// FR-357: replay an operator-level claim.
///
/// `wire` is the original proving context: its `package_id`, package
/// reference, dependencies and byte provision are the call site's, its
/// `selected_function` names the enclosing function (an undeclared name
/// refuses [`ReplayRefusal::UnknownFunction`], and the scalar node must lie in
/// that function's body), its
/// `obligation_identity` is the obligation CG minted, and its `source` is
/// not read. `claim` carries the scalar node, the operator and operands, the
/// result range, the limits and the generated outcome. The returned report
/// carries the obligation identity unchanged on every outcome, a refusal of
/// the context included.
pub fn replay_operator_parity(
    wire: ReplayRequestWire,
    claim: OperatorClaim,
) -> OperatorParityReport {
    let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
    let request = match ReplayRequest::decode(wire) {
        Ok(request) => request,
        Err(refusal) => {
            return OperatorParityReport::new(
                obligation,
                OperatorParityResult::Refused(Box::new(refusal.into())),
            );
        }
    };
    let result = match settle(&request, claim) {
        Ok(result) => result,
        Err(refusal) => OperatorParityResult::Refused(Box::new(refusal)),
    };
    OperatorParityReport::new(obligation, result)
}

fn settle(
    request: &ReplayRequest,
    claim: OperatorClaim,
) -> Result<OperatorParityResult, ReplayRefusal> {
    let compiled = recompile(request)?;
    let package = compiled.emitted.package().package_id();
    let graph = compiled.checked.package().graph().semantic_graph();
    let operator = claim.operation.operator();
    let node = graph
        .resolve_wire(claim.node)
        .and_then(|key| graph.node(key))
        .ok_or_else(|| {
            ReplayRefusal::ScalarIdentity(Box::new(ScalarIdentityMismatch::Node {
                node: claim.node,
                package,
            }))
        })?;
    if !applies(node, operator) {
        return Err(ReplayRefusal::ScalarIdentity(Box::new(
            ScalarIdentityMismatch::Operator {
                node: claim.node,
                operator,
                package,
            },
        )));
    }
    let (_, function) = lookup(&compiled, request.selected_function())?;
    if !in_body(graph, function.identity, node.key()) {
        return Err(ReplayRefusal::ScalarIdentity(Box::new(
            ScalarIdentityMismatch::Function {
                node: claim.node,
                function: request.selected_function().clone(),
                package,
            },
        )));
    }
    compare(request.obligation_identity(), claim).map_err(ReplayRefusal::Fault)
}

/// Whether `target` is `function`'s node or one its body reaches. The walk
/// follows every node reference of a node's preimage, and stops at another
/// function's node: a call to another function is not that function's body.
#[qsl_attrs::string_edge]
fn in_body(graph: &SemanticGraph, function: NodeKey, target: NodeKey) -> bool {
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
fn applies(node: &SemanticNode, operator: ScalarOperator) -> bool {
    serde_json::from_slice::<serde_json::Value>(node.preimage())
        .ok()
        .is_some_and(|preimage| {
            preimage["body"]["operation"]["identity"] == operator.catalog_identity()
        })
}
