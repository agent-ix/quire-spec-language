// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4 over a model subject (FR-189 "Inputs", "Tests" and "Per sample";
//! FR-190): the request, the tests of the subject, and the
//! [`BehaviourSampler`] that draws FR-188's behaviours and reads the checked
//! claim on each with the layer-5 evaluator.
//!
//! A sample of test `j` starts at the test's initial state and draws one
//! step at a time, reading the claim's state predicates at each position
//! with the test's `over` binding bound, until the claim's event or measure
//! is decided: a bounded event when FR-126's monitor decides it or its
//! `h + 1` positions are read, a measure when it ends, is censored or is not
//! activated within its window, a fraction once its window is read. Every
//! predicate the claim reads is read at every position read, as EN-5's
//! product reads them, so the first undefined evaluation along a sample
//! (ADR-018 UE-5) is a function of the positions alone. A regeneration walk
//! reads the long-run fraction's predicate at each position and cuts the
//! behaviour at each return to its initial state's key.

use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::rc::Rc;

use qsl_eval::automaton::{translate_body, AutomatonError, Monitor, PropertyAutomaton, StateId};
use qsl_eval::model_subject::{ModelInstance, ModelSubject};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem, PredicateReading};
use qsl_eval::simulation::{
    NotSimulated, Permutation, ProbabilisticSampler, SampleStop, SampledPosition, SampledStep,
    StepAt, SymmetryGroup, TotalAboveDrawRange, TransitionSystem,
};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::selection::DefinitionRef;
use qsl_package::CheckedPackage;
use qsl_replay::statistical::{
    ClaimForm, Confidence, MeasureValue, StatisticalLimits, StatisticalMethod, StatisticalOutcome,
    StatisticalRefusal, TestProvenance, WitnessReadStop, WitnessReader, WitnessReading,
    WitnessValue,
};
use qsl_replay::{
    ClaimPredicate, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
    DEFAULT_MAX_AUTOMATON_STATES,
};
use qsl_semantics::check::{
    Bound as CheckedBound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim,
    CheckedTemporalFormula, Event, FractionMeasure, ProbForm, ProbScheduler, TemporalProfile,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{Cancel, Identifier, Integer, ObjectReference, Rational, Value, ValueType};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};

use super::claim::{reduce_quantile, Bound, Comparison, MeasureEvent, QuantileComparison};
use super::{
    check_sampled, require_statistical, violating, BehaviourSampler, Drawn, RegenerationWalk,
    Sample, SampleHalt, SampledRequest, StatisticalClaim, TestSubject, UnitFraction, WalkStep,
    Weight,
};
use crate::exact_probabilistic::plan::{read_letter, PlanFault};
use crate::exact_probabilistic::{FormulaState, MonitorVerdict};
use crate::model_check::false_extension::false_extension_rejects;
use crate::model_check::{admit, SymmetryDeclaration};

/// FR-189's statistical request over a model subject.
pub struct StatisticalRequest<'a> {
    /// FR-125's model subject.
    pub subject: ModelSubject<'a>,
    /// FR-186's checked claim: under a workload, with confidence parameters.
    pub claim: &'a CheckedProbabilisticClaim,
    /// The method.
    pub method: StatisticalMethod,
    /// The seed.
    pub seed: u64,
    /// The sampler, checked by identity: `quire.simulation.sampler/v1`.
    pub sampler: DefinitionRef,
    /// The budgets (FR-191).
    pub limits: StatisticalLimits,
    /// The most sampled witnesses kept beside an undefined one (FR-193).
    pub max_witnesses: u32,
    /// The symmetry declarations (ADR-021 SYM-5); empty selects none. Under
    /// an admitted declaration, when the workload and every random
    /// distribution are invariant under its group, one test runs per orbit
    /// of (initial state, binding) pairs (FR-189 "Tests", ADR-024 RX-1).
    /// Workload weights are per operation and a support holds no reference
    /// (S3 refuses a reference literal), so the invariance condition always
    /// holds today. SYM-4's closure is not checked: only given pairs merge.
    pub symmetry: Vec<SymmetryDeclaration>,
}

fn fault(invariant: &'static str) -> StatisticalRefusal {
    StatisticalRefusal::Internal(InternalFault::new("statistical", invariant))
}

/// FR-189: decide `request`'s claim over its subject, polling `cancel`
/// before each draw and each step.
///
/// The claim is refused before the subject is read when only exact evidence
/// decides it ([`require_statistical`]) or the method does not decide its
/// form; the subject is then admitted, its tests listed, and the claim
/// decided by [`check_sampled`] over FR-188's behaviours.
///
/// # Errors
///
/// [`StatisticalRefusal::EveryScheduler`], [`StatisticalRefusal::ExactOnlyForm`]
/// or [`StatisticalRefusal::MissingConfidence`] for a claim only exact
/// evidence decides; [`StatisticalRefusal::NotStatistical`];
/// [`StatisticalRefusal::Admission`] when the subject does not admit;
/// [`StatisticalRefusal::RequiresBound`] when the `over` type's population has
/// no universe; [`StatisticalRefusal::GeneratorMismatch`]; and every refusal
/// of [`check_sampled`].
pub fn check_statistical(
    request: StatisticalRequest<'_>,
    cancel: &Cancel,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    let StatisticalRequest {
        subject,
        claim,
        method,
        seed,
        sampler,
        limits,
        max_witnesses,
        symmetry,
    } = request;
    require_statistical(claim)?;
    let (statistical, reading) = reduce(claim)?;
    let form = statistical.form();
    if !form.admits(method) {
        return Err(StatisticalRefusal::NotStatistical { method, form });
    }
    let package = subject.simulation.package;
    let system = ModelSystem::new(subject.simulation)
        .map_err(|failure| StatisticalRefusal::Admission(Box::new(failure)))?;
    let group = if symmetry.is_empty() {
        None
    } else {
        Some(admit(&system, &symmetry).map_err(StatisticalRefusal::Symmetry)?)
    };
    let (shared, mut tests) = open(&system, package, claim, reading, &sampler, cancel)?;
    if let Some(group) = group {
        tests = orbit_representatives(&system, &group, &shared.initial, tests)?;
    }
    let mut model = ModelSampler {
        shared: Rc::new(RefCell::new(shared)),
    };
    check_sampled(
        SampledRequest {
            claim: &statistical,
            tests: &tests,
            method,
            seed,
            sampler: &sampler,
            limits,
            max_witnesses,
        },
        &mut model,
        cancel,
    )
}

