// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330-AC-17: run a temporal clause over caller-supplied record samples.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::temporal::{
    AggregateAtom, AtomReading, CaptureReading, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, TemporalTrace, UndefinedCause, WindowRead, WindowSource, WindowTrace,
    DEFAULT_MAX_WINDOW_VALUES,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::source_map::NativeLanguage;
use qsl_semantics::check::{
    CheckedAggregateConditionsRef, CheckedInterval, CheckedNode, FormulaNode, FormulaNodeId,
    StatePredicate, TemporalClauseNode, TemporalProfile, UnaryTemporal,
};
use qsl_semantics::model::observation::{DocumentProvision, ObservationLimits, Provisions};
use quire_eval::{FamilyOutcome, FamilyResult};
use quire_exact::{Cancel, Meter, Origin, Outcome, Rational, Role, ScalarLimits, Value, ValueType};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};
use quire_semantic_value::declaration::CompositeShape;

use super::{
    front_end, ClauseDisposition, ClauseRunRefusal, ClauseRunReport, ClauseRunSource,
    ClauseRunUsage, CompiledRun, TemporalUndefinedCause,
};
/// Inputs for the non-model trace-subject clause route.
pub struct TraceSubjectClauseRequest {
    /// Program source bytes or an extracted native source.
    pub source: ClauseRunSource,
    /// The package manifest's top-level name.
    pub package_name: String,
    /// FR-056 package input.
    pub packages: qsl_semantics::model::intake::PackageInput,
    /// FR-099 dependency input.
    pub dependencies: super::DependencyInput,
    /// The selected temporal clause.
    pub name: String,
    /// The caller's declared record type for the samples.
    pub subject: ValueType,
    /// Ordered samples, one record for each event position.
    pub samples: Vec<Value>,
    /// Spine stage limits.
    pub limits: super::SpineLimits,
    /// Evaluation accounting limits.
    pub accounting: ScalarLimits,
}

/// Run one temporal clause on a closed finite event-position trace of record
/// samples. Model-subject clauses continue to use [`super::run_clause`].
pub fn run_trace_subject_clause(
    request: TraceSubjectClauseRequest,
) -> Result<ClauseRunReport, ClauseRunRefusal> {
    let unit = request.source.unit();
    if unit.bytes.is_empty() {
        return Err(ClauseRunRefusal::EmptySource);
    }
    let empty_report = |disposition| ClauseRunReport {
        basis: super::undecided_basis(&disposition),
        witness: None,
        disposition,
        usage: ClauseRunUsage::default(),
        consequent_source_events: super::ConsequentSourceEvents::Unavailable,
        finite_window: None,
        counterexample: None,
        decisive: None,
        ledger: None,
        authenticated_temporal_selection: None,
        refused_at: None,
    };
    if let Some(language) = unit.language {
        if NativeLanguage::of(language).is_none() {
            return Ok(empty_report(ClauseDisposition::UnknownLanguage {
                language: language.to_owned(),
            }));
        }
    }
    let cancel = Cancel::new();
    let compile = || -> Result<_, front_end::FrontEndFailure> {
        let parsed = front_end::parse(
            &front_end::ParseRequest {
                source: unit.identity,
                path: unit.path,
                bytes: unit.bytes,
            },
            request.limits.source,
            &cancel,
        )?
        .into_value();
        let temporal_selections = temporal_clock_selections(&parsed);
        let models = front_end::select(&parsed, &request.packages, request.limits.model, &cancel)?
            .into_value();
        let checked = front_end::check_with_temporal_selections_and_execution_points(
            &parsed,
            request.package_name.clone(),
            &crate::spine::StateClauseExecutionPoints::new(),
            &temporal_selections,
            &models,
            &request.dependencies,
            request.limits,
            &cancel,
        )?
        .into_value();
        let emitted = front_end::package_with_temporal_selections(
            &checked,
            &temporal_selections,
            front_end::PackageLimits::default(),
            &cancel,
        )?
        .into_value();
        Ok((models, checked, emitted))
    };
    let (_models, checked, _emitted) = match compile() {
        Ok(compiled) => compiled,
        Err(failure) => {
            let disposition = match front_end::refusal_or_fault(failure) {
                Ok(refusal) => ClauseDisposition::Compile(refusal),
                Err(fault) => ClauseDisposition::EvaluateFault(fault),
            };
            return Ok(empty_report(disposition));
        }
    };
    let package = checked.package();
    let empty_snapshots = DocumentProvision::default();
    let empty_invocations = DocumentProvision::default();
    let mut sources = Vec::with_capacity(1 + checked.libraries().len());
    sources.push(checked.source().clone());
    sources.extend(checked.libraries().iter().cloned());
    let run = CompiledRun {
        packages: &request.packages,
        model_limits: request.limits.model,
        provisions: Provisions {
            snapshots: &empty_snapshots,
            invocations: &empty_invocations,
        },
        observation_limits: ObservationLimits::default(),
        accounting: request.accounting,
        package,
        sources: &sources,
        progress_capacity: request.limits.progress.covered_positions,
    };
    let Some(node) = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == request.name)
    else {
        return Ok(run.report(ClauseDisposition::MissingName {
            name: request.name.clone(),
        }));
    };
    let record_type = match &request.subject {
        ValueType::Composite(key) => package
            .graph()
            .scope()
            .types()
            .composite(key.clone())
            .is_some_and(|declaration| matches!(declaration.shape(), CompositeShape::Record(_))),
        _ => false,
    };
    let matches_subject = request.subject == node.clause().over().1;
    let profile_supported = node.clause().profile() == TemporalProfile::EventPositionFalseExtension;
    if request.samples.is_empty() || !record_type || !matches_subject || !profile_supported {
        return Ok(run.report(refuse("invalid-value", BTreeMap::new())));
    }
    for sample in &request.samples {
        if !package
            .graph()
            .scope()
            .types()
            .admits(&request.subject, sample)
            .is_ok_and(|admitted| admitted)
        {
            return Ok(run.report(refuse("invalid-value", BTreeMap::new())));
        }
    }
    let len = match u64::try_from(request.samples.len()) {
        Ok(len) => len,
        Err(_) => return Ok(run.report(refuse("invalid-value", BTreeMap::new()))),
    };
    let mut meter = Meter::new(request.accounting);
    let mut letters = TraceSubjectLetters {
        package,
        node,
        samples: &request.samples,
        trace: WindowTrace::positions(len),
        captures: Vec::new(),
        atom_values: BTreeMap::new(),
        undefined_atom: None,
    };
    let result = qsl_eval::temporal::evaluate(
        node.clause(),
        TemporalTrace::Finite { len },
        &mut letters,
        &mut meter,
    );
    let (disposition, decisive) = match result {
        Ok(outcome) => {
            let decisive = decisive_aggregate(node, &outcome, &letters.atom_values);
            (
                ClauseDisposition::Temporal(map_outcome(outcome, letters.undefined_atom)),
                decisive,
            )
        }
        Err(_) => (
            ClauseDisposition::EvaluateFault(InternalFault::new(
                "FR-330",
                "trace-subject-temporal-evaluation",
            )),
            None,
        ),
    };
    let mut report = run.report(disposition);
    report.usage = ClauseRunUsage::from_meter(&meter, Default::default());
    if let Some((atom, position)) = decisive {
        let atom = u64::try_from(atom).ok();
        let Some(atom) = atom else {
            return Ok(report);
        };
        let occurrence =
            OccurrenceKey::new(node.identity(), Origin::new(Role::new("expression"), atom));
        report.decisive = Some(crate::TemporalDecisive::aggregate(
            occurrence,
            position,
            qsl_eval::value::ObservationIdentity {
                authority: "trace-subject".to_owned(),
                identity: position.to_string(),
            },
        ));
    }
    Ok(report)
}

