// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-193 "Replay" and "The qualitative claim" (ADR-024 SV-7, SV-8): the
//! replay of one sampled witness, and its conversion to an ADR-018
//! counterexample.
//!
//! A witness replays by re-running the sampler, never by trusting its
//! record. [`qsl_eval::simulation::replay_sampled`] holds the draw checks
//! (each drawn value in its support, each choice recomputed from the seed,
//! the trace index, the step and the choice index, the path probability
//! recomputed exactly). The value check reads the claim again on the
//! recomputed sample. The layer-A engine owns that reading (ADR-011 layering:
//! `qsl-replay` is layer 6 and does not depend on `qsl-analyze`), so it
//! reaches this module through the [`WitnessReader`] seam, as FR-128's
//! replay reaches its `holds` atoms through `ModelLetters`.

use qsl_eval::simulation::model::{ClauseOutcome, ModelSystem, TransitionId};
use qsl_eval::simulation::{
    replay_sampled, SampleStop, SampledDisagreement, SampledRecord, SampledReplay,
    SampledReplayRefusal, SampledStep, TransitionSystem,
};
use qsl_semantics::check::Workload;
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

use super::{SampledWitness, StatisticalProvenance, TestProvenance, WitnessValue};
use crate::model_check::counterexample::{CounterexampleKind, ModelStep, TemporalCounterexample};

/// What the claim reads on one recomputed sample (FR-193 "Replay"): the
/// value it would keep as a witness, and the sample's steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessReading {
    /// The value of a sample on the violating side of the claim's bound, or
    /// the undefined evaluation of the claim; `None` for a sample that is
    /// neither.
    pub value: Option<WitnessValue>,
    /// The recomputed sample's steps.
    pub steps: Vec<SampledStep>,
}

/// Why the claim could not be read on a recomputed sample.
#[derive(Clone, Debug, PartialEq)]
pub enum WitnessReadStop {
    /// The sampler stopped.
    Sample(Box<SampleStop>),
    /// The bounded event's automaton would exceed its state limit, at the
    /// value it was set to (FR-126 `ResourceExhausted`).
    AutomatonStates {
        /// The limit's value.
        value: u64,
    },
    /// A claim predicate's read was refused with no value undefined.
    PredicateRefused(Box<ClauseOutcome>),
}

/// The seam to the layer-A engine that read the claim when the witness was
/// kept (FR-189 "Per sample").
pub trait WitnessReader {
    /// Draw the sample of `test` at `trace` again under `seed` and read the
    /// claim on it. `steps` is the recorded witness's step count, which
    /// bounds the walk of a long-run fraction, whose only witness is an
    /// undefined one.
    ///
    /// # Errors
    ///
    /// A [`WitnessReadStop`] when the sample cannot be drawn and read.
    fn read(
        &mut self,
        test: &TestProvenance,
        seed: u64,
        trace: u64,
        steps: u64,
    ) -> Result<WitnessReading, WitnessReadStop>;
}

/// The subject a witness replays over: the admitted system, the workload
/// the claim runs under, and what the verdict recorded (FR-191).
#[derive(Clone, Copy)]
pub struct WitnessReplay<'a> {
    /// The admitted model system.
    pub system: &'a ModelSystem<'a>,
    /// The workload the claim runs under.
    pub workload: &'a Workload,
    /// The verdict's provenance: the seed, the sampler and each test's
    /// initial state and binding.
    pub provenance: &'a StatisticalProvenance,
    /// The cancellation handle (FR-276).
    pub cancel: &'a Cancel,
}

/// Where a replayed witness disagrees with its record (FR-193 "Replay").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampledParity {
    /// A recomputed draw, or the path probability, differs.
    Draw(SampledDisagreement),
    /// The recomputed sample takes a different number of steps.
    Length {
        /// The recorded step count.
        recorded: u64,
        /// The recomputed step count.
        replayed: u64,
    },
    /// The claim reads another value on the recomputed sample.
    Value {
        /// The recorded value.
        recorded: Box<WitnessValue>,
        /// The recomputed value, `None` when the sample is not a witness.
        replayed: Option<Box<WitnessValue>>,
    },
}

/// A sampled witness's replay (FR-193 "Outputs"). It never changes the
/// item's `measured` record, which the caller holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampledWitnessReplay {
    /// Every check agrees.
    Reproduced,
    /// `Inconclusive(ReplayParity)`: a check disagrees.
    Inconclusive(SampledParity),
}

