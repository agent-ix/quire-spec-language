// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-265 (ADR-031 SW-1 to SW-4, SW-9): derive a state clause's settlement
//! basis and, when that basis is decisive, its one separating witness
//! record, from its S6a evaluation. The FR-109 clause run
//! (`spine::check_clause`) and the FR-122 replay
//! (`execute::replay_state_clause`) both call [`derive_separating_witness`],
//! so producer and replay derive the record the same way.

use qsl_foundation::diagnostic::InternalFault;
use qsl_semantics::check::{CheckedStateClause, Connective, Node, NodeKind, Visit};
use qsl_semantics::family::FamilyOutcome;
use quire_exact::{Outcome, Value};

use qsl_eval::value::ClauseEvaluation;

use crate::proof_result::SettlementBasis;
use crate::result::SeparatingWitnessRecord;

/// FR-265's output for a clause that completed a Boolean: the basis, and
/// the record exactly when the basis is decisive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Derived {
    /// `decisive-counterexample`, `decisive-witness` or `closed-scope`.
    pub(crate) basis: SettlementBasis,
    /// The separating witness record, present exactly when `basis` is
    /// decisive.
    pub(crate) record: Option<SeparatingWitnessRecord>,
}

/// FR-265: walk `clause`'s decision path from its claim root over
/// `evaluation`'s recorded decisions (ADR-031 SW-2): into the body of a
/// `let`, the selected branch of an `if`, the operand of `not`, the operand
/// of `and` or `or` on which evaluation stopped (the first `false` operand
/// of a `false` `and`, the first `true` operand of a `true` `or`, otherwise
/// the right operand), and for `implies` the antecedent when it is `false`,
/// otherwise the consequent. The walk ends at the first node of any other
/// kind and descends the claim's tree, so it visits each node at most once.
/// A walk that ends at a `forall` or `exists` with a stop report settles
/// the decisive basis for the clause's truth and builds the record from
/// that report; any other end settles `closed-scope` with no record. The
/// basis follows the clause's truth, not the polarity at the end of the
/// path (ADR-031 SW-3).
///
/// `Ok(None)` when the evaluation completed no Boolean (FR-266 gives that
/// result basis `unavailable`). It reads only the clause, the evaluation's
/// recorded decisions and stop reports, and evaluates nothing.
///
/// # Errors
///
/// An `InternalFault` when a node the walk reaches has no recorded
/// decision: every such node is on the claim's own level and was evaluated,
/// so S6a recorded it.
pub(crate) fn derive_separating_witness(
    clause: &CheckedStateClause,
    evaluation: &ClauseEvaluation,
) -> Result<Option<Derived>, InternalFault> {
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) =
        &evaluation.evaluation.outcome
    else {
        return Ok(None);
    };
    let decision = |node: &Node| {
        evaluation
            .decision(node.location())
            .ok_or_else(|| InternalFault::new("witness", "decision-path-node-recorded"))
    };
    let mut node = clause.body();
    loop {
        node = match node.kind() {
            NodeKind::Let { body, .. } => body,
            NodeKind::If {
                then, otherwise, ..
            } => {
                if decision(node)? {
                    then
                } else {
                    otherwise
                }
            }
            NodeKind::Not(operand) => operand,
            NodeKind::Connective(connective, left, right) => {
                let left_value = decision(node)?;
                let stopped_on_left = match connective {
                    Connective::And | Connective::Implies => !left_value,
                    Connective::Or => left_value,
                };
                if stopped_on_left {
                    left
                } else {
                    right
                }
            }
            NodeKind::Query {
                visit: Visit::Forall | Visit::Exists,
                ..
            } => {
                return Ok(Some(match evaluation.stop(node.location()) {
                    Some(report) => Derived {
                        basis: SettlementBasis::decisive(*truth),
                        record: Some(SeparatingWitnessRecord::from_stop(report)),
                    },
                    None => closed_scope(),
                }))
            }
            // ADR-031 SW-2: "The path ends at the first node of any other
            // kind" -- the rule itself is the catch-all, so a node kind
            // added later ends the walk too.
            _ => return Ok(Some(closed_scope())),
        };
    }
}

fn closed_scope() -> Derived {
    Derived {
        basis: SettlementBasis::ClosedScope,
        record: None,
    }
}
