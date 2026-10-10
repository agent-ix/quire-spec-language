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
use qsl_semantics::library::PackageId;
use quire_exact::Origin;
use quire_semantic_value::declaration::EqualityOperator;

use super::parity_identity::{
    parity_obligation, Domain, IdentityEncodeError, ParityArgument, ParityPreimage,
};
use super::scalar_site::{applies, check_node_operands, in_body};
use super::{lookup, recompile, request_limits, ReplayRefusal};
use crate::bounds::ReplayLimits;
use crate::identity::{ObligationIdentity, QualifiedName};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{
    compare, OperatorClaim, OperatorIdentity, OperatorParityReport, OperatorParityResult,
    ScalarOperator,
};

/// FR-357's `stale_dependency`/`content-mismatch`: a claim identity that is
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
    /// The request's obligation identity is not the one ADR-013 O-09's
    /// operator-parity preimage gives for the claim.
    #[error("the request's obligation identity {claimed} is not the {recomputed} the claim's preimage gives")]
    Obligation {
        /// The request's obligation identity.
        claimed: ObligationIdentity,
        /// The identity recomputed from the claim.
        recomputed: ObligationIdentity,
    },
    /// The request's obligation identity cannot be recomputed: the claim's
    /// preimage is beyond the encoder, so no identity exists to compare.
    #[error("the claim's obligation preimage cannot be encoded: {0}")]
    Encoding(IdentityEncodeError),
    /// The claim's occurrence key is no occurrence of the scalar node.
    #[error("scalar node {node} has no occurrence {occurrence:?}")]
    Occurrence {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The claimed occurrence key.
        occurrence: Origin,
    },
    /// The claim names a different number of operands than the node applies
    /// its operator to.
    #[error("the claim names {claimed} operands but scalar node {node} has {found} arguments")]
    OperandCount {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The operands the claim names.
        claimed: usize,
        /// The arguments the node has.
        found: usize,
    },
    /// A `GraphChild` operand is not the node's child at its position.
    #[error("operand {position} of scalar node {node} is claimed to be {claimed} but is {}", found.map_or_else(|| "no node reference".to_owned(), |found| found.to_string()))]
    OperandChild {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The operand's position.
        position: usize,
        /// The node the claim names.
        claimed: WireNodeId,
        /// The node the argument references, or `None` for an inline term.
        found: Option<WireNodeId>,
    },
    /// An `InlineLiteral` operand is not an inline integer literal of the
    /// node at its position.
    #[error(
        "operand {position} of scalar node {node} is claimed to be an inline literal but is not"
    )]
    NotInlineLiteral {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The operand's position.
        position: usize,
    },
    /// An `InlineLiteral` operand's value or singleton range is not the
    /// literal's.
    #[error("operand {position} of scalar node {node} is the inline literal {found}, not the claimed value and singleton range")]
    LiteralValue {
        /// The claim's scalar node.
        node: WireNodeId,
        /// The operand's position.
        position: usize,
        /// The literal's value in the package.
        found: String,
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
    /// FR-358: the claimed node is not an application of the claimed
    /// equality operator.
    #[error("scalar node {node} of package {} is no {operator:?} equality application", .package.hex())]
    Equality {
        /// The claim's node.
        node: WireNodeId,
        /// The claim's operator.
        operator: EqualityOperator,
        /// The recompiled package.
        package: PackageId,
    },
    /// FR-358: an operand of the claimed equality is neither a parameter
    /// reference nor a literal.
    #[error("operand {position} of scalar node {node} of package {} is neither a parameter nor a literal", .package.hex())]
    Operand {
        /// The claim's node.
        node: WireNodeId,
        /// The operand's position: 0 for the left, 1 for the right.
        position: usize,
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
/// result range, the limits, the generated outcome and the observation
/// digest. The returned report carries the full claim identity unchanged on
/// every outcome, a refusal of the context included
/// ([`OperatorParityReport::claim`]); the observation digest is carried and
/// never recomputed or authenticated. `obligation_identity` is recomputed
/// from the claim's ADR-013 O-09 preimage and a mismatch refuses
/// [`ReplayRefusal::ScalarIdentity`]. `replay_limits` is `replay.input_bytes`,
/// as [`crate::replay`] takes it.
pub fn replay_operator_parity(
    wire: ReplayRequestWire,
    claim: OperatorClaim,
    replay_limits: ReplayLimits,
) -> OperatorParityReport {
    let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
    let identity = claim.identity(obligation);
    let request = match ReplayRequest::decode(wire, replay_limits) {
        Ok(request) => request,
        Err(refusal) => {
            return OperatorParityReport::new(
                identity,
                OperatorParityResult::Refused(Box::new(refusal.into())),
            );
        }
    };
    let result = match settle(&request, &identity, replay_limits) {
        Ok(result) => result,
        Err(refusal) => OperatorParityResult::Refused(Box::new(refusal)),
    };
    OperatorParityReport::new(identity, result)
}

/// The identity ADR-013 O-09's preimage gives `claim`, which must be the
/// request's `claimed` one. A preimage the encoder refuses gives no identity,
/// so it refuses whatever `claimed` is.
pub(super) fn verify_obligation(
    claim: &OperatorIdentity,
    claimed: ObligationIdentity,
) -> Result<(), ScalarIdentityMismatch> {
    let recomputed =
        parity_obligation(&preimage_of(claim)).map_err(ScalarIdentityMismatch::Encoding)?;
    if recomputed == claimed {
        Ok(())
    } else {
        Err(ScalarIdentityMismatch::Obligation {
            claimed,
            recomputed,
        })
    }
}

/// The O-09 preimage of an operator claim: one range-domain argument per
/// operand.
pub(super) fn preimage_of(claim: &OperatorIdentity) -> ParityPreimage {
    let (first, second) = claim.operation.operands();
    ParityPreimage {
        node: claim.node,
        occurrence: claim.occurrence.clone(),
        obligation_kind: claim.obligation_kind.clone(),
        arguments: std::iter::once(first)
            .chain(second)
            .map(|operand| ParityArgument {
                identity: operand.identity,
                domain: Domain::Range(operand.range.clone()),
            })
            .collect(),
    }
}

fn settle(
    request: &ReplayRequest,
    claim: &OperatorIdentity,
    replay_limits: ReplayLimits,
) -> Result<OperatorParityResult, ReplayRefusal> {
    let limits = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile(request, &limits)?;
    let package = compiled.emitted.package().package_id();
    let checked_graph = compiled.checked.package().graph();
    let graph = checked_graph.semantic_graph();
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
    if checked_graph
        .occurrence(node.key(), &claim.occurrence)
        .is_none()
    {
        return Err(ReplayRefusal::ScalarIdentity(Box::new(
            ScalarIdentityMismatch::Occurrence {
                node: claim.node,
                occurrence: claim.occurrence.clone(),
            },
        )));
    }
    check_node_operands(node, claim.node, &claim.operation)?;
    verify_obligation(claim, request.obligation_identity())
        .map_err(|mismatch| ReplayRefusal::ScalarIdentity(Box::new(mismatch)))?;
    compare(claim).map_err(ReplayRefusal::Fault)
}
