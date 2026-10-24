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
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Cancel, Incomplete, Meter, ScalarLimits};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::supplied_stop::{SuppliedFailure, SuppliedRun, SuppliedStop};

use super::argument::{self, Stopped};
use super::{
    arguments, call_failure_to_replay_refusal, consumed, declaration_occurrence, function_target,
    recompile, request_limits, select, value_parity_obligation, value_parity_preimage, wire_id,
    Claim, ReplayRefusal, ScalarIdentityMismatch, Selected,
};
use crate::bounds::ReplayLimits;
use crate::identity::QualifiedName;
use crate::proof_result::{IncompleteCause, InconclusiveCause, TerminalValue};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{
    evaluated, Evaluated, ScalarAgreement, ScalarClaim, ScalarOutcome, ValueClaim, ValueIdentity,
};
use crate::target::ReplayEntry;

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
    /// A pre-call limit of the request stopped the arguments' conversion or
    /// admission before the call (FR-277), so there is nothing to compare.
    PreCallIncomplete(Box<SuppliedStop>),
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
            Self::Incomplete(_) | Self::PreCallIncomplete(_) => {
                TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
            }
            Self::Refused(refusal) => TerminalValue::from_replay_refusal(refusal),
        }
    }
}

/// FR-357: replay a function-level value-parity claim.
///
/// `wire` names the proved package, the non-`Boolean` function `f` and the
/// bindings of `f`'s declared parameters, and nothing else, in its `source`
/// (FR-098's request, whose `source` is the witness's bindings).
/// `value_claim` is the driver's observation: the generated outcome at those
/// bindings and the generated-content identity, carried unchanged (FR-484).
/// After the target, recompile, package, `UnknownFunction` and
/// `NotAValueFunction` checks and before the argument join, the request's
/// occurrence key must be `f`'s `declaration` occurrence
/// ([`ReplayRefusal::ClaimOccurrence`]) and its obligation identity the one
/// FR-484's preimage gives ([`ReplayRefusal::ScalarIdentity`]). A binding that names no
/// parameter of `f`, a parameter bound twice or not at all, and every other
/// failure before an outcome refuses as FR-098's does
/// ([`ValueParityResult::Refused`]); a function whose declared result is
/// `Boolean` refuses [`ReplayRefusal::NotAValueFunction`]. The report carries
/// the full claim identity on every outcome ([`ValueParityReport::claim`]).
/// `replay_limits` is `replay.input_bytes`, as [`crate::replay`] takes it.
pub fn replay_value_parity(
    wire: ReplayRequestWire,
    value_claim: ValueClaim,
    replay_limits: ReplayLimits,
) -> ValueParityReport {
    let claim = ValueIdentity::sent(&wire, &value_claim);
    let result = match settle(wire, &claim, replay_limits) {
        Ok(result) => result,
        Err(refusal) => match refusal.input_refusal() {
            Some(input) => ValueParityResult::RefusedInput(input.clone()),
            None => ValueParityResult::Refused(Box::new(refusal)),
        },
    };
    ValueParityReport { claim, result }
}

/// A supplied-value phase stopped before the call: a ceiling is the report's
/// pre-call incomplete, any other stop its own typed refusal.
fn pre_call(failure: SuppliedFailure) -> Result<ValueParityResult, ReplayRefusal> {
    match failure {
        SuppliedFailure::Stop(stop) if stop.limit().is_some() => {
            Ok(ValueParityResult::PreCallIncomplete(stop))
        }
        SuppliedFailure::Stop(stop) => Err(ReplayRefusal::Supplied(stop)),
        SuppliedFailure::Unsupported(stop) => Err(ReplayRefusal::UnsupportedHelper(stop)),
        SuppliedFailure::Fault(fault) => Err(ReplayRefusal::Fault(fault)),
    }
}

/// FR-484: the request's occurrence key is the selected function's one
/// `declaration` occurrence, then the request's obligation identity is the
/// one the recompiled package's preimage gives. The preimage is built from
/// the package alone; the request carries the digest and nothing else of it.
fn check_identity(
    request: &ReplayRequest,
    package: &CheckedPackage,
    call: &Selected,
    function: &QualifiedName,
) -> Result<(), ReplayRefusal> {
    let key = request.replay_target().occurrence_key().ok_or_else(|| {
        ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "function-request-carries-an-occurrence-key",
        ))
    })?;
    if *key != declaration_occurrence(wire_id(call.identity)) {
        return Err(ReplayRefusal::ClaimOccurrence {
            key: key.clone(),
            function: function.clone(),
        });
    }
    let mismatch = |cause| ReplayRefusal::ScalarIdentity(Box::new(cause));
    let preimage = value_parity_preimage(package, call.identity, &call.parameters)
        .map_err(ReplayRefusal::Fault)?;
    let recomputed = value_parity_obligation(&preimage)
        .map_err(|error| mismatch(ScalarIdentityMismatch::Encoding(error)))?;
    let claimed = request.obligation_identity();
    if claimed == recomputed {
        Ok(())
    } else {
        Err(mismatch(ScalarIdentityMismatch::Obligation {
            claimed,
            recomputed,
        }))
    }
}

#[deny(clippy::wildcard_enum_match_arm)]
fn settle(
    wire: ReplayRequestWire,
    claim: &ValueIdentity,
    replay_limits: ReplayLimits,
) -> Result<ValueParityResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    let function = function_target(&request, ReplayEntry::ReplayValueParity)?;
    // FR-484: a value-parity claim has no proving run, so it carries no
    // profile selection; lock rows bind through `package_id`, never read here.
    if !request.profile_selections().is_empty() {
        return Err(ReplayRefusal::UnusedProfileSelections {
            selections: request.profile_selections().to_vec(),
        });
    }
    let limits = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile(&request, &limits)?;
    let package = compiled.checked.package();
    let call = select(&compiled, function, Claim::ValueParity)?;
    check_identity(&request, package, &call, function)?;
    let joined = arguments(
        package,
        &call,
        request.source(),
        request.obligation_identity(),
        None,
    )?;
    let limits = request.accounting_limits();
    let mut run = SuppliedRun::new(request.supplied_limits(), Cancel::new())
        .map_err(ReplayRefusal::Supplied)?;
    let converted = match argument::convert_arguments(
        package,
        &call.types,
        &joined,
        &limits,
        &mut run,
        &compiled.reference_model(),
    ) {
        Ok(converted) => converted,
        Err(Stopped::Refusal(refusal)) => return Err(*refusal),
        Err(Stopped::Supplied(failure)) => return pre_call(failure),
    };
    let mut meter = Meter::new(limits);
    let evaluation = match package.call_supplied(
        &call.name,
        converted.values,
        &ObjectEnvironment::default(),
        &mut meter,
        &mut run,
    ) {
        Ok(evaluation) => evaluation,
        Err(failure) => {
            return match run.failure() {
                Some(recorded) => pre_call(recorded.clone()),
                None => Err(call_failure_to_replay_refusal(failure)),
            };
        }
    };
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
