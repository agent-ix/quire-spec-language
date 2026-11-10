// SPDX-License-Identifier: AGPL-3.0-or-later
//! The wire spelling of an `analyze` or `prove` item's counterexample
//! (FR-286 "Analyze items and their counterexamples"): QSpec FR-364's
//! `TemporalCounterexample` over a model subject, and canonical assignments.

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::protocol::{
    blocked_json, fairness_json, location_json, send_record_json, transition_json,
    ProtocolFairness, ProtocolTransition,
};
use qsl_eval::simulation::SubjectTransition;
use qsl_foundation::node_wire::node_key_to_json;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{ObjectReference, Rational, Value};
use quire_semantic_value::fairness::FairnessConstraint;
use quire_semantic_value::model_transition::ModelTransition;
use serde_json::{json, Value as Json};

use super::{undefined_reason, TimedStateWire, TimedTransitionWire};
use crate::model_check::counterexample::{
    CounterexampleKind, ModelStep, Race, TemporalCounterexample, UndefinedEvaluation,
    UndefinedLocus,
};
use crate::refinement::{RefinementFailure, StepRejection};

/// Why a counterexample has no wire spelling here.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CounterexampleWireError {
    /// The counterexample's member has no FR-286 spelling yet.
    #[error("the counterexample's {0} has no wire spelling")]
    Unspelled(&'static str),
}

fn rational(value: &Rational) -> Json {
    json!({
        "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string(),
    })
}

fn state(key: &qsl_eval::simulation::StateKey) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(TimedStateWire::from(key))
        .map_err(|_| CounterexampleWireError::Unspelled("state key"))
}

fn model_transition(identity: &ModelTransition) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(TimedTransitionWire::from(identity))
        .map_err(|_| CounterexampleWireError::Unspelled("transition"))
}

fn canonical<T: quire_canonical::Encode + ?Sized>(
    value: &T,
    member: &'static str,
) -> Result<Json, CounterexampleWireError> {
    qsl_eval::simulation::model::canonical_json(value)
        .ok_or(CounterexampleWireError::Unspelled(member))
}

/// How a subject's steps, send records and fairness members are written.
trait Spelling: SubjectTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError>;
    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError>;
    fn fairness(member: &Self::Fairness) -> Result<Json, CounterexampleWireError>;
}

impl Spelling for ModelTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        model_transition(self)
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        Ok(None)
    }

    fn fairness(member: &FairnessConstraint) -> Result<Json, CounterexampleWireError> {
        canonical(member, "fairness")
    }
}

impl Spelling for ProtocolTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        transition_json(self).ok_or(CounterexampleWireError::Unspelled("protocol transition"))
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        ProtocolTransition::send_record(self)
            .map(|record| {
                send_record_json(record).ok_or(CounterexampleWireError::Unspelled("send record"))
            })
            .transpose()
    }

    fn fairness(member: &ProtocolFairness) -> Result<Json, CounterexampleWireError> {
        fairness_json(member).ok_or(CounterexampleWireError::Unspelled("protocol fairness"))
    }
}

fn step<T: Spelling>(step: &ModelStep<T>) -> Result<Json, CounterexampleWireError> {
    let mut written = json!({
        "transition": step.transition.transition()?,
        "post_state": state(&step.post_state)?,
    });
    if let Some(record) = step.transition.send_record()? {
        written["send_record"] = record;
    }
    if let Some(delay) = &step.delay {
        written["delay"] = rational(delay);
    }
    Ok(written)
}

fn reference(reference: &ObjectReference) -> Result<Json, CounterexampleWireError> {
    qsl_eval::simulation::model::value_json(&Value::Reference(reference.clone()))
        .ok_or(CounterexampleWireError::Unspelled("object reference"))
}

/// The `{node, occurrence}` of the item's claim. The engines keep an
/// undefined expression as a formula atom, capture, contract clause,
/// terminal predicate, constraint, protocol step or mapping row, none of which
/// holds a node of its own, so the claim that contains it is the locus.
fn locus(claim: Option<&OccurrenceKey>) -> Result<Json, CounterexampleWireError> {
    let claim = claim.ok_or(CounterexampleWireError::Unspelled("undefined locus"))?;
    Ok(json!({
        "node": node_key_to_json(claim.node())
            .map_err(|_| CounterexampleWireError::Unspelled("undefined locus"))?,
        "occurrence": {
            "role": claim.origin().role().to_string(),
            "ordinal": claim.origin().ordinal(),
        },
    }))
}

