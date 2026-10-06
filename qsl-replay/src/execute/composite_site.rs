// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-358 steps 2 and 3: where a claimed equality node sits in the
//! recompiled package, and what its two operands are.
//!
//! Each operand is a parameter reference or a literal. The node is found in
//! the selected function's checked body by the source location its
//! occurrence records, so the operands' kinds and a literal's value come from
//! the checked body itself: a literal operand's value is the closed
//! subexpression, evaluated.

use qsl_eval::value::evaluate_closed;
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{CheckedNode, NodeKind};
use qsl_semantics::family::FamilyOutcome;
use quire_exact::Origin as BodyKey;
use quire_exact::{Meter, Outcome, Value, ValueType};
use quire_semantic_value::declaration::EqualityOperator;
use quire_semantic_value::location::Origin as BodyOrigin;

use super::operator_parity::ScalarIdentityMismatch;
use super::scalar_site::{equality_operator, in_body};
use super::{callable_parameter_keys, lookup, wire_id, Recompiled, ReplayRefusal};
use crate::identity::QualifiedName;

/// One operand of the claimed equality.
pub(super) enum Operand {
    /// A reference to a parameter of the enclosing function.
    Parameter {
        /// The parameter's node id.
        node: WireNodeId,
        /// The parameter's declared type.
        value_type: ValueType,
    },
    /// A literal: its type and its value in the recompiled package.
    Literal {
        /// The operand's checked type.
        value_type: ValueType,
        /// The literal's value.
        value: Value,
    },
}

impl Operand {
    /// The operand's declared type: a parameter's declared type, or a
    /// literal's checked type.
    pub(super) fn value_type(&self) -> &ValueType {
        match self {
            Self::Parameter { value_type, .. } | Self::Literal { value_type, .. } => value_type,
        }
    }
}

/// The claimed node: its occurrence key and its two operands.
pub(super) struct Site {
    pub(super) occurrence: OccurrenceKey,
    pub(super) operands: [Operand; 2],
}

fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay", invariant))
}

/// Locate the equality `operator` at `node` in the body of `function`
/// (FR-358 step 2) and read its operands (step 3).
pub(super) fn locate(
    compiled: &Recompiled,
    function: &QualifiedName,
    node: WireNodeId,
    occurrence: &BodyKey,
    operator: EqualityOperator,
) -> Result<Site, ReplayRefusal> {
    let package_id = compiled.emitted.package().package_id();
    let mismatch = |cause: ScalarIdentityMismatch| ReplayRefusal::ScalarIdentity(Box::new(cause));
    let (segment, callable) = lookup(compiled, function)?;
    let package = compiled.checked.package();
    let graph = package.graph().semantic_graph();
    let semantic = graph
        .resolve_wire(node)
        .and_then(|key| graph.node(key))
        .ok_or_else(|| {
            mismatch(ScalarIdentityMismatch::Node {
                node,
                package: package_id,
            })
        })?;
    if !in_body(graph, callable.identity, semantic.key()) {
        return Err(mismatch(ScalarIdentityMismatch::Function {
            node,
            function: function.clone(),
            package: package_id,
        }));
    }
    if equality_operator(semantic) != Some(operator) {
        return Err(mismatch(ScalarIdentityMismatch::Equality {
            node,
            operator,
            package: package_id,
        }));
    }
    // The claimed occurrence must be one of this node in this function's
    // body: a node shared by two occurrences is two obligations.
    let refused = || {
        mismatch(ScalarIdentityMismatch::Occurrence {
            node,
            occurrence: occurrence.clone(),
        })
    };
    let location = package
        .graph()
        .occurrence(semantic.key(), occurrence)
        .filter(|location| match &location.origin {
            BodyOrigin::Body { function, .. } => function == segment.as_str(),
            BodyOrigin::Measure { .. }
            | BodyOrigin::Expression
            | BodyOrigin::TypeDeclaration { .. }
            | BodyOrigin::StateClause { .. }
            | BodyOrigin::ProtocolAttempt { .. } => false,
        })
        .cloned()
        .ok_or_else(refused)?;
    let state = package
        .graph()
        .function_by_identity(callable.identity)
        .ok_or_else(|| fault("function-identity-has-a-checked-body"))?;
    let (left, right) = equality_operands(state.body, &location)
        .ok_or_else(|| fault("claimed-node-has-a-checked-twin"))?;
    let parameters = callable_parameter_keys(package, &callable).map_err(ReplayRefusal::Fault)?;
    let operand = |position: usize, checked: CheckedNode<'_>| -> Result<Operand, ReplayRefusal> {
        let refused = || {
            mismatch(ScalarIdentityMismatch::Operand {
                node,
                position,
                package: package_id,
            })
        };
        if let NodeKind::Local(slot) = checked.kind() {
            let (_, value_type) = callable.parameters.get(*slot).ok_or_else(refused)?;
            let key = parameters
                .get(*slot)
                .ok_or_else(|| fault("parameter-slot-has-a-node"))?;
            return Ok(Operand::Parameter {
                node: wire_id(*key),
                value_type: value_type.clone(),
            });
        }
        if reads_a_local(checked) {
            return Err(refused());
        }
        let mut meter = Meter::new(super::argument::UNLIMITED);
        let evaluation = evaluate_closed(package, checked, state.slots, &mut meter)
            .map_err(ReplayRefusal::Fault)?;
        let FamilyOutcome::Evaluated(Outcome::Completed(value)) = evaluation.outcome else {
            return Err(fault("literal-operand-evaluates-to-a-value"));
        };
        Ok(Operand::Literal {
            value_type: checked.value_type().clone(),
            value,
        })
    };
    Ok(Site {
        occurrence: OccurrenceKey::new(node, occurrence.clone()),
        operands: [operand(0, left)?, operand(1, right)?],
    })
}

/// The two operand nodes of the equality at `location` in `body`, found by a
/// walk on an explicit stack.
fn equality_operands<'a>(
    body: CheckedNode<'a>,
    location: &quire_semantic_value::location::Location,
) -> Option<(CheckedNode<'a>, CheckedNode<'a>)> {
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        if node.location() == location {
            if let NodeKind::Equality(_, _, left, right) = node.kind() {
                return Some((node.at(*left), node.at(*right)));
            }
        }
        pending.extend(node.children());
    }
    None
}

/// Whether the subexpression at `node` reads a local, so it is no closed
/// literal.
fn reads_a_local(node: CheckedNode<'_>) -> bool {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let NodeKind::Local(_) = node.kind() {
            return true;
        }
        pending.extend(node.children());
    }
    false
}