/// The sampler state over an admitted `system`, and the claim's tests in
/// subject order: one per (initial state, `over` binding) pair.
fn open<'s, 'a, 'c>(
    system: &'s ModelSystem<'a>,
    package: &'s CheckedPackage,
    claim: &'c CheckedProbabilisticClaim,
    reading: Reading<'c>,
    sampler: &DefinitionRef,
    cancel: &'s Cancel,
) -> Result<(Shared<'s, 'a, 'c>, Vec<TestSubject>), StatisticalRefusal> {
    let ProbScheduler::Workload(name) = &claim.scheduler else {
        return Err(fault("a-statistical-claim-is-under-a-workload"));
    };
    let workload = package
        .graph()
        .probabilistic_model()
        .workload(name)
        .ok_or_else(|| fault("claim-workload-is-declared"))?;
    let bindings = bindings(system, claim)?;
    let initial = system.initial();
    let mut tests = Vec::with_capacity(initial.len().saturating_mul(bindings.len()));
    for index in 0..initial.len() {
        let index = u32::try_from(index).map_err(|_| StatisticalRefusal::TooManyTests {
            count: initial.len().saturating_mul(bindings.len()),
        })?;
        tests.extend(bindings.iter().map(|(binding, _)| TestSubject {
            initial: index,
            binding: binding.clone(),
        }));
    }
    let generator =
        ProbabilisticSampler::new(system, workload, sampler).map_err(|stop| match stop {
            SampleStop::GeneratorMismatch { supplied } => {
                StatisticalRefusal::GeneratorMismatch { supplied }
            }
            _ => fault("a-sampler-refuses-only-its-generator"),
        })?;
    let shared = Shared {
        generator,
        system,
        reading,
        initial,
        bindings,
        truths: BTreeMap::new(),
        cancel,
    };
    Ok((shared, tests))
}

/// The least key that exchanges with `key` under a transposition fixing the
/// state `canonical`, `key` itself when none does: the key's block among the
/// keys the state cannot tell apart (FR-152).
fn block_key(
    system: &ModelSystem<'_>,
    group: &SymmetryGroup,
    canonical: (&ModelSimState, &[u8]),
    binding: &ModelInstance,
) -> Result<String, StatisticalRefusal> {
    let Some(class) = group.class_of(&binding.population, &binding.key) else {
        return Ok(binding.key.clone());
    };
    for member in &class.keys {
        if *member == binding.key {
            break;
        }
        let swap = Permutation::of(
            &binding.population,
            [
                (member.clone(), binding.key.clone()),
                (binding.key.clone(), member.clone()),
            ],
        );
        let swapped = system
            .permute(canonical.0, &swap)
            .map_err(|_| fault("a-canonical-state-permutes"))?;
        let swapped = system
            .state_key(&swapped)
            .map_err(|_| fault("a-permuted-state-has-a-key"))?;
        if swapped.0 == canonical.1 {
            return Ok(member.clone());
        }
    }
    Ok(binding.key.clone())
}

/// FR-189 "Symmetry": one test per orbit of (initial state, binding) pairs
/// under `group`, on the orbit's least pair in key order, the tests in
/// subject order. Two pairs share an orbit when their initial states reach
/// the same canonical state (FR-152) and their bindings, carried to it, are
/// the same key or keys that a transposition fixing that state exchanges, so
/// the orbits found are never larger than the true ones: a pair whose orbit
/// is missed only adds a test, which `m` counts, so no verdict is wrong.
fn orbit_representatives(
    system: &ModelSystem<'_>,
    group: &SymmetryGroup,
    initial: &[ModelSimState],
    tests: Vec<TestSubject>,
) -> Result<Vec<TestSubject>, StatisticalRefusal> {
    // An orbit: its canonical state's key and its binding's block. Its
    // least member so far: the initial state's key and the binding.
    type Orbit = (Vec<u8>, Option<(DeclarationKey, String)>);
    type Member = (Vec<u8>, Option<ModelInstance>);
    let mut least: BTreeMap<Orbit, (Member, usize)> = BTreeMap::new();
    for (position, test) in tests.iter().enumerate() {
        let start = usize::try_from(test.initial)
            .ok()
            .and_then(|index| initial.get(index))
            .ok_or_else(|| fault("a-test-starts-at-an-initial-state"))?;
        let key_of = |state: &ModelSimState| {
            system
                .state_key(state)
                .map(|(key, _)| key)
                .map_err(|_| fault("an-initial-state-has-a-key"))
        };
        let own = key_of(start)?;
        let (canonical, carry) = system
            .canonicalise(start, group)
            .map_err(|_| fault("an-initial-state-canonicalises"))?;
        let canonical_key = key_of(&canonical)?;
        let block = match &test.binding {
            None => None,
            Some(binding) => {
                let carried = ModelInstance {
                    population: binding.population.clone(),
                    key: carry.image(&binding.population, &binding.key).to_owned(),
                };
                let key = block_key(system, group, (&canonical, &canonical_key), &carried)?;
                Some((binding.population.clone(), key))
            }
        };
        let member = (own, test.binding.clone());
        match least.entry((canonical_key, block)) {
            Entry::Vacant(slot) => {
                slot.insert((member, position));
            }
            Entry::Occupied(mut slot) => {
                if member < slot.get().0 {
                    slot.insert((member, position));
                }
            }
        }
    }
    let mut kept: Vec<usize> = least.values().map(|(_, position)| *position).collect();
    kept.sort_unstable();
    Ok(kept
        .into_iter()
        .filter_map(|position| tests.get(position).cloned())
        .collect())
}