fn undefined(
    evaluation: &UndefinedEvaluation,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    // The place and clock valuation of a timed site follow from the prefix's
    // delays, and FR-364's `where` has no member for them.
    let site = &evaluation.site;
    let mut at = json!({
        "position": site.position,
        "state_key": state(&site.state_key)?,
        "locus": locus(claim)?,
    });
    if let Some(guard) = &site.transition {
        at["transition"] = model_transition(guard)?;
    }
    if let UndefinedLocus::Mapping(mapping) = &site.locus {
        at["row"] = json!(mapping_row(&mapping.row)?);
    }
    Ok(json!({
        "type": "undefined-evaluation",
        "where": at,
        "cause": undefined_reason(&evaluation.cause),
    }))
}

fn race(race: &Race) -> Result<Json, CounterexampleWireError> {
    Ok(json!({
        "location": location_json(&race.location)
            .ok_or(CounterexampleWireError::Unspelled("race location"))?,
        "earlier": race.earlier.to_string(),
        "later": race.later.to_string(),
    }))
}

/// The mapping row an `Undefined` failure names (QSpec FR-379): its kind and
/// the declarations that identify it, as one string.
fn mapping_row(row: &crate::refinement::MappingRowRef) -> Result<String, CounterexampleWireError> {
    use crate::refinement::{MappingRowRef, StepArgument};
    let key = |key: &quire_checked::key::DeclarationKey| {
        qsl_eval::simulation::model::declaration_identity_text(key)
            .ok_or(CounterexampleWireError::Unspelled("mapping row"))
    };
    Ok(match row {
        MappingRowRef::Field { object_type, field } => {
            format!("field {} {}", key(object_type)?, key(field)?)
        }
        MappingRowRef::Object { concrete_type } => format!("object {}", key(concrete_type)?),
        MappingRowRef::Argument {
            operation,
            argument,
        } => match argument {
            StepArgument::Receiver => format!("argument {} receiver", key(operation)?),
            StepArgument::Parameter(position) => {
                format!("argument {} parameter {position}", key(operation)?)
            }
        },
        MappingRowRef::History {
            object_type,
            field,
            operation,
        } => format!("history {} {field} {}", key(object_type)?, key(operation)?),
    })
}

/// QSpec FR-379's `refinement_failure` member.
fn refinement(failure: &RefinementFailure) -> Result<Json, CounterexampleWireError> {
    Ok(match failure {
        RefinementFailure::InitialNotAbstract { initial } => {
            json!({"kind": "InitialNotAbstract", "initial": initial})
        }
        RefinementFailure::StutterChanged { position } => {
            json!({"kind": "StutterChanged", "position": position})
        }
        RefinementFailure::NoAbstractMatch { position } => {
            json!({"kind": "NoAbstractMatch", "position": position})
        }
        RefinementFailure::AbstractStepRejected {
            position,
            transition,
            cause,
        } => json!({
            "kind": "AbstractStepRejected",
            "position": position,
            "transition": model_transition(transition)?,
            "cause": match cause {
                StepRejection::Precondition { clause } => {
                    json!({"type": "Precondition", "clause": clause})
                }
                StepRejection::Frame { code } => json!({"type": "Frame", "code": code.code()}),
                StepRejection::Postcondition => json!({"type": "Postcondition"}),
            },
        }),
        RefinementFailure::Divergence { constraint, .. } => {
            let constraint = constraint
                .as_ref()
                .ok_or(CounterexampleWireError::Unspelled("divergence constraint"))?;
            json!({"kind": "Divergence", "constraint": canonical(constraint, "fairness")?})
        }
        RefinementFailure::AbstractUnfair { constraint } => {
            json!({"kind": "AbstractUnfair", "constraint": canonical(constraint, "fairness")?})
        }
        RefinementFailure::RefusalNotPermitted { position, label } => json!({
            "kind": "RefusalNotPermitted",
            "position": position,
            "label": serde_json::to_value(label)
                .map_err(|_| CounterexampleWireError::Unspelled("observation label"))?,
        }),
        RefinementFailure::TerminalSuccessMismatch {
            position,
            direction,
        } => json!({
            "kind": "TerminalSuccessMismatch",
            "position": position,
            "direction": serde_json::to_value(direction)
                .map_err(|_| CounterexampleWireError::Unspelled("terminal success direction"))?,
        }),
        RefinementFailure::Undefined { position, row } => json!({
            "kind": "Undefined",
            "position": position,
            "row": mapping_row(row)?,
        }),
    })
}

