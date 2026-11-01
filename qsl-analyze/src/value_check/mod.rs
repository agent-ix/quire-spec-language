// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `value-validity` check arm (QSpec FR-483, FR-484): decide a claim over
//! a checked package by evaluating every assignment of its function's
//! parameters in their finite domains through the native evaluator.
//!
//! * A `clause` (a `Boolean` function's own claim) is `proved` with basis
//!   `exhaustive` and certification `uncertified` when it holds at every
//!   assignment, and `refuted` at the first assignment where it is false or
//!   undefined, once the native replay reproduces that violation.
//! * A `scalar-operation` is `refuted` when the evaluation stops undefined at
//!   its occurrence or its observed value is outside the selected result
//!   bound. A reached occurrence with an admitted value counts as `holds`;
//!   an assignment that does not reach it is not decisive. If no assignment
//!   reaches the occurrence, the path is vacuous and does not establish a
//!   proof.
//! * The `max_cases` budget charges one case for each parameter assignment
//!   and each function-body evaluation, including replay. Reaching it before
//!   enumeration is complete is `incomplete` `limit-reached`.
//!
//! [`value_item`] is the only constructor of the arm's work.

pub mod manifest;

use std::num::NonZeroU64;

use qsl_foundation::RequestIndex;
use qsl_replay::value_check::{
    integers, ClaimForm, ParameterDomain, ReproducedViolation, Selection, Stop, ValueTarget,
    Verdict,
};
use qsl_replay::{Cancel, DeclineCode, ProofRefusalCause, Value, ValueCheckUnsupported};
use qsl_route::Mode;
use qsl_semantics::check::Capability;
use quire_semantic_value::catalog::{CatalogCode, InternalFault};

use crate::analyze::{AnalyzeItem, EngineReport, Refusal};

/// What the arm found, before settlement.
#[derive(Debug)]
pub enum ValueOutcome {
    /// The goal holds throughout enumeration; `cases` counts charged
    /// assignments and function-body evaluations.
    Proved {
        /// The assignments evaluated.
        cases: u64,
    },
    /// A violation the native replay reproduced.
    Refuted(Box<ReproducedViolation>),
    /// A candidate violation the native replay did not reproduce.
    ReplayParity,
    /// No assignment satisfies the premises or reaches the selected
    /// scalar-operation occurrence.
    Vacuous,
    /// The arm neither established the goal nor found a violation.
    Undecided,
    /// The case budget was reached before the domain was exhausted.
    MaxCases(u64),
    /// The evaluation was stopped.
    Stopped(Stop),
}

/// Why the arm settles without a check.
#[derive(Debug)]
pub enum ValueRefusal {
    /// A parameter is the mathematical `Integer` with no bound.
    RequiresBound,
    /// The form or an element it reads lies outside what the arm decides.
    Unsupported(ClaimForm, ValueCheckUnsupported),
    /// A broken invariant.
    Fault(InternalFault),
}

/// One value-validity request item: the claim's target and its case budget.
pub struct ValueRequest<'p> {
    /// The claim's target, with the item's bounds applied.
    pub target: ValueTarget<'p>,
    /// The item's `check.limits.max_cases`; absent takes the default the
    /// candidate's descriptor publishes under `bounds.max_cases`.
    pub max_cases: Option<NonZeroU64>,
    /// A candidate violation an external source reports for the claim (an
    /// engine's counterexample). The arm confirms it by the native replay
    /// before it reports `refuted`, and a candidate the replay does not
    /// reproduce is `inconclusive` `replay-parity` (QSpec FR-483
    /// "Counterexample and evidence kind"). Absent, the arm searches the
    /// domain itself.
    pub reported: Option<qsl_replay::value_check::Assignment>,
}

/// What one evaluated assignment means for the search.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Step {
    /// The goal holds there, or the evaluation showed nothing against it.
    Continue,
    /// The goal is violated there.
    Violated(qsl_replay::value_check::Violation),
    /// A scalar-operation occurrence was not reached in this assignment.
    NotReached,
    /// The evaluation was stopped.
    Stop(Stop),
    /// A called function body is outside the checked package closure.
    CallNotEvaluable,
    /// The remaining item case budget could not admit another function body.
    CaseLimit,
    /// The evaluator refused: the arm cannot decide the claim.
    Refuse,
    /// The goal is undefined where the package names no single occurrence:
    /// the arm cannot report the violation and does not claim the goal.
    Unlocated,
}