/// A test's `over` binding, and the reference its predicates read.
type Binding = (Option<ModelInstance>, Option<ObjectReference>);

/// Each `over` binding of `claim`, in instance order, with the reference
/// its predicates read; one unbound test for a claim with no `over`.
fn bindings(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
) -> Result<Vec<Binding>, StatisticalRefusal> {
    let Some(over) = &claim.over else {
        return Ok(vec![(None, None)]);
    };
    let ValueType::Reference(object) = &over.value_type else {
        return Err(fault("over-is-an-object-type"));
    };
    let instances = system
        .instances(&over.value_type)
        .map_err(|refusal| match refusal {
            NotSimulated::RequiresBound(bound) => StatisticalRefusal::RequiresBound(bound),
            _ => fault("over-instances-classify"),
        })?;
    instances
        .into_iter()
        .map(|instance| {
            let reference = system
                .bound_reference(*object, &instance)
                .map_err(StatisticalRefusal::Internal)?;
            Ok((Some(instance), Some(reference)))
        })
        .collect()
}

/// What a sample of the claim reads, and how its reading is judged.
enum Reading<'c> {
    /// A bounded event: FR-126's monitor over `h + 1` positions.
    Formula {
        formula: &'c CheckedTemporalFormula,
        profile: TemporalProfile,
        horizon: u64,
        /// The monitor and the predicates of its atoms, built at the first
        /// sample, so that a state limit stops the run (FR-126).
        compiled: Option<Compiled<'c>>,
    },
    /// A bounded measure, and the event or reduced event it is judged by.
    Measure {
        measure: Measure<'c>,
        judge: MeasureJudge,
    },
    /// A fraction, and the event it is judged by; `None` for a mean.
    Fraction {
        fraction: &'c FractionMeasure,
        event: Option<CheckedBound>,
    },
    /// A long-run fraction's predicate and weight (FR-190).
    LongRun {
        predicate: &'c CheckedPredicate,
        weight: Option<&'c Identifier>,
    },
}

/// A bounded event's FR-126 monitor, and the predicate of each of its atoms
/// in letter order.
struct Compiled<'c> {
    monitor: Box<Monitor<PropertyAutomaton>>,
    reads: Vec<(UndefinedLocus, &'c CheckedPredicate)>,
}

/// The halt of an automaton refusal: a state limit is FR-126's
/// `ResourceExhausted`; any other refusal of a checked event is a broken
/// invariant.
fn automaton_halt(error: AutomatonError) -> SampleHalt {
    match error {
        AutomatonError::StateLimit { limit } => SampleHalt::AutomatonStates { value: limit },
        _ => broken(),
    }
}

/// The halt of a monitor step that failed.
fn plan_halt(fault: PlanFault) -> SampleHalt {
    match fault {
        PlanFault::Automaton(error) => automaton_halt(error),
        PlanFault::Broken(_) => broken(),
    }
}

/// Build the monitor of `formula` under `profile` within FR-126's default
/// `max_automaton_states`.
fn compile<'c>(
    formula: &'c CheckedTemporalFormula,
    profile: TemporalProfile,
) -> Result<Compiled<'c>, SampleHalt> {
    let automaton = translate_body(formula.formula(), profile, DEFAULT_MAX_AUTOMATON_STATES)
        .map_err(automaton_halt)?;
    let reads = automaton
        .atoms()
        .iter()
        .map(|atom| {
            formula
                .condition(atom.index())
                .map(|predicate| (UndefinedLocus::Atom(*atom), predicate))
                .ok_or_else(broken)
        })
        .collect::<Result<_, _>>()?;
    let monitor =
        Monitor::bad_prefix(automaton, DEFAULT_MAX_AUTOMATON_STATES).map_err(automaton_halt)?;
    Ok(Compiled {
        monitor: Box::new(monitor),
        reads,
    })
}

/// An `accumulate` or `steps` measure over `[0, within]`.
struct Measure<'c> {
    /// The reward summed; `None` for `steps`, 1 per step.
    reward: Option<&'c Identifier>,
    from: &'c CheckedPredicate,
    until: &'c CheckedPredicate,
    within: u64,
}