fn decisive_aggregate(
    node: &TemporalClauseNode,
    outcome: &TemporalOutcome,
    atom_values: &BTreeMap<(usize, u64), Option<bool>>,
) -> Option<(usize, u64)> {
    let TemporalOutcome::Violated { at } = outcome else {
        return None;
    };
    let formula = node.clause().formula();
    let root = formula.nodes().get(formula.root().index())?;
    let (formula_node, start, end) = match root {
        FormulaNode::Aggregate { .. } => (formula.root(), at.index(), at.index()),
        FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval: Some(CheckedInterval::Steps(interval)),
            operand,
        } => {
            let qsl_foundation::bound::TemporalUpper::Finite(upper) = interval.upper() else {
                return None;
            };
            let start = at.index().checked_add(interval.lower())?;
            let end = at.index().checked_add(upper)?;
            (*operand, start, end)
        }
        _ => return None,
    };
    let positions: BTreeSet<_> = atom_values
        .keys()
        .filter_map(|(_, position)| (*position >= start && *position <= end).then_some(*position))
        .collect();
    first_decisive_aggregate(
        positions
            .into_iter()
            .map(|position| descend_false_formula(node, formula_node, position, atom_values)),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FormulaDecision {
    True,
    False(Option<(usize, u64)>),
    Unknown,
}

fn first_decisive_aggregate(
    decisions: impl IntoIterator<Item = FormulaDecision>,
) -> Option<(usize, u64)> {
    for decision in decisions {
        match decision {
            FormulaDecision::True | FormulaDecision::Unknown => {}
            FormulaDecision::False(Some(aggregate)) => return Some(aggregate),
            FormulaDecision::False(None) => return None,
        }
    }
    None
}

fn combine_and(left: FormulaDecision, right: FormulaDecision) -> FormulaDecision {
    match left {
        FormulaDecision::False(_) => left,
        FormulaDecision::True => right,
        FormulaDecision::Unknown => match right {
            FormulaDecision::False(_) => right,
            FormulaDecision::True | FormulaDecision::Unknown => FormulaDecision::Unknown,
        },
    }
}

fn combine_or(left: FormulaDecision, right: FormulaDecision) -> FormulaDecision {
    match left {
        FormulaDecision::True => FormulaDecision::True,
        FormulaDecision::False(left_decisive) => match right {
            FormulaDecision::True => FormulaDecision::True,
            FormulaDecision::False(_) => FormulaDecision::False(left_decisive),
            FormulaDecision::Unknown => FormulaDecision::Unknown,
        },
        FormulaDecision::Unknown => match right {
            FormulaDecision::True => FormulaDecision::True,
            FormulaDecision::False(_) | FormulaDecision::Unknown => FormulaDecision::Unknown,
        },
    }
}

fn descend_false_formula(
    node: &TemporalClauseNode,
    formula_node: FormulaNodeId,
    position: u64,
    atom_values: &BTreeMap<(usize, u64), Option<bool>>,
) -> FormulaDecision {
    let Some(formula) = node.clause().formula().nodes().get(formula_node.index()) else {
        return FormulaDecision::Unknown;
    };
    match formula {
        FormulaNode::Aggregate { .. } | FormulaNode::Holds { .. } => {
            match atom_values.get(&(formula_node.index(), position)) {
                Some(Some(true)) => FormulaDecision::True,
                Some(Some(false)) => FormulaDecision::False(
                    matches!(formula, FormulaNode::Aggregate { .. })
                        .then_some((formula_node.index(), position)),
                ),
                Some(None) | None => FormulaDecision::Unknown,
            }
        }
        FormulaNode::And(left, right) => {
            let left_decision = descend_false_formula(node, *left, position, atom_values);
            if matches!(left_decision, FormulaDecision::False(_)) {
                return left_decision;
            }
            combine_and(
                left_decision,
                descend_false_formula(node, *right, position, atom_values),
            )
        }
        FormulaNode::Or(left, right) => {
            let left_decision = descend_false_formula(node, *left, position, atom_values);
            if matches!(left_decision, FormulaDecision::True) {
                return FormulaDecision::True;
            }
            combine_or(
                left_decision,
                descend_false_formula(node, *right, position, atom_values),
            )
        }
        _ => FormulaDecision::Unknown,
    }
}

#[cfg(test)]
mod decisive_aggregate_tests {
    use super::*;

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn a_false_aggregate_decides_after_an_undefined_holds_conjunct() {
        assert_eq!(
            combine_and(
                FormulaDecision::Unknown,
                FormulaDecision::False(Some((1, 0)))
            ),
            FormulaDecision::False(Some((1, 0))),
            "false is decisive for `undefined and false`"
        );
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn a_false_left_or_operand_precedes_a_false_right_aggregate() {
        assert_eq!(
            combine_or(
                FormulaDecision::False(None),
                FormulaDecision::False(Some((1, 0)))
            ),
            FormulaDecision::False(None),
            "the left false holds operand is selected before the right aggregate"
        );
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn an_unknown_position_does_not_hide_a_later_false_aggregate() {
        assert_eq!(
            first_decisive_aggregate([
                FormulaDecision::Unknown,
                FormulaDecision::False(Some((1, 1))),
            ]),
            Some((1, 1))
        );
    }
}

#[ix_trace_rs::string_edge]
fn temporal_clock_selections(
    parsed: &front_end::ParsedUnitSource,
) -> Vec<qsl_package::TemporalClockSelection> {
    use qsl_forms::DeclarationForm;
    use qsl_package::{ClockConfiguration, TemporalClockSelection};

    let profiles = &parsed.unit().selections().profiles;
    parsed
        .unit()
        .forms()
        .iter()
        .filter_map(|form| {
            let DeclarationForm::Temporal(clause) = form.form() else {
                return None;
            };
            let selected = profiles
                .iter()
                .find(|profile| profile.alias == clause.profile.alias)?;
            let profile = TemporalProfile::from_identity(&selected.identity)?;
            let clock = match profile {
                TemporalProfile::EventPositionFalseExtension => {
                    ClockConfiguration::EventPosition {
                        sequence_authority: "caller-supplied-trace".into(),
                    }
                }
                TemporalProfile::FixedSampleFalseExtension => ClockConfiguration::FixedSample {
                    epoch: qsl_package::ClockExactNumber::Integer {
                        decimal: "0".into(),
                    },
                    period: qsl_package::ClockExactNumber::Integer {
                        decimal: "1".into(),
                    },
                    unit: "sample".into(),
                },
                TemporalProfile::TimestampedEventFiniteWindow => {
                    ClockConfiguration::TimestampedEvent {
                        timestamp_unit: "caller-supplied-time".into(),
                    }
                }
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed => return None,
            };
            Some(TemporalClockSelection {
                source: parsed.source().reference().clone(),
                declaration_span: form.span(),
                clock,
            })
        })
        .collect()
}

fn refuse(cause: &'static str, fields: BTreeMap<&'static str, String>) -> ClauseDisposition {
    ClauseDisposition::Admit(
        qsl_semantics::model::observation::AdmissionFailure::Refused(Box::new(
            qsl_semantics::model::observation::AdmissionRecord {
                catalog: CatalogCode::new("invalid_runtime_input", cause),
                fields,
                path: Vec::new(),
                kernel: None,
            },
        )),
    )
}

fn map_outcome(outcome: TemporalOutcome, undefined_atom: Option<usize>) -> super::TemporalVerdict {
    use super::TemporalVerdict;
    match outcome {
        TemporalOutcome::Holds => TemporalVerdict::Holds,
        TemporalOutcome::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        TemporalOutcome::Undefined(undefined) => TemporalVerdict::Undefined {
            at: undefined.at.index(),
            cause: match undefined.cause {
                UndefinedCause::Kernel(cause) => TemporalUndefinedCause::Kernel(cause),
                UndefinedCause::Family(cause) => TemporalUndefinedCause::Family(cause),
                UndefinedCause::InsufficientData(data) => {
                    TemporalUndefinedCause::InsufficientData {
                        atom: undefined_atom.unwrap_or_default(),
                        count: data.count,
                        min_count: data.min_count,
                    }
                }
            },
        },
        TemporalOutcome::InsufficientData(data) => TemporalVerdict::Undefined {
            at: data.at.index(),
            cause: TemporalUndefinedCause::InsufficientData {
                atom: data.atom.index(),
                count: data.data.count,
                min_count: data.data.min_count,
            },
        },
        TemporalOutcome::Pending => TemporalVerdict::Pending,
        TemporalOutcome::MissingFairnessPremise { constraint } => {
            TemporalVerdict::MissingFairnessPremise { constraint }
        }
        TemporalOutcome::Incomplete(incomplete) => TemporalVerdict::Incomplete(incomplete),
        TemporalOutcome::WindowLimitReached(reached) => TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: reached.value,
            setting: reached.setting,
        },
    }
}

fn fault(code: &'static str) -> LetterStop {
    LetterStop::Fault(InternalFault::new("FR-330.trace-subject", code))
}

struct Predicate<'a> {
    body: CheckedNode<'a>,
    slots: usize,
}

impl StatePredicate for Predicate<'_> {
    fn body(&self) -> CheckedNode<'_> {
        self.body
    }
    fn slots(&self) -> usize {
        self.slots
    }
}

