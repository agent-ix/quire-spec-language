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

use super::argument::{self, Stopped};
use super::{
    arguments, call_failure_to_replay_refusal, consumed, exceeded_before_call, recompile,
    request_limits, select, Claim, ReplayRefusal,
};
use crate::bounds::ReplayLimits;
use crate::identity::ObligationIdentity;
use crate::proof_result::{IncompleteCause, InconclusiveCause, TerminalValue};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{
    evaluated, Evaluated, ScalarAgreement, ScalarClaim, ScalarOutcome, ValueIdentity,
};

/// FR-357: what a value-parity replay found. None of these is a `Refuted`
/// result.
#[derive(Debug)]
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
    /// The request failed before any outcome: FR-098's refusals (a stale
    /// package, an unknown function, an unbound parameter, a `Boolean`
    /// function) and a broken checked invariant.
    Refused(Box<ReplayRefusal>),
}

/// What a value-parity replay settled, with the full claim identity carried
/// through unchanged on every outcome, a refusal included.
#[derive(Debug)]
pub struct ValueParityReport {
    claim: ValueIdentity,
    result: ValueParityResult,
}

impl ValueParityReport {
    /// The claim identity the request and the generated outcome fixed. A
    /// consumer checks `claim()` equals the claim it sent.
    pub fn claim(&self) -> &ValueIdentity {
        &self.claim
    }

    /// What the replay found.
    pub fn result(&self) -> &ValueParityResult {
        &self.result
    }

    /// The terminal value this report settles ([`ValueParityResult::terminal_value`]).
    pub fn terminal_value(&self) -> TerminalValue {
        self.result.terminal_value()
    }
}

impl ValueParityResult {
    /// The terminal value this result settles: `Failed` for a divergence,
    /// `Inconclusive(ScalarAgrees)` for an agreement,
    /// `Inconclusive(ReplayRefused)` with the admission refusal's code for a
    /// refused input, `Incomplete` for a reached limit and the request
    /// refusal's own settlement for a refusal.
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
            Self::Refused(refusal) => TerminalValue::from_replay_refusal(refusal),
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
/// failure before an outcome refuses as FR-098's does
/// ([`ValueParityResult::Refused`]); a function whose declared result is
/// `Boolean` refuses [`ReplayRefusal::NotAValueFunction`]. The report carries
/// the full claim identity on every outcome ([`ValueParityReport::claim`]).
/// `replay_limits` is `replay.input_bytes`, as [`crate::replay`] takes it.
pub fn replay_value_parity(
    wire: ReplayRequestWire,
    generated: ScalarOutcome,
    replay_limits: ReplayLimits,
) -> ValueParityReport {
    let claim = ValueIdentity {
        obligation: ObligationIdentity::from_digest(wire.obligation_identity),
        package_id: wire.package_id.clone(),
        function: wire.selected_function.clone(),
        source: wire.source.clone(),
        limits: wire.effective_accounting_limits(),
        generated,
    };
    let result = match settle(wire, &claim, replay_limits) {
        Ok(result) => result,
        Err(ReplayRefusal::Input(refusal)) => ValueParityResult::RefusedInput(refusal),
        Err(refusal) => ValueParityResult::Refused(Box::new(refusal)),
    };
    ValueParityReport { claim, result }
}

#[deny(clippy::wildcard_enum_match_arm)]
fn settle(
    wire: ReplayRequestWire,
    claim: &ValueIdentity,
    replay_limits: ReplayLimits,
) -> Result<ValueParityResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    let limits = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile(&request, &limits)?;
    let package = compiled.checked.package();
    let call = select(&compiled, request.selected_function(), Claim::ValueParity)?;
    let joined = arguments(
        package,
        &call,
        request.source(),
        request.obligation_identity(),
    )?;
    let limits = request.accounting_limits();
    let converted = match argument::convert_arguments(package, &call.types, &joined, &limits) {
        Ok(converted) => converted,
        Err(Stopped::Refusal(refusal)) => return Err(*refusal),
        Err(Stopped::Limit(incomplete)) => return Ok(ValueParityResult::Incomplete(incomplete)),
    };
    if let Some(incomplete) = exceeded_before_call(&converted, &limits) {
        return Ok(ValueParityResult::Incomplete(Box::new(incomplete)));
    }
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &call.name,
            converted.values,
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
    Ok(if qsl.same_as(&claim.generated) {
        ValueParityResult::Agrees {
            agreement: ScalarAgreement::new(ScalarClaim::Function(Box::new(claim.clone())), qsl),
            charges,
        }
    } else {
        ValueParityResult::Diverged {
            qsl,
            generated: claim.generated.clone(),
            charges,
        }
    })
}