fn temporal<T: Spelling>(
    counterexample: &TemporalCounterexample<T>,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    let mut race_member = None;
    let kind = match &counterexample.kind {
        CounterexampleKind::Formula => json!("formula"),
        CounterexampleKind::Deadlock => json!("deadlock"),
        CounterexampleKind::TimeLock => json!("time-lock"),
        CounterexampleKind::UndefinedEvaluation(evaluation) => undefined(evaluation, claim)?,
        CounterexampleKind::Race(found) => {
            race_member = Some(race(found)?);
            json!("race")
        }
    };
    let steps = counterexample
        .prefix
        .iter()
        .map(step::<T>)
        .collect::<Result<Vec<_>, _>>()?;
    let (stem, mut looped) = match counterexample.lasso {
        Some(lasso) => {
            let split = lasso.entry.min(steps.len());
            let mut stem = steps;
            let looped = stem.split_off(split);
            (stem, looped)
        }
        None => (steps, Vec::new()),
    };
    if counterexample
        .lasso
        .is_some_and(|lasso| lasso.terminal_stutter)
    {
        let last = counterexample
            .prefix
            .last()
            .ok_or(CounterexampleWireError::Unspelled(
                "initial terminal stutter",
            ))?;
        looped.push(json!({
            "transition": {"type": "terminal-stutter"},
            "post_state": state(&last.post_state)?,
        }));
    }
    let over_binding = match (&counterexample.over_binding, over) {
        (None, _) => Json::Null,
        (Some(_), Some(found)) => reference(found)?,
        (Some(ModelInstance { .. }), None) => {
            return Err(CounterexampleWireError::Unspelled("over binding"))
        }
    };
    let fairness = counterexample
        .fairness
        .iter()
        .map(T::fairness)
        .collect::<Result<Vec<Json>, _>>()?;
    let mut written = json!({
        "trace": "model",
        "kind": kind,
        "initial_state": counterexample.initial_state,
        "prefix": stem,
        "loop": looped,
        "over_binding": over_binding,
        "fairness": fairness,
        "interval": Json::Null,
        "trace_position": counterexample.trace_position(),
    });
    if let Some(delay) = &counterexample.final_delay {
        written["final_delay"] = rational(delay);
    }
    if let Some(found) = race_member {
        written["race"] = found;
    }
    if let Some(failure) = &counterexample.refinement {
        written["refinement_failure"] = refinement(failure)?;
    }
    Ok(written)
}

/// QSpec FR-364's wire form of a model-arm counterexample. `over` is the
/// reference the clause's `over` parameter is bound to, which only the
/// subject can name, and `claim` the item's claim, the locus of an undefined
/// evaluation.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a member FR-364's table does not spell, a
/// terminal stutter at an initial state, an `over` binding given no
/// reference, or an undefined evaluation given no claim.
pub fn temporal_wire(
    counterexample: &TemporalCounterexample,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    temporal(counterexample, over, claim)
}

/// The protocol subject a counterexample was found over (QSpec FR-433's
/// `subject` and `max_live_instances` members).
#[derive(Clone, Debug)]
pub struct ProtocolWire {
    /// The protocol clause's qualified name.
    pub protocol: String,
    /// The reference the clause's `over` parameter is bound to.
    pub over: ObjectReference,
    /// Each static role and the domain package whose model plays it.
    pub roles: Vec<(String, String)>,
    /// For an `on each` subject, the instance budget the run used.
    pub max_live_instances: Option<u64>,
}