/// The step a verdict is. A refused evaluation is never a held goal.
pub(crate) fn judge(verdict: Verdict) -> Step {
    match verdict {
        Verdict::Holds => Step::Continue,
        Verdict::NotDecisive => Step::NotReached,
        Verdict::Violated(violation) => Step::Violated(violation),
        Verdict::Stopped(stop) => Step::Stop(stop),
        Verdict::CallNotEvaluable => Step::CallNotEvaluable,
        Verdict::CaseLimit => Step::CaseLimit,
        Verdict::Refused => Step::Refuse,
        Verdict::Unlocated => Step::Unlocated,
    }
}

/// The outcome of a replay: `refuted` when the native replay reproduced the
/// violation, `replay-parity` when it did not (QSpec FR-483 "Counterexample
/// and evidence kind"). The parity branch is defence in depth: the arm's own
/// search and its replay share one evaluator, so only a reported candidate
/// (or an evaluator that is not deterministic) reaches it.
pub(crate) fn replayed(reproduced: Option<ReproducedViolation>) -> ValueOutcome {
    match reproduced {
        Some(reproduced) => ValueOutcome::Refuted(Box::new(reproduced)),
        None => ValueOutcome::ReplayParity,
    }
}

/// Replay `violation` at `assignment` natively and settle by the result.
fn reproduce(
    target: &ValueTarget<'_>,
    assignment: &[Value],
    violation: &qsl_replay::value_check::Violation,
    cancel: &Cancel,
    budget: u64,
    used: u64,
) -> Result<(ValueOutcome, u64), ValueRefusal> {
    let remaining = budget.saturating_sub(used);
    if remaining == 0 {
        return Ok((ValueOutcome::MaxCases(budget), 0));
    }
    target
        .replay_with_case_budget(assignment, violation, cancel, remaining)
        .map(|(reproduced, body_cases, exhausted)| {
            if exhausted {
                (ValueOutcome::MaxCases(budget), body_cases)
            } else {
                (replayed(reproduced), body_cases)
            }
        })
        .map_err(ValueRefusal::Fault)
}

/// Confirm a candidate violation an external source reports: `refuted` only
/// when the evaluator violates the goal at it, `replay-parity` otherwise.
fn confirm_reported(
    target: &ValueTarget<'_>,
    reported: &[Value],
    cancel: &Cancel,
    budget: u64,
) -> Result<(ValueOutcome, u64), ValueRefusal> {
    // A candidate that is no assignment of the target (wrong arity, type or
    // outside its domain) is not a violation the native replay can
    // reproduce: `replay-parity`, never a fault.
    if !target.admits(reported) {
        return Ok((ValueOutcome::ReplayParity, 0));
    }
    let (verdict, body_cases) = target
        .evaluate_with_case_budget(reported, cancel, budget.saturating_sub(1))
        .map_err(ValueRefusal::Fault)?;
    match judge(verdict) {
        Step::Violated(violation) => {
            reproduce(target, reported, &violation, cancel, budget, 1 + body_cases)
                .map(|(outcome, replay_cases)| (outcome, body_cases + replay_cases))
        }
        Step::Continue | Step::NotReached | Step::Unlocated => {
            Ok((ValueOutcome::ReplayParity, body_cases))
        }
        Step::Stop(stop) => Ok((ValueOutcome::Stopped(stop), body_cases)),
        Step::CaseLimit => Ok((ValueOutcome::MaxCases(budget), body_cases)),
        Step::CallNotEvaluable => Err(ValueRefusal::Unsupported(
            target.form(),
            ValueCheckUnsupported::CallNotEvaluable,
        )),
        Step::Refuse => Err(ValueRefusal::Unsupported(
            target.form(),
            ValueCheckUnsupported::EvaluationRefused,
        )),
    }
}

/// One parameter's position in the enumeration: the values it has left and
/// the one it holds. Values are produced one at a time, so a domain of any
/// size costs a position, never its extent.
struct Digit {
    first: Value,
    values: Box<dyn Iterator<Item = Value>>,
    current: Value,
}

/// The iterator over `domain`'s values, in order.
fn values(domain: &ParameterDomain) -> Box<dyn Iterator<Item = Value>> {
    match domain {
        ParameterDomain::Integers(Some(interval)) => {
            Box::new(integers(interval).map(Value::Integer))
        }
        ParameterDomain::Boolean => {
            Box::new([Value::Boolean(false), Value::Boolean(true)].into_iter())
        }
        ParameterDomain::Values(values) => Box::new(values.clone().into_iter()),
        ParameterDomain::Union(cases) => Box::new(UnionValues::new(cases.clone())),
        ParameterDomain::Integers(None)
        | ParameterDomain::Unbounded
        | ParameterDomain::Unsupported(_) => Box::new(std::iter::empty()),
    }
}