/// Why a witness is refused before it settles (FR-193 "Replay").
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SampledWitnessRefusal {
    /// The provenance records no test of the witness's index.
    #[error("the provenance records no test {test}")]
    UnknownTest {
        /// The witness's test index.
        test: u32,
    },
    /// The test's initial state is not one of the subject's.
    #[error("test {test} starts at initial state {initial}, which the subject does not have")]
    UnknownInitial {
        /// The test's index.
        test: u32,
        /// The initial-state index.
        initial: u32,
    },
    /// A recorded value lies outside its support, or the recomputation
    /// stopped.
    #[error(transparent)]
    Draw(#[from] SampledReplayRefusal),
    /// The claim could not be read on the recomputed sample.
    #[error("the claim could not be read on the recomputed sample: {0:?}")]
    Read(WitnessReadStop),
}

/// `invalid_runtime_input`/`invalid-value`: a witness no subject has.
const INVALID_VALUE: CatalogCode = CatalogCode::new("invalid_runtime_input", "invalid-value");

/// `cancelled`/`caller-cancelled`: the caller's `Cancel` handle fired.
const CANCELLED: CatalogCode = CatalogCode::new("cancelled", "caller-cancelled");

/// `unsupported_projection`/`unsupported-requested-capability`: the witness
/// names a sampler this one is not (FR-192).
const UNSUPPORTED: CatalogCode =
    CatalogCode::new("unsupported_projection", "unsupported-requested-capability");

/// `unsupported_projection`/`not-markov`: the sample took a step that is not
/// Markov (ADR-024 PM-3).
const NOT_MARKOV: CatalogCode = CatalogCode::new("unsupported_projection", "not-markov");

/// `resource_exhausted`/`max-automaton-states`: FR-126's limit on the bounded
/// event's automaton, not the evaluation meter's.
const AUTOMATON_STATES: CatalogCode =
    CatalogCode::new("resource_exhausted", "max-automaton-states");

/// `runtime_invariant`/`established-invariant-broken`: a stop with no code
/// of FR-193's own.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

impl SampledWitnessRefusal {
    /// The catalog code of the refusal. A witness the provenance or subject
    /// does not place, a value outside its support and a
    /// `TotalAboveDrawRange` are `invalid_runtime_input`/`invalid-value`
    /// (FR-193 "Replay"); an expansion stop keeps its own code; a cancelled
    /// replay, another sampler, a non-Markov step and the automaton limit
    /// each have their own; only an undecided successor or a refused
    /// predicate is a broken invariant.
    pub fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnknownTest { .. } | Self::UnknownInitial { .. } => INVALID_VALUE,
            Self::Draw(refusal) => refusal.catalog_code().unwrap_or_else(|| match refusal {
                SampledReplayRefusal::Sample(stop) => stop_code(stop),
                SampledReplayRefusal::OutsideSupport { .. } => INVALID_VALUE,
            }),
            Self::Read(WitnessReadStop::Sample(stop)) => stop_code(stop),
            Self::Read(WitnessReadStop::AutomatonStates { .. }) => AUTOMATON_STATES,
            Self::Read(WitnessReadStop::PredicateRefused(_)) => BROKEN,
        }
    }
}

fn stop_code(stop: &SampleStop) -> CatalogCode {
    match stop {
        SampleStop::TotalAboveDrawRange(_) => INVALID_VALUE,
        SampleStop::Expansion(stop) => stop.cause,
        SampleStop::GeneratorMismatch { .. } => UNSUPPORTED,
        SampleStop::NotMarkov(_) => NOT_MARKOV,
        SampleStop::Cancelled(_) => CANCELLED,
        // A replayed step is one the sampler already took, so an undecided
        // successor there is a broken invariant.
        SampleStop::UndecidedSuccessor => BROKEN,
    }
}

/// FR-193 "Replay": replay `witness` over `replay`. The draw checks run
/// first and refuse a recorded value outside its support; `reader` then
/// reads the claim on the recomputed sample, whose length and value must be
/// the record's. The witness and the verdict it came from are not changed.
///
/// # Errors
///
/// [`SampledWitnessRefusal`] for a witness the provenance does not place,
/// a value outside its support, a recomputation that stops, or a claim that
/// cannot be read.
pub fn replay_sampled_witness(
    replay: &WitnessReplay<'_>,
    witness: &SampledWitness,
    reader: &mut dyn WitnessReader,
) -> Result<SampledWitnessReplay, SampledWitnessRefusal> {
    let test = test_of(replay.provenance, witness)?;
    let start = usize::try_from(test.initial)
        .ok()
        .and_then(|index| replay.system.initial().into_iter().nth(index))
        .ok_or(SampledWitnessRefusal::UnknownInitial {
            test: test.index,
            initial: test.initial,
        })?;
    let drawn = replay_sampled(
        replay.system,
        replay.workload,
        &start,
        &replay.provenance.sampler,
        replay.provenance.seed,
        SampledRecord {
            trace: witness.trace,
            steps: &witness.steps,
            path_probability: &witness.path_probability,
        },
        replay.cancel,
    )?;
    if let SampledReplay::Disagrees(disagreement) = drawn {
        return Ok(SampledWitnessReplay::Inconclusive(SampledParity::Draw(
            disagreement,
        )));
    }
    let recorded = u64::try_from(witness.steps.len()).unwrap_or(u64::MAX);
    let reading = reader
        .read(test, replay.provenance.seed, witness.trace, recorded)
        .map_err(SampledWitnessRefusal::Read)?;
    let replayed = u64::try_from(reading.steps.len()).unwrap_or(u64::MAX);
    if replayed != recorded {
        return Ok(SampledWitnessReplay::Inconclusive(SampledParity::Length {
            recorded,
            replayed,
        }));
    }
    if reading.value.as_ref() != Some(&witness.value) {
        return Ok(SampledWitnessReplay::Inconclusive(SampledParity::Value {
            recorded: Box::new(witness.value.clone()),
            replayed: reading.value.map(Box::new),
        }));
    }
    Ok(SampledWitnessReplay::Reproduced)
}