struct TraceSubjectLetters<'a> {
    package: &'a qsl_package::CheckedPackage,
    node: &'a TemporalClauseNode,
    samples: &'a [Value],
    trace: WindowTrace,
    captures: Vec<Value>,
    atom_values: BTreeMap<(usize, u64), Option<bool>>,
    undefined_atom: Option<usize>,
}

impl TraceSubjectLetters<'_> {
    fn sample(&self, position: TemporalPosition) -> Result<Value, LetterStop> {
        usize::try_from(position.index())
            .ok()
            .and_then(|index| self.samples.get(index))
            .cloned()
            .ok_or_else(|| fault("position-has-record-sample"))
    }

    fn evaluate(
        &mut self,
        body: CheckedNode<'_>,
        slots: usize,
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<ExpressionReading, LetterStop> {
        let predicate = Predicate { body, slots };
        let environment = Default::default();
        let evaluation = qsl_eval::value::evaluate_state_predicate_with(
            self.package,
            &predicate,
            &environment,
            None,
            bindings,
            meter,
        )
        .map_err(|_error| fault("record-expression-evaluation"))?;
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => {
                Ok(ExpressionReading::Defined(value))
            }
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => Ok(
                ExpressionReading::Undefined(UndefinedCause::Kernel(undefined)),
            ),
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(LetterStop::Refused),
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                ExpressionReading::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("probe outcome is not constructed here"),
        }
    }

    fn bindings(&self, position: TemporalPosition) -> Result<Vec<Value>, LetterStop> {
        let sample = self.sample(position)?;
        let mut bindings = vec![sample.clone()];
        if self.node.clause().trigger().is_some() {
            bindings.push(sample);
        }
        bindings.extend(self.captures.iter().cloned());
        Ok(bindings)
    }
}