impl Digit {
    /// The first position of `domain`; `None` when it holds no value.
    fn first(domain: &ParameterDomain) -> Option<Self> {
        let mut values = values(domain);
        let current = values.next()?;
        Some(Self {
            first: current.clone(),
            values,
            current,
        })
    }

    /// Move to the next value; `false` after the last, when the digit is
    /// back at its first value.
    fn advance(&mut self, domain: &ParameterDomain) -> bool {
        if let Some(next) = self.values.next() {
            self.current = next;
            return true;
        }
        self.values = values(domain);
        self.values.next();
        self.current = self.first.clone();
        false
    }
}

/// A lazy product of a union's member payload domains. The checked union
/// remains ordered by its declaration; each member's last payload position
/// advances fastest.
struct UnionValues {
    cases: Vec<(quire_exact::UnionMember, Vec<ParameterDomain>)>,
    case: usize,
    digits: Option<Vec<Digit>>,
}

impl UnionValues {
    fn new(cases: Vec<(quire_exact::UnionMember, Vec<ParameterDomain>)>) -> Self {
        let mut values = Self {
            cases,
            case: 0,
            digits: None,
        };
        values.start_case();
        values
    }

    fn start_case(&mut self) {
        while let Some((_, domains)) = self.cases.get(self.case) {
            let Some(digits) = domains.iter().map(Digit::first).collect() else {
                self.case += 1;
                continue;
            };
            self.digits = Some(digits);
            return;
        }
        self.digits = None;
    }
}

impl Iterator for UnionValues {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let (member, domains) = self.cases.get(self.case)?;
        let digits = self.digits.as_mut()?;
        let payload = digits.iter().map(|digit| digit.current.clone()).collect();
        let value = quire_exact::UnionValue::from_admitted(member.clone(), payload);
        let mut advanced = false;
        for at in (0..digits.len()).rev() {
            if digits[at].advance(&domains[at]) {
                advanced = true;
                break;
            }
        }
        if !advanced {
            self.case += 1;
            self.start_case();
        }
        Some(value)
    }
}

/// Decide `request` (QSpec FR-483, FR-484) under the case budget `budget`.
///
/// # Errors
///
/// [`ValueRefusal`] when the item settles without a check.
pub fn check_value(
    request: &ValueRequest<'_>,
    budget: NonZeroU64,
    cancel: &Cancel,
) -> Result<ValueOutcome, ValueRefusal> {
    let target = &request.target;
    let form = target.form();
    let budget = budget.get();
    let domains: Vec<&ParameterDomain> = target
        .parameters()
        .iter()
        .map(|parameter| parameter.domain())
        .collect();
    if domains
        .iter()
        .any(|d| matches!(d, ParameterDomain::Unbounded))
    {
        return Err(ValueRefusal::RequiresBound);
    }
    if domains
        .iter()
        .any(|d| matches!(d, ParameterDomain::Unsupported(_)))
    {
        return Err(ValueRefusal::Unsupported(
            form,
            ValueCheckUnsupported::ParameterDomain,
        ));
    }
    let Some(mut digits) = domains
        .iter()
        .map(|domain| Digit::first(domain))
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(ValueOutcome::Vacuous);
    };
    let mut cases = 0_u64;
    let mut checked_cases = 0_u64;
    if let Some(reported) = &request.reported {
        let (outcome, _) = confirm_reported(target, reported, cancel, budget)?;
        return Ok(outcome);
    }
    loop {
        if cases >= budget {
            return Ok(ValueOutcome::MaxCases(budget));
        }
        cases += 1;
        let assignment: Vec<Value> = digits.iter().map(|d| d.current.clone()).collect();
        let (verdict, body_cases) = target
            .evaluate_with_case_budget(&assignment, cancel, budget - cases)
            .map_err(ValueRefusal::Fault)?;
        cases += body_cases;
        match judge(verdict) {
            Step::Continue => checked_cases += 1,
            Step::NotReached => {}
            Step::Violated(violation) => {
                let (outcome, _) =
                    reproduce(target, &assignment, &violation, cancel, budget, cases)?;
                return Ok(outcome);
            }
            Step::Stop(stop) => return Ok(ValueOutcome::Stopped(stop)),
            Step::CaseLimit => return Ok(ValueOutcome::MaxCases(budget)),
            Step::CallNotEvaluable => {
                return Err(ValueRefusal::Unsupported(
                    form,
                    ValueCheckUnsupported::CallNotEvaluable,
                ))
            }
            // A violation that cannot be named is not a pass: the claim is
            // undecided (`solver-inconclusive`, an element the arm does not
            // cover).
            Step::Unlocated => return Ok(ValueOutcome::Undecided),
            Step::Refuse => {
                return Err(ValueRefusal::Unsupported(
                    form,
                    ValueCheckUnsupported::EvaluationRefused,
                ))
            }
        }
        // Advance the odometer, the last parameter fastest.
        let mut at = digits.len();
        loop {
            if at == 0 {
                return Ok(match form {
                    ClaimForm::Clause => ValueOutcome::Proved { cases },
                    ClaimForm::ScalarOperation if checked_cases == 0 => ValueOutcome::Vacuous,
                    ClaimForm::ScalarOperation => ValueOutcome::Proved { cases },
                });
            }
            at -= 1;
            if digits[at].advance(domains[at]) {
                break;
            }
        }
    }
}

