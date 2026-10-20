// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-183 Inputs (ADR-023 HX-1): a hyper counterexample, the payload of a
//! `WitnessEnvelope<HyperCounterexample>`, mirroring QSpec FR-400 member for
//! member.
//!
//! This is the `lockstep` (HP-2), `witness-exhausted` (HP-3), `projected`
//! (HP-6) and `undefined` arms that FR-176's, FR-177's and FR-178's EN-1
//! engines return. The `step-tuple` arm joins with the engine that returns it
//! (FR-179).

use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::FormulaNodeId;
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_check::counterexample::ModelStep;
use crate::witness::FamilyPayload;

/// One step of a hyper trace: a model step, or the stutter marker of a
/// terminal component (QSpec FR-400 `HyperStep`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperStep {
    /// An FR-120 transition identity with its post-state's digest.
    Model(ModelStep),
    /// The terminal stutter step. Its post-state is its pre-state.
    TerminalStutter,
}

/// Whether a step of a `Projected` counterexample's trace is skipped or
/// visible (ADR-023 PA-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepMark {
    /// The step's operation is in the clause's `align skip` list.
    Skipped,
    /// The step's operation is not.
    Visible,
}

/// One trace of a tuple: the behaviour of one trace variable (QSpec FR-400
/// `HyperTrace`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperTrace {
    /// The trace variable.
    pub variable: String,
    /// Its subject's model alias.
    pub alias: String,
    /// The index of its first state among the subject's initial states, in
    /// the subject's order.
    pub initial: u32,
    /// Its steps, in order.
    pub steps: Vec<HyperStep>,
    /// For a `Projected` counterexample, the mark of each step, in step
    /// order; empty for every other kind.
    pub marks: Vec<StepMark>,
}

/// The expression of a hyper clause that has no value (QSpec FR-400
/// `UndefinedEvaluation` `where.locus`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperLocus {
    /// A `holds` atom of the clause's body.
    Atom(FormulaNodeId),
    /// A universal conjunct of the clause's `match` block, by its index.
    Match {
        /// The conjunct's index among the clause's universal conjuncts.
        conjunct: usize,
    },
    /// A conjunct of the clause's `match` block that reads an existential
    /// variable, by its index among those conjuncts.
    ExistentialMatch {
        /// The conjunct's index among the clause's existential conjuncts.
        conjunct: usize,
    },
    /// A `pre` or `post` clause of an application's contract conjunction,
    /// by its name; it reads one component's state.
    Contract(String),
}

/// QSpec FR-400's `UndefinedEvaluation`: where a hyper evaluation was
/// undefined, and why. A hyper evaluation point is one joint position over
/// several components, so it names every component's state there; a contract
/// reads one component's state and names that component alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperUndefined {
    /// The joint position.
    pub position: u64,
    /// The `quire.simulation.state-key/v1` digest of each named component's
    /// state at `position`, in quantifier order.
    pub state_keys: Vec<DigestRecord>,
    /// The trace variables the evaluation reads with their aliases, in the
    /// same order.
    pub variables: Vec<(String, String)>,
    /// The expression that has no value.
    pub locus: HyperLocus,
    /// For a contract's guard only: the transition identity it guards.
    pub transition: Option<ModelTransition>,
    /// Why the expression has no value.
    pub cause: UndefinedCause,
}

/// What a hyper counterexample refutes (QSpec FR-400 `kind`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperCounterexampleKind {
    /// HP-2: a fair tuple of universal lassos, matched at every joint step,
    /// on which the body is false.
    Lockstep {
        /// The joint position at which every component's loop re-enters.
        loop_entry: u64,
    },
    /// HP-3: a fair universal lasso on which no existential tuple stays
    /// matched; its traces are the universal variables' alone.
    WitnessExhausted {
        /// The joint position at which every component's loop re-enters.
        loop_entry: u64,
        /// The first joint position at which the witness set is empty.
        position: u64,
    },
    /// HP-6: one finite trace per variable to a rejecting state of the body's
    /// safety monitor, each step marked skipped or visible. Every trace has
    /// the same number of visible steps and the traces are of unequal total
    /// length (ADR-023 PA-6); there is no loop, and `trace_position` is the
    /// projected position the visible steps reach.
    Projected,
    /// Any form: the traces are finite paths to the state an undefined
    /// evaluation reads, one per trace variable (ADR-023 HV-8).
    Undefined(HyperUndefined),
}

/// FR-183's `HyperCounterexample`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperCounterexample {
    /// What it refutes.
    pub kind: HyperCounterexampleKind,
    /// The clause instance: each object parameter and the universe key it is
    /// bound to, in declaration order; empty for a clause with no object
    /// parameter.
    pub instance: Vec<(String, String)>,
    /// One trace per trace variable, in quantifier order.
    pub traces: Vec<HyperTrace>,
    /// The failing joint position (ADR-014 TR-2).
    pub trace_position: u64,
}

impl FamilyPayload for HyperCounterexample {
    /// The inline size plus each trace's steps and the strings it holds.
    fn measured_bytes(&self) -> usize {
        let traces: usize = self
            .traces
            .iter()
            .map(|trace| {
                std::mem::size_of::<HyperTrace>()
                    .saturating_add(trace.variable.len())
                    .saturating_add(trace.alias.len())
                    .saturating_add(
                        trace
                            .steps
                            .len()
                            .saturating_mul(std::mem::size_of::<HyperStep>()),
                    )
                    .saturating_add(
                        trace
                            .marks
                            .len()
                            .saturating_mul(std::mem::size_of::<StepMark>()),
                    )
            })
            .sum();
        let instance: usize = self
            .instance
            .iter()
            .map(|(parameter, key)| parameter.len().saturating_add(key.len()))
            .sum();
        std::mem::size_of_val(self)
            .saturating_add(traces)
            .saturating_add(instance)
    }
}

impl HyperCounterexample {
    /// The number of steps every trace of a `Lockstep` counterexample has,
    /// and of an `Undefined` one's paths in lockstep. A `Projected`
    /// counterexample's traces differ in step count.
    pub fn steps(&self) -> usize {
        self.traces.first().map_or(0, |trace| trace.steps.len())
    }
}