enum ExpressionReading {
    Defined(Value),
    Undefined(UndefinedCause),
}

impl TemporalLetters for TraceSubjectLetters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        if let Some(expressions) = self.node.aggregate_conditions(atom) {
            let checked = self
                .node
                .clause()
                .aggregates()
                .get(&atom.index())
                .ok_or_else(|| fault("aggregate-term-retained"))?;
            let bindings = self.bindings(position)?;
            let (threshold_body, threshold_slots) = expressions.threshold;
            let threshold = match self.evaluate(threshold_body, threshold_slots, bindings, meter)? {
                ExpressionReading::Defined(value) => value,
                ExpressionReading::Undefined(cause) => return Ok(AtomReading::Undefined(cause)),
            };
            let threshold = match threshold {
                Value::Integer(value) => Rational::from_integer(value),
                Value::Rational(value) => value,
                _ => return Err(fault("aggregate-threshold-is-number")),
            };
            let aggregate = AggregateAtom::of_checked(checked, threshold)
                .map_err(|_| fault("checked-aggregate-term-is-valid"))?;
            let mut source = SubjectWindowSource {
                package: self.package,
                expressions,
                samples: self.samples,
                captures: &self.captures,
                subject_trigger: self.node.clause().trigger().is_some(),
            };
            let reading = aggregate.evaluate(
                &self.trace,
                &mut source,
                DEFAULT_MAX_WINDOW_VALUES,
                position,
                meter,
            )?;
            if matches!(
                &reading,
                AtomReading::Undefined(UndefinedCause::InsufficientData(_))
            ) {
                self.undefined_atom = Some(atom.index());
            }
            self.atom_values.insert(
                (atom.index(), position.index()),
                match reading {
                    AtomReading::True => Some(true),
                    AtomReading::False => Some(false),
                    AtomReading::Undefined(_) => None,
                },
            );
            return Ok(reading);
        }
        let (body, slots) = self
            .node
            .condition(atom)
            .ok_or_else(|| fault("holds-condition-retained"))?;
        let bindings = self.bindings(position)?;
        let reading = match self.evaluate(body, slots, bindings, meter)? {
            ExpressionReading::Defined(Value::Boolean(true)) => Ok(AtomReading::True),
            ExpressionReading::Defined(Value::Boolean(false)) => Ok(AtomReading::False),
            ExpressionReading::Defined(_) => Err(fault("holds-condition-is-boolean")),
            ExpressionReading::Undefined(cause) => Ok(AtomReading::Undefined(cause)),
        }?;
        self.atom_values.insert(
            (atom.index(), position.index()),
            match reading {
                AtomReading::True => Some(true),
                AtomReading::False => Some(false),
                AtomReading::Undefined(_) => None,
            },
        );
        Ok(reading)
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let Some((guard, slots)) = self.node.guard() else {
            return Ok(AtomReading::True);
        };
        let sample = self.sample(position)?;
        match self.evaluate(guard, slots, vec![sample.clone(), sample], meter)? {
            ExpressionReading::Defined(Value::Boolean(true)) => Ok(AtomReading::True),
            ExpressionReading::Defined(Value::Boolean(false)) => Ok(AtomReading::False),
            ExpressionReading::Defined(_) => Err(fault("activation-guard-is-boolean")),
            ExpressionReading::Undefined(cause) => Ok(AtomReading::Undefined(cause)),
        }
    }

    fn captures(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        self.captures.clear();
        let mut bindings = vec![self.sample(position)?];
        if self.node.clause().trigger().is_some() {
            bindings.push(bindings[0].clone());
        }
        for index in 0..self.node.clause().captures().len() {
            let Some((body, slots)) = self.node.capture(index) else {
                return Err(fault("capture-expression-retained"));
            };
            let value = self.evaluate(body, slots, bindings.clone(), meter)?;
            match value {
                ExpressionReading::Defined(value) => {
                    self.captures.push(value.clone());
                    bindings.push(value);
                }
                ExpressionReading::Undefined(cause) => {
                    let capture = self
                        .node
                        .clause()
                        .captures()
                        .nth(index)
                        .map(|(name, _)| name.to_owned())
                        .ok_or_else(|| fault("capture-name-retained"))?;
                    return Ok(CaptureReading::Undefined { capture, cause });
                }
            }
        }
        Ok(CaptureReading::Fixed)
    }
}