/// The record a refuted item keeps: the reproduced violation.
#[derive(Debug)]
pub struct ValueCounterexample {
    /// The claim's `{node, occurrence}`.
    pub claim: qsl_replay::OccurrenceKey,
    /// The parameters' node identities, in the order of the assignment.
    pub parameters: Vec<quire_exact::NodeKey>,
    /// The reproduced violation.
    pub reproduced: ReproducedViolation,
}

/// What the arm reports for one item.
pub(crate) enum ValueReport {
    /// The arm's finding, with the counterexample's identity.
    Found(
        ValueOutcome,
        Vec<quire_exact::NodeKey>,
        qsl_replay::OccurrenceKey,
    ),
    /// An arm `unsupported` outcome.
    Unsupported(ClaimForm, ValueCheckUnsupported),
}

fn invalid_input() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(qsl_foundation::Code::InvalidRuntimeInput),
        CatalogCode::new("invalid_runtime_input", "invalid-value"),
    )
}

fn incomplete_input() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::IncompleteInput,
        DeclineCode::Qsl(qsl_foundation::Code::MissingDeclaration),
        CatalogCode::new("missing_declaration", "missing-name"),
    )
}

/// The `value-validity` item that runs `selection` on the arm: a claim that
/// selects no form is `declined` `invalid-input`, one whose package lacks a
/// required node `incomplete-input`, and neither runs a check (QSpec FR-483
/// "The check input").
pub fn value_item<'a>(
    index: RequestIndex,
    selection: Result<ValueRequest<'a>, Selection>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new_with_engine(
        index,
        Capability::ValueValidity,
        Mode::Bounded,
        manifest::BACKEND_ID,
        move |cancel, engine| {
            let request = match selection {
                Ok(request) => request,
                Err(Selection::NoForm) => return EngineReport::Refused(invalid_input()),
                Err(Selection::MissingNode) => return EngineReport::Refused(incomplete_input()),
                Err(Selection::Fault(fault)) => {
                    return EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            };
            let parameters = request
                .target
                .parameters()
                .iter()
                .map(|parameter| parameter.wire())
                .collect();
            let claim = request.target.claim().clone();
            let Some(budget) = request.max_cases.or_else(|| engine.max_cases()) else {
                return EngineReport::Refused(Refusal::Failed(
                    InternalFault::new("value-check", "descriptor-publishes-max-cases")
                        .catalog_code(),
                ));
            };
            match check_value(&request, budget, cancel) {
                Ok(outcome) => {
                    EngineReport::Value(Box::new(ValueReport::Found(outcome, parameters, claim)))
                }
                Err(ValueRefusal::RequiresBound) => EngineReport::Refused(Refusal::RequiresBound),
                Err(ValueRefusal::Unsupported(form, reason)) => {
                    EngineReport::Value(Box::new(ValueReport::Unsupported(form, reason)))
                }
                Err(ValueRefusal::Fault(fault)) => {
                    EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            }
        },
    )
}