/// How a measured sample becomes the sample the statistics count.
enum MeasureJudge {
    /// A probability bound over `M <= c` or `M >= c`.
    Compare {
        /// The measure's threshold `c`.
        threshold: CheckedBound,
        /// The probability bound's comparison.
        claim: Comparison,
    },
    /// A quantile claim's reduced event (ADR-024 PF-4).
    Quantile(MeasureEvent),
}

fn bound_of(bound: &CheckedBound) -> Bound {
    let (comparison, threshold) = match bound {
        CheckedBound::AtLeast(value) => (Comparison::AtLeast, value),
        CheckedBound::AtMost(value) => (Comparison::AtMost, value),
    };
    Bound {
        comparison,
        threshold: threshold.clone(),
    }
}

fn measure_of(measure: &BoundedMeasure) -> Option<Measure<'_>> {
    match measure {
        BoundedMeasure::Accumulate {
            reward,
            from,
            until,
            within,
        } => Some(Measure {
            reward: Some(reward),
            from,
            until,
            within: *within,
        }),
        BoundedMeasure::Steps {
            from,
            until,
            within,
        } => Some(Measure {
            reward: None,
            from,
            until,
            within: *within,
        }),
        BoundedMeasure::Fraction(_) => None,
    }
}

/// `claim` as the statistics decide it, and what each sample reads.
fn reduce(
    claim: &CheckedProbabilisticClaim,
) -> Result<(StatisticalClaim, Reading<'_>), StatisticalRefusal> {
    let confidence = claim
        .confidence
        .as_ref()
        .ok_or(StatisticalRefusal::MissingConfidence)?;
    let confidence = Confidence::new(
        confidence.alpha.clone(),
        confidence.beta.clone(),
        confidence.indifference.clone(),
    )
    .map_err(|_| fault("checked-confidence-is-in-the-unit-interval"))?;
    let (form, bound, reading) = match &claim.form {
        ProbForm::Probability { bound, event } => {
            let reading = match event {
                Event::Formula { formula, horizon } => {
                    let profile = TemporalProfile::from_identity(claim.profile.identity())
                        .ok_or_else(|| fault("claim-profile-is-temporal"))?;
                    Reading::Formula {
                        formula,
                        profile,
                        horizon: *horizon,
                        compiled: None,
                    }
                }
                Event::Compare {
                    measure: BoundedMeasure::Fraction(fraction),
                    bound,
                } => Reading::Fraction {
                    fraction,
                    event: Some(bound.clone()),
                },
                Event::Compare {
                    measure,
                    bound: threshold,
                } => Reading::Measure {
                    measure: measure_of(measure).ok_or_else(|| fault("a-measure-is-bounded"))?,
                    judge: MeasureJudge::Compare {
                        threshold: threshold.clone(),
                        claim: bound_of(bound).comparison,
                    },
                },
            };
            (ClaimForm::Probability, bound_of(bound), reading)
        }
        ProbForm::Quantile {
            q,
            measure,
            threshold,
        } => {
            let (comparison, c) = match threshold {
                CheckedBound::AtLeast(c) => (QuantileComparison::AtLeast, c),
                CheckedBound::AtMost(c) => (QuantileComparison::AtMost, c),
            };
            let (event, bound) = reduce_quantile(q.clone(), comparison, c.clone());
            let measure = measure_of(measure).ok_or_else(|| fault("a-quantile-measure-ends"))?;
            (
                ClaimForm::Quantile,
                bound,
                Reading::Measure {
                    measure,
                    judge: MeasureJudge::Quantile(event),
                },
            )
        }
        ProbForm::MeanOfFraction { bound, fraction } => (
            ClaimForm::MeanOfFraction,
            bound_of(bound),
            Reading::Fraction {
                fraction,
                event: None,
            },
        ),
        ProbForm::LongRunFraction {
            bound,
            predicate,
            weight,
        } => (
            ClaimForm::LongRunFraction,
            bound_of(bound),
            Reading::LongRun {
                predicate,
                weight: weight.as_ref(),
            },
        ),
        // `require_statistical` refused every exact-only form.
        ProbForm::Reach { .. } | ProbForm::ExpectedReward { .. } => {
            return Err(StatisticalRefusal::ExactOnlyForm)
        }
    };
    let claim = StatisticalClaim::new(form, bound, confidence)
        .map_err(|_| fault("a-checked-claim-has-an-indifference-region"))?;
    Ok((claim, reading))
}

/// The run's sampler state, shared by its regeneration walks.
struct Shared<'s, 'a, 'c> {
    generator: ProbabilisticSampler<'s, 'a>,
    system: &'s ModelSystem<'a>,
    reading: Reading<'c>,
    initial: Vec<ModelSimState>,
    bindings: Vec<Binding>,
    /// The readings of the claim's predicates by binding and state key. It
    /// grows with the states a run visits; no FR-191 budget bounds it, a known
    /// memory cost over a large reachable space.
    truths: BTreeMap<(usize, Vec<u8>), Vec<PredicateReading>>,
    cancel: &'s Cancel,
}

/// FR-189's sampler over a model subject: FR-188's behaviours, read by the
/// layer-5 evaluator.
struct ModelSampler<'s, 'a, 'c> {
    shared: Rc<RefCell<Shared<'s, 'a, 'c>>>,
}

/// A predicate reading at a position: the truths, in read order, or the
/// first that has no value.
type Truths = Result<Vec<bool>, Box<UndefinedEvaluation>>;

/// A sample's or a walk's halt, or an undecided successor, which ends a
/// sample as a sample.
enum Ended {
    Halt(SampleHalt),
    Undecided,
}