struct SubjectWindowSource<'a> {
    package: &'a qsl_package::CheckedPackage,
    expressions: CheckedAggregateConditionsRef<'a>,
    samples: &'a [Value],
    captures: &'a [Value],
    subject_trigger: bool,
}

impl SubjectWindowSource<'_> {
    fn read(
        &mut self,
        expression: Option<(CheckedNode<'_>, usize)>,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Value>, LetterStop> {
        let (body, slots) = expression.ok_or_else(|| fault("aggregate-expression-present"))?;
        let Some(sample) = self
            .samples
            .get(usize::try_from(position.index()).map_err(|_| fault("position-fits-usize"))?)
            .cloned()
        else {
            return Ok(WindowRead::Missing);
        };
        let mut bindings = vec![sample.clone()];
        if self.subject_trigger {
            bindings.push(sample);
        }
        bindings.extend(self.captures.iter().cloned());
        let predicate = Predicate { body, slots };
        let env = Default::default();
        let evaluation = qsl_eval::value::evaluate_state_predicate_with(
            self.package,
            &predicate,
            &env,
            None,
            bindings,
            meter,
        )
        .map_err(|_| fault("aggregate-expression-call"))?;
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(WindowRead::Defined(value)),
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
                Ok(WindowRead::Undefined(UndefinedCause::Kernel(undefined)))
            }
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                WindowRead::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(LetterStop::Refused),
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("probe outcome is not constructed here"),
        }
    }
}