/// Settle the arm's report as the item's FR-331 terminal record (QSpec
/// FR-483 "Outcomes and their mapping"), with the counterexample a
/// `refuted` item keeps.
pub(crate) fn settle(
    report: ValueReport,
    index: RequestIndex,
    backend: qsl_foundation::BackendId,
) -> (qsl_replay::TerminalRecord, Option<ValueCounterexample>) {
    use qsl_replay::{
        DisagreementCause, IncompleteCause, InconclusiveCause, ModelCheckLimit, TerminalRecord,
        UnavailabilityCause, Verdict,
    };
    use quire_semantic_value::catalog::Category;

    let (outcome, parameters, claim) = match report {
        ValueReport::Unsupported(form, reason) => {
            return (
                TerminalRecord::unsupported(
                    index,
                    backend,
                    UnavailabilityCause::ValueCheck { form, reason },
                ),
                None,
            )
        }
        ValueReport::Found(outcome, parameters, claim) => (outcome, parameters, claim),
    };
    match outcome {
        ValueOutcome::Proved { .. } => (TerminalRecord::exhaustive(index, backend), None),
        ValueOutcome::Refuted(reproduced) => (
            TerminalRecord::refuted_by_assignment(index, backend, &reproduced),
            Some(ValueCounterexample {
                claim,
                parameters,
                reproduced: *reproduced,
            }),
        ),
        ValueOutcome::ReplayParity => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::ReplayParity(DisagreementCause::Verdicts {
                    proved: Verdict::from_category(Category::Violation),
                    replayed: Verdict::from_category(Category::Success),
                }),
            ),
            None,
        ),
        // QSpec FR-483 gives a no-assignment run an inconclusive result with
        // the shared vacuity cause; it is distinct from a Kani proof with
        // zero SUCCESS checks.
        ValueOutcome::Vacuous => (
            TerminalRecord::inconclusive(index, backend, InconclusiveCause::KaniVacuousProof),
            None,
        ),
        ValueOutcome::Undecided => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::SolverInconclusive {
                    trace: None,
                    replay: None,
                },
            ),
            None,
        ),
        ValueOutcome::MaxCases(value) => (
            TerminalRecord::incomplete(
                index,
                backend,
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::ValueCheckMaxCases,
                    value,
                },
            ),
            None,
        ),
        ValueOutcome::Stopped(Stop::Cancelled(source)) => (
            TerminalRecord::incomplete(index, backend, IncompleteCause::Cancelled { source }),
            None,
        ),
        ValueOutcome::Stopped(Stop::Meter(incomplete)) => (
            TerminalRecord::incomplete(
                index,
                backend,
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: incomplete.limit_kind,
                    },
                    value: incomplete.limit,
                },
            ),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qsl_replay::value_check::Violation;
    use quire_exact::CancelCause;

    /// A verdict is a step: a refused evaluation is never a held goal, a
    /// stop is never a pass, and a violation keeps the violation it found.
    #[test]
    fn a_verdict_is_judged_without_turning_a_refusal_or_a_stop_into_a_pass() {
        assert_eq!(judge(Verdict::Holds), Step::Continue);
        assert_eq!(judge(Verdict::NotDecisive), Step::NotReached);
        assert_eq!(judge(Verdict::Refused), Step::Refuse);
        assert_eq!(judge(Verdict::CallNotEvaluable), Step::CallNotEvaluable);
        assert_eq!(judge(Verdict::CaseLimit), Step::CaseLimit);
        assert_eq!(judge(Verdict::Unlocated), Step::Unlocated);
        let violation = Violation { undefined: None };
        assert_eq!(
            judge(Verdict::Violated(violation.clone())),
            Step::Violated(violation)
        );
        let stop = Stop::Cancelled(CancelCause::Deadline);
        assert_eq!(judge(Verdict::Stopped(stop.clone())), Step::Stop(stop));
    }

    /// A replay that does not reproduce is `replay-parity`, with no
    /// counterexample; one that does is `refuted`.
    #[test]
    fn a_replay_that_does_not_reproduce_is_replay_parity() {
        assert!(matches!(replayed(None), ValueOutcome::ReplayParity));
    }

    /// An undecided claim (a violation the package cannot locate, or a
    /// scalar operation) settles `inconclusive` `solver-inconclusive`, and
    /// a replay-parity outcome `inconclusive` `replay-parity`; neither
    /// records a counterexample.
    #[test]
    fn undecided_and_parity_outcomes_settle_inconclusive_without_a_counterexample() {
        use qsl_replay::{InconclusiveCause, TerminalValue};
        let settle_one = |outcome| {
            let report = ValueReport::Found(outcome, Vec::new(), claim_key());
            settle(
                report,
                RequestIndex::new(0),
                qsl_foundation::BackendId::new("value-checker"),
            )
        };
        let (undecided, none) = settle_one(ValueOutcome::Undecided);
        assert!(none.is_none());
        assert!(matches!(
            undecided.value(),
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
        ));
        let (parity, none) = settle_one(ValueOutcome::ReplayParity);
        assert!(none.is_none());
        assert!(matches!(
            parity.value(),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
        ));
    }

    fn claim_key() -> qsl_replay::OccurrenceKey {
        qsl_replay::OccurrenceKey::new(
            crate::test_ids::node_key([1]),
            quire_exact::Origin::new(quire_exact::Role::new("claim"), 0),
        )
    }
}