impl From<SampleStop> for Ended {
    fn from(stop: SampleStop) -> Self {
        Self::Halt(match stop {
            SampleStop::UndecidedSuccessor => return Self::Undecided,
            SampleStop::GeneratorMismatch { supplied } => {
                SampleHalt::GeneratorMismatch { supplied }
            }
            SampleStop::Expansion(stop) => SampleHalt::Expansion(stop),
            SampleStop::NotMarkov(cause) => SampleHalt::NotMarkov(Box::new(cause)),
            SampleStop::TotalAboveDrawRange(range) => {
                SampleHalt::TotalAboveDrawRange { total: range.total }
            }
            SampleStop::Cancelled(cause) => SampleHalt::Cancelled(cause),
        })
    }
}

impl From<SampleHalt> for Ended {
    fn from(halt: SampleHalt) -> Self {
        Self::Halt(halt)
    }
}

/// `runtime_invariant`/`established-invariant-broken`: a stop of a broken
/// invariant, which settles `failed` (FR-192).
fn broken() -> SampleHalt {
    SampleHalt::Expansion(qsl_eval::simulation::ExpansionStop {
        cause: CatalogCode::new("runtime_invariant", "established-invariant-broken"),
        limit: None,
    })
}

/// A step's reward `name`, as a magnitude in the reward's own unit.
fn magnitude(step: &SampledStep, name: &Identifier) -> Result<Rational, SampleHalt> {
    let (_, value) = step
        .rewards
        .iter()
        .find(|(reward, _)| reward == name)
        .ok_or_else(broken)?;
    match value {
        Value::Integer(integer) => Ok(Rational::from_integer(integer.clone())),
        Value::Quantity(quantity) => Ok(quantity.magnitude().clone()),
        _ => Err(broken()),
    }
}

/// One behaviour being drawn: its position, its steps, and where its next
/// step draws.
struct Cursor {
    position: SampledPosition,
    steps: Vec<SampledStep>,
    seed: u64,
    trace: u64,
}

impl Shared<'_, '_, '_> {
    /// The binding index of `test`, and the test's start.
    fn start(&self, test: &TestSubject) -> Result<(usize, ModelSimState), SampleHalt> {
        let binding = self
            .bindings
            .iter()
            .position(|(binding, _)| *binding == test.binding)
            .ok_or_else(broken)?;
        let start = usize::try_from(test.initial)
            .ok()
            .and_then(|index| self.initial.get(index))
            .ok_or_else(broken)?;
        Ok((binding, start.clone()))
    }

    fn cursor(&self, start: ModelSimState, seed: u64, trace: u64) -> Result<Cursor, Ended> {
        Ok(Cursor {
            position: self.generator.position(start)?,
            steps: Vec::new(),
            seed,
            trace,
        })
    }

    /// Take `cursor`'s next step, after polling the cancellation handle.
    fn advance(&mut self, cursor: &mut Cursor) -> Result<(), Ended> {
        if let Some(cause) = self.cancel.cause() {
            return Err(SampleHalt::Cancelled(cause).into());
        }
        let step = u64::try_from(cursor.steps.len()).map_err(|_| broken())?;
        let at = StepAt {
            seed: cursor.seed,
            trace: cursor.trace,
            step,
        };
        let (position, taken) = self.generator.step(&cursor.position, at)?;
        cursor.position = position;
        cursor.steps.push(taken);
        Ok(())
    }

    /// The predicates `reads` names at `position`, with the
    /// binding `binding` bound, at the sample's position `at`.
    fn read(
        &mut self,
        binding: usize,
        position: &SampledPosition,
        at: u64,
        reads: &[(UndefinedLocus, &CheckedPredicate)],
    ) -> Result<Truths, SampleHalt> {
        let key = (binding, position.key().to_vec());
        let readings = match self.truths.get(&key) {
            Some(known) => known.clone(),
            None => {
                let over = self
                    .bindings
                    .get(binding)
                    .and_then(|(_, over)| over.as_ref());
                let predicates: Vec<(&CheckedPredicate, Option<&ObjectReference>)> = reads
                    .iter()
                    .map(|(_, predicate)| (*predicate, over))
                    .collect();
                let readings = self
                    .system
                    .state_predicates(position.state(), &predicates)
                    .map_err(SampleHalt::Expansion)?;
                self.truths.insert(key, readings.clone());
                readings
            }
        };
        let mut truths = Vec::with_capacity(readings.len());
        for ((locus, _), reading) in reads.iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    return Ok(Err(Box::new(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: at,
                            state_key: position.digest(),
                            locus: locus.clone(),
                            transition: None,
                        },
                        cause,
                    })))
                }
                // As EN-5 returns it (FR-196 `PredicateRefused`).
                PredicateReading::Refused(outcome) => {
                    return Err(SampleHalt::PredicateRefused(outcome))
                }
            }
        }
        Ok(Ok(truths))
    }
}

/// What a sample read before its claim was decided.
enum Read {
    Sample(Sample),
    Undefined(Box<UndefinedEvaluation>),
}

/// `value` against `bound`: a censored measure exceeds every threshold.
fn compares(bound: &CheckedBound, value: &MeasureValue) -> bool {
    match (bound, value) {
        (CheckedBound::AtMost(c), MeasureValue::Value(value)) => value <= c,
        (CheckedBound::AtLeast(c), MeasureValue::Value(value)) => value >= c,
        (CheckedBound::AtMost(_), MeasureValue::Censored) => false,
        (CheckedBound::AtLeast(_), MeasureValue::Censored) => true,
    }
}