impl WindowSource for SubjectWindowSource<'_> {
    fn admits(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        let filter = self.expressions.filter;
        match filter {
            None => Ok(WindowRead::Defined(true)),
            Some(expression) => match self.read(Some(expression), position, meter)? {
                WindowRead::Defined(Value::Boolean(value)) => Ok(WindowRead::Defined(value)),
                WindowRead::Defined(_) => Err(fault("aggregate-filter-is-boolean")),
                WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
                WindowRead::Missing => Ok(WindowRead::Missing),
            },
        }
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop> {
        let expression = self.expressions.value;
        match self.read(expression, position, meter)? {
            WindowRead::Defined(Value::Integer(value)) => {
                Ok(WindowRead::Defined(Rational::from_integer(value)))
            }
            WindowRead::Defined(Value::Rational(value)) => Ok(WindowRead::Defined(value)),
            WindowRead::Defined(_) => Err(fault("aggregate-value-is-number")),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
            WindowRead::Missing => Ok(WindowRead::Missing),
        }
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        let expression = self.expressions.predicate;
        match self.read(expression, position, meter)? {
            WindowRead::Defined(Value::Boolean(value)) => Ok(WindowRead::Defined(value)),
            WindowRead::Defined(_) => Err(fault("aggregate-predicate-is-boolean")),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
            WindowRead::Missing => Ok(WindowRead::Missing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::receiver_frames::fixture as model_fixture;
    use super::*;
    use qsl_foundation::SourceIdentity;
    use qsl_semantics::model::intake::PackageInput;
    use quire_exact::FieldValue;

    const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         record Other { level: Int[0, 9]; }\n\
         temporal A using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] (count where holds(c.level != 2) over past[0,0] min_count 1 >= 1)\n\
         }\n\
         temporal B using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,0] (count where holds(c.level > 9) over past[0,0] min_count 0 >= 1)\n\
         }\n\
         temporal Each using t over (c: Counter) clock \"model-steps\" on each (q: Counter) {\n\
           always[0,0] (count where holds(c.level > 9) over past[0,0] min_count 0 >= 1)\n\
         }\n\
         temporal BoundedFalse using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] ((count where holds(c.level > 9) over past[0,0] min_count 0 >= 1) and holds(c.level > 0))\n\
         }\n\
         temporal ComposedHeld using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] ((count where holds(c.level = 0) over past[0,0] min_count 0 >= 1) and holds(c.level > 0))\n\
         }\n\
         temporal ComposedOr using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] ((count where holds(c.level > 9) over past[0,0] min_count 0 >= 1) or holds(c.level > 0))\n\
         }\n\
         temporal ComposedOrReverse using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] (holds(c.level > 9) or (count where holds(c.level > 9) over past[0,0] min_count 0 >= 1))\n\
         }\n\
         temporal ComposedNested using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,2] (((count where holds(c.level > 9) over past[0,0] min_count 0 >= 1) and holds(c.level > 0)) and holds(c.level > 0))\n\
         }\n\
         temporal UnknownThenFalse using t over (c: Counter) clock \"model-steps\" on each (q: Counter) {\n\
           always[0,2] (count where holds(c.level = 0) over past[1,1] min_count 1 >= 2)\n\
         }\n\
         temporal Plain using t over (c: Counter) clock \"model-steps\" on origin {\n\
           always[0,0] holds(c.level > 0)\n\
         }\n";

    const UNSUPPORTED_PROFILE_SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile i = \"quire.temporal.fixed-sample.false-extension/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal Infinite using i over (c: Counter) clock \"sample\" on origin {\n\
           always[0,0] holds(c.level > 0)\n\
         }\n";

    fn sample(key: quire_exact::NodeKey, level: i64) -> Value {
        quire_exact::from_admitted_slots(
            key,
            vec![FieldValue::Present(Value::Integer(level.into()))].into_boxed_slice(),
        )
    }

    fn source(bytes: &str) -> ClauseRunSource {
        ClauseRunSource::Program {
            identity: SourceIdentity::new("agent-ix", "trace-subject-test"),
            path: "trace-subject.native".to_owned(),
            bytes: bytes.as_bytes().to_vec(),
        }
    }

    fn checked_types() -> (ValueType, ValueType) {
        let identity = SourceIdentity::new("agent-ix", "trace-subject-test");
        let bytes = SOURCE.as_bytes();
        let limits = super::super::super::SpineLimits::default();
        let cancel = Cancel::new();
        let parsed = front_end::parse(
            &front_end::ParseRequest {
                source: &identity,
                path: "trace-subject.native",
                bytes,
            },
            limits.source,
            &cancel,
        )
        .expect("source parses")
        .into_value();
        let packages = PackageInput::default();
        let models = front_end::select(&parsed, &packages, limits.model, &cancel)
            .expect("no models selected")
            .into_value();
        let checked = front_end::check_with_temporal_selections_and_execution_points(
            &parsed,
            "fixture".to_owned(),
            &crate::spine::StateClauseExecutionPoints::new(),
            &[],
            &models,
            &super::super::super::DependencyInput::default(),
            limits,
            &cancel,
        )
        .expect("temporal records check")
        .into_value();
        let types = checked.package().graph().scope().types();
        let composite = |name: &str| {
            ValueType::Composite(
                types
                    .composites()
                    .find(|declaration| declaration.name() == name)
                    .expect("record declaration exists")
                    .key(),
            )
        };
        (composite("Counter"), composite("Other"))
    }

    fn model_subject() -> (String, PackageInput, ValueType) {
        let document = serde_json::to_vec(&model_fixture::document(&[], &[]))
            .expect("model fixture serializes");
        let packages = qsl_semantics::model::intake::package_input([document.as_slice()])
            .expect("model fixture is admitted");
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             profile t = \"quire.temporal.event-position.false-extension/v1\";\n\
             model Counters = {package:?};\n\
             temporal ModelPlain using t over (c: Counters::Counter) clock \"model-steps\" \
             on origin {{ always[0,0] holds(true) }}\n",
            package = model_fixture::PACKAGE,
        );
        let identity = SourceIdentity::new("agent-ix", "trace-subject-model-test");
        let limits = super::super::super::SpineLimits::default();
        let cancel = Cancel::new();
        let parsed = front_end::parse(
            &front_end::ParseRequest {
                source: &identity,
                path: "trace-subject-model.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("model source parses")
        .into_value();
        let models = front_end::select(&parsed, &packages, limits.model, &cancel)
            .expect("model selected")
            .into_value();
        let checked = front_end::check_with_temporal_selections_and_execution_points(
            &parsed,
            "fixture".to_owned(),
            &crate::spine::StateClauseExecutionPoints::new(),
            &[],
            &models,
            &super::super::super::DependencyInput::default(),
            limits,
            &cancel,
        )
        .expect("the model-subject clause checks")
        .into_value();
        let subject = checked
            .package()
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == "ModelPlain")
            .expect("model clause exists")
            .clause()
            .over()
            .1
            .clone();
        (text, packages, subject)
    }

    fn request(name: &str, subject: ValueType, samples: Vec<Value>) -> TraceSubjectClauseRequest {
        TraceSubjectClauseRequest {
            source: source(if name == "Infinite" {
                UNSUPPORTED_PROFILE_SOURCE
            } else {
                SOURCE
            }),
            package_name: "fixture".to_owned(),
            packages: PackageInput::default(),
            dependencies: super::super::super::DependencyInput::default(),
            name: name.to_owned(),
            subject,
            samples,
            limits: super::super::super::SpineLimits::default(),
            accounting: crate::spine::default_accounting(crate::spine::DEFAULT_WORK_UNITS),
        }
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn trace_subject_aggregates_report_insufficient_data_and_decisive_occurrences() {
        let (counter, _) = checked_types();
        let ValueType::Composite(key) = &counter else {
            unreachable!("Counter is a record")
        };
        let samples = [0, 1, 2].map(|level| sample(key.clone(), level)).to_vec();

        let insufficient = run_trace_subject_clause(request("A", counter.clone(), samples.clone()))
            .expect("valid input reports");
        assert!(
            matches!(
                insufficient.disposition,
                ClauseDisposition::Temporal(super::super::TemporalVerdict::Undefined {
                    at: 2,
                    cause: TemporalUndefinedCause::InsufficientData {
                        atom: 1,
                        count: 0,
                        min_count: 1,
                    }
                })
            ),
            "{:?}",
            insufficient.disposition
        );
        assert_eq!(insufficient.disposition.category().as_str(), "undefined");
        assert_eq!(insufficient.disposition.exit_code(), 10);
        assert_eq!(insufficient.disposition.truth(), None);
        assert!(insufficient.counterexample.is_none());
        assert!(insufficient.decisive.is_none());

        let violation = run_trace_subject_clause(request("B", counter.clone(), samples))
            .expect("valid input reports");
        assert!(matches!(
            violation.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        assert_eq!(violation.disposition.exit_code(), 10);
        assert_eq!(violation.disposition.truth(), Some(false));
        let decisive = violation
            .decisive
            .expect("aggregate false decides the clause");
        assert!(decisive.aggregate.is_some());
        assert_eq!(decisive.position, Some(0));

        let each_samples = [0, 1, 2].map(|level| sample(key.clone(), level)).to_vec();
        let each = run_trace_subject_clause(request("Each", counter.clone(), each_samples.clone()))
            .expect("valid on-each input reports");
        assert!(matches!(
            each.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        assert_eq!(
            each.decisive
                .as_ref()
                .and_then(|decisive| decisive.position),
            Some(0),
            "the later false aggregate must not replace the first failing activation"
        );

        let bounded = run_trace_subject_clause(request(
            "BoundedFalse",
            counter.clone(),
            each_samples.clone(),
        ))
        .expect("valid bounded aggregate input reports");
        assert!(matches!(
            bounded.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        assert_eq!(
            bounded
                .decisive
                .as_ref()
                .and_then(|decisive| decisive.position),
            Some(0),
            "always[0,2] descends to the least eligible false aggregate position"
        );

        let unknown_then_false = run_trace_subject_clause(request(
            "UnknownThenFalse",
            counter.clone(),
            each_samples.clone(),
        ))
        .expect("valid formula with insufficient then false aggregate reports");
        assert!(matches!(
            &unknown_then_false.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 1 })
        ));
        let decisive = unknown_then_false
            .decisive
            .expect("the later false aggregate decides after earlier insufficient data");
        assert!(decisive.aggregate.is_some());
        assert_eq!(decisive.position, Some(1));

        let composed_aggregate_false = run_trace_subject_clause(request(
            "BoundedFalse",
            counter.clone(),
            each_samples.clone(),
        ))
        .expect("valid composed formula reports");
        assert!(matches!(
            composed_aggregate_false.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        let decisive = composed_aggregate_false
            .decisive
            .expect("the false aggregate is the first conjunct deciding the activation");
        assert!(decisive.aggregate.is_some());
        assert_eq!(decisive.position, Some(0));

        let composed_holds_false = run_trace_subject_clause(request(
            "ComposedHeld",
            counter.clone(),
            each_samples.clone(),
        ))
        .expect("valid composed formula reports");
        assert!(matches!(
            composed_holds_false.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        assert!(
            composed_holds_false.decisive.is_none(),
            "a false later aggregate must not replace the first failing holds conjunct"
        );

        let composed_or_false =
            run_trace_subject_clause(request("ComposedOr", counter.clone(), each_samples.clone()))
                .expect("valid composed disjunction reports");
        assert!(matches!(
            composed_or_false.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        let decisive = composed_or_false
            .decisive
            .expect("the false left aggregate is selected in the disjunction");
        assert!(decisive.aggregate.is_some());
        assert_eq!(decisive.position, Some(0));

        let composed_nested = run_trace_subject_clause(request(
            "ComposedNested",
            counter.clone(),
            each_samples.clone(),
        ))
        .expect("valid nested formula reports");
        assert!(matches!(
            composed_nested.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        let decisive = composed_nested
            .decisive
            .expect("recursive descent reaches the nested false aggregate");
        assert!(decisive.aggregate.is_some());
        assert_eq!(decisive.position, Some(0));

        let reverse_or =
            run_trace_subject_clause(request("ComposedOrReverse", counter, each_samples))
                .expect("valid reverse-order disjunction reports");
        assert!(matches!(
            reverse_or.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Violated { at: 0 })
        ));
        assert!(
            reverse_or.decisive.is_none(),
            "the false left holds atom precedes the right aggregate"
        );
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn trace_subject_refuses_malformed_and_unsupported_requests_before_evaluation() {
        let (counter, other) = checked_types();
        let ValueType::Composite(key) = &counter else {
            unreachable!("Counter is a record")
        };
        let valid = [0, 1, 2].map(|level| sample(key.clone(), level)).to_vec();
        let ValueType::Composite(other_key) = &other else {
            unreachable!("Other is a record")
        };
        let other_samples = [0, 1, 2]
            .map(|level| sample(other_key.clone(), level))
            .to_vec();
        let requests = [
            request("A", counter.clone(), Vec::new()),
            request("A", counter.clone(), vec![Value::Boolean(true)]),
            request("A", other, other_samples),
            request("Infinite", counter.clone(), valid.clone()),
        ];
        for request in requests {
            let name = request.name.clone();
            let report = run_trace_subject_clause(request).expect("request reports");
            assert_eq!(
                report.disposition.stage(),
                super::super::ClauseRunStage::Admit,
                "{name}: {:?}",
                report.disposition
            );
            assert_eq!(report.disposition.category().as_str(), "refusal");
            assert_eq!(report.usage.evaluation_admissions, 0);
        }
        let missing = run_trace_subject_clause(request("Absent", counter, valid))
            .expect("well-formed request reports");
        assert_eq!(
            missing.disposition.stage(),
            super::super::ClauseRunStage::Select
        );
        assert!(matches!(
            missing.disposition,
            ClauseDisposition::MissingName { .. }
        ));
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn trace_subject_evaluates_record_clauses_without_aggregates() {
        let (counter, _) = checked_types();
        let ValueType::Composite(key) = &counter else {
            unreachable!("Counter is a record")
        };
        let samples = [1, 2, 3].map(|level| sample(key.clone(), level)).to_vec();
        let report = run_trace_subject_clause(request("Plain", counter, samples))
            .expect("valid record trace reports");
        assert!(matches!(
            report.disposition,
            ClauseDisposition::Temporal(super::super::TemporalVerdict::Holds)
        ));
        assert!(report.decisive.is_none());
    }

    #[ix_trace_rs::trace("TC-840", "FR-330-AC-17")]
    #[test]
    fn trace_subject_refuses_model_subject_without_aggregate_as_non_record() {
        let (text, packages, subject) = model_subject();
        let request = TraceSubjectClauseRequest {
            source: source(&text),
            package_name: "fixture".to_owned(),
            packages,
            dependencies: super::super::super::DependencyInput::default(),
            name: "ModelPlain".to_owned(),
            subject,
            samples: vec![Value::Boolean(true)],
            limits: super::super::super::SpineLimits::default(),
            accounting: crate::spine::default_accounting(crate::spine::DEFAULT_WORK_UNITS),
        };
        let report = run_trace_subject_clause(request).expect("request reports");
        assert_eq!(
            report.disposition.stage(),
            super::super::ClauseRunStage::Admit
        );
        assert_eq!(report.disposition.category().as_str(), "refusal");
        assert_eq!(report.usage.evaluation_admissions, 0);
    }
}