fn test_of<'p>(
    provenance: &'p StatisticalProvenance,
    witness: &SampledWitness,
) -> Result<&'p TestProvenance, SampledWitnessRefusal> {
    provenance
        .tests
        .iter()
        .find(|test| test.index == witness.test)
        .ok_or(SampledWitnessRefusal::UnknownTest { test: witness.test })
}

impl SampledWitness {
    /// FR-193 "The qualitative claim" (ADR-024 SV-7): the ADR-018
    /// counterexample over the same subject that this witness is: the
    /// initial state and binding the provenance records for its test, and
    /// its steps up to a terminal stutter. An undefined witness is the
    /// undefined-evaluation counterexample of its evaluation (UE-5); any
    /// other refutes the formula.
    ///
    /// # Errors
    ///
    /// [`SampledWitnessRefusal::UnknownTest`] when the provenance records no
    /// test of the witness's index.
    pub fn counterexample(
        &self,
        provenance: &StatisticalProvenance,
    ) -> Result<TemporalCounterexample, SampledWitnessRefusal> {
        let test = test_of(provenance, self)?;
        let initial_state =
            usize::try_from(test.initial).map_err(|_| SampledWitnessRefusal::UnknownInitial {
                test: test.index,
                initial: test.initial,
            })?;
        let prefix = self
            .steps
            .iter()
            .map_while(|step| match &step.transition {
                TransitionId::Operation(transition) => Some(ModelStep {
                    transition: transition.clone(),
                    post_state: step.post_digest,
                }),
                TransitionId::Stutter => None,
            })
            .collect();
        let kind = match &self.value {
            WitnessValue::Undefined(evaluation) => {
                CounterexampleKind::UndefinedEvaluation(evaluation.clone())
            }
            WitnessValue::Event(_) | WitnessValue::Fraction(_) | WitnessValue::Measure(_) => {
                CounterexampleKind::Formula
            }
        };
        Ok(TemporalCounterexample {
            initial_state,
            prefix,
            over_binding: test.binding.clone(),
            fairness: Vec::new(),
            kind,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use qsl_eval::simulation::model::{NotMarkov, ScheduledIdentity};
    use qsl_foundation::selection::DefinitionRef;
    use quire_exact::{CancelCause, EffectiveId, ObjectId, ObjectReference, UniverseId};

    use super::*;

    fn code(stop: SampleStop) -> CatalogCode {
        SampledWitnessRefusal::Draw(SampledReplayRefusal::Sample(Box::new(stop))).catalog_code()
    }

    /// FR-193 "Replay": each stop of a replay settles under its own code,
    /// and only a true invariant break is `runtime_invariant`.
    #[test]
    fn each_stop_has_its_own_catalog_code() {
        assert_eq!(
            code(SampleStop::Cancelled(CancelCause::Requested)),
            CANCELLED
        );
        let other = DefinitionRef::new("agent-ix", "quire.simulation.other/v1").expect("valid");
        assert_eq!(
            code(SampleStop::GeneratorMismatch { supplied: other }),
            UNSUPPORTED
        );
        let not_markov = NotMarkov {
            state: qsl_foundation::digest::DigestRecord::raw_artifact(b"s"),
            transition: ScheduledIdentity {
                operation: "tick".to_owned(),
                receiver: ObjectReference::new(
                    UniverseId::from_digest([1; 32]),
                    EffectiveId::from_digest([2; 32]),
                    ObjectId::new("n").expect("a key"),
                ),
                arguments: Vec::new(),
            },
            random: Vec::new(),
            post_states: 2,
        };
        assert_eq!(code(SampleStop::NotMarkov(not_markov)), NOT_MARKOV);
        assert_eq!(code(SampleStop::UndecidedSuccessor), BROKEN);
        assert_eq!(
            SampledWitnessRefusal::Read(WitnessReadStop::AutomatonStates { value: 1 })
                .catalog_code(),
            AUTOMATON_STATES
        );
    }
}
