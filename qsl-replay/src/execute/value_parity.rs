// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357: [`replay_value_parity`], the replay facade's entry for a
//! function-level value-parity claim about a non-`Boolean` function `f`.
//!
//! The claim says the generated code agrees with QSL's semantics: at
//! `f`'s declared parameter bindings `b`, the generated outcome equals QSL's
//! exact `f(b)`. It is a claim about the lowering, not about the spec.
//! The replay recompiles the package by FR-098's rules, admits `b` against
//! `f`'s declared signature (S6a), evaluates `f(b)` with the exact
//! evaluator and compares QSL's outcome to the generated one. It never
//! settles a `Refuted` result: that stays with a `Boolean` predicate that is
//! false at the bindings ([`super::replay`]).

use qsl_eval::value::{input_refusal_code, CheckedPackageEvaluation};
use qsl_foundation::diagnostic::InternalFault;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Incomplete, Meter, ScalarLimits};
use quire_semantic_value::call::InputRefusal;

use super::{
    arguments, call_failure_to_replay_refusal, consumed, recompile, request_limits, select, Claim,
    ReplayRefusal,
};
use crate::bounds::ReplayLimits;
use crate::proof_result::{IncompleteCause, InconclusiveCause, TerminalValue};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{evaluated, Evaluated, ScalarAgreement, ScalarClaim, ScalarOutcome};

/// FR-357: what a value-parity replay found. None of these is a `Refuted`
/// result.
#[derive(Clone, Debug)]
pub enum ValueParityResult {
    /// QSL's outcome differs from the generated one: the generated code
    /// does not implement QSL's semantics, a lowering defect and a generator
    /// fault (`TerminalValue::Failed`).
    Diverged {
        /// QSL's outcome at the bindings.
        qsl: ScalarOutcome,
        /// The generated outcome the witness carried.
        generated: ScalarOutcome,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// QSL's outcome equals the generated one: the counterexample does not
    /// reproduce, which is `inconclusive` with
    /// `InconclusiveCause::ScalarAgrees`, a harness defect.
    Agrees {
        /// The claim and the outcome both sides reached.
        agreement: ScalarAgreement,
        /// The accounting charges the evaluation incurred.
        charges: ScalarLimits,
    },
    /// The bindings fail S6a admission against the function's declared
    /// signature (a value of the wrong kind, or outside a parameter's
    /// declared domain). Inputs are refused; no outcome is compared.
    RefusedInput(InputRefusal),
    /// The evaluation reached a limit before it produced an outcome, so
    /// there is nothing to compare.
    Incomplete(Box<Incomplete>),
}

impl ValueParityResult {
    /// The terminal value this result settles: `Failed` for a divergence,
    /// `Inconclusive(ScalarAgrees)` for an agreement,
    /// `Inconclusive(ReplayRefused)` with the admission refusal's code for a
    /// refused input and `Incomplete` for a reached limit.
    pub fn terminal_value(&self) -> TerminalValue {
        match self {
            Self::Diverged { .. } => TerminalValue::Failed,
            Self::Agrees { agreement, .. } => {
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement.clone()))
            }
            Self::RefusedInput(refusal) => TerminalValue::Inconclusive(
                InconclusiveCause::ReplayRefused(input_refusal_code(refusal)),
            ),
            Self::Incomplete(_) => TerminalValue::Incomplete(IncompleteCause::ResourceExhausted),
        }
    }
}

/// FR-357: replay a function-level value-parity claim.
///
/// `wire` names the proved package, the non-`Boolean` function `f` and the
/// bindings of `f`'s declared parameters, and nothing else, in its `source`
/// (FR-098's request, whose `source` is the witness's bindings). `generated`
/// is the generated code's outcome at those bindings. A binding that names no
/// parameter of `f`, a parameter bound twice or not at all, and every other
/// failure before an outcome refuses as FR-098's does (`ReplayRefusal`); a
/// function whose declared result is `Boolean` refuses
/// [`ReplayRefusal::NotAValueFunction`].
pub fn replay_value_parity(
    wire: ReplayRequestWire,
    generated: ScalarOutcome,
) -> Result<ValueParityResult, ReplayRefusal> {
    match settle(wire, generated) {
        Err(ReplayRefusal::Input(refusal)) => Ok(ValueParityResult::RefusedInput(refusal)),
        settled => settled,
    }
}

#[deny(clippy::wildcard_enum_match_arm)]
fn settle(
    wire: ReplayRequestWire,
    generated: ScalarOutcome,
) -> Result<ValueParityResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, ReplayLimits::default())?;
    let limits = request_limits(request.stage_limits(), ReplayLimits::default())?;
    let compiled = recompile(&request, &limits)?;
    let package = compiled.checked.package();
    let call = select(&compiled, request.selected_function(), Claim::ValueParity)?;
    let arguments = arguments(
        package,
        &call,
        request.source(),
        request.obligation_identity(),
    )?;
    let bindings = arguments.clone();
    let mut meter = Meter::new(request.accounting_limits());
    let evaluation = package
        .call(
            &call.name,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .map_err(call_failure_to_replay_refusal)?;
    let outcome = match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        // A function's call is a kernel evaluation; a family-owned result
        // is not one.
        FamilyOutcome::FamilyEvaluated(_) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "function-call-completes-a-kernel-outcome",
            )));
        }
        // See `replay`: the probe variant has no arm of its own.
        #[cfg(seam_probe_replay_downstream)]
        FamilyOutcome::__SeamProbe => {
            unreachable!("never constructed outside the probe build")
        }
    };
    let qsl = match evaluated(outcome).map_err(ReplayRefusal::Fault)? {
        Evaluated::Outcome(qsl) => qsl,
        Evaluated::Incomplete(incomplete) => return Ok(ValueParityResult::Incomplete(incomplete)),
    };
    let charges = consumed(&meter);
    Ok(if qsl.same_as(&generated) {
        ValueParityResult::Agrees {
            agreement: ScalarAgreement::new(
                ScalarClaim::Function {
                    obligation: request.obligation_identity(),
                    function: request.selected_function().clone(),
                    bindings,
                },
                qsl,
            ),
            charges,
        }
    } else {
        ValueParityResult::Diverged {
            qsl,
            generated,
            charges,
        }
    })
}