impl<'c> Shared<'_, '_, 'c> {
    /// Draw and read one sample of `test` at `trace` (FR-189 "Per sample").
    fn sample(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Result<Drawn, SampleHalt> {
        let (binding, start) = self.start(test)?;
        let mut cursor = match self.cursor(start, seed, trace) {
            Ok(cursor) => cursor,
            Err(Ended::Halt(halt)) => return Err(halt),
            Err(Ended::Undecided) => return Err(broken()),
        };
        let read = match &self.reading {
            Reading::Formula { .. } => self.formula(binding, &mut cursor),
            Reading::Measure { .. } => self.measure(binding, &mut cursor),
            Reading::Fraction { .. } => self.fraction(binding, &mut cursor),
            Reading::LongRun { .. } => Err(Ended::Halt(broken())),
        };
        let sample = match read {
            Ok(Read::Sample(sample)) => sample,
            Ok(Read::Undefined(evaluation)) => Sample::Undefined(*evaluation),
            Err(Ended::Undecided) => Sample::UndecidedSuccessor,
            Err(Ended::Halt(halt)) => return Err(halt),
        };
        Ok(Drawn {
            sample,
            steps: cursor.steps,
        })
    }

    /// A bounded event: the monitor reads one letter per position until it
    /// decides. At a terminal model state (ADR-018 SM-4), and after the
    /// `h + 1`-th letter, the profile's false extension closes it (FR-126),
    /// as EN-5's product does.
    fn formula(&mut self, binding: usize, cursor: &mut Cursor) -> Result<Read, Ended> {
        let Reading::Formula {
            formula,
            profile,
            horizon,
            compiled,
        } = &mut self.reading
        else {
            return Err(broken().into());
        };
        let horizon = *horizon;
        if compiled.is_none() {
            *compiled = Some(compile(formula, *profile)?);
        }
        let reads = compiled.as_ref().ok_or_else(broken)?.reads.clone();
        let mut state: Option<StateId> = None;
        for at in 0..=horizon {
            if at > 0 {
                self.advance(cursor)?;
            }
            let truths = match self.read(binding, &cursor.position, at, &reads)? {
                Ok(truths) => truths,
                Err(undefined) => return Ok(Read::Undefined(undefined)),
            };
            let monitor = self.monitor()?;
            let from = match state {
                Some(state) => state,
                None => monitor.initial().map_err(automaton_halt)?,
            };
            let next = match read_letter(monitor, from, &truths).map_err(plan_halt)? {
                FormulaState::Decided(verdict) => {
                    return Ok(Read::Sample(Sample::Event(
                        verdict == MonitorVerdict::Accept,
                    )))
                }
                FormulaState::Running(next) => next,
            };
            state = Some(next);
            if at < horizon && self.generator.is_terminal(&cursor.position)? {
                break;
            }
        }
        let running = state.ok_or_else(broken)?;
        let rejects = false_extension_rejects(self.monitor()?, running).map_err(automaton_halt)?;
        Ok(Read::Sample(Sample::Event(!rejects)))
    }

    /// The bounded event's monitor, once compiled.
    fn monitor(&mut self) -> Result<&mut Monitor<PropertyAutomaton>, SampleHalt> {
        match &mut self.reading {
            Reading::Formula {
                compiled: Some(compiled),
                ..
            } => Ok(&mut compiled.monitor),
            _ => Err(broken()),
        }
    }

    /// An `accumulate` or `steps` measure (ADR-024 PF-2): the first
    /// activation `a <= h`, the first end `b` with `a <= b <= h`, and the
    /// reward of the steps into `a + 1` to `b`.
    fn measure(&mut self, binding: usize, cursor: &mut Cursor) -> Result<Read, Ended> {
        let Reading::Measure { measure, .. } = &self.reading else {
            return Err(broken().into());
        };
        let (reward, within) = (measure.reward, measure.within);
        let reads: Vec<(UndefinedLocus, &'c CheckedPredicate)> = vec![
            (
                UndefinedLocus::ClaimPredicate(ClaimPredicate::From),
                measure.from,
            ),
            (
                UndefinedLocus::ClaimPredicate(ClaimPredicate::Until),
                measure.until,
            ),
        ];
        let mut active = false;
        let mut amount = Rational::from_integer(Integer::zero());
        let mut value = None;
        for at in 0..=within {
            if at > 0 {
                self.advance(cursor)?;
                if active {
                    let step = cursor.steps.last().ok_or_else(broken)?;
                    let added = match reward {
                        Some(name) => magnitude(step, name)?,
                        None => Rational::from_integer(Integer::one()),
                    };
                    amount = amount.add(&added);
                }
            }
            let truths = match self.read(binding, &cursor.position, at, &reads)? {
                Ok(truths) => truths,
                Err(undefined) => return Ok(Read::Undefined(undefined)),
            };
            let [from, until] = truths[..] else {
                return Err(broken().into());
            };
            active = active || from;
            if active && until {
                value = Some(MeasureValue::Value(amount));
                break;
            }
        }
        let Reading::Measure { judge, .. } = &self.reading else {
            return Err(broken().into());
        };
        let value = match (value, judge) {
            (Some(value), _) => value,
            (None, _) if active => MeasureValue::Censored,
            // ADR-024 PF-4, ST-6: a quantile is conditioned on activation.
            (None, MeasureJudge::Quantile(_)) => return Ok(Read::Sample(Sample::NotActivated)),
            // PF-3's probability is over every behaviour, and FR-189 gives
            // the comparison no value on one that never activates: it is
            // counted on the side that violates the bound, so it never helps
            // the claim be accepted.
            (None, MeasureJudge::Compare { claim, .. }) => {
                let violating = match claim {
                    Comparison::AtLeast => false,
                    Comparison::AtMost | Comparison::Below => true,
                };
                return Ok(Read::Sample(Sample::Event(violating)));
            }
        };
        Ok(Read::Sample(match judge {
            MeasureJudge::Compare { threshold, .. } => Sample::Event(compares(threshold, &value)),
            MeasureJudge::Quantile(event) => Sample::Measured {
                holds: event.holds(&value),
                value,
            },
        }))
    }

    /// A fraction of the positions `[0, h]` (ADR-024 PF-2): unweighted over
    /// `h + 1`, or each position weighted by the reward of the step that
    /// leaves it, so a weighted fraction draws the step leaving position `h`.
    fn fraction(&mut self, binding: usize, cursor: &mut Cursor) -> Result<Read, Ended> {
        let Reading::Fraction { fraction, event } = &self.reading else {
            return Err(broken().into());
        };
        let (fraction, event) = (*fraction, event.clone());
        let reads = vec![(
            UndefinedLocus::ClaimPredicate(ClaimPredicate::Target),
            &fraction.predicate,
        )];
        let zero = Rational::from_integer(Integer::zero());
        let (mut held, mut total) = (zero.clone(), zero);
        // The state key at the window's last position.
        let mut last = None;
        for at in 0..=fraction.window {
            if at > 0 && fraction.weight.is_none() {
                self.advance(cursor)?;
            }
            let truths = match self.read(binding, &cursor.position, at, &reads)? {
                Ok(truths) => truths,
                Err(undefined) => return Ok(Read::Undefined(undefined)),
            };
            let holds = truths.first().copied().ok_or_else(broken)?;
            last = Some(cursor.position.digest());
            let weight = match &fraction.weight {
                None => Rational::from_integer(Integer::one()),
                // The step that leaves the position weighs it.
                Some(name) => {
                    self.advance(cursor)?;
                    magnitude(cursor.steps.last().ok_or_else(broken)?, name)?
                }
            };
            if holds {
                held = held.add(&weight);
            }
            total = total.add(&weight);
        }
        // A weighted window whose every step has weight 0 divides 0 by 0:
        // the fraction is undefined at the window's last position (ADR-018
        // UE-1), which rejects the test (ADR-024 SV-11).
        let Some(value) = held.div(&total) else {
            return Ok(Read::Undefined(Box::new(UndefinedEvaluation {
                site: UndefinedSite {
                    position: fraction.window,
                    state_key: last.ok_or_else(broken)?,
                    locus: UndefinedLocus::ClaimPredicate(ClaimPredicate::Target),
                    transition: None,
                },
                cause: UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero),
            })));
        };
        Ok(Read::Sample(match event {
            Some(bound) => Sample::Event(compares(&bound, &MeasureValue::Value(value))),
            None => Sample::Fraction(UnitFraction::try_from(value).map_err(|_| broken())?),
        }))
    }
}