/// QSpec FR-433's wire form of a counterexample over a protocol subject.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a member FR-433 does not spell here.
pub fn protocol_temporal_wire(
    counterexample: &TemporalCounterexample<ProtocolTransition>,
    subject: &ProtocolWire,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    let mut written = temporal(counterexample, Some(&subject.over), claim)?;
    let roles: serde_json::Map<String, Json> = subject
        .roles
        .iter()
        .map(|(role, model)| (role.clone(), json!(model)))
        .collect();
    written["subject"] = json!({
        "type": "protocol-subject",
        "protocol": subject.protocol,
        "over": reference(&subject.over)?,
        "roles": roles,
    });
    if let Some(bound) = subject.max_live_instances {
        written["max_live_instances"] = json!(bound);
    }
    if matches!(counterexample.kind, CounterexampleKind::Deadlock) {
        written["blocked"] = Json::Array(
            counterexample
                .blocked
                .iter()
                .map(|thread| {
                    blocked_json(thread).ok_or(CounterexampleWireError::Unspelled("blocked thread"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok(written)
}

/// FR-286's `Assignments` spelling `{type: "assignments", values}`: one typed
/// value per parameter node, in the parameters' order, each in FR-181's typed
/// canonical form read from `types` and `variants`.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a parameter node or value with no typed
/// form, or an assignment that does not give one value per parameter.
pub fn assignments_wire(
    parameters: &[quire_exact::NodeKey],
    values: &[Value],
    types: &quire_semantic_value::declaration::TypeEnvironment,
    variants: &[qsl_semantics::check::VariantTypeDeclaration],
) -> Result<Json, CounterexampleWireError> {
    if parameters.len() != values.len() {
        return Err(CounterexampleWireError::Unspelled("assignment arity"));
    }
    let values = parameters
        .iter()
        .zip(values)
        .map(|(parameter, value)| {
            Ok(json!({
                "parameter": node_key_to_json(parameter)
                    .map_err(|_| CounterexampleWireError::Unspelled("parameter"))?,
                "value": qsl_eval::simulation::model::typed_value_json(value, types, variants)
                    .ok_or(CounterexampleWireError::Unspelled("assignment value"))?,
            }))
        })
        .collect::<Result<Vec<_>, CounterexampleWireError>>()?;
    Ok(json!({"type": "assignments", "values": values}))
}

/// A `ProbabilisticCounterexample`'s `path-set` spelling (QSpec FR-413
/// "ProbabilisticCounterexample"): the subject's one initial state, the
/// binding, the claim's bound, the witness scheduler's entries and each path's
/// steps with their drawn random vectors. `state_key` reads a state's FR-181
/// key and `scheduler` is the scheduler's entries by the key of the state
/// each is made at.
///
/// # Errors
///
/// [`CounterexampleWireError::Unspelled`] when a state or value has no
/// encoding.
pub fn probabilistic_wire(
    witness: &crate::ProbabilisticCounterexample,
    bound: &qsl_semantics::check::Bound,
    types: &quire_semantic_value::declaration::TypeEnvironment,
    variants: &[qsl_semantics::check::VariantTypeDeclaration],
    state_key: &dyn Fn(
        &qsl_eval::simulation::model::ModelSimState,
    ) -> Option<qsl_eval::simulation::StateKey>,
    scheduler: &[(
        qsl_eval::simulation::StateKey,
        qsl_eval::simulation::model::ScheduledIdentity,
    )],
) -> Result<Json, CounterexampleWireError> {
    use CounterexampleWireError::Unspelled;
    let typed = |value: &Value| {
        qsl_eval::simulation::model::typed_value_json(value, types, variants)
            .ok_or(Unspelled("probabilistic value"))
    };
    let key = |at: &qsl_eval::simulation::model::ModelSimState| {
        state_key(at)
            .ok_or(Unspelled("probabilistic state"))
            .and_then(|key| state(&key))
    };
    let identity = |identity: &qsl_eval::simulation::model::ScheduledIdentity| {
        let operation: Json = serde_json::from_str(&identity.operation)
            .map_err(|_| Unspelled("scheduled operation"))?;
        Ok::<_, CounterexampleWireError>(json!({
            "type": "transition",
            "operation": operation,
            "receiver": reference(&identity.receiver)?,
            "arguments": identity.arguments.iter().map(typed).collect::<Result<Vec<_>, _>>()?,
        }))
    };
    let paths = witness
        .paths
        .iter()
        .map(|path| {
            path.steps
                .iter()
                .map(|step| {
                    let mut written = json!({
                        "transition": model_transition(&step.transition)?,
                        "post_state": key(&step.post)?,
                        "random": step.draw.iter().map(typed).collect::<Result<Vec<_>, _>>()?,
                    });
                    if step.elapsed != Rational::from_integer(quire_exact::Integer::from(0_i64)) {
                        written["delay"] = rational(&step.elapsed);
                    }
                    Ok(written)
                })
                .collect::<Result<Vec<_>, CounterexampleWireError>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (comparison, threshold) = match bound {
        qsl_semantics::check::Bound::AtLeast(threshold) => (">=", threshold),
        qsl_semantics::check::Bound::AtMost(threshold) => ("<=", threshold),
    };
    let mut written = json!({
        "type": "probabilistic",
        "initial_state": 0,
        "binding": witness.binding.as_ref().map(reference).transpose()?,
        "bound": {"comparison": comparison, "threshold": rational(threshold)},
        "evidence": {"type": "path-set", "paths": paths},
    });
    if !scheduler.is_empty() {
        let entries = scheduler
            .iter()
            .map(|(at, choice)| Ok(json!({"state": state(at)?, "choice": identity(choice)?})))
            .collect::<Result<Vec<_>, CounterexampleWireError>>()?;
        written["scheduler"] = json!({"entries": entries});
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_eval::simulation::StateKey;
    use qsl_semantics::check::FormulaNodeId;
    use quire_exact::{Integer, Origin, Role, Undefined};
    use quire_semantic_value::model_transition::ModelTransition;

    use super::*;
    use crate::model_check::counterexample::{Lasso, UndefinedSite};
    use crate::refinement::RefinementFailure;
    use quire_exact::NodeKey;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("a nonzero denominator")
    }

    fn key(index: usize) -> StateKey {
        StateKey::from_canonical_bytes(format!("[{index},[]]").into_bytes())
    }

    fn step(index: usize) -> ModelStep {
        ModelStep {
            transition: ModelTransition::new(
                format!(r#"{{"name":"s{index}"}}"#),
                None,
                Vec::new(),
                quire_canonical::Limits::new(u64::MAX),
            )
            .expect("a canonical transition"),
            post_state: key(index),
            delay: None,
        }
    }

    fn counterexample(steps: usize, kind: CounterexampleKind) -> TemporalCounterexample {
        TemporalCounterexample {
            initial_state: 0,
            prefix: (1..=steps).map(step).collect(),
            over_binding: None,
            fairness: Vec::new(),
            kind,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        }
    }

    fn claim() -> OccurrenceKey {
        OccurrenceKey::new(crate::fixture_node(7), Origin::new(Role::new("claim"), 2))
    }

    fn undefined_at(time: bool) -> CounterexampleKind {
        CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
            site: UndefinedSite {
                position: 2,
                state_key: key(2),
                locus: UndefinedLocus::Atom(FormulaNodeId::from_arena_index(1)),
                transition: None,
                time: time.then(|| {
                    Box::new(crate::model_check::counterexample::TimedPoint {
                        stamp: rational(3, 2),
                        clocks: vec![rational(1, 2)],
                    })
                }),
            },
            cause: quire_eval_undefined(),
        })
    }

    fn quire_eval_undefined() -> qsl_eval::temporal::UndefinedCause {
        qsl_eval::temporal::UndefinedCause::Kernel(Undefined::DivisionByZero)
    }

    /// FR-286-AC-14: a lasso writes its stem as `prefix` and its loop as
    /// `loop`, split at the entry the lasso re-enters.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_lasso_splits_its_stem_from_a_non_empty_loop() {
        let mut found = counterexample(3, CounterexampleKind::Formula);
        found.lasso = Some(Lasso {
            entry: 1,
            terminal_stutter: false,
        });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(1));
        let looped = wire["loop"].as_array().expect("a loop");
        assert_eq!(looped.len(), 2);
        assert_eq!(looped[1]["post_state"], state(&key(3)).expect("a key"));
        assert_eq!(wire["trace_position"], 0);
    }

    /// FR-286-AC-14: a loop that is the terminal stutter appends the
    /// terminal-stutter identity at the last post-state, and one at an
    /// initial state has no step to repeat.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_terminal_stutter_loop_repeats_the_last_post_state() {
        let mut found = counterexample(2, CounterexampleKind::Formula);
        found.lasso = Some(Lasso {
            entry: 2,
            terminal_stutter: true,
        });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            wire["loop"],
            json!([{
                "transition": {"type": "terminal-stutter"},
                "post_state": state(&key(2)).expect("a key"),
            }])
        );
        let mut initial = counterexample(0, CounterexampleKind::Formula);
        initial.lasso = Some(Lasso {
            entry: 0,
            terminal_stutter: true,
        });
        assert_eq!(
            temporal_wire(&initial, None, None),
            Err(CounterexampleWireError::Unspelled(
                "initial terminal stutter"
            ))
        );
    }

    /// FR-286-AC-14: a time-lock counterexample carries its `final_delay`,
    /// and no other kind does.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_time_lock_carries_its_final_delay() {
        let mut found = counterexample(1, CounterexampleKind::TimeLock);
        found.final_delay = Some(rational(3, 2));
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["kind"], "time-lock");
        assert_eq!(
            wire["final_delay"],
            json!({"numerator": "3", "denominator": "2"})
        );
        assert_eq!(wire["trace_position"], Json::Null);
        let formula = temporal_wire(&counterexample(1, CounterexampleKind::Formula), None, None)
            .expect("spelled");
        assert!(formula.get("final_delay").is_none());
    }

    /// FR-286-AC-14: a timed step carries the delay that elapses before it.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_timed_step_carries_its_delay() {
        let mut found = counterexample(2, CounterexampleKind::Formula);
        found.prefix[1].delay = Some(rational(1, 4));
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert!(wire["prefix"][0].get("delay").is_none());
        assert_eq!(
            wire["prefix"][1]["delay"],
            json!({"numerator": "1", "denominator": "4"})
        );
    }

    /// FR-286-AC-14: an undefined evaluation is QSpec FR-364's envelope: the
    /// item's claim is the `{node, occurrence}` locus, a timed site is
    /// spelled without its clock valuation, and a counterexample with no
    /// claim has no locus.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn an_undefined_evaluation_names_the_claim_as_its_locus() {
        for time in [false, true] {
            let found = counterexample(2, undefined_at(time));
            let wire = temporal_wire(&found, None, Some(&claim())).expect("spelled");
            assert_eq!(wire["kind"]["type"], "undefined-evaluation");
            assert_eq!(wire["kind"]["cause"], "division-by-zero");
            let at = &wire["kind"]["where"];
            assert_eq!(at["position"], 2);
            assert_eq!(at["state_key"], state(&key(2)).expect("a key"));
            assert_eq!(
                at["locus"]["occurrence"],
                json!({"role": "claim", "ordinal": 2})
            );
            assert!(at["locus"]["node"]["path"].is_array());
            assert!(at.get("time").is_none());
            assert_eq!(at.as_object().map(serde_json::Map::len), Some(3));
        }
        assert_eq!(
            temporal_wire(&counterexample(2, undefined_at(false)), None, None),
            Err(CounterexampleWireError::Unspelled("undefined locus"))
        );
    }

    /// QSpec FR-379: a `Divergence` names the constraint it violates, and one
    /// without a constraint is not spelled.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_divergence_without_a_constraint_is_not_spelled() {
        let mut found = counterexample(1, CounterexampleKind::Formula);
        found.refinement = Some(RefinementFailure::Divergence {
            constraint: None,
            protocol_class: None,
        });
        assert_eq!(
            temporal_wire(&found, None, None),
            Err(CounterexampleWireError::Unspelled("divergence constraint"))
        );
    }

    /// FR-286: a refinement failure with a position writes the FR-379 member
    /// and no other.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_refinement_failure_is_written_as_its_own_member() {
        let mut found = counterexample(1, CounterexampleKind::Formula);
        found.refinement = Some(RefinementFailure::StutterChanged { position: 1 });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(
            wire["refinement_failure"],
            json!({"kind": "StutterChanged", "position": 1})
        );
    }

    /// FR-286: an assignment gives one value per parameter, and one that does
    /// not is not spelled.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn an_assignment_that_does_not_match_its_parameters_is_not_spelled() {
        let parameter = NodeKey::new(
            "fixture",
            vec![quire_exact::PathSegment::Text("x".to_owned())],
            0,
        )
        .expect("a node");
        let types = quire_semantic_value::declaration::TypeEnvironment::default();
        let one = [Value::Integer(Integer::from(1_i64))];
        let wire = assignments_wire(std::slice::from_ref(&parameter), &one, &types, &[])
            .expect("one value per parameter");
        assert_eq!(
            wire["values"][0]["value"],
            json!({"type": "integer", "value": "1"})
        );
        for (parameters, values) in [
            (std::slice::from_ref(&parameter), &[][..]),
            (&[][..], &one[..]),
        ] {
            assert_eq!(
                assignments_wire(parameters, values, &types, &[]),
                Err(CounterexampleWireError::Unspelled("assignment arity"))
            );
        }
    }
}