impl<'s, 'a, 'c> BehaviourSampler for ModelSampler<'s, 'a, 'c> {
    type Walk = ModelWalk<'s, 'a, 'c>;

    fn sample(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Result<Drawn, SampleHalt> {
        self.shared.borrow_mut().sample(test, seed, trace)
    }

    fn walk(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Self::Walk {
        ModelWalk {
            shared: Rc::clone(&self.shared),
            test: test.clone(),
            seed,
            trace,
            walk: None,
        }
    }
}

/// Where a regeneration walk stands.
struct Walking {
    binding: usize,
    start: ModelSimState,
    /// The initial state's key: a position with this key closes a cycle.
    initial: Vec<u8>,
    position: SampledPosition,
    /// Steps taken.
    taken: u64,
}

/// One regeneration walk of a long-run fraction claim (FR-190): one
/// behaviour from the test's initial state at trace index `j`, read one
/// position at a time.
struct ModelWalk<'s, 'a, 'c> {
    shared: Rc<RefCell<Shared<'s, 'a, 'c>>>,
    test: TestSubject,
    seed: u64,
    trace: u64,
    walk: Option<Walking>,
}

impl ModelWalk<'_, '_, '_> {
    fn step_or_end(&mut self) -> Result<WalkStep, Ended> {
        let mut shared = self.shared.borrow_mut();
        let Reading::LongRun { predicate, weight } = &shared.reading else {
            return Err(broken().into());
        };
        let (predicate, weight) = (*predicate, *weight);
        let walk = match self.walk.take() {
            Some(walk) => walk,
            None => {
                let (binding, start) = shared.start(&self.test)?;
                let position = shared.generator.position(start.clone())?;
                Walking {
                    binding,
                    start,
                    initial: position.key().to_vec(),
                    position,
                    taken: 0,
                }
            }
        };
        let reads = [(
            UndefinedLocus::ClaimPredicate(ClaimPredicate::Target),
            predicate,
        )];
        let holds = match shared.read(walk.binding, &walk.position, walk.taken, &reads)? {
            Ok(truths) => truths.first().copied().ok_or_else(broken)?,
            Err(evaluation) => {
                // The witness's steps: the walk again, to the position.
                let mut cursor = shared.cursor(walk.start.clone(), self.seed, self.trace)?;
                for _ in 0..walk.taken {
                    shared.advance(&mut cursor)?;
                }
                return Ok(WalkStep::Undefined {
                    evaluation: *evaluation,
                    steps: cursor.steps,
                });
            }
        };
        if let Some(cause) = shared.cancel.cause() {
            return Err(SampleHalt::Cancelled(cause).into());
        }
        let at = StepAt {
            seed: self.seed,
            trace: self.trace,
            step: walk.taken,
        };
        let (position, step) = shared.generator.step(&walk.position, at)?;
        let weight = match weight {
            None => Rational::from_integer(Integer::one()),
            Some(name) => magnitude(&step, name)?,
        };
        let returned = position.key() == walk.initial.as_slice();
        self.walk = Some(Walking {
            position,
            taken: walk.taken.checked_add(1).ok_or_else(broken)?,
            ..walk
        });
        Ok(WalkStep::Position {
            holds,
            weight: Weight::try_from(weight).map_err(|_| broken())?,
            returned,
        })
    }
}

impl RegenerationWalk for ModelWalk<'_, '_, '_> {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        match self.step_or_end() {
            Ok(step) => Ok(step),
            Err(Ended::Undecided) => Ok(WalkStep::UndecidedSuccessor),
            Err(Ended::Halt(halt)) => Err(halt),
        }
    }
}

/// FR-193's reader over a model subject: the sampler and claim of a
/// statistical run, kept to read the claim again on a recomputed sample.
pub struct ModelWitnessReader<'s, 'a, 'c> {
    shared: Rc<RefCell<Shared<'s, 'a, 'c>>>,
    bound: Bound,
}

/// The reader of `claim` over the admitted `system` of `package`, drawing
/// with `sampler` (FR-193 "Replay").
///
/// # Errors
///
/// The refusals of [`check_statistical`] that depend on the claim and the
/// sampler alone: [`StatisticalRefusal::EveryScheduler`],
/// [`StatisticalRefusal::ExactOnlyForm`],
/// [`StatisticalRefusal::MissingConfidence`],
/// [`StatisticalRefusal::RequiresBound`] and
/// [`StatisticalRefusal::GeneratorMismatch`].
pub fn witness_reader<'s, 'a, 'c>(
    system: &'s ModelSystem<'a>,
    package: &'s CheckedPackage,
    claim: &'c CheckedProbabilisticClaim,
    sampler: &DefinitionRef,
    cancel: &'s Cancel,
) -> Result<ModelWitnessReader<'s, 'a, 'c>, StatisticalRefusal> {
    require_statistical(claim)?;
    let (statistical, reading) = reduce(claim)?;
    let bound = statistical.bound().clone();
    let (shared, _) = open(system, package, claim, reading, sampler, cancel)?;
    Ok(ModelWitnessReader {
        shared: Rc::new(RefCell::new(shared)),
        bound,
    })
}

impl From<SampleHalt> for WitnessReadStop {
    fn from(halt: SampleHalt) -> Self {
        let stop = match halt {
            SampleHalt::NotMarkov(cause) => SampleStop::NotMarkov(*cause),
            SampleHalt::TotalAboveDrawRange { total } => {
                SampleStop::TotalAboveDrawRange(TotalAboveDrawRange { total })
            }
            SampleHalt::Expansion(stop) => SampleStop::Expansion(stop),
            SampleHalt::GeneratorMismatch { supplied } => {
                SampleStop::GeneratorMismatch { supplied }
            }
            SampleHalt::Cancelled(cause) => SampleStop::Cancelled(cause),
            SampleHalt::AutomatonStates { value } => {
                return Self::AutomatonStates { value };
            }
            SampleHalt::PredicateRefused(outcome) => {
                return Self::PredicateRefused(Box::new(outcome));
            }
        };
        Self::Sample(Box::new(stop))
    }
}

impl WitnessReader for ModelWitnessReader<'_, '_, '_> {
    fn read(
        &mut self,
        test: &TestProvenance,
        seed: u64,
        trace: u64,
        steps: u64,
    ) -> Result<WitnessReading, WitnessReadStop> {
        let subject = TestSubject {
            initial: test.initial,
            binding: test.binding.clone(),
        };
        let long_run = matches!(self.shared.borrow().reading, Reading::LongRun { .. });
        if long_run {
            // A long-run fraction keeps only an undefined witness: walk to
            // the recorded position.
            let mut walk = ModelWalk {
                shared: Rc::clone(&self.shared),
                test: subject,
                seed,
                trace,
                walk: None,
            };
            for _ in 0..=steps {
                match walk.step_or_end() {
                    Ok(WalkStep::Undefined { evaluation, steps }) => {
                        return Ok(WitnessReading {
                            value: Some(WitnessValue::Undefined(evaluation)),
                            steps,
                        })
                    }
                    Ok(WalkStep::Position { .. }) => {}
                    Ok(WalkStep::UndecidedSuccessor) | Err(Ended::Undecided) => break,
                    Err(Ended::Halt(halt)) => return Err(halt.into()),
                }
            }
            return Ok(WitnessReading {
                value: None,
                steps: Vec::new(),
            });
        }
        let Drawn { sample, steps } = self.shared.borrow_mut().sample(&subject, seed, trace)?;
        let value = match sample {
            Sample::Undefined(evaluation) => Some(WitnessValue::Undefined(evaluation)),
            other => violating(&self.bound, &other),
        };
        Ok(WitnessReading { value, steps })
    }
}
