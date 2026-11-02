// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244's zone certificate and FR-245's check of it in the qualified core
//! (ADR-026 CF-1, CF-3, CF-4, CF-6).
//!
//! EN-6, the zone search in `qsl-analyze`, returns a [`ZoneCertificate`]
//! with every proof. [`check_zone_certificate`] verifies it with the core's
//! own zones ([`super::zone`]), reading nothing from the search but the
//! certificate. It re-admits the subject, rebuilds its timed reading and
//! recomputes every successor of every node it reaches from the initial
//! states:
//!
//! 1. the `Malformed` members: the scale (every constant of the model and
//!    the claim exact at it), the LU bounds (each at least the
//!    bound the checker computes from the model), the nodes, the edges, and
//!    the component part (present for a liveness clause only) and
//!    divergence part (for the time-lock-freedom item only), each shaped to
//!    the nodes;
//! 2. the initial coverage: each initial symbolic state is covered by a
//!    named initial node of its discrete state (`InitialMissing`, at the
//!    initial discrete state);
//! 3. the nodes in node order: every symbolic successor has an edge of its
//!    transition identity and guard part to a node of its discrete state
//!    whose zone covers it under the aLU simulation with the certificate's
//!    bounds (`SuccessorMissing`); no node is bad unless it is `locked`, and
//!    from a `locked` node no quiescent node and no cycle that lets time
//!    diverge is reachable (`BadState`);
//! 4. for a liveness clause, the component part (`product`): every edge
//!    goes to an equal or greater component (`BackwardEdge`, at the edge),
//!    and each component's reason holds (`WitnessFails`, at the component);
//! 5. for the time-lock-freedom item, the divergence part (`divergence`):
//!    each node's path leads from it to a divergent target, and its every
//!    valuation diverges (`WitnessFails`, at the node);
//! 6. `admitted`: a path of such edges from an initial node to a quiescent
//!    node, a terminal node under a tick source, or a cycle that lets time
//!    advance (`WitnessFails`, at its last node).
//!
//! The first failure is the rejection. A node no edge reaches from an
//! initial node is read nowhere and checked for nothing.
//!
//! Coverage is read under the aLU simulation with the certificate's
//! bounds; for the deadlock-freedom item under the aM bisimulation
//! (`L = U = max(L, U)`), since the simulation does not preserve deadlocks.
//! A successor matches an edge by its transition identity, guard part,
//! horizon piece, post-state and automaton state, never by its transition
//! alone, and every successor is recomputed from the node's zone held to
//! its state's own invariants.
//!
//! Time divergence is checked by sufficient conditions the checker proves
//! itself, never by the search's evidence. The `admitted` path is
//! recomputed in exact zones from the initial state. Its cycle target lets
//! time advance when, under a tick source, it takes a tick, and under a
//! dense source, some clock it resets to `r` is guarded on it by `x >= c`
//! (or `x > c`) with `c > r` before its next reset, so each repetition
//! takes at least `c - r`; and it repeats forever: the greatest set of the
//! exact valuations at its start each of which takes the cycle back into
//! the set is not empty. A strongly connected set of nodes lets no time
//! diverge when every member is urgent, or some clock that no edge inside
//! it resets is bounded above by every member state's own time invariants.
//! A certificate whose divergence needs more than these is rejected, never
//! accepted on trust.
//!
//! Supported items are a TT-1 invariant (`always holds(p)`), a TT-2 window
//! claim, whose automaton the checker rebuilds with qsl-eval's own
//! translation (FR-242), any other infinite-trace or timed clause on
//! FR-240's liveness product, whose negation automaton it rebuilds the same
//! way, and the deadlock-freedom and time-lock-freedom items. A
//! certificate numbers its automaton states in its own order; the checker
//! binds each number to the state it recomputes, one to one.
//!
//! A timed refinement's safety half (QSpec FR-430) is checked in FR-244's
//! refinement form: the part's refinement is compared
//! with the recompiled one first; each node's second component is a
//! position, not an automaton state; `check_initial` and `check_step`,
//! run on the listed positions, replace the claim automaton and its
//! badness (`InitialMismatch`, `StepFails`, `PostDiffers`); the locked
//! region, entered only by a failing step into a `locked` entry node, is
//! read on concrete steps alone; and `admitted` starts at a positioned
//! initial node of the certificate's `initials` (QSpec FR-418).

mod divergence;
mod product;

use num_integer::Integer as NumInteger;
use quire_exact::NodeKey;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::hash::{Hash, Hasher};

use qsl_eval::automaton::{
    position_automaton, AutomatonError, ClockedEdge, Read, Stamp, WindowClaim, WindowRefusal,
};
use qsl_eval::fairness::{components, EdgeLabel, FairnessError, Graph, Identity};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{
    GuardedStep, ModelSimState, ModelSimulationRequest, ModelSystem, PlaceConstraint,
    TerminalClass, TimedSubjectRefusal, TimedSystem,
};
use qsl_eval::simulation::StateKey;
use qsl_eval::simulation::SubjectTransition;
use qsl_eval::temporal::AtomReading;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::{CheckedPackage, EmitRefusal, EmittedPackage};
use qsl_semantics::check::{
    Arrival, CheckedRefinement, CheckedScheduleClaim, ClockOp, FormulaNodeId, ScheduleClaim,
    SchedulingPolicy, TaskSet, TemporalProfile, TimedPropertyForm, Wcet,
};
pub use qsl_semantics::check::{
    TaskAction, TaskSubtraction, TaskTransitionIdentity, TaskTransitionTemplate,
};
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef};
use quire_exact::{Cancel, Integer, Meter, Rational};
use quire_semantic_value::model_transition::ModelTransition;

use super::closure::CertificateRejection;
use super::simulation::{position_matches, CertifiedPosition, EvaluationOutcome};
use super::zone::{Exact, LuBounds, Zone, ZoneError};
use super::{CertificateLocus, CertificateMember, CertificateRule};
use crate::model_check::identity::node_occurrence;
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;
use crate::refinement::{
    check_initial, check_step, open_systems, ConcreteStep, RefinementPosition, StepVerdict,
    SubjectRefusal, Taken,
};

/// FR-244's certificate of a zone search's proof of the item `obligation`
/// names (ADR-013 O-09).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneCertificate<O> {
    /// The item it proves.
    pub obligation: O,
    /// The scale factor of every zone bound (FR-238): a bound `c` stands
    /// for `c / scale` time units.
    pub scale: Integer,
    /// Each clock place's `L` and `U`, in scaled units: the search uses one
    /// pair per place for every discrete state.
    pub lu_bounds: LuBounds,
    /// The nodes, in discovery order.
    pub nodes: Vec<CertNode>,
    /// The edges, by source node in discovery order and, from each node, in
    /// canonical transition order: one per convex part of an enabled
    /// step's guard (spec gap STD-493: FR-244 names one edge per transition
    /// identity).
    pub edges: Vec<CertEdge>,
    /// QSpec FR-418 `initials`, nonempty: the node that covers each
    /// initial state's symbolic state, one per initial state that admits a
    /// valuation, in the subject's initial-state order. FR-418 names no
    /// `initial` member; FR-244's single initial node is `initials[0]`.
    pub initials: Vec<usize>,
    /// FR-244's Büchi part (ADR-026 CF-2): for a claim decided on FR-240's
    /// liveness product, each node's component and each component's
    /// reason; `None` for a reachability proof.
    pub components: Option<Components>,
    /// FR-244's divergence part: for the time-lock-freedom item, each
    /// node's path to a divergent target, by node; `None` otherwise.
    pub divergence: Option<Vec<DivergenceWitness>>,
    /// The witness that the subject admits a time-divergent behaviour
    /// (ADR-026 TD-6).
    pub admitted: DivergenceWitness,
    /// FR-244's refinement part, for the safety half of a timed
    /// refinement (QSpec FR-430); `None` otherwise.
    pub refinement: Option<RefinementPart>,
    /// FR-418's complete fixed-work task-product contract and finite-product
    /// gate; absent for ordinary timed subjects.
    pub task_product: Option<TaskProductCertificate>,
}

/// QSpec FR-418's recomputable fixed-work task-product certificate data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductCertificate {
    /// The bounded-subtraction/diagonal abstraction version.
    pub abstraction: TaskProductAbstraction,
    /// Recompiled per-task queue bounds and exact work premises.
    pub queue_bounds: Vec<TaskQueueBound>,
    /// Every auxiliary-clock ceiling, in the common integer scale.
    pub clock_ceilings: Vec<TaskClockCeiling>,
    /// Every diagonal predicate preserved by refined regions.
    pub diagonal_constraints: Vec<TaskDiagonalConstraint>,
    /// Every typed bounded-subtraction transition.
    pub subtractions: Vec<TaskSubtraction>,
    /// The finite-product gate declaration; closure is recomputed by reader.
    pub time_progress: TaskTimeProgress,
}

/// The source and finite scheduler-control keys that make up one FR-418
/// task-product state. The encoded form is versioned and length-delimited so
/// the independent reader can recover each component without interpreting
/// one key as the other.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductStateKey {
    source: StateKey,
    scheduler_control: Vec<u8>,
}

impl TaskProductStateKey {
    /// Canonical finite-control key for a scheduler with no jobs, no running
    /// task, no activated arrival phase and no pending releases.
    pub fn empty_scheduler_control() -> Vec<u8> {
        let mut bytes = TASK_CONTROL_PREFIX.to_vec();
        bytes.extend_from_slice(&0_u64.to_be_bytes()); // jobs
        bytes.push(0); // running job
        bytes.extend_from_slice(&0_u64.to_be_bytes()); // activation phases
        bytes.extend_from_slice(&0_u64.to_be_bytes()); // pending releases
        bytes
    }

    /// Construct the complete product key from the model state's canonical
    /// key and the scheduler's canonical finite-control key.
    pub fn new(source: StateKey, scheduler_control: Vec<u8>) -> Self {
        Self {
            source,
            scheduler_control,
        }
    }

    /// The model-state component.
    pub fn source(&self) -> &StateKey {
        &self.source
    }

    /// The canonical finite scheduler-control component.
    pub fn scheduler_control(&self) -> &[u8] {
        &self.scheduler_control
    }

    /// Encode the complete key as a versioned, unambiguous byte string.
    pub fn into_state_key(self) -> Result<StateKey, TaskProductStateKeyError> {
        let source_len = u64::try_from(self.source.as_bytes().len())
            .map_err(|_| TaskProductStateKeyError::LengthOverflow)?;
        let scheduler_len = u64::try_from(self.scheduler_control.len())
            .map_err(|_| TaskProductStateKeyError::LengthOverflow)?;
        let total = TASK_PRODUCT_STATE_PREFIX
            .len()
            .checked_add(8)
            .and_then(|size| size.checked_add(self.source.as_bytes().len()))
            .and_then(|size| size.checked_add(8))
            .and_then(|size| size.checked_add(self.scheduler_control.len()))
            .ok_or(TaskProductStateKeyError::LengthOverflow)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve(total)
            .map_err(|_| TaskProductStateKeyError::AllocationFailed)?;
        bytes.extend_from_slice(TASK_PRODUCT_STATE_PREFIX);
        bytes.extend_from_slice(&source_len.to_be_bytes());
        bytes.extend_from_slice(self.source.as_bytes());
        bytes.extend_from_slice(&scheduler_len.to_be_bytes());
        bytes.extend_from_slice(&self.scheduler_control);
        Ok(StateKey::from_canonical_bytes(bytes))
    }

    /// Decode a task-product state key. A key without the version prefix is
    /// an ordinary model-state key; a prefixed but malformed key is refused.
    pub fn decode(
        key: &StateKey,
    ) -> Result<Option<TaskProductStateKeyRef<'_>>, TaskProductStateKeyError> {
        let bytes = key.as_bytes();
        if !bytes.starts_with(TASK_PRODUCT_STATE_PREFIX) {
            return Ok(None);
        }
        let mut cursor = TASK_PRODUCT_STATE_PREFIX.len();
        let source_len = take_u64(bytes, &mut cursor)?;
        let source = take_bytes(bytes, &mut cursor, source_len)?;
        let scheduler_len = take_u64(bytes, &mut cursor)?;
        let scheduler_control = take_bytes(bytes, &mut cursor, scheduler_len)?;
        if cursor != bytes.len() {
            return Err(TaskProductStateKeyError::TrailingBytes);
        }
        let decoded = TaskProductStateKeyRef {
            source,
            scheduler_control,
        };
        if !decoded.has_valid_scheduler_control() {
            return Err(TaskProductStateKeyError::InvalidSchedulerControl);
        }
        Ok(Some(decoded))
    }
}

/// Borrowed source and scheduler-control components decoded from a complete
/// FR-418 task-product state key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskProductStateKeyRef<'a> {
    source: &'a [u8],
    scheduler_control: &'a [u8],
}

impl TaskProductStateKeyRef<'_> {
    /// The source model's canonical state-key bytes.
    pub fn source(&self) -> &[u8] {
        self.source
    }

    /// The scheduler's canonical finite-control key bytes.
    pub fn scheduler_control(&self) -> &[u8] {
        self.scheduler_control
    }

    /// Parse the independently validated finite scheduler-control component.
    ///
    /// The reader uses these rows to reconstruct task successors; the parser
    /// exposes no producer search code or DBM operations.
    pub fn scheduler_state(&self) -> Result<TaskSchedulerControl, TaskProductStateKeyError> {
        decode_scheduler_control(self.scheduler_control)
    }

    fn has_valid_scheduler_control(&self) -> bool {
        let bytes = self.scheduler_control;
        let prefix = TASK_CONTROL_PREFIX;
        if !bytes.starts_with(prefix) {
            return false;
        }
        let mut cursor = prefix.len();
        let Some(job_count) = control_count(bytes, &mut cursor) else {
            return false;
        };
        let mut jobs = Vec::new();
        for _ in 0..job_count {
            let Some(taskset) = control_text(bytes, &mut cursor) else {
                return false;
            };
            let Some(task) = control_text(bytes, &mut cursor) else {
                return false;
            };
            let Some(rank) = control_u64(bytes, &mut cursor) else {
                return false;
            };
            let Some(slot) = control_u64(bytes, &mut cursor) else {
                return false;
            };
            let Some(release) = control_bytes(bytes, &mut cursor) else {
                return false;
            };
            if !valid_transition_bytes(release) {
                return false;
            }
            let Some(phase) = control_byte(bytes, &mut cursor) else {
                return false;
            };
            if phase > 4 {
                return false;
            }
            let Some(demand) = control_text(bytes, &mut cursor) else {
                return false;
            };
            if demand.is_empty() {
                return false;
            }
            let Some(priority_tag) = control_byte(bytes, &mut cursor) else {
                return false;
            };
            match priority_tag {
                0 => {}
                1 if control_u64(bytes, &mut cursor).is_some() => {}
                _ => return false,
            }
            jobs.push((taskset, task, rank, slot, release.to_vec(), phase));
        }
        if jobs
            .windows(2)
            .any(|pair| (&pair[0].0, &pair[0].1, pair[0].2) >= (&pair[1].0, &pair[1].1, pair[1].2))
        {
            return false;
        }
        let running = match control_byte(bytes, &mut cursor) {
            Some(0) => None,
            Some(1) => {
                let Some(taskset) = control_text(bytes, &mut cursor) else {
                    return false;
                };
                let Some(task) = control_text(bytes, &mut cursor) else {
                    return false;
                };
                let Some(slot) = control_u64(bytes, &mut cursor) else {
                    return false;
                };
                let Some(release) = control_bytes(bytes, &mut cursor) else {
                    return false;
                };
                if !valid_transition_bytes(release) {
                    return false;
                }
                Some((taskset, task, slot, release.to_vec()))
            }
            _ => return false,
        };
        let Some(activation_count) = control_count(bytes, &mut cursor) else {
            return false;
        };
        let mut activations = Vec::new();
        for _ in 0..activation_count {
            let Some(taskset) = control_text(bytes, &mut cursor) else {
                return false;
            };
            let Some(task) = control_text(bytes, &mut cursor) else {
                return false;
            };
            if control_byte(bytes, &mut cursor) != Some(1) {
                return false;
            }
            activations.push((taskset, task));
        }
        if activations.windows(2).any(|pair| pair[0] >= pair[1]) {
            return false;
        }
        let Some(pending_count) = control_count(bytes, &mut cursor) else {
            return false;
        };
        for _ in 0..pending_count {
            if control_text(bytes, &mut cursor).is_none()
                || control_text(bytes, &mut cursor).is_none()
            {
                return false;
            }
            let Some(source) = control_bytes(bytes, &mut cursor) else {
                return false;
            };
            if !valid_transition_bytes(source) {
                return false;
            }
        }
        if cursor != bytes.len() {
            return false;
        }
        let mut by_task: BTreeMap<(&str, &str), Vec<(u64, u64)>> = BTreeMap::new();
        for (taskset, task, rank, slot, _, _) in &jobs {
            by_task
                .entry((taskset.as_str(), task.as_str()))
                .or_default()
                .push((*rank, *slot));
        }
        if let Some((taskset, task, slot, release)) = running {
            let running_jobs = jobs
                .iter()
                .filter(|job| {
                    job.0 == taskset
                        && job.1 == task
                        && job.3 == slot
                        && job.4 == release
                        && job.5 == 2
                })
                .count();
            if running_jobs != 1 || jobs.iter().filter(|job| job.5 == 2).count() != 1 {
                return false;
            }
        } else if jobs.iter().any(|job| job.5 == 2) {
            return false;
        }
        by_task.values_mut().all(|rows| {
            rows.sort_unstable();
            rows.iter()
                .enumerate()
                .all(|(rank, (actual, _))| u64::try_from(rank).ok() == Some(*actual))
                && rows
                    .iter()
                    .map(|(_, slot)| *slot)
                    .collect::<BTreeSet<_>>()
                    .len()
                    == rows.len()
        })
    }
}

/// Parsed finite scheduler state carried by an FR-418 product key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSchedulerControl {
    /// Live task jobs in canonical task-set/task/rank order.
    pub jobs: Vec<TaskSchedulerJob>,
    /// The currently executing job, when any.
    pub running: Option<TaskSchedulerRunning>,
    /// Task arrivals whose minimum inter-arrival phase is established.
    pub activations: Vec<(String, String)>,
    /// Source releases awaiting their zero-time derived activation edges.
    pub pending_releases: Vec<TaskSchedulerPendingRelease>,
}

/// One parsed live job row from the canonical scheduler key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSchedulerJob {
    /// Checked task-set name.
    pub taskset: String,
    /// Checked task name.
    pub task: String,
    /// Canonical rank among this task's live jobs.
    pub rank: u64,
    /// Finite storage slot.
    pub slot: u64,
    /// Source transition that released this job.
    pub release: ModelTransition,
    /// Pending, ready, running, suspended, or missed phase (0 through 4).
    pub phase: u8,
    /// Exact demand spelling retained by the state key.
    pub demand: String,
    /// Checked fixed-priority rank, when declared.
    pub priority: Option<u64>,
}

/// Running-job identity encoded by the scheduler control key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSchedulerRunning {
    /// Checked task-set name.
    pub taskset: String,
    /// Checked task name.
    pub task: String,
    /// Finite storage slot.
    pub slot: u64,
    /// Source transition that released this job.
    pub release: ModelTransition,
}

/// One source release waiting for a zero-time task activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSchedulerPendingRelease {
    /// Checked task-set name.
    pub taskset: String,
    /// Checked task name.
    pub task: String,
    /// Source transition whose occurrence created the pending release.
    pub source: ModelTransition,
}

fn decode_scheduler_control(
    bytes: &[u8],
) -> Result<TaskSchedulerControl, TaskProductStateKeyError> {
    let invalid = || TaskProductStateKeyError::InvalidSchedulerControl;
    if !bytes.starts_with(TASK_CONTROL_PREFIX) {
        return Err(invalid());
    }
    let mut cursor = TASK_CONTROL_PREFIX.len();
    let count = control_count(bytes, &mut cursor).ok_or_else(invalid)?;
    let mut jobs = Vec::new();
    for _ in 0..count {
        let taskset = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let task = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let rank = control_u64(bytes, &mut cursor).ok_or_else(invalid)?;
        let slot = control_u64(bytes, &mut cursor).ok_or_else(invalid)?;
        let release = control_bytes(bytes, &mut cursor).ok_or_else(invalid)?;
        let release = decode_transition(release).ok_or_else(invalid)?;
        let phase = control_byte(bytes, &mut cursor).ok_or_else(invalid)?;
        let demand = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let priority = match control_byte(bytes, &mut cursor).ok_or_else(invalid)? {
            0 => None,
            1 => Some(control_u64(bytes, &mut cursor).ok_or_else(invalid)?),
            _ => return Err(invalid()),
        };
        jobs.push(TaskSchedulerJob {
            taskset,
            task,
            rank,
            slot,
            release,
            phase,
            demand,
            priority,
        });
    }
    let running = match control_byte(bytes, &mut cursor).ok_or_else(invalid)? {
        0 => None,
        1 => {
            let taskset = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
            let task = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
            let slot = control_u64(bytes, &mut cursor).ok_or_else(invalid)?;
            let release = control_bytes(bytes, &mut cursor).ok_or_else(invalid)?;
            Some(TaskSchedulerRunning {
                taskset,
                task,
                slot,
                release: decode_transition(release).ok_or_else(invalid)?,
            })
        }
        _ => return Err(invalid()),
    };
    let count = control_count(bytes, &mut cursor).ok_or_else(invalid)?;
    let mut activations = Vec::new();
    for _ in 0..count {
        let taskset = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let task = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        if control_byte(bytes, &mut cursor) != Some(1) {
            return Err(invalid());
        }
        activations.push((taskset, task));
    }
    let count = control_count(bytes, &mut cursor).ok_or_else(invalid)?;
    let mut pending_releases = Vec::new();
    for _ in 0..count {
        let taskset = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let task = control_text(bytes, &mut cursor).ok_or_else(invalid)?;
        let source = control_bytes(bytes, &mut cursor).ok_or_else(invalid)?;
        pending_releases.push(TaskSchedulerPendingRelease {
            taskset,
            task,
            source: decode_transition(source).ok_or_else(invalid)?,
        });
    }
    if cursor != bytes.len() {
        return Err(invalid());
    }
    Ok(TaskSchedulerControl {
        jobs,
        running,
        activations,
        pending_releases,
    })
}

/// Encode parsed finite scheduler control using FR-418's canonical control
/// grammar. This reader-side encoder deliberately excludes concrete
/// occurrence, generation, jitter and deadline data: those belong to the
/// typed transition witness and the task clocks, not the finite quotient.
fn encode_scheduler_control(control: &TaskSchedulerControl) -> Option<Vec<u8>> {
    fn count(bytes: &mut Vec<u8>, value: usize) -> Option<()> {
        bytes.extend_from_slice(&u64::try_from(value).ok()?.to_be_bytes());
        Some(())
    }
    fn text(bytes: &mut Vec<u8>, value: &str) -> Option<()> {
        count(bytes, value.len())?;
        bytes.extend_from_slice(value.as_bytes());
        Some(())
    }
    fn blob(bytes: &mut Vec<u8>, value: &[u8]) -> Option<()> {
        count(bytes, value.len())?;
        bytes.extend_from_slice(value);
        Some(())
    }
    let transition = |value: &ModelTransition| {
        value
            .canonical_bytes(quire_canonical::Limits::new(1 << 20))
            .ok()
    };

    let mut bytes = TASK_CONTROL_PREFIX.to_vec();
    count(&mut bytes, control.jobs.len())?;
    for job in &control.jobs {
        text(&mut bytes, &job.taskset)?;
        text(&mut bytes, &job.task)?;
        bytes.extend_from_slice(&job.rank.to_be_bytes());
        bytes.extend_from_slice(&job.slot.to_be_bytes());
        blob(&mut bytes, &transition(&job.release)?)?;
        bytes.push(job.phase);
        text(&mut bytes, &job.demand)?;
        match job.priority {
            Some(priority) => {
                bytes.push(1);
                bytes.extend_from_slice(&priority.to_be_bytes());
            }
            None => bytes.push(0),
        }
    }
    match &control.running {
        Some(running) => {
            bytes.push(1);
            text(&mut bytes, &running.taskset)?;
            text(&mut bytes, &running.task)?;
            bytes.extend_from_slice(&running.slot.to_be_bytes());
            blob(&mut bytes, &transition(&running.release)?)?;
        }
        None => bytes.push(0),
    }
    count(&mut bytes, control.activations.len())?;
    for (taskset, task) in &control.activations {
        text(&mut bytes, taskset)?;
        text(&mut bytes, task)?;
        bytes.push(1);
    }
    count(&mut bytes, control.pending_releases.len())?;
    for pending in &control.pending_releases {
        text(&mut bytes, &pending.taskset)?;
        text(&mut bytes, &pending.task)?;
        blob(&mut bytes, &transition(&pending.source)?)?;
    }
    Some(bytes)
}

fn decode_transition(bytes: &[u8]) -> Option<ModelTransition> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;
    if object.get("type")?.as_str()? != "transition" {
        return None;
    }
    let operation = object.get("operation")?.as_str()?.to_owned();
    // ModelTransition stores receiver and argument values as their own
    // canonical JSON documents. They are embedded as JSON values in the
    // transition identity, so restore the canonical document text here.
    let receiver = match object.get("receiver")? {
        serde_json::Value::Null => None,
        value => Some(serde_json::to_string(value).ok()?),
    };
    let arguments = object
        .get("arguments")?
        .as_array()?
        .iter()
        .map(|argument| serde_json::to_string(argument).ok())
        .collect::<Option<Vec<_>>>()?;
    let transition = ModelTransition::new(
        operation,
        receiver,
        arguments,
        quire_canonical::Limits::new(1 << 20),
    )
    .ok()?;
    (transition
        .canonical_bytes(quire_canonical::Limits::new(1 << 20))
        .ok()?
        == bytes)
        .then_some(transition)
}

const TASK_PRODUCT_STATE_PREFIX: &[u8] = b"qsl-en6-task-state-v1\0";
const TASK_CONTROL_PREFIX: &[u8] = b"qsl-task-control-v1\0";

fn take_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, TaskProductStateKeyError> {
    let end = cursor
        .checked_add(8)
        .ok_or(TaskProductStateKeyError::LengthOverflow)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(TaskProductStateKeyError::Truncated)?;
    let value: [u8; 8] = value
        .try_into()
        .map_err(|_| TaskProductStateKeyError::Truncated)?;
    *cursor = end;
    Ok(u64::from_be_bytes(value))
}

fn take_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: u64,
) -> Result<&'a [u8], TaskProductStateKeyError> {
    let length = usize::try_from(length).map_err(|_| TaskProductStateKeyError::LengthOverflow)?;
    let end = cursor
        .checked_add(length)
        .ok_or(TaskProductStateKeyError::LengthOverflow)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(TaskProductStateKeyError::Truncated)?;
    *cursor = end;
    Ok(value)
}

fn control_u64(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
    let end = cursor.checked_add(8)?;
    let raw: [u8; 8] = bytes.get(*cursor..end)?.try_into().ok()?;
    *cursor = end;
    Some(u64::from_be_bytes(raw))
}

fn control_count(bytes: &[u8], cursor: &mut usize) -> Option<usize> {
    let value = usize::try_from(control_u64(bytes, cursor)?).ok()?;
    (value <= bytes.len()).then_some(value)
}

fn control_bytes<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let length = control_count(bytes, cursor)?;
    let end = cursor.checked_add(length)?;
    let value = bytes.get(*cursor..end)?;
    *cursor = end;
    Some(value)
}

fn control_text(bytes: &[u8], cursor: &mut usize) -> Option<String> {
    let value = std::str::from_utf8(control_bytes(bytes, cursor)?).ok()?;
    (!value.is_empty()).then(|| value.to_owned())
}

fn control_byte(bytes: &[u8], cursor: &mut usize) -> Option<u8> {
    let value = *bytes.get(*cursor)?;
    *cursor = cursor.checked_add(1)?;
    Some(value)
}

fn valid_transition_bytes(bytes: &[u8]) -> bool {
    decode_transition(bytes).is_some()
}

/// A malformed internal encoding of the joint FR-418 state key.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductStateKeyError {
    /// A component length cannot be represented or the buffer size overflowed.
    #[error("task-product state-key length overflow")]
    LengthOverflow,
    /// The process could not reserve the complete encoded key.
    #[error("could not reserve task-product state-key buffer")]
    AllocationFailed,
    /// The key ended before a length-delimited component was complete.
    #[error("truncated task-product state key")]
    Truncated,
    /// The key contains bytes after both declared components.
    #[error("trailing bytes in task-product state key")]
    TrailingBytes,
    /// The scheduler-control component does not follow its canonical grammar.
    #[error("malformed task scheduler-control key")]
    InvalidSchedulerControl,
}

/// One task row in the certificate's finite queue bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskQueueBound {
    /// Checked task-set name.
    pub taskset: String,
    /// Checked task name.
    pub task: String,
    /// Enforced arrival kind (`periodic` or `sporadic`).
    pub arrival: TaskArrivalKind,
    /// Minimum nominal inter-arrival time.
    pub interarrival: Rational,
    /// Nominal relative deadline.
    pub relative_deadline: Rational,
    /// Maximum release jitter.
    pub max_jitter: Rational,
    /// The exact selected demand's lower and upper bounds.
    pub work_lower: Rational,
    /// The exact selected demand's upper bound.
    pub work_upper: Rational,
    /// The maximum live slots for this task type.
    pub max_jobs: u64,
}

/// One auxiliary clock and its recomputed finite maximum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskClockCeiling {
    /// The stable task-clock identity.
    pub clock: String,
    /// The maximum scaled integer clock value.
    pub maximum: Integer,
}

/// One product-internal diagonal clock predicate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskDiagonalConstraint {
    /// The left clock identity.
    pub left: String,
    /// The strict or non-strict comparison.
    pub operator: TaskDiagonalOperator,
    /// The right clock identity.
    pub right: String,
    /// The exact scaled integer constant.
    pub constant: Integer,
}

/// The finite refined-product closure gate. Its declaration is not trusted
/// as evidence that the product is actually closed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTimeProgress {
    /// The prescribed closure procedure name.
    pub method: TaskTimeProgressMethod,
    /// The bounded live-job count to recompile and check.
    pub live_job_cap: u64,
    /// The required reader recomputation label.
    pub closure: TaskClosureMethod,
}

/// The prescribed bounded-subtraction/diagonal abstraction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskProductAbstraction {
    /// QSpec FR-415's exact finite-region abstraction.
    BoundedSubtractionDiagonalV1,
}

/// One enforced task-arrival rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskArrivalKind {
    /// Exactly one nominal activation per period.
    Periodic,
    /// At least the declared minimum nominal inter-arrival interval.
    Sporadic,
}

/// One supported task-product diagonal comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskDiagonalOperator {
    /// Strictly less than.
    Less,
    /// Less than or equal.
    LessEqual,
    /// Strictly greater than.
    Greater,
    /// Greater than or equal.
    GreaterEqual,
}

/// The finite-product reader procedure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskTimeProgressMethod {
    /// Exhaustively close the finite refined product.
    ClosedFiniteProductV1,
    /// Close the source graph when no checked task release is reachable.
    ClosedNoReleaseProductV1,
}

/// The required independent-reader closure operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskClosureMethod {
    /// Recompute every reachable successor.
    RecomputeAllReachableSuccessorsV1,
    /// Recompute every source successor and prove no checked task release is
    /// reachable from any initial state.
    RecomputeAllSourceSuccessorsNoTaskReleaseV1,
}

/// FR-331's auditable frontier for a bounded task-automaton exploration.
/// This evidence can explain an incomplete result but never establish a
/// proof of schedulability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSearchEvidence<O> {
    /// The obligation this stopped search belongs to.
    pub identity: O,
    /// The retained finite task-product nodes.
    pub nodes: Vec<TaskSearchNode>,
    /// Every exact product edge explored before the stop.
    pub edges: Vec<TaskSearchEdge>,
    /// The unexpanded reachable frontier and dimensions still open there.
    pub frontier: Vec<TaskSearchFrontier>,
    /// The effective limit that stopped the run, absent when an unresolved
    /// continuous successor stopped analysis before any configured limit.
    pub limit: Option<TaskSearchLimit>,
}

/// A task-product state retained by bounded search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSearchNode {
    /// The complete source-plus-scheduler discrete state key.
    pub state: StateKey,
    /// Shortest discovered path length from the initial product state.
    pub depth: u64,
    /// The canonical DBM zone in the task-product clock layout.
    pub zone: Zone,
    /// Whether the search completely enumerated this node's successors.
    pub expanded: bool,
}

/// One exact transition in a bounded task-product search frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSearchEdge {
    /// The source node index.
    pub node: usize,
    /// The source or exact task scheduler transition.
    pub transition: TaskSearchTransition,
    /// The target node index.
    pub target: usize,
}

/// Exact transition identity retained for task-search replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskSearchTransition {
    /// A source-model operation edge.
    Source(ModelTransition),
    /// A concrete task scheduler action, including its global occurrence.
    Task(Box<TaskTransitionIdentity>),
}

impl SubjectTransition for TaskSearchTransition {
    type Fairness = quire_semantic_value::fairness::FairnessConstraint;
}

/// Why bounded exploration stopped expanding a reachable product node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSearchStopReason {
    /// The stored product-state count reached its limit.
    StateLimit,
    /// The retained transition count reached its limit.
    TransitionLimit,
    /// The task-product clock count reached its limit.
    ClockLimit,
    /// A configured path-depth limit stopped expansion.
    DepthLimit,
    /// The configured task-product expansion work limit stopped expansion.
    WorkLimit,
    /// A continuous successor could not be finitely enumerated.
    UnresolvedChoice,
}

/// A node whose successors remain unexplored when a bounded run stopped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSearchFrontier {
    /// The unexpanded node index.
    pub node: usize,
    /// The effective stopping reason at this node.
    pub reason: TaskSearchStopReason,
    /// Successor dimensions that remain open.
    pub open: Vec<TaskSearchOpen>,
}

/// An open successor dimension on a bounded-search frontier row.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TaskSearchOpen {
    /// Source and derived discrete successors.
    Successors,
    /// Positive or zero-delay choices.
    Delay,
    /// A variable per-instance demand choice.
    Demand,
    /// A task mode-change budget choice.
    BudgetChange,
    /// Queue or activation choices.
    Queue,
}

/// Effective FR-331 resource limit recorded by a task-search frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSearchLimit {
    /// The named limit in the result cause.
    pub kind: TaskSearchLimitKind,
    /// The effective configured limit value.
    pub value: u64,
    /// The QSpec setting that supplied the effective value.
    pub setting: String,
}

/// Named resource limit kinds currently used by bounded task exploration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSearchLimitKind {
    /// `max_states`.
    States,
    /// `max_transitions`.
    Transitions,
    /// `max_clocks`.
    Clocks,
    /// `max_task_product_work`.
    Work,
}

impl<O> TaskSearchEvidence<O> {
    /// Check the frontier's finite wire shape before the independent reader
    /// recomputes its task-product graph.
    pub fn has_valid_shape(&self) -> bool {
        if self.nodes.is_empty() || self.frontier.is_empty() {
            return false;
        }
        if self
            .limit
            .as_ref()
            .is_some_and(|limit| limit.value == 0 || limit.setting != limit.kind.setting())
        {
            return false;
        }
        let mut product_nodes: HashMap<u64, Vec<usize>> = HashMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            if !matches!(TaskProductStateKey::decode(&node.state), Ok(Some(_))) {
                return false;
            }
            let Some(digest) = task_search_node_digest(node) else {
                return false;
            };
            let collisions = product_nodes.entry(digest).or_default();
            if collisions.iter().any(|prior| {
                self.nodes.get(*prior).is_some_and(|previous| {
                    previous.state == node.state && previous.zone == node.zone
                })
            }) {
                return false;
            }
            collisions.push(index);
        }
        // A checked timed subject can have multiple initial model states, so
        // every depth-zero product state is a root of the replay graph.
        let roots: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (node.depth == 0).then_some(index))
            .collect();
        if roots.is_empty() {
            return false;
        }

        let mut adjacency = vec![Vec::new(); self.nodes.len()];
        for edge in &self.edges {
            if edge.node >= self.nodes.len() || edge.target >= self.nodes.len() {
                return false;
            }
            adjacency[edge.node].push(edge.target);
        }
        let mut reachable = vec![false; self.nodes.len()];
        let mut queue = std::collections::VecDeque::from(roots.clone());
        for root in roots {
            reachable[root] = true;
        }
        while let Some(node) = queue.pop_front() {
            for &target in &adjacency[node] {
                if !reachable[target] {
                    reachable[target] = true;
                    queue.push_back(target);
                }
            }
        }
        if reachable.iter().any(|reachable| !reachable) {
            return false;
        }

        let mut frontier_nodes = BTreeSet::new();
        for frontier in &self.frontier {
            if frontier.node >= self.nodes.len()
                || self.nodes[frontier.node].expanded
                || frontier.open.is_empty()
                || !frontier_nodes.insert(frontier.node)
            {
                return false;
            }
            let open: BTreeSet<_> = frontier.open.iter().copied().collect();
            if open.len() != frontier.open.len() {
                return false;
            }
            if self.limit.is_none()
                && ![
                    TaskSearchOpen::Demand,
                    TaskSearchOpen::BudgetChange,
                    TaskSearchOpen::Queue,
                ]
                .iter()
                .any(|dimension| open.contains(dimension))
            {
                return false;
            }
            match (&self.limit, frontier.reason) {
                (Some(limit), TaskSearchStopReason::StateLimit)
                    if limit.kind == TaskSearchLimitKind::States => {}
                (Some(limit), TaskSearchStopReason::TransitionLimit)
                    if limit.kind == TaskSearchLimitKind::Transitions => {}
                (Some(limit), TaskSearchStopReason::ClockLimit)
                    if limit.kind == TaskSearchLimitKind::Clocks => {}
                (Some(limit), TaskSearchStopReason::WorkLimit)
                    if limit.kind == TaskSearchLimitKind::Work => {}
                (Some(_), TaskSearchStopReason::DepthLimit) => {}
                (None, TaskSearchStopReason::UnresolvedChoice) => {}
                _ => return false,
            }
        }
        self.nodes
            .iter()
            .enumerate()
            .all(|(index, node)| node.expanded != frontier_nodes.contains(&index))
    }
}

fn task_search_node_digest(node: &TaskSearchNode) -> Option<u64> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    node.state.as_bytes().hash(&mut hasher);
    node.zone.dim().hash(&mut hasher);
    for left in 0..node.zone.dim() {
        for right in 0..node.zone.dim() {
            match node.zone.bound(left, right).ok()? {
                Some((value, strict)) => {
                    1_u8.hash(&mut hasher);
                    value.to_string().hash(&mut hasher);
                    strict.hash(&mut hasher);
                }
                None => 0_u8.hash(&mut hasher),
            }
        }
    }
    Some(hasher.finish())
}

impl TaskSearchLimitKind {
    /// The FR-331 setting key paired with this task-search limit.
    pub fn setting(self) -> &'static str {
        match self {
            Self::States => "zone_check.max_symbolic_states",
            Self::Transitions => "zone_check.max_symbolic_transitions",
            Self::Clocks => "model_check.max_clocks",
            Self::Work => "zone_check.max_task_product_work",
        }
    }
}

impl TaskProductCertificate {
    /// Whether the typed value satisfies the FR-418-AC-11 structural
    /// invariants before the checker recompiles the source/task product.
    pub fn has_valid_shape(&self) -> bool {
        if self.queue_bounds.is_empty() {
            return false;
        }
        let no_release =
            self.time_progress.method == TaskTimeProgressMethod::ClosedNoReleaseProductV1;
        if no_release {
            if self.time_progress.live_job_cap != 0
                || self.time_progress.closure
                    != TaskClosureMethod::RecomputeAllSourceSuccessorsNoTaskReleaseV1
                || !self.clock_ceilings.is_empty()
                || !self.diagonal_constraints.is_empty()
                || !self.subtractions.is_empty()
            {
                return false;
            }
            return self.queue_bounds.iter().all(|row| {
                !row.taskset.is_empty()
                    && !row.task.is_empty()
                    && row.interarrival > Rational::from_integer(Integer::zero())
                    && row.relative_deadline > Rational::from_integer(Integer::zero())
                    && row.max_jitter >= Rational::from_integer(Integer::zero())
                    && row.work_lower >= Rational::from_integer(Integer::zero())
                    && row.work_lower <= row.work_upper
                    && row.work_upper > Rational::from_integer(Integer::zero())
                    && row.max_jobs > 0
            });
        }
        if self.time_progress.method != TaskTimeProgressMethod::ClosedFiniteProductV1
            || self.time_progress.closure != TaskClosureMethod::RecomputeAllReachableSuccessorsV1
            || self.time_progress.live_job_cap == 0
            || self.clock_ceilings.is_empty()
        {
            return false;
        }
        let mut cap = 0_u64;
        for row in &self.queue_bounds {
            if row.taskset.is_empty()
                || row.task.is_empty()
                || row.interarrival <= Rational::from_integer(Integer::zero())
                || row.relative_deadline <= Rational::from_integer(Integer::zero())
                || row.max_jitter < Rational::from_integer(Integer::zero())
                || row.work_lower <= Rational::from_integer(Integer::zero())
                || row.work_lower != row.work_upper
                || row.max_jobs == 0
            {
                return false;
            }
            let Some(next) = cap.checked_add(row.max_jobs) else {
                return false;
            };
            cap = next;
        }
        if cap != self.time_progress.live_job_cap {
            return false;
        }
        let mut clock_names = BTreeSet::new();
        for ceiling in &self.clock_ceilings {
            if ceiling.clock.is_empty()
                || ceiling.maximum <= Integer::zero()
                || !clock_names.insert(ceiling.clock.as_str())
            {
                return false;
            }
        }
        if self.diagonal_constraints.iter().any(|constraint| {
            !clock_names.contains(constraint.left.as_str())
                || !clock_names.contains(constraint.right.as_str())
        }) {
            return false;
        }
        self.subtractions.iter().all(|subtraction| {
            clock_names.contains(subtraction.clock.as_str())
                && subtraction.constant > Integer::zero()
        })
    }
}

/// Recompute the no-release certificate's authored per-task queue bounds
/// from the independently checked task set.
fn no_release_queue_bounds(task_set: &TaskSet) -> Option<Vec<TaskQueueBound>> {
    let mut rows = Vec::with_capacity(task_set.tasks.len());
    for task in &task_set.tasks {
        let (arrival, interarrival) = match &task.arrival {
            Arrival::Periodic { period } => (TaskArrivalKind::Periodic, period.clone()),
            Arrival::Sporadic { min_interarrival } => {
                (TaskArrivalKind::Sporadic, min_interarrival.clone())
            }
        };
        let ratio = task.deadline.div(&interarrival)?;
        let max_jobs = Integer::from_big(
            ratio
                .numerator()
                .as_big()
                .div_ceil(ratio.denominator().as_big()),
        )
        .add(&Integer::one())
        .to_u64()
        .filter(|value| *value > 0)?;
        let work_upper = match &task.wcet {
            Wcet::Single(value) => value.clone(),
            Wcet::Dual { hi, .. } => hi.clone(),
        };
        rows.push(TaskQueueBound {
            taskset: task_set.name.clone(),
            task: task.name.clone(),
            arrival,
            interarrival,
            relative_deadline: task.deadline.clone(),
            max_jitter: task.jitter.clone(),
            work_lower: task.bcet.clone(),
            work_upper,
            max_jobs,
        });
    }
    (!rows.is_empty()).then_some(rows)
}

/// Recompute the task bounds that a closed finite product may claim. A
/// bounded or invalid task set cannot authenticate the exact certificate
/// arm, even when its queue rows are internally well formed.
fn fixed_work_queue_bounds(
    task_set: &TaskSet,
    policy: SchedulingPolicy,
) -> Option<Vec<TaskQueueBound>> {
    if !matches!(
        policy,
        SchedulingPolicy::FixedPriority | SchedulingPolicy::Edf
    ) {
        return None;
    }
    let zero = Rational::from_integer(Integer::zero());
    let mut priorities = BTreeSet::new();
    let mut rows = Vec::with_capacity(task_set.tasks.len());
    for task in &task_set.tasks {
        if task.deadline <= zero
            || task.jitter < zero
            || task.blocking < zero
            || (policy == SchedulingPolicy::Edf
                && (!task.jitter.is_zero() || !task.blocking.is_zero()))
        {
            return None;
        }
        let work = task.fixed_work()?.clone();
        if policy == SchedulingPolicy::FixedPriority {
            if !task.jitter.is_zero() || !task.blocking.is_zero() {
                return None;
            }
            let priority = task.priority?;
            if !priorities.insert(priority) {
                return None;
            }
        }
        let (arrival, interarrival) = match &task.arrival {
            Arrival::Periodic { period } => (TaskArrivalKind::Periodic, period.clone()),
            Arrival::Sporadic { min_interarrival } => {
                (TaskArrivalKind::Sporadic, min_interarrival.clone())
            }
        };
        if interarrival <= zero {
            return None;
        }
        let ratio = task.deadline.div(&interarrival)?;
        let max_jobs = Integer::from_big(
            ratio
                .numerator()
                .as_big()
                .div_ceil(ratio.denominator().as_big()),
        )
        .add(&Integer::one())
        .to_u64()
        .filter(|value| *value > 0)?;
        rows.push(TaskQueueBound {
            taskset: task_set.name.clone(),
            task: task.name.clone(),
            arrival,
            interarrival,
            relative_deadline: task.deadline.clone(),
            max_jitter: task.jitter.clone(),
            work_lower: work.clone(),
            work_upper: work,
            max_jobs,
        });
    }
    (!rows.is_empty()).then_some(rows)
}

/// Independently derive the fixed product's clock ceilings and diagonal
/// region predicates from the recompiled task rows and certificate scale.
fn fixed_work_clock_metadata(
    rows: &[TaskQueueBound],
    scale: &Integer,
    listed_ceilings: &[TaskClockCeiling],
    listed_diagonals: &[TaskDiagonalConstraint],
) -> Option<(Vec<TaskClockCeiling>, Vec<TaskDiagonalConstraint>)> {
    if scale <= &Integer::zero() || rows.is_empty() {
        return None;
    }
    let mut slot_count = 0_usize;
    let mut jitter_slot_count = 0_usize;
    for row in rows {
        let slots = usize::try_from(row.max_jobs).ok()?;
        slot_count = slot_count.checked_add(slots)?;
        if !row.max_jitter.is_zero() {
            jitter_slot_count = jitter_slot_count.checked_add(slots)?;
        }
    }
    let expected_clock_count = rows
        .len()
        .checked_add(jitter_slot_count)?
        .checked_add(slot_count.checked_mul(2)?)?;
    let expected_diagonal_count = slot_count.checked_mul(slot_count.saturating_sub(1))?;
    if expected_clock_count != listed_ceilings.len()
        || expected_diagonal_count != listed_diagonals.len()
    {
        return None;
    }

    let scaled = |value: &Rational| {
        let product = value.mul(&Rational::from_integer(scale.clone()));
        (product.denominator() == &Integer::one()).then(|| product.numerator().clone())
    };
    let mut ceilings = Vec::with_capacity(expected_clock_count);
    for row in rows {
        ceilings.push(TaskClockCeiling {
            clock: format!("task:{}:{}:arrival", row.taskset, row.task),
            maximum: scaled(&row.interarrival)?,
        });
    }
    for row in rows {
        if row.max_jitter.is_zero() {
            continue;
        }
        for slot in 0..row.max_jobs {
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:slot{slot}:jitter", row.taskset, row.task),
                maximum: scaled(&row.max_jitter)?,
            });
        }
    }
    for row in rows {
        let service_maximum = row.work_upper.clone().max(row.relative_deadline.clone());
        let service_maximum = scaled(&service_maximum)?;
        let deadline_maximum = scaled(&row.relative_deadline)?;
        for slot in 0..row.max_jobs {
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:slot{slot}:service", row.taskset, row.task),
                maximum: service_maximum.clone(),
            });
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:slot{slot}:deadline", row.taskset, row.task),
                maximum: deadline_maximum.clone(),
            });
        }
    }

    let mut diagonals = Vec::with_capacity(expected_diagonal_count);
    for left in rows {
        let left_work = scaled(&left.work_upper)?;
        for left_slot in 0..left.max_jobs {
            let left_clock = format!(
                "task:{}:{}:slot{left_slot}:service",
                left.taskset, left.task
            );
            for right in rows {
                for right_slot in 0..right.max_jobs {
                    let right_clock = format!(
                        "task:{}:{}:slot{right_slot}:service",
                        right.taskset, right.task
                    );
                    if left_clock != right_clock {
                        diagonals.push(TaskDiagonalConstraint {
                            left: left_clock.clone(),
                            operator: TaskDiagonalOperator::Less,
                            right: right_clock,
                            constant: left_work.clone(),
                        });
                    }
                }
            }
        }
    }
    Some((ceilings, diagonals))
}

/// The reader's independently reconstructed FR-415 clock positions. The
/// order is derived from the checked task rows and the published ceiling
/// table; no analyzer layout or producer helper is shared here.
struct ReaderTaskClockLayout {
    dim: usize,
    arrival: BTreeMap<(String, String), usize>,
    service: BTreeMap<(String, String, u64), usize>,
    deadline: BTreeMap<(String, String, u64), usize>,
}

impl ReaderTaskClockLayout {
    fn new(rows: &[TaskQueueBound], base_dim: usize) -> Option<Self> {
        let mut arrival = BTreeMap::new();
        let mut service = BTreeMap::new();
        let mut deadline = BTreeMap::new();
        let mut next = base_dim;
        for row in rows {
            arrival.insert((row.taskset.clone(), row.task.clone()), next);
            next = next.checked_add(1)?;
        }
        // The exact fixed-work gate admits only zero jitter. A nonzero
        // jitter clock belongs to the bounded/incomplete path.
        if rows.iter().any(|row| !row.max_jitter.is_zero()) {
            return None;
        }
        for row in rows {
            for slot in 0..row.max_jobs {
                service.insert((row.taskset.clone(), row.task.clone(), slot), next);
                next = next.checked_add(1)?;
                deadline.insert((row.taskset.clone(), row.task.clone(), slot), next);
                next = next.checked_add(1)?;
            }
        }
        Some(Self {
            dim: next,
            arrival,
            service,
            deadline,
        })
    }

    fn arrival(&self, taskset: &str, task: &str) -> Option<usize> {
        self.arrival
            .get(&(taskset.to_owned(), task.to_owned()))
            .copied()
    }

    fn service(&self, taskset: &str, task: &str, slot: u64) -> Option<usize> {
        self.service
            .get(&(taskset.to_owned(), task.to_owned(), slot))
            .copied()
    }

    fn deadline(&self, taskset: &str, task: &str, slot: u64) -> Option<usize> {
        self.deadline
            .get(&(taskset.to_owned(), task.to_owned(), slot))
            .copied()
    }
}

fn reader_scaled(value: &Rational, scale: &Integer) -> Option<Integer> {
    let scaled = value.mul(&Rational::from_integer(scale.clone()));
    (scaled.denominator() == &Integer::one()).then(|| scaled.numerator().clone())
}

fn reader_constrain_places(
    zone: &mut Zone,
    constraints: &[PlaceConstraint],
    scale: &Integer,
) -> Option<()> {
    for constraint in constraints {
        let bound = reader_scaled(constraint.bound(), scale)?;
        let (left, right, value, strict) = match constraint.op() {
            ClockOp::Less => (constraint.place() + 1, 0, bound, true),
            ClockOp::LessOrEqual => (constraint.place() + 1, 0, bound, false),
            ClockOp::Equal => {
                zone.constrain(constraint.place() + 1, 0, &bound, false)
                    .ok()?;
                (0, constraint.place() + 1, bound.negated(), false)
            }
            ClockOp::GreaterOrEqual => (0, constraint.place() + 1, bound.negated(), false),
            ClockOp::Greater => (0, constraint.place() + 1, bound.negated(), true),
        };
        zone.constrain(left, right, &value, strict).ok()?;
    }
    Some(())
}

fn reader_task_invariants(
    zone: &mut Zone,
    control: &TaskSchedulerControl,
    rows: &[TaskQueueBound],
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
) -> Option<()> {
    for job in &control.jobs {
        if job.phase == 4 {
            continue;
        }
        let row = rows
            .iter()
            .find(|row| row.taskset == job.taskset && row.task == job.task)?;
        let service = layout.service(&job.taskset, &job.task, job.slot)?;
        let deadline = layout.deadline(&job.taskset, &job.task, job.slot)?;
        let work = reader_scaled(&row.work_upper, scale)?;
        let service_cap = reader_scaled(
            &row.work_upper.clone().max(row.relative_deadline.clone()),
            scale,
        )?;
        let deadline_cap = reader_scaled(&row.relative_deadline, scale)?;
        zone.constrain(service, 0, &service_cap, false).ok()?;
        if job.phase == 2 {
            zone.constrain(service, 0, &work, false).ok()?;
        }
        zone.constrain(deadline, 0, &deadline_cap, false).ok()?;
    }
    Some(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the independent reader keeps the timed and task premises explicit"
)]
fn reader_settle_product(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    control: &TaskSchedulerControl,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    policy: SchedulingPolicy,
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
    zone: &mut Zone,
) -> Option<()> {
    reader_constrain_places(zone, &timed.invariants(state).ok()?, scale)?;
    reader_task_invariants(zone, control, rows, layout, scale)?;
    let urgent = !control.pending_releases.is_empty()
        || reader_task_action_urgent(control, task_set, policy)?;
    if !zone.is_empty()
        && !urgent
        && timed.tick_period().is_none()
        && !timed.is_urgent(state).ok()?
    {
        zone.up();
        reader_constrain_places(zone, &timed.invariants(state).ok()?, scale)?;
        reader_task_invariants(zone, control, rows, layout, scale)?;
    }
    Some(())
}

/// Whether zero-time dispatch or preemption is enabled in finite scheduler
/// control. EDF's exact deadline-region selection is applied separately to
/// candidate zones; this predicate only determines whether the state may
/// elapse before taking some scheduler action.
fn reader_task_action_urgent(
    control: &TaskSchedulerControl,
    task_set: &TaskSet,
    policy: SchedulingPolicy,
) -> Option<bool> {
    if control.running.is_none() {
        return Some(control.jobs.iter().any(|job| matches!(job.phase, 1 | 3)));
    }
    let running = control.running.as_ref()?;
    let running_task = task_set
        .tasks
        .iter()
        .find(|task| task.name == running.task)?;
    match policy {
        SchedulingPolicy::FixedPriority => {
            let running_priority = running_task.priority?;
            control
                .jobs
                .iter()
                .filter(|job| job.phase == 1)
                .try_fold(false, |urgent, job| {
                    let priority = task_set
                        .tasks
                        .iter()
                        .find(|task| task.name == job.task)?
                        .priority?;
                    Some(urgent || priority < running_priority)
                })
        }
        SchedulingPolicy::Edf => {
            let running_deadline = &running_task.deadline;
            let running_rank = control
                .jobs
                .iter()
                .find(|candidate| {
                    candidate.taskset == running.taskset
                        && candidate.task == running.task
                        && candidate.slot == running.slot
                        && candidate.release == running.release
                })?
                .rank;
            let mut urgent = false;
            for job in control.jobs.iter().filter(|job| job.phase == 1) {
                let ready = task_set.tasks.iter().find(|task| task.name == job.task)?;
                let order = ready
                    .deadline
                    .cmp(running_deadline)
                    .then_with(|| ready.name.cmp(&running_task.name))
                    .then_with(|| job.release.cmp(&running.release))
                    .then_with(|| job.rank.cmp(&running_rank));
                urgent |= order == std::cmp::Ordering::Less;
            }
            Some(urgent)
        }
        SchedulingPolicy::Amc => None,
    }
}

fn reader_apply_task_action(
    control: &TaskSchedulerControl,
    template: &TaskTransitionTemplate,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
) -> Option<TaskSchedulerControl> {
    let mut next = control.clone();
    let row = rows
        .iter()
        .find(|row| row.taskset == template.taskset && row.task == template.task)?;
    let task = task_set
        .tasks
        .iter()
        .find(|task| task.name == template.task)?;
    if task.priority != template.priority {
        return None;
    }
    match template.action {
        TaskAction::Activate => {
            let pending = next.pending_releases.first()?;
            if pending.taskset != template.taskset
                || pending.task != template.task
                || pending.source != template.release
                || template.source_transition.as_ref() != Some(&template.release)
            {
                return None;
            }
            let live: Vec<_> = next
                .jobs
                .iter()
                .filter(|job| job.taskset == template.taskset && job.task == template.task)
                .collect();
            if u64::try_from(live.len()).ok()? != template.job_rank
                || live.len() >= usize::try_from(row.max_jobs).ok()?
            {
                return None;
            }
            let slot = (0..row.max_jobs).find(|slot| live.iter().all(|job| job.slot != *slot))?;
            if slot != template.slot {
                return None;
            }
            next.pending_releases.remove(0);
            next.jobs.push(TaskSchedulerJob {
                taskset: template.taskset.clone(),
                task: template.task.clone(),
                rank: template.job_rank,
                slot: template.slot,
                release: template.release.clone(),
                phase: 1,
                demand: row.work_upper.to_string(),
                priority: task.priority,
            });
            next.jobs.sort_by(|left, right| {
                (&left.taskset, &left.task, left.rank).cmp(&(
                    &right.taskset,
                    &right.task,
                    right.rank,
                ))
            });
            if !next
                .activations
                .iter()
                .any(|(taskset, name)| taskset == &template.taskset && name == &template.task)
            {
                next.activations
                    .push((template.taskset.clone(), template.task.clone()));
                next.activations.sort();
            }
        }
        TaskAction::Dispatch | TaskAction::Resume => {
            if next.running.is_some() {
                return None;
            }
            let phase = if template.action == TaskAction::Dispatch {
                1
            } else {
                3
            };
            let job = next.jobs.iter_mut().find(|job| {
                job.taskset == template.taskset
                    && job.task == template.task
                    && job.rank == template.job_rank
                    && job.slot == template.slot
                    && job.release == template.release
            })?;
            if job.phase != phase {
                return None;
            }
            job.phase = 2;
            next.running = Some(TaskSchedulerRunning {
                taskset: template.taskset.clone(),
                task: template.task.clone(),
                slot: template.slot,
                release: template.release.clone(),
            });
        }
        TaskAction::Preempt => {
            let running = next.running.as_ref()?;
            if running.taskset != template.taskset
                || running.task != template.task
                || running.slot != template.slot
                || running.release != template.release
            {
                return None;
            }
            let job = next.jobs.iter_mut().find(|job| {
                job.taskset == template.taskset
                    && job.task == template.task
                    && job.rank == template.job_rank
                    && job.slot == template.slot
                    && job.release == template.release
                    && job.phase == 2
            })?;
            job.phase = 3;
            next.running = None;
        }
        TaskAction::Complete | TaskAction::DeadlineMiss => {
            let index = next.jobs.iter().position(|job| {
                job.taskset == template.taskset
                    && job.task == template.task
                    && job.rank == template.job_rank
                    && job.slot == template.slot
                    && job.release == template.release
                    && (template.action == TaskAction::DeadlineMiss || job.phase == 2)
            })?;
            next.jobs.remove(index);
            if next.running.as_ref().is_some_and(|running| {
                running.taskset == template.taskset
                    && running.task == template.task
                    && running.slot == template.slot
                    && running.release == template.release
            }) {
                next.running = None;
            }
            for job in &mut next.jobs {
                if job.taskset == template.taskset
                    && job.task == template.task
                    && job.rank > template.job_rank
                {
                    job.rank -= 1;
                }
            }
        }
        TaskAction::Release | TaskAction::BudgetChange => return None,
    }
    Some(next)
}

struct ReaderProductSuccessor {
    state: ModelSimState,
    key: StateKey,
    transition: CertificateTransition,
    part: Option<usize>,
    zone: Zone,
}

fn reader_product_key(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    control: &TaskSchedulerControl,
) -> Option<StateKey> {
    let (_, source) = system.state_key(state).ok()?;
    TaskProductStateKey::new(source, encode_scheduler_control(control)?)
        .into_state_key()
        .ok()
}

#[allow(
    clippy::too_many_arguments,
    reason = "the release guard recomputes each checked binding from explicit inputs"
)]
fn reader_source_release_guard(
    zone: &mut Zone,
    control: &TaskSchedulerControl,
    transition: &ModelTransition,
    releases: &[qsl_semantics::check::Release],
    system: &ModelSystem<'_>,
    rows: &[TaskQueueBound],
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
) -> Option<Vec<TaskSchedulerPendingRelease>> {
    if !control.pending_releases.is_empty() {
        return None;
    }
    let mut pending = Vec::new();
    for release in releases.iter().filter(|release| {
        system.operation_text(release.operation().declaring(), release.operation().name())
            == Some(transition.operation())
    }) {
        let row = rows
            .iter()
            .find(|row| row.taskset == release.task_set() && row.task == release.task())?;
        if control
            .activations
            .iter()
            .any(|(taskset, task)| taskset == &row.taskset && task == &row.task)
        {
            let clock = layout.arrival(&row.taskset, &row.task)?;
            let spacing = reader_scaled(&row.interarrival, scale)?;
            zone.constrain(0, clock, &spacing.negated(), false).ok()?;
            if row.arrival == TaskArrivalKind::Periodic {
                zone.constrain(clock, 0, &spacing, false).ok()?;
            }
        }
        pending.push(TaskSchedulerPendingRelease {
            taskset: row.taskset.clone(),
            task: row.task.clone(),
            source: transition.clone(),
        });
    }
    Some(pending)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the reader successor relation has separate checked source and task premises"
)]
fn reader_source_task_successors(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    control: &TaskSchedulerControl,
    zone: &Zone,
    releases: &[qsl_semantics::check::Release],
    system: &ModelSystem<'_>,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    policy: SchedulingPolicy,
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
) -> Option<Vec<ReaderProductSuccessor>> {
    if !control.pending_releases.is_empty() {
        return Some(Vec::new());
    }
    let mut source_zone = zone.clone();
    reader_settle_product(
        timed,
        state,
        control,
        rows,
        task_set,
        policy,
        layout,
        scale,
        &mut source_zone,
    )?;
    if source_zone.is_empty() {
        return Some(Vec::new());
    }
    let mut found = Vec::new();
    for step in timed.guarded_steps(state).ok()? {
        for (part_index, part) in step.parts().iter().enumerate() {
            let mut next_zone = source_zone.clone();
            reader_constrain_places(&mut next_zone, part, scale)?;
            if next_zone.is_empty() {
                continue;
            }
            let pending = reader_source_release_guard(
                &mut next_zone,
                control,
                step.transition(),
                releases,
                system,
                rows,
                layout,
                scale,
            )?;
            if next_zone.is_empty() {
                continue;
            }
            let carries = step
                .carries()
                .iter()
                .map(|(target, source)| (target + 1, source + 1))
                .collect::<Vec<_>>();
            let resets = step
                .resets()
                .iter()
                .map(|(clock, value)| Some((clock + 1, reader_scaled(value, scale)?)))
                .collect::<Option<Vec<_>>>()?;
            next_zone.assign_clocks(&carries, &resets).ok()?;
            reader_constrain_places(&mut next_zone, &timed.invariants(step.post()).ok()?, scale)?;
            let mut next_control = control.clone();
            next_control.pending_releases.extend(pending);
            reader_task_invariants(&mut next_zone, &next_control, rows, layout, scale)?;
            if next_zone.is_empty() {
                continue;
            }
            reader_settle_product(
                timed,
                step.post(),
                &next_control,
                rows,
                task_set,
                policy,
                layout,
                scale,
                &mut next_zone,
            )?;
            if next_zone.is_empty() {
                continue;
            }
            found.push(ReaderProductSuccessor {
                state: step.post().clone(),
                key: reader_product_key(system, step.post(), &next_control)?,
                transition: CertificateTransition::Source(step.transition().clone()),
                part: Some(part_index),
                zone: next_zone,
            });
        }
    }
    Some(found)
}

fn reader_template_for_job(
    action: TaskAction,
    job: &TaskSchedulerJob,
    task_set: &TaskSet,
) -> Option<TaskTransitionTemplate> {
    let task = task_set.tasks.iter().find(|task| task.name == job.task)?;
    Some(TaskTransitionTemplate {
        action,
        taskset: job.taskset.clone(),
        task: job.task.clone(),
        job_rank: job.rank,
        slot: job.slot,
        release: job.release.clone(),
        source_transition: None,
        priority: task.priority,
    })
}

fn reader_edf_selected(
    candidate: &TaskSchedulerJob,
    others: &[&TaskSchedulerJob],
    zone: &mut Zone,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
) -> Option<()> {
    let candidate_row = rows
        .iter()
        .find(|row| row.taskset == candidate.taskset && row.task == candidate.task)?;
    let left = layout.deadline(&candidate.taskset, &candidate.task, candidate.slot)?;
    for other in others {
        if candidate == *other {
            continue;
        }
        let other_row = rows
            .iter()
            .find(|row| row.taskset == other.taskset && row.task == other.task)?;
        let right = layout.deadline(&other.taskset, &other.task, other.slot)?;
        let delta = reader_scaled(
            &other_row
                .relative_deadline
                .sub(&candidate_row.relative_deadline),
            scale,
        )?;
        let candidate_task = task_set
            .tasks
            .iter()
            .find(|task| task.name == candidate.task)?;
        let other_task = task_set.tasks.iter().find(|task| task.name == other.task)?;
        let tie = (&candidate_task.name, &candidate.release, candidate.rank).cmp(&(
            &other_task.name,
            &other.release,
            other.rank,
        ));
        zone.constrain(right, left, &delta, tie != std::cmp::Ordering::Less)
            .ok()?;
        if zone.is_empty() {
            return Some(());
        }
    }
    Some(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "each reader action is recomputed from explicit checked product inputs"
)]
fn reader_task_action_successors(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    control: &TaskSchedulerControl,
    zone: &Zone,
    system: &ModelSystem<'_>,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    policy: SchedulingPolicy,
    layout: &ReaderTaskClockLayout,
    scale: &Integer,
) -> Option<Vec<ReaderProductSuccessor>> {
    let mut candidates = Vec::new();
    if let Some(pending) = control.pending_releases.first() {
        let live = control
            .jobs
            .iter()
            .filter(|job| job.taskset == pending.taskset && job.task == pending.task)
            .count();
        let row = rows
            .iter()
            .find(|row| row.taskset == pending.taskset && row.task == pending.task)?;
        let slot = (0..row.max_jobs).find(|slot| {
            control.jobs.iter().all(|job| {
                job.taskset != pending.taskset || job.task != pending.task || job.slot != *slot
            })
        })?;
        let task = task_set
            .tasks
            .iter()
            .find(|task| task.name == pending.task)?;
        candidates.push(TaskTransitionTemplate {
            action: TaskAction::Activate,
            taskset: pending.taskset.clone(),
            task: pending.task.clone(),
            job_rank: u64::try_from(live).ok()?,
            slot,
            release: pending.source.clone(),
            source_transition: Some(pending.source.clone()),
            priority: task.priority,
        });
    } else {
        let ready: Vec<_> = control
            .jobs
            .iter()
            .filter(|job| matches!(job.phase, 1 | 3))
            .collect();
        let selected: Vec<&TaskSchedulerJob> = match policy {
            SchedulingPolicy::FixedPriority => {
                let mut sorted = ready.clone();
                sorted.sort_by_key(|job| {
                    let priority = task_set
                        .tasks
                        .iter()
                        .find(|task| task.name == job.task)
                        .and_then(|task| task.priority)
                        .unwrap_or(u64::MAX);
                    (priority, job.task.as_str(), job.rank, job.slot)
                });
                sorted.into_iter().take(1).collect()
            }
            SchedulingPolicy::Edf => ready,
            SchedulingPolicy::Amc => return None,
        };
        if control.running.is_none() {
            for job in selected {
                candidates.push(reader_template_for_job(
                    if job.phase == 3 {
                        TaskAction::Resume
                    } else {
                        TaskAction::Dispatch
                    },
                    job,
                    task_set,
                )?);
            }
        } else if let Some(running) = &control.running {
            let running_job = control.jobs.iter().find(|job| {
                job.taskset == running.taskset
                    && job.task == running.task
                    && job.slot == running.slot
                    && job.release == running.release
            })?;
            let running_template =
                reader_template_for_job(TaskAction::Preempt, running_job, task_set)?;
            if policy == SchedulingPolicy::FixedPriority {
                let running_priority = running_template.priority?;
                if selected.iter().any(|job| {
                    task_set
                        .tasks
                        .iter()
                        .find(|task| task.name == job.task)
                        .and_then(|task| task.priority)
                        .is_some_and(|priority| priority < running_priority)
                }) {
                    candidates.push(running_template);
                }
            } else {
                candidates.push(running_template);
            }
        }
        if let Some(running) = &control.running {
            let job = control.jobs.iter().find(|job| {
                job.taskset == running.taskset
                    && job.task == running.task
                    && job.slot == running.slot
                    && job.release == running.release
            })?;
            candidates.push(reader_template_for_job(
                TaskAction::Complete,
                job,
                task_set,
            )?);
        }
        for job in &control.jobs {
            candidates.push(reader_template_for_job(
                TaskAction::DeadlineMiss,
                job,
                task_set,
            )?);
        }
    }

    let mut found = Vec::new();
    for template in candidates {
        let mut action_zones = vec![zone.clone()];
        if template.action == TaskAction::Preempt && policy == SchedulingPolicy::Edf {
            action_zones.clear();
            let running = control.jobs.iter().find(|job| {
                job.taskset == template.taskset
                    && job.task == template.task
                    && job.rank == template.job_rank
                    && job.slot == template.slot
                    && job.release == template.release
            })?;
            for ready in control.jobs.iter().filter(|job| job.phase == 1) {
                let peers = [running, ready];
                let mut selected = zone.clone();
                reader_edf_selected(ready, &peers, &mut selected, rows, task_set, layout, scale)?;
                if !selected.is_empty() {
                    action_zones.push(selected);
                }
            }
        }
        'candidate: for mut next_zone in action_zones {
            match template.action {
                TaskAction::Activate => {
                    let pending = control.pending_releases.first()?;
                    if control
                        .activations
                        .iter()
                        .any(|(taskset, task)| taskset == &pending.taskset && task == &pending.task)
                    {
                        let row = rows.iter().find(|row| {
                            row.taskset == pending.taskset && row.task == pending.task
                        })?;
                        let arrival = layout.arrival(&pending.taskset, &pending.task)?;
                        let spacing = reader_scaled(&row.interarrival, scale)?;
                        next_zone
                            .constrain(0, arrival, &spacing.negated(), false)
                            .ok()?;
                        if row.arrival == TaskArrivalKind::Periodic {
                            next_zone.constrain(arrival, 0, &spacing, false).ok()?;
                        }
                    }
                    let arrival = layout.arrival(&template.taskset, &template.task)?;
                    let service =
                        layout.service(&template.taskset, &template.task, template.slot)?;
                    let deadline =
                        layout.deadline(&template.taskset, &template.task, template.slot)?;
                    next_zone
                        .assign_clocks(
                            &[],
                            &[
                                (arrival, Integer::zero()),
                                (service, Integer::zero()),
                                (deadline, Integer::zero()),
                            ],
                        )
                        .ok()?;
                }
                TaskAction::Dispatch => {
                    let job = control.jobs.iter().find(|job| {
                        job.taskset == template.taskset
                            && job.task == template.task
                            && job.rank == template.job_rank
                            && job.slot == template.slot
                            && job.release == template.release
                    })?;
                    if policy == SchedulingPolicy::Edf {
                        let peers = control
                            .jobs
                            .iter()
                            .filter(|job| matches!(job.phase, 1 | 3))
                            .collect::<Vec<_>>();
                        reader_edf_selected(
                            job,
                            &peers,
                            &mut next_zone,
                            rows,
                            task_set,
                            layout,
                            scale,
                        )?;
                    }
                    let service =
                        layout.service(&template.taskset, &template.task, template.slot)?;
                    next_zone.reset(service, &Integer::zero()).ok()?;
                }
                TaskAction::Resume => {
                    let job = control.jobs.iter().find(|job| {
                        job.taskset == template.taskset
                            && job.task == template.task
                            && job.rank == template.job_rank
                            && job.slot == template.slot
                            && job.release == template.release
                    })?;
                    if policy == SchedulingPolicy::Edf {
                        let peers = control
                            .jobs
                            .iter()
                            .filter(|job| matches!(job.phase, 1 | 3))
                            .collect::<Vec<_>>();
                        reader_edf_selected(
                            job,
                            &peers,
                            &mut next_zone,
                            rows,
                            task_set,
                            layout,
                            scale,
                        )?;
                    }
                }
                TaskAction::Preempt => {}
                TaskAction::Complete => {
                    let row = rows
                        .iter()
                        .find(|row| row.taskset == template.taskset && row.task == template.task)?;
                    let service =
                        layout.service(&template.taskset, &template.task, template.slot)?;
                    let work = reader_scaled(&row.work_upper, scale)?;
                    next_zone.constrain(service, 0, &work, false).ok()?;
                    next_zone
                        .constrain(0, service, &work.negated(), false)
                        .ok()?;
                    for suspended in control.jobs.iter().filter(|job| job.phase == 3) {
                        let clock =
                            layout.service(&suspended.taskset, &suspended.task, suspended.slot)?;
                        let suspended_row = rows.iter().find(|row| {
                            row.taskset == suspended.taskset && row.task == suspended.task
                        })?;
                        let ceiling = reader_scaled(
                            &suspended_row
                                .work_upper
                                .clone()
                                .max(suspended_row.relative_deadline.clone()),
                            scale,
                        )?;
                        if next_zone.subtract_bounded(clock, &work, &ceiling).is_err() {
                            continue 'candidate;
                        }
                    }
                    next_zone.reset(service, &Integer::zero()).ok()?;
                }
                TaskAction::DeadlineMiss => {
                    let row = rows
                        .iter()
                        .find(|row| row.taskset == template.taskset && row.task == template.task)?;
                    let deadline =
                        layout.deadline(&template.taskset, &template.task, template.slot)?;
                    let deadline_bound = reader_scaled(&row.relative_deadline, scale)?;
                    next_zone
                        .constrain(deadline, 0, &deadline_bound, false)
                        .ok()?;
                    next_zone
                        .constrain(0, deadline, &deadline_bound.negated(), false)
                        .ok()?;
                    let work = reader_scaled(&row.work_upper, scale)?;
                    let service =
                        layout.service(&template.taskset, &template.task, template.slot)?;
                    if row.work_upper.is_zero()
                        && control
                            .jobs
                            .iter()
                            .find(|job| {
                                job.taskset == template.taskset
                                    && job.task == template.task
                                    && job.rank == template.job_rank
                                    && job.slot == template.slot
                            })?
                            .phase
                            != 2
                    {
                        continue;
                    }
                    if control
                        .jobs
                        .iter()
                        .find(|job| {
                            job.taskset == template.taskset
                                && job.task == template.task
                                && job.rank == template.job_rank
                                && job.slot == template.slot
                        })?
                        .phase
                        == 3
                    {
                        if let Some(running) = &control.running {
                            let running_service =
                                layout.service(&running.taskset, &running.task, running.slot)?;
                            next_zone
                                .constrain(service, running_service, &work, true)
                                .ok()?;
                        } else {
                            next_zone.constrain(service, 0, &work, true).ok()?;
                        }
                    } else {
                        next_zone.constrain(service, 0, &work, true).ok()?;
                    }
                    next_zone.free(service).ok()?;
                    next_zone.free(deadline).ok()?;
                }
                TaskAction::Release | TaskAction::BudgetChange => return None,
            }
            if next_zone.is_empty() {
                continue;
            }
            let Some(next_control) = reader_apply_task_action(control, &template, rows, task_set)
            else {
                continue;
            };
            reader_task_invariants(&mut next_zone, &next_control, rows, layout, scale)?;
            reader_settle_product(
                timed,
                state,
                &next_control,
                rows,
                task_set,
                policy,
                layout,
                scale,
                &mut next_zone,
            )?;
            if next_zone.is_empty() {
                continue;
            }
            found.push(ReaderProductSuccessor {
                state: state.clone(),
                key: reader_product_key(system, state, &next_control)?,
                transition: CertificateTransition::Task(Box::new(template.clone())),
                part: None,
                zone: next_zone,
            });
        }
    }
    Some(found)
}

enum FixedProductClosureFailure {
    Unsupported,
    Rejected(CertificateRejection),
    Stopped(IncompleteCause),
}

#[allow(
    clippy::too_many_arguments,
    reason = "independent closure inputs remain explicit at this trust boundary"
)]
fn fixed_product_closure<O>(
    timed: &TimedSystem<'_, '_>,
    system: &ModelSystem<'_>,
    certificate: &ZoneCertificate<O>,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    releases: &[qsl_semantics::check::Release],
    policy: SchedulingPolicy,
    cancel: &Cancel,
) -> Result<(), FixedProductClosureFailure> {
    let fail = |rule, at| FixedProductClosureFailure::Rejected(CertificateRejection { rule, at });
    let layout = ReaderTaskClockLayout::new(rows, timed.places().len() + 1)
        .ok_or(FixedProductClosureFailure::Unsupported)?;
    if certificate
        .nodes
        .iter()
        .any(|node| node.zone.dim() != layout.dim)
    {
        return Err(fail(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::TaskProduct),
        ));
    }
    let empty = TaskSchedulerControl {
        jobs: Vec::new(),
        running: None,
        activations: Vec::new(),
        pending_releases: Vec::new(),
    };
    let mut reached = vec![false; certificate.nodes.len()];
    let mut source_states: Vec<Option<ModelSimState>> = vec![None; certificate.nodes.len()];
    let mut queue = Vec::new();
    for start in timed.initial() {
        if cancel.poll() {
            let Some(source) = cancel.cause() else {
                return Err(FixedProductClosureFailure::Unsupported);
            };
            return Err(FixedProductClosureFailure::Stopped(
                IncompleteCause::Cancelled { source },
            ));
        }
        let mut zone = Zone::zero(layout.dim);
        reader_settle_product(
            timed,
            start.discrete(),
            &empty,
            rows,
            task_set,
            policy,
            &layout,
            &certificate.scale,
            &mut zone,
        )
        .ok_or(FixedProductClosureFailure::Unsupported)?;
        if zone.is_empty() {
            continue;
        }
        let key = reader_product_key(system, start.discrete(), &empty)
            .ok_or(FixedProductClosureFailure::Unsupported)?;
        let target = certificate.initials.iter().copied().find(|&index| {
            certificate.nodes.get(index).is_some_and(|node| {
                node.state_key == key
                    && zone
                        .alu_covered_by(&node.zone, &certificate.lu_bounds)
                        .unwrap_or(false)
            })
        });
        let Some(target) = target else {
            return Err(fail(
                CertificateRule::InitialMissing,
                CertificateLocus::ModelState(key),
            ));
        };
        reached[target] = true;
        source_states[target].get_or_insert_with(|| start.discrete().clone());
        queue.push(target);
    }
    if queue.is_empty() {
        return Err(FixedProductClosureFailure::Unsupported);
    }

    let mut cursor = 0;
    while let Some(&node_index) = queue.get(cursor) {
        cursor += 1;
        if cancel.poll() {
            let Some(source) = cancel.cause() else {
                return Err(FixedProductClosureFailure::Unsupported);
            };
            return Err(FixedProductClosureFailure::Stopped(
                IncompleteCause::Cancelled { source },
            ));
        }
        let node = &certificate.nodes[node_index];
        let state = source_states[node_index]
            .as_ref()
            .ok_or(FixedProductClosureFailure::Unsupported)?;
        let key = TaskProductStateKey::decode(&node.state_key)
            .ok()
            .flatten()
            .ok_or(FixedProductClosureFailure::Unsupported)?;
        let control = key
            .scheduler_state()
            .map_err(|_| FixedProductClosureFailure::Unsupported)?;
        let mut successors = reader_source_task_successors(
            timed,
            state,
            &control,
            &node.zone,
            releases,
            system,
            rows,
            task_set,
            policy,
            &layout,
            &certificate.scale,
        )
        .ok_or(FixedProductClosureFailure::Unsupported)?;
        successors.extend(
            reader_task_action_successors(
                timed,
                state,
                &control,
                &node.zone,
                system,
                rows,
                task_set,
                policy,
                &layout,
                &certificate.scale,
            )
            .ok_or(FixedProductClosureFailure::Unsupported)?,
        );

        let outgoing: Vec<_> = certificate
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node_index)
            .collect();
        let mut edge_used = vec![false; outgoing.len()];
        for successor in successors {
            if matches!(
                successor.transition,
                CertificateTransition::Task(ref template)
                    if template.action == TaskAction::DeadlineMiss
            ) {
                return Err(fail(CertificateRule::BadState, at_node(node_index)));
            }
            let matching = outgoing
                .iter()
                .enumerate()
                .find_map(|(position, (_, edge))| {
                    if edge.transition.as_ref() != Some(&successor.transition)
                        || edge.part != successor.part
                    {
                        return None;
                    }
                    let target = certificate.nodes.get(edge.to)?;
                    if target.state_key != successor.key
                        || !successor
                            .zone
                            .alu_covered_by(&target.zone, &certificate.lu_bounds)
                            .ok()?
                    {
                        return None;
                    }
                    Some((position, edge.to))
                });
            let Some((position, target)) = matching else {
                return Err(fail(CertificateRule::SuccessorMissing, at_node(node_index)));
            };
            edge_used[position] = true;
            if !reached[target] {
                reached[target] = true;
                source_states[target].get_or_insert(successor.state);
                queue.push(target);
            }
        }
        if let Some((position, _)) = edge_used.iter().enumerate().find(|(_, used)| !**used) {
            return Err(fail(
                CertificateRule::PostDiffers,
                at_edge(outgoing[position].0),
            ));
        }
    }
    if let Some(unreached) = reached.iter().position(|reached| !reached) {
        return Err(fail(CertificateRule::SuccessorMissing, at_node(unreached)));
    }
    Ok(())
}

/// Validate a dense-time idle suffix for a fixed-work product certificate.
/// The suffix is admitted only from a reachable initial product node with no
/// releases or live jobs, with no urgent source action or finite upper time
/// invariant. The finite-product closure check separately proves every
/// source/task successor from that node and all other reachable nodes.
fn fixed_product_idle_witness<O>(
    timed: &TimedSystem<'_, '_>,
    system: &ModelSystem<'_>,
    certificate: &ZoneCertificate<O>,
    task_set: &TaskSet,
    rows: &[TaskQueueBound],
    layout: &ReaderTaskClockLayout,
    policy: SchedulingPolicy,
) -> Option<()> {
    let witness = &certificate.admitted;
    let DivergentTarget::Quiescent(target) = witness.target else {
        return None;
    };
    if timed.tick_period().is_some()
        || target != witness.start
        || !witness.path.is_empty()
        || !certificate.initials.contains(&witness.start)
    {
        return None;
    }
    let node = certificate.nodes.get(witness.start)?;
    let empty = TaskSchedulerControl {
        jobs: Vec::new(),
        running: None,
        activations: Vec::new(),
        pending_releases: Vec::new(),
    };
    let state = timed.initial().into_iter().find(|initial| {
        reader_product_key(system, initial.discrete(), &empty).as_ref() == Some(&node.state_key)
    })?;
    if reader_task_action_urgent(&empty, task_set, policy)?
        || timed.is_urgent(state.discrete()).ok()?
    {
        return None;
    }
    // Any finite upper invariant would cap the idle suffix. Lower bounds are
    // compatible with unbounded dense-time elapse.
    if timed
        .invariants(state.discrete())
        .ok()?
        .iter()
        .any(|constraint| {
            matches!(
                constraint.op(),
                ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal
            )
        })
    {
        return None;
    }
    let mut zone = node.zone.clone();
    zone.up();
    reader_constrain_places(
        &mut zone,
        &timed.invariants(state.discrete()).ok()?,
        &certificate.scale,
    )?;
    reader_task_invariants(&mut zone, &empty, rows, layout, &certificate.scale)?;
    (!zone.is_empty()).then_some(())
}

/// A task counterexample did not replay from the re-admitted fixed-work
/// subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("task-product counterexample did not replay at step {step}")]
pub struct TaskProductReplayRefusal {
    /// The first step whose source or scheduler semantics differed.
    pub step: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReplayedTaskJob {
    identity: TaskTransitionIdentity,
    phase: u8,
    service: Rational,
}

#[derive(Clone, Debug)]
struct ReplayedTaskState {
    source: qsl_eval::simulation::model::TimedState,
    control: TaskSchedulerControl,
    jobs: Vec<ReplayedTaskJob>,
    arrivals: BTreeMap<(String, String), Rational>,
    generations: BTreeMap<(String, String, u64), u64>,
    activations: BTreeMap<(String, String), u64>,
    time: Rational,
    key: StateKey,
}

/// Independently replay a concrete FR-419 miss and its positive-time loop.
/// The typed identities are compared in full on every activation and
/// scheduler action; the finite state key is used only to check the
/// capture-avoiding loop quotient.
pub fn replay_task_counterexample(
    timed: &TimedSystem<'_, '_>,
    task_set: &TaskSet,
    releases: &[qsl_semantics::check::Release],
    policy: SchedulingPolicy,
    payload: &crate::model_check::counterexample::TemporalCounterexample<TaskSearchTransition>,
) -> Result<(), TaskProductReplayRefusal> {
    use crate::model_check::counterexample::CounterexampleKind;
    let refusal = |step| TaskProductReplayRefusal { step };
    let Some(lasso) = payload.lasso.filter(|lasso| !lasso.terminal_stutter) else {
        return Err(refusal(payload.prefix.len()));
    };
    if lasso.entry > payload.prefix.len()
        || !matches!(payload.kind, CounterexampleKind::Formula)
        || !releases
            .iter()
            .all(|release| release.task_set() == task_set.name)
    {
        return Err(refusal(0));
    }
    let queue_bounds = fixed_work_queue_bounds(task_set, policy).ok_or_else(|| refusal(0))?;
    let system = timed.system();
    let initial = timed
        .initial()
        .get(payload.initial_state)
        .cloned()
        .ok_or_else(|| refusal(0))?;
    let empty = TaskSchedulerControl {
        jobs: Vec::new(),
        running: None,
        activations: Vec::new(),
        pending_releases: Vec::new(),
    };
    let initial_key =
        reader_product_key(system, initial.discrete(), &empty).ok_or_else(|| refusal(0))?;
    let mut state = ReplayedTaskState {
        source: initial,
        control: empty,
        jobs: Vec::new(),
        arrivals: task_set
            .tasks
            .iter()
            .map(|task| ((task_set.name.clone(), task.name.clone()), zero_rational()))
            .collect(),
        generations: BTreeMap::new(),
        activations: BTreeMap::new(),
        time: zero_rational(),
        key: initial_key,
    };
    let mut positions = vec![state.clone()];
    let mut saw_miss = false;

    for (index, item) in payload.prefix.iter().enumerate() {
        let delay = item.delay.as_ref().ok_or_else(|| refusal(index))?;
        if delay.numerator().is_negative()
            || (!state.control.pending_releases.is_empty() && !delay.is_zero())
            || (reader_task_action_urgent(&state.control, task_set, policy)
                .ok_or_else(|| refusal(index))?
                && !delay.is_zero())
        {
            return Err(refusal(index));
        }
        let Some(delayed_source) = timed.delay(&state.source, delay).ok().flatten() else {
            return Err(refusal(index));
        };
        let next_time = state.time.add(delay);
        let mut next_jobs = state.jobs.clone();
        for job in &mut next_jobs {
            if state.control.running.as_ref().is_some_and(|running| {
                running.taskset == job.identity.taskset
                    && running.task == job.identity.task
                    && running.slot == job.identity.slot
                    && running.release == job.identity.release
            }) {
                job.service = job.service.add(delay);
            }
            let row = fixed_row(task_set, &job.identity.task).ok_or_else(|| refusal(index))?;
            if next_time > job.identity.deadline
                || job.service > fixed_task_work(row).ok_or_else(|| refusal(index))?
            {
                return Err(refusal(index));
            }
        }
        let mut next_arrivals = state.arrivals.clone();
        for value in next_arrivals.values_mut() {
            *value = value.add(delay);
        }
        let mut next = ReplayedTaskState {
            source: delayed_source,
            control: state.control.clone(),
            jobs: next_jobs,
            arrivals: next_arrivals,
            generations: state.generations.clone(),
            activations: state.activations.clone(),
            time: next_time,
            key: state.key.clone(),
        };
        match &item.transition {
            TaskSearchTransition::Source(transition) => {
                if !next.control.pending_releases.is_empty() {
                    return Err(refusal(index));
                }
                let candidate = timed
                    .steps(&next.source)
                    .ok()
                    .into_iter()
                    .flatten()
                    .find(|(step, _)| step.transition() == transition)
                    .map(|(_, reached)| reached)
                    .ok_or_else(|| refusal(index))?;
                for release in releases {
                    if system
                        .operation_text(release.operation().declaring(), release.operation().name())
                        == Some(transition.operation())
                    {
                        let key = (release.task_set().to_owned(), release.task().to_owned());
                        let arrival = next.arrivals.get(&key).ok_or_else(|| refusal(index))?;
                        let task =
                            fixed_row(task_set, release.task()).ok_or_else(|| refusal(index))?;
                        let task_key = (release.task_set().to_owned(), release.task().to_owned());
                        let has_activated = next.control.activations.contains(&task_key);
                        if has_activated
                            && (*arrival < task_interarrival(task)
                                || (matches!(task.arrival, Arrival::Periodic { .. })
                                    && *arrival > task_interarrival(task)))
                        {
                            return Err(refusal(index));
                        }
                        next.control
                            .pending_releases
                            .push(TaskSchedulerPendingRelease {
                                taskset: release.task_set().to_owned(),
                                task: release.task().to_owned(),
                                source: transition.clone(),
                            });
                    }
                }
                next.source = candidate;
            }
            TaskSearchTransition::Task(identity) => {
                let identity = identity.as_ref();
                let row = fixed_row(task_set, &identity.task).ok_or_else(|| refusal(index))?;
                if identity.taskset != task_set.name {
                    return Err(refusal(index));
                }
                let demand = fixed_task_work(row).ok_or_else(|| refusal(index))?;
                if identity.demand != demand
                    || !identity.jitter.is_zero()
                    || identity.priority
                        != task_set
                            .tasks
                            .iter()
                            .find(|task| task.name == identity.task)
                            .and_then(|task| task.priority)
                    || identity.budget_delta.is_some()
                {
                    return Err(refusal(index));
                }
                let rank = if identity.action == TaskAction::Activate {
                    u64::try_from(
                        next.control
                            .jobs
                            .iter()
                            .filter(|job| {
                                job.taskset == identity.taskset && job.task == identity.task
                            })
                            .count(),
                    )
                    .map_err(|_| refusal(index))?
                } else {
                    next.control
                        .jobs
                        .iter()
                        .find(|job| {
                            job.taskset == identity.taskset
                                && job.task == identity.task
                                && job.slot == identity.slot
                                && job.release == identity.release
                        })
                        .map(|job| job.rank)
                        .ok_or_else(|| refusal(index))?
                };
                let template = TaskTransitionTemplate {
                    action: identity.action,
                    taskset: identity.taskset.clone(),
                    task: identity.task.clone(),
                    job_rank: rank,
                    slot: identity.slot,
                    release: identity.release.clone(),
                    source_transition: identity.source_transition.clone(),
                    priority: identity.priority,
                };
                match identity.action {
                    TaskAction::Activate => {
                        let pending = next
                            .control
                            .pending_releases
                            .first()
                            .ok_or_else(|| refusal(index))?;
                        let task_key = (identity.taskset.clone(), identity.task.clone());
                        let activation = next.activations.get(&task_key).copied().unwrap_or(0);
                        let generation_key = (
                            identity.taskset.clone(),
                            identity.task.clone(),
                            identity.slot,
                        );
                        let generation =
                            next.generations.get(&generation_key).copied().unwrap_or(0);
                        let arrival = next.arrivals.get(&task_key).ok_or_else(|| refusal(index))?;
                        if pending.taskset != identity.taskset
                            || pending.task != identity.task
                            || pending.source != identity.release
                            || identity.source_transition.as_ref() != Some(&identity.release)
                            || identity.job != activation
                            || identity.generation != generation
                            || identity.deadline != next.time.add(&row.deadline)
                            || (next.control.activations.contains(&task_key)
                                && (*arrival < task_interarrival(row)
                                    || (matches!(row.arrival, Arrival::Periodic { .. })
                                        && *arrival > task_interarrival(row))))
                        {
                            return Err(refusal(index));
                        }
                        next.jobs.push(ReplayedTaskJob {
                            identity: identity.clone(),
                            phase: 1,
                            service: zero_rational(),
                        });
                        next.arrivals.insert(task_key.clone(), zero_rational());
                        next.activations
                            .insert(task_key, activation.saturating_add(1));
                    }
                    TaskAction::Dispatch | TaskAction::Resume => {
                        let job = next
                            .jobs
                            .iter_mut()
                            .find(|job| same_replay_job(&job.identity, identity))
                            .ok_or_else(|| refusal(index))?;
                        if next.control.running.is_some()
                            || (identity.action == TaskAction::Dispatch && job.phase != 1)
                            || (identity.action == TaskAction::Resume && job.phase != 3)
                        {
                            return Err(refusal(index));
                        }
                        if next
                            .control
                            .jobs
                            .iter()
                            .any(|candidate| matches!(candidate.phase, 1 | 3))
                        {
                            let selected = next
                                .jobs
                                .iter()
                                .filter(|candidate| matches!(candidate.phase, 1 | 3))
                                .min_by(|left, right| {
                                    replay_priority_order(left, right, task_set, policy)
                                });
                            if selected.is_some_and(|selected| {
                                !same_replay_job(&selected.identity, identity)
                            }) {
                                return Err(refusal(index));
                            }
                        }
                    }
                    TaskAction::Preempt => {
                        let running = next
                            .control
                            .running
                            .as_ref()
                            .ok_or_else(|| refusal(index))?;
                        if running.taskset != identity.taskset
                            || running.task != identity.task
                            || running.slot != identity.slot
                            || running.release != identity.release
                        {
                            return Err(refusal(index));
                        }
                        let Some(running_job) = next
                            .jobs
                            .iter()
                            .find(|job| same_replay_job(&job.identity, identity))
                        else {
                            return Err(refusal(index));
                        };
                        if running_job.phase != 2 {
                            return Err(refusal(index));
                        }
                    }
                    TaskAction::Complete => {
                        let job = next
                            .jobs
                            .iter()
                            .find(|job| same_replay_job(&job.identity, identity))
                            .ok_or_else(|| refusal(index))?;
                        if job.phase != 2 || job.service != demand {
                            return Err(refusal(index));
                        }
                    }
                    TaskAction::DeadlineMiss => {
                        let job = next
                            .jobs
                            .iter()
                            .find(|job| same_replay_job(&job.identity, identity))
                            .ok_or_else(|| refusal(index))?;
                        if next.time < job.identity.deadline || job.service >= demand {
                            return Err(refusal(index));
                        }
                        saw_miss = true;
                    }
                    TaskAction::Release | TaskAction::BudgetChange => return Err(refusal(index)),
                }
                next.control =
                    reader_apply_task_action(&next.control, &template, &queue_bounds, task_set)
                        .ok_or_else(|| refusal(index))?;
                match identity.action {
                    TaskAction::Activate => {}
                    TaskAction::Dispatch | TaskAction::Resume => {
                        let job = next
                            .jobs
                            .iter_mut()
                            .find(|job| same_replay_job(&job.identity, identity))
                            .ok_or_else(|| refusal(index))?;
                        job.phase = 2;
                    }
                    TaskAction::Preempt => {
                        let job = next
                            .jobs
                            .iter_mut()
                            .find(|job| same_replay_job(&job.identity, identity))
                            .ok_or_else(|| refusal(index))?;
                        job.phase = 3;
                    }
                    TaskAction::Complete | TaskAction::DeadlineMiss => {
                        next.jobs
                            .retain(|job| !same_replay_job(&job.identity, identity));
                        let generation_key = (
                            identity.taskset.clone(),
                            identity.task.clone(),
                            identity.slot,
                        );
                        next.generations
                            .insert(generation_key, identity.generation.saturating_add(1));
                    }
                    TaskAction::Release | TaskAction::BudgetChange => return Err(refusal(index)),
                }
            }
        }
        let expected = reader_product_key(system, next.source.discrete(), &next.control)
            .ok_or_else(|| refusal(index))?;
        if expected != item.post_state {
            return Err(refusal(index));
        }
        next.key = expected;
        state = next;
        positions.push(state.clone());
    }
    if !saw_miss || !payload.prefix[..lasso.entry].iter().any(|step| matches!(step.transition, TaskSearchTransition::Task(ref task) if task.action == TaskAction::DeadlineMiss)) {
        return Err(refusal(payload.prefix.len()));
    }
    let Some(entry) = positions.get(lasso.entry) else {
        return Err(refusal(payload.prefix.len()));
    };
    if entry.key != state.key || entry.control != state.control || entry.arrivals != state.arrivals
    {
        return Err(refusal(payload.prefix.len()));
    }
    let entry_jobs = replayed_job_quotient(&entry.jobs, &entry.time);
    let final_jobs = replayed_job_quotient(&state.jobs, &state.time);
    if entry_jobs != final_jobs
        || !payload.prefix[lasso.entry..]
            .iter()
            .filter_map(|step| step.delay.as_ref())
            .any(|delay| !delay.is_zero())
    {
        return Err(refusal(payload.prefix.len()));
    }
    Ok(())
}

fn zero_rational() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn fixed_row<'a>(task_set: &'a TaskSet, name: &str) -> Option<&'a qsl_semantics::check::Task> {
    task_set.tasks.iter().find(|task| task.name == name)
}

fn task_interarrival(task: &qsl_semantics::check::Task) -> Rational {
    match &task.arrival {
        Arrival::Periodic { period } => period.clone(),
        Arrival::Sporadic { min_interarrival } => min_interarrival.clone(),
    }
}

fn fixed_task_work(task: &qsl_semantics::check::Task) -> Option<Rational> {
    task.fixed_work().cloned()
}

fn same_replay_job(left: &TaskTransitionIdentity, right: &TaskTransitionIdentity) -> bool {
    left.taskset == right.taskset
        && left.task == right.task
        && left.job == right.job
        && left.slot == right.slot
        && left.generation == right.generation
        && left.release == right.release
}

fn bounded_task_search_scale(timed: &TimedSystem<'_, '_>) -> Option<Integer> {
    let model = timed.model();
    let mut constants = vec![Rational::from_integer(Integer::one())];
    let mut parts: Vec<_> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .collect();
    while let Some(part) = parts.pop() {
        constants.extend(
            part.clocks()
                .iter()
                .map(|constraint| constraint.bound().clone()),
        );
        parts.extend(part.families().iter().flat_map(|family| family.parts()));
    }
    constants.extend(
        model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds())
            .map(|constraint| constraint.bound().clone()),
    );
    constants.extend(
        model
            .resets()
            .iter()
            .flat_map(|resets| resets.resets())
            .map(|reset| reset.value().clone()),
    );
    let factor = constants
        .iter()
        .fold(num_bigint::BigInt::from(1), |factor, value| {
            factor.lcm(value.denominator().as_big())
        });
    Some(Integer::from_big(factor))
}

fn reader_bounded_source_settle(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    scale: &Integer,
    allow_delay: bool,
    zone: &mut Zone,
) -> Option<()> {
    let invariants = timed.invariants(state).ok()?;
    reader_constrain_places(zone, &invariants, scale)?;
    if allow_delay && !zone.is_empty() && !timed.is_urgent(state).ok()? {
        zone.up();
        reader_constrain_places(zone, &invariants, scale)?;
    }
    Some(())
}

fn reader_free_dead_source_clocks(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    zone: &mut Zone,
) -> Option<()> {
    for (index, place) in timed.places().iter().enumerate() {
        if !timed.live(place, state) {
            zone.free(index + 1).ok()?;
        }
    }
    Some(())
}

fn replay_priority_order(
    left: &ReplayedTaskJob,
    right: &ReplayedTaskJob,
    task_set: &TaskSet,
    policy: SchedulingPolicy,
) -> std::cmp::Ordering {
    let left_task = task_set
        .tasks
        .iter()
        .find(|task| task.name == left.identity.task);
    let right_task = task_set
        .tasks
        .iter()
        .find(|task| task.name == right.identity.task);
    match policy {
        SchedulingPolicy::FixedPriority => left_task
            .and_then(|task| task.priority)
            .cmp(&right_task.and_then(|task| task.priority))
            .then_with(|| left.identity.task.cmp(&right.identity.task))
            .then_with(|| left.identity.job.cmp(&right.identity.job)),
        SchedulingPolicy::Edf => left
            .identity
            .deadline
            .cmp(&right.identity.deadline)
            .then_with(|| left.identity.task.cmp(&right.identity.task))
            .then_with(|| left.identity.release.cmp(&right.identity.release))
            .then_with(|| left.identity.job.cmp(&right.identity.job)),
        SchedulingPolicy::Amc => std::cmp::Ordering::Equal,
    }
}

fn replayed_job_quotient(
    jobs: &[ReplayedTaskJob],
    now: &Rational,
) -> Vec<(String, String, u64, u8, Rational, Rational)> {
    let mut quotient = jobs
        .iter()
        .map(|job| {
            (
                job.identity.taskset.clone(),
                job.identity.task.clone(),
                job.identity.slot,
                job.phase,
                job.service.clone(),
                job.identity.deadline.sub(now),
            )
        })
        .collect::<Vec<_>>();
    quotient.sort();
    quotient
}

/// Independently replay FR-331 evidence for a bounded task subject whose
/// source has no clock places. This route never decides schedulability: it
/// re-admits each retained source edge and verifies that every unresolved
/// release is represented by a real demand/queue frontier, or that the
/// configured work limit actually accounts for the stop.
pub fn replay_bounded_task_search_evidence<O>(
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedScheduleClaim,
    task_set: &TaskSet,
    releases: &[qsl_semantics::check::Release],
    evidence: &TaskSearchEvidence<O>,
) -> Result<(), TaskProductReplayRefusal> {
    let refusal = |step| TaskProductReplayRefusal { step };
    if !evidence.has_valid_shape()
        || claim.task_set != task_set.name
        || !matches!(claim.claim, ScheduleClaim::Schedulable(_))
        || timed.tick_period().is_some()
        || !releases
            .iter()
            .all(|release| release.task_set() == task_set.name)
    {
        return Err(refusal(0));
    }
    let system = timed.system();
    let scale = bounded_task_search_scale(timed).ok_or_else(|| refusal(0))?;
    let dim = timed
        .places()
        .len()
        .checked_add(1)
        .ok_or_else(|| refusal(0))?;
    let (lower, upper) = computed_bounds(timed, None, &scale).ok_or_else(|| refusal(0))?;
    let coverage = LuBounds { lower, upper };
    if evidence.nodes.iter().any(|node| node.zone.dim() != dim) {
        return Err(refusal(0));
    }
    let empty = TaskSchedulerControl {
        jobs: Vec::new(),
        running: None,
        activations: Vec::new(),
        pending_releases: Vec::new(),
    };
    let mut source_states: Vec<Option<ModelSimState>> = vec![None; evidence.nodes.len()];
    let mut queue = Vec::new();
    for initial in timed.initial() {
        let mut zone = Zone::zero(dim);
        reader_bounded_source_settle(timed, initial.discrete(), &scale, true, &mut zone)
            .ok_or_else(|| refusal(0))?;
        reader_free_dead_source_clocks(timed, initial.discrete(), &mut zone)
            .ok_or_else(|| refusal(0))?;
        if zone.is_empty() {
            continue;
        }
        let key =
            reader_product_key(system, initial.discrete(), &empty).ok_or_else(|| refusal(0))?;
        let Some(root) = evidence.nodes.iter().position(|node| {
            node.depth == 0
                && node.state == key
                && zone.alu_covered_by(&node.zone, &coverage).unwrap_or(false)
        }) else {
            return Err(refusal(0));
        };
        source_states[root].get_or_insert_with(|| initial.discrete().clone());
        queue.push(root);
    }
    if queue.is_empty() {
        return Err(refusal(0));
    }
    if evidence.limit.as_ref().is_some_and(|limit| {
        limit.kind == TaskSearchLimitKind::Work
            && evidence.nodes.iter().filter(|node| node.expanded).count() as u64 != limit.value
    }) {
        return Err(refusal(0));
    }
    let mut reached = vec![false; evidence.nodes.len()];
    for &root in &queue {
        reached[root] = true;
    }
    let mut used = vec![false; evidence.edges.len()];
    let mut cursor = 0;
    while let Some(node_index) = queue.get(cursor).copied() {
        cursor += 1;
        let node = evidence
            .nodes
            .get(node_index)
            .ok_or_else(|| refusal(node_index))?;
        let key = TaskProductStateKey::decode(&node.state)
            .map_err(|_| refusal(node_index))?
            .ok_or_else(|| refusal(node_index))?;
        let control = key.scheduler_state().map_err(|_| refusal(node_index))?;
        if !control.jobs.is_empty() || control.running.is_some() || !control.activations.is_empty()
        {
            return Err(refusal(node_index));
        }
        if !control.pending_releases.is_empty() {
            if control.pending_releases.iter().any(|pending| {
                pending.taskset != task_set.name
                    || !task_set.tasks.iter().any(|task| task.name == pending.task)
                    || !releases.iter().any(|release| {
                        release.task_set() == pending.taskset
                            && release.task() == pending.task
                            && system.operation_text(
                                release.operation().declaring(),
                                release.operation().name(),
                            ) == Some(pending.source.operation())
                    })
            }) {
                return Err(refusal(node_index));
            }
            let frontier = evidence
                .frontier
                .iter()
                .find(|row| row.node == node_index)
                .ok_or_else(|| refusal(node_index))?;
            let budget_change_open = task_set
                .tasks
                .iter()
                .any(|task| task.criticality == qsl_semantics::check::Criticality::Hi);
            if !node.expanded
                && frontier.reason == TaskSearchStopReason::UnresolvedChoice
                && frontier.open.contains(&TaskSearchOpen::Demand)
                && frontier.open.contains(&TaskSearchOpen::Queue)
                && frontier.open.contains(&TaskSearchOpen::BudgetChange) == budget_change_open
                && frontier.open.len() == if budget_change_open { 3 } else { 2 }
            {
                continue;
            }
            return Err(refusal(node_index));
        }
        if !node.expanded {
            let frontier = evidence
                .frontier
                .iter()
                .find(|row| row.node == node_index)
                .ok_or_else(|| refusal(node_index))?;
            if evidence.limit.is_some()
                && frontier.reason != TaskSearchStopReason::UnresolvedChoice
                && frontier.open.contains(&TaskSearchOpen::Successors)
            {
                continue;
            }
            return Err(refusal(node_index));
        }
        let source = source_states[node_index]
            .as_ref()
            .ok_or_else(|| refusal(node_index))?;
        let mut source_zone = node.zone.clone();
        reader_bounded_source_settle(timed, source, &scale, true, &mut source_zone)
            .ok_or_else(|| refusal(node_index))?;
        if source_zone.is_empty() {
            return Err(refusal(node_index));
        }
        let mut successors = Vec::new();
        for step in timed
            .guarded_steps(source)
            .map_err(|_| refusal(node_index))?
        {
            for (part_index, part) in step.parts().iter().enumerate() {
                let mut next_zone = source_zone.clone();
                reader_constrain_places(&mut next_zone, part, &scale)
                    .ok_or_else(|| refusal(node_index))?;
                if next_zone.is_empty() {
                    continue;
                }
                let carries = step
                    .carries()
                    .iter()
                    .map(|(target, from)| (target + 1, from + 1))
                    .collect::<Vec<_>>();
                let resets = step
                    .resets()
                    .iter()
                    .map(|(clock, value)| Some((clock + 1, reader_scaled(value, &scale)?)))
                    .collect::<Option<Vec<_>>>();
                next_zone
                    .assign_clocks(&carries, &resets.ok_or_else(|| refusal(node_index))?)
                    .map_err(|_| refusal(node_index))?;
                reader_constrain_places(
                    &mut next_zone,
                    &timed
                        .invariants(step.post())
                        .map_err(|_| refusal(node_index))?,
                    &scale,
                )
                .ok_or_else(|| refusal(node_index))?;
                let mut next_control = control.clone();
                for release in releases.iter().filter(|release| {
                    system
                        .operation_text(release.operation().declaring(), release.operation().name())
                        == Some(step.transition().operation())
                }) {
                    next_control
                        .pending_releases
                        .push(TaskSchedulerPendingRelease {
                            taskset: release.task_set().to_owned(),
                            task: release.task().to_owned(),
                            source: step.transition().clone(),
                        });
                }
                reader_bounded_source_settle(
                    timed,
                    step.post(),
                    &scale,
                    next_control.pending_releases.is_empty(),
                    &mut next_zone,
                )
                .ok_or_else(|| refusal(node_index))?;
                reader_free_dead_source_clocks(timed, step.post(), &mut next_zone)
                    .ok_or_else(|| refusal(node_index))?;
                if next_zone.is_empty() {
                    continue;
                }
                let key = reader_product_key(system, step.post(), &next_control)
                    .ok_or_else(|| refusal(node_index))?;
                successors.push((
                    TaskSearchTransition::Source(step.transition().clone()),
                    step.post().clone(),
                    key,
                    next_zone,
                    part_index,
                ));
            }
        }
        let outgoing: Vec<_> = evidence
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.node == node_index)
            .collect();
        let mut matched = BTreeSet::new();
        for (transition, reached_source, target_key, next_zone, _part_index) in successors {
            let edge_index = outgoing
                .iter()
                .find_map(|(index, edge)| {
                    (!matched.contains(index)
                        && edge.transition == transition
                        && evidence.nodes.get(edge.target).is_some_and(|target| {
                            target.state == target_key
                                && target.depth == node.depth.saturating_add(1)
                                && next_zone
                                    .alu_covered_by(&target.zone, &coverage)
                                    .unwrap_or(false)
                        }))
                    .then_some(*index)
                })
                .ok_or_else(|| refusal(node_index))?;
            matched.insert(edge_index);
            used[edge_index] = true;
            let target = evidence.edges[edge_index].target;
            source_states[target].get_or_insert(reached_source);
            if !reached[target] {
                reached[target] = true;
                queue.push(target);
            }
        }
        if matched.len() != outgoing.len() {
            return Err(refusal(node_index));
        }
    }
    if reached.iter().any(|reached| !reached) || used.iter().any(|used| !used) {
        return Err(refusal(evidence.nodes.len()));
    }
    Ok(())
}

/// Recompute every bounded subtraction named by the listed finite-product
/// graph. Each completion edge subtracts the completing task's exact fixed
/// work from each suspended job's service clock. The graph closure itself is
/// checked separately; this metadata check prevents a certificate from
/// omitting or altering the subtraction contract for edges it does list.
fn fixed_work_subtractions<O>(
    certificate: &ZoneCertificate<O>,
    rows: &[TaskQueueBound],
    scale: &Integer,
) -> Option<Vec<TaskSubtraction>> {
    if scale <= &Integer::zero() {
        return None;
    }
    let mut expected = Vec::new();
    for edge in &certificate.edges {
        let Some(CertificateTransition::Task(template)) = &edge.transition else {
            continue;
        };
        if template.action != TaskAction::Complete {
            continue;
        }
        let row = rows
            .iter()
            .find(|row| row.taskset == template.taskset && row.task == template.task)?;
        let scaled = row.work_upper.mul(&Rational::from_integer(scale.clone()));
        if scaled.denominator() != &Integer::one() {
            return None;
        }
        let amount = scaled.numerator().clone();
        let source_node = certificate.nodes.get(edge.from)?;
        let product_key = TaskProductStateKey::decode(&source_node.state_key)
            .ok()??
            .scheduler_state()
            .ok()?;
        for suspended in product_key.jobs.iter().filter(|job| job.phase == 3) {
            let subtraction = TaskSubtraction {
                transition: template.as_ref().clone(),
                clock: format!(
                    "task:{}:{}:slot{}:service",
                    suspended.taskset, suspended.task, suspended.slot
                ),
                constant: amount.clone(),
            };
            if !expected.contains(&subtraction) {
                expected.push(subtraction);
            }
        }
    }
    Some(expected)
}

/// Check each listed joint key against the recompiled task-set rows. The
/// scheduler key is evidence, not authority: it may contain only checked
/// tasks, exact fixed demand and declared release bindings. A no-release
/// certificate must carry the empty scheduler control at every source node.
fn task_product_controls_match<O>(
    certificate: &ZoneCertificate<O>,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
    releases: &[qsl_semantics::check::Release],
    system: &ModelSystem<'_>,
    no_release: bool,
) -> bool {
    for node in &certificate.nodes {
        let Ok(Some(key)) = TaskProductStateKey::decode(&node.state_key) else {
            return false;
        };
        let Ok(control) = key.scheduler_state() else {
            return false;
        };
        if no_release {
            if !control.jobs.is_empty()
                || control.running.is_some()
                || !control.activations.is_empty()
                || !control.pending_releases.is_empty()
            {
                return false;
            }
            continue;
        }

        let has_task = |taskset: &str, name: &str| {
            taskset == task_set.name
                && rows
                    .iter()
                    .any(|row| row.taskset == taskset && row.task == name)
        };
        if control
            .activations
            .iter()
            .any(|(taskset, name)| !has_task(taskset, name))
            || control.pending_releases.iter().any(|pending| {
                !has_task(&pending.taskset, &pending.task)
                    || !releases.iter().any(|release| {
                        release.task_set() == pending.taskset
                            && release.task() == pending.task
                            && release.task_set() == task_set.name
                            && system.operation_text(
                                release.operation().declaring(),
                                release.operation().name(),
                            ) == Some(pending.source.operation())
                    })
            })
        {
            return false;
        }
        for job in &control.jobs {
            if !has_task(&job.taskset, &job.task) {
                return false;
            }
            let Some(row) = rows
                .iter()
                .find(|row| row.taskset == job.taskset && row.task == job.task)
            else {
                return false;
            };
            let Some(task) = task_set.tasks.iter().find(|task| task.name == job.task) else {
                return false;
            };
            if job.rank >= row.max_jobs
                || job.slot >= row.max_jobs
                || job.demand != row.work_upper.to_string()
                || job.priority != task.priority
                || !releases.iter().any(|release| {
                    release.task_set() == job.taskset
                        && release.task() == job.task
                        && release.task_set() == task_set.name
                        && system.operation_text(
                            release.operation().declaring(),
                            release.operation().name(),
                        ) == Some(job.release.operation())
                })
            {
                return false;
            }
        }
    }
    true
}

/// Independently check that each listed source edge queues exactly the
/// checked release declarations bound to that operation, in declaration
/// order. Pending activations are drained before another source step, so a
/// source edge may begin only with an empty pending list.
fn fixed_work_source_release_edges_match<O>(
    certificate: &ZoneCertificate<O>,
    releases: &[qsl_semantics::check::Release],
    system: &ModelSystem<'_>,
) -> bool {
    for edge in &certificate.edges {
        let Some(CertificateTransition::Source(transition)) = &edge.transition else {
            continue;
        };
        let Some(from) = certificate.nodes.get(edge.from) else {
            return false;
        };
        let Some(to) = certificate.nodes.get(edge.to) else {
            return false;
        };
        let Ok(Some(from_key)) = TaskProductStateKey::decode(&from.state_key) else {
            return false;
        };
        let Ok(Some(to_key)) = TaskProductStateKey::decode(&to.state_key) else {
            return false;
        };
        let Ok(mut expected) = from_key.scheduler_state() else {
            return false;
        };
        let Ok(actual) = to_key.scheduler_state() else {
            return false;
        };
        if !expected.pending_releases.is_empty() {
            return false;
        }
        for release in releases {
            if system.operation_text(release.operation().declaring(), release.operation().name())
                == Some(transition.operation())
            {
                expected.pending_releases.push(TaskSchedulerPendingRelease {
                    taskset: release.task_set().to_owned(),
                    task: release.task().to_owned(),
                    source: transition.clone(),
                });
            }
        }
        if actual != expected {
            return false;
        }
    }
    true
}

/// Recompute finite scheduler-control changes for every listed task edge.
/// Time guards and complete enabled-successor inventory are checked by the
/// product closure traversal; this check independently prevents a typed edge
/// from changing unrelated control fields or naming a nonexistent job.
fn fixed_work_task_control_edges_match<O>(
    certificate: &ZoneCertificate<O>,
    rows: &[TaskQueueBound],
    task_set: &TaskSet,
) -> bool {
    for edge in &certificate.edges {
        let Some(CertificateTransition::Task(template)) = &edge.transition else {
            continue;
        };
        let Some(from) = certificate.nodes.get(edge.from) else {
            return false;
        };
        let Some(to) = certificate.nodes.get(edge.to) else {
            return false;
        };
        let Ok(Some(from_key)) = TaskProductStateKey::decode(&from.state_key) else {
            return false;
        };
        let Ok(Some(to_key)) = TaskProductStateKey::decode(&to.state_key) else {
            return false;
        };
        let Ok(mut expected) = from_key.scheduler_state() else {
            return false;
        };
        let Ok(actual) = to_key.scheduler_state() else {
            return false;
        };
        let Some(row) = rows
            .iter()
            .find(|row| row.taskset == template.taskset && row.task == template.task)
        else {
            return false;
        };
        let Some(task) = task_set
            .tasks
            .iter()
            .find(|task| task.name == template.task)
        else {
            return false;
        };
        if template.priority != task.priority
            || (template.action != TaskAction::Activate && template.source_transition.is_some())
        {
            return false;
        }
        match template.action {
            TaskAction::Activate => {
                let Some(pending) = expected.pending_releases.first() else {
                    return false;
                };
                if pending.taskset != template.taskset
                    || pending.task != template.task
                    || pending.source != template.release
                    || template.source_transition.as_ref() != Some(&template.release)
                {
                    return false;
                }
                let same_task: Vec<_> = expected
                    .jobs
                    .iter()
                    .filter(|job| job.taskset == template.taskset && job.task == template.task)
                    .collect();
                if u64::try_from(same_task.len()).ok() != Some(template.job_rank)
                    || same_task.len() >= usize::try_from(row.max_jobs).unwrap_or(usize::MAX)
                {
                    return false;
                }
                let Some(slot) =
                    (0..row.max_jobs).find(|slot| same_task.iter().all(|job| job.slot != *slot))
                else {
                    return false;
                };
                if slot != template.slot {
                    return false;
                }
                expected.pending_releases.remove(0);
                expected.jobs.push(TaskSchedulerJob {
                    taskset: template.taskset.clone(),
                    task: template.task.clone(),
                    rank: template.job_rank,
                    slot: template.slot,
                    release: template.release.clone(),
                    phase: 1,
                    demand: row.work_upper.to_string(),
                    priority: task.priority,
                });
                expected.jobs.sort_by(|left, right| {
                    (&left.taskset, &left.task, left.rank).cmp(&(
                        &right.taskset,
                        &right.task,
                        right.rank,
                    ))
                });
                if !expected
                    .activations
                    .iter()
                    .any(|(taskset, name)| taskset == &template.taskset && name == &template.task)
                {
                    expected
                        .activations
                        .push((template.taskset.clone(), template.task.clone()));
                    expected.activations.sort();
                }
            }
            TaskAction::Dispatch | TaskAction::Resume => {
                if expected.running.is_some() {
                    return false;
                }
                let wanted_phase = if template.action == TaskAction::Dispatch {
                    1
                } else {
                    3
                };
                let Some(job) = expected.jobs.iter_mut().find(|job| {
                    job.taskset == template.taskset
                        && job.task == template.task
                        && job.rank == template.job_rank
                        && job.slot == template.slot
                        && job.release == template.release
                }) else {
                    return false;
                };
                if job.phase != wanted_phase {
                    return false;
                }
                job.phase = 2;
                expected.running = Some(TaskSchedulerRunning {
                    taskset: template.taskset.clone(),
                    task: template.task.clone(),
                    slot: template.slot,
                    release: template.release.clone(),
                });
            }
            TaskAction::Preempt => {
                let Some(job) = expected.jobs.iter_mut().find(|job| {
                    job.taskset == template.taskset
                        && job.task == template.task
                        && job.rank == template.job_rank
                        && job.slot == template.slot
                        && job.release == template.release
                }) else {
                    return false;
                };
                if job.phase != 2
                    || expected.running.as_ref().is_none_or(|running| {
                        running.taskset != template.taskset
                            || running.task != template.task
                            || running.slot != template.slot
                            || running.release != template.release
                    })
                {
                    return false;
                }
                job.phase = 3;
                expected.running = None;
            }
            TaskAction::Complete | TaskAction::DeadlineMiss => {
                let Some(index) = expected.jobs.iter().position(|job| {
                    job.taskset == template.taskset
                        && job.task == template.task
                        && job.rank == template.job_rank
                        && job.slot == template.slot
                        && job.release == template.release
                        && (template.action == TaskAction::DeadlineMiss || job.phase == 2)
                }) else {
                    return false;
                };
                expected.jobs.remove(index);
                if expected.running.as_ref().is_some_and(|running| {
                    running.taskset == template.taskset
                        && running.task == template.task
                        && running.slot == template.slot
                        && running.release == template.release
                }) {
                    expected.running = None;
                }
                for job in &mut expected.jobs {
                    if job.taskset == template.taskset
                        && job.task == template.task
                        && job.rank > template.job_rank
                    {
                        job.rank -= 1;
                    }
                }
            }
            TaskAction::Release | TaskAction::BudgetChange => return false,
        }
        if expected != actual {
            return false;
        }
    }
    true
}

/// FR-244's refinement part: the reachability part is read over the
/// refinement product, each node's `automaton_state` an index into
/// `positions`, or `positions.len()` for a node of the locked region.
///
/// The concrete initial states' nodes are the certificate's own
/// [`ZoneCertificate::initials`] (QSpec FR-418): the part lists no
/// `initials` of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementPart {
    /// QSpec FR-418 `refinement_node`: the refinement node's id.
    pub refinement_node: NodeKey,
    /// The product positions, FR-149's type.
    pub positions: Vec<CertifiedPosition>,
    /// What `check_step` took on each edge, one per entry of `edges` in
    /// order; `None` for an edge into or inside the locked region.
    pub taken: Vec<Option<Taken>>,
}

/// What FR-245 compares a refinement part's identity by.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementIdentity {
    /// The refinement node's id.
    pub refinement_node: NodeKey,
    /// The emitted package the abstract side is opened over, compared
    /// structurally: the certified refinement's own package on the listed
    /// side, the request's `abstract_package` on the recompiled side.
    pub abstract_package: EmittedPackage,
}

/// What a node of FR-240's liveness product carries beyond a search
/// node: the instance whose claim automaton it reads, and how it was
/// entered, which places it in the product's acceptance sets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductMark {
    /// The index of the `over` instance, in the subject's order.
    pub instance: usize,
    /// Entered by a position: the initial position, a step or a stutter.
    pub positioned: bool,
    /// Entered by the divergence clock's reset, or under a tick source by
    /// a tick.
    pub divergent: bool,
    /// In the idle tail (FR-231): the behaviour takes no further step.
    pub tail: bool,
}

/// A product edge that is no position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SilentMove {
    /// A delay into the next cell of the node's state (FR-240's split by
    /// the fairness guards).
    Wait,
    /// The reset of the divergence clock `z`, the zone's last clock, once
    /// it has reached 1 (the non-Zeno construction).
    Diverge,
}

/// A component numbering (FR-244's Büchi part): every edge between
/// numbered nodes goes to an equal or greater index, so every cycle lies in
/// one component.
///
/// Its `Clone`, `Debug`, `PartialEq` and `Drop` read nested numberings
/// without recursion: a crafted certificate may nest strong reasons deeper
/// than the stack (SR-1647001 FND-001, FND-004).
pub struct Components {
    /// Each node's component, by node; `None` for a node the numbering does
    /// not cover.
    pub numbering: Vec<Option<usize>>,
    /// Each component's reason, by index.
    pub reasons: Vec<ComponentReason>,
}

/// A reason of one level of a flattened [`Components`]: a leaf reason, or
/// a strong reason's constraints and the level of its nested numbering.
#[derive(Debug, PartialEq)]
enum LevelReason<'c> {
    Leaf(&'c ComponentReason),
    Strong(&'c [FairnessMember], usize),
}

/// One level of a flattened [`Components`]: its numbering and reasons.
type Level<'c> = (&'c [Option<usize>], Vec<LevelReason<'c>>);

impl Components {
    /// Every numbering of `self`, nested ones included, breadth first:
    /// level 0 is `self`, and a strong reason names the later level of its
    /// nested numbering. Read without recursion.
    fn levels(&self) -> Vec<Level<'_>> {
        let mut order: Vec<&Self> = vec![self];
        let mut levels = Vec::new();
        let mut at = 0;
        while let Some(&components) = order.get(at) {
            let mut reasons = Vec::with_capacity(components.reasons.len());
            for reason in &components.reasons {
                reasons.push(match reason {
                    ComponentReason::Strong {
                        constraints,
                        nested,
                    } => {
                        order.push(nested);
                        LevelReason::Strong(constraints, order.len() - 1)
                    }
                    leaf => LevelReason::Leaf(leaf),
                });
            }
            levels.push((components.numbering.as_slice(), reasons));
            at += 1;
        }
        levels
    }
}

impl Clone for Components {
    /// Rebuilt from its levels, deepest first.
    fn clone(&self) -> Self {
        let levels = self.levels();
        let mut built: Vec<Option<Self>> = (0..levels.len()).map(|_| None).collect();
        for (index, (numbering, reasons)) in levels.iter().enumerate().rev() {
            let reasons = reasons
                .iter()
                .map(|reason| match reason {
                    // A leaf holds no numbering: its own clone is shallow.
                    LevelReason::Leaf(leaf) => (*leaf).clone(),
                    LevelReason::Strong(constraints, level) => ComponentReason::Strong {
                        constraints: constraints.to_vec(),
                        nested: Box::new(built[*level].take().unwrap_or_else(|| Self {
                            numbering: Vec::new(),
                            reasons: Vec::new(),
                        })),
                    },
                })
                .collect();
            built[index] = Some(Self {
                numbering: numbering.to_vec(),
                reasons,
            });
        }
        built
            .first_mut()
            .and_then(Option::take)
            .unwrap_or_else(|| Self {
                numbering: Vec::new(),
                reasons: Vec::new(),
            })
    }
}

impl PartialEq for Components {
    fn eq(&self, other: &Self) -> bool {
        self.levels() == other.levels()
    }
}

impl Eq for Components {}

impl fmt::Debug for Components {
    /// Its levels, breadth first, each strong reason naming its nested
    /// numbering's level.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Components")
            .field("levels", &self.levels())
            .finish()
    }
}

impl Drop for Components {
    /// Drops nested numberings without recursion: a crafted certificate
    /// may nest strong reasons deeper than the stack (SR-1647001 FND-001).
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.reasons);
        while let Some(reason) = pending.pop() {
            if let ComponentReason::Strong { mut nested, .. } = reason {
                pending.append(&mut nested.reasons);
            }
        }
    }
}

/// Why a component holds no fair accepting cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentReason {
    /// No edge joins two of its nodes: it holds no cycle.
    Acyclic,
    /// None of its nodes is in the acceptance set.
    Lacks(AcceptanceSet),
    /// The weak fairness constraint is enabled at every valuation of every
    /// node and taken by no edge inside.
    Weak(FairnessMember),
    /// The strong fairness constraints are taken by no edge inside, and the
    /// component without the nodes that enable any of them at some
    /// valuation has the nested numbering, each of whose components has its
    /// own reason.
    Strong {
        /// The constraints.
        constraints: Vec<FairnessMember>,
        /// The numbering of the rest.
        nested: Box<Components>,
    },
}

/// An acceptance set of the liveness product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptanceSet {
    /// The claim automaton's acceptance set at this index.
    Automaton(usize),
    /// The nodes entered by the divergence clock's reset or a tick.
    Divergence,
    /// The nodes entered by a position.
    Position,
}

/// One fairness constraint of the clause: the constraint at `constraint`
/// in the clause's fairness set, and for an `each` constraint the one
/// transition identity it is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FairnessMember {
    /// The constraint's index in the clause's fairness set.
    pub constraint: usize,
    /// The transition identity of an `each` constraint; `None` for a
    /// `whole` one.
    pub transition: Option<ModelTransition>,
}

/// One certificate node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertNode {
    /// The state key of the model state's state key.
    pub state_key: qsl_eval::simulation::StateKey,
    /// The claim-automaton state; 0 for an item that needs no automaton.
    pub automaton_state: usize,
    /// The zone, in scaled units, over the reference clock, the clock
    /// places and, for a TT-2 claim, the horizon clock.
    pub zone: Zone,
    /// Whether the node violates the item only on time-locked valuations,
    /// which FR-239 continues past.
    pub locked: bool,
    /// For a node of FR-240's liveness product, its instance and entry.
    pub product: Option<ProductMark>,
}

/// One certificate edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertEdge {
    /// The source node.
    pub from: usize,
    /// The source or task step's typed transition identity; `None` for an
    /// idle-tail stutter position of a TT-2 claim's automaton (FR-242).
    pub transition: Option<CertificateTransition>,
    /// The convex part of the step's guard taken; `None` for a stutter.
    pub part: Option<usize>,
    /// Under a dense source, the piece of a TT-2 claim's horizon axis the
    /// position lies in, as constraints on the horizon clock; `None` with
    /// no automaton, past the formula's horizon, and under a tick source
    /// (spec gap STD-493: FR-244's edge names no piece).
    pub horizon: Option<Vec<PlaceConstraint>>,
    /// The node whose zone holds, or under the aLU simulation covers, the
    /// successor.
    pub to: usize,
    /// A liveness product's silent edge; `transition` and `part` are then
    /// `None`.
    pub silent: Option<SilentMove>,
}

/// One transition identity carried by a zone-certificate edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificateTransition {
    /// An admitted source-model operation transition.
    Source(ModelTransition),
    /// An FR-419 task-derived scheduler transition.
    Task(Box<TaskTransitionTemplate>),
}

impl CertificateTransition {
    /// The source-model operation identity, when this is a source edge.
    pub fn source(&self) -> Option<&ModelTransition> {
        match self {
            Self::Source(transition) => Some(transition),
            Self::Task(_) => None,
        }
    }
}

/// A path of certificate edges to a divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivergenceWitness {
    /// The node the path starts at.
    pub start: usize,
    /// The edges of the path, by index.
    pub path: Vec<usize>,
    /// Where it ends.
    pub target: DivergentTarget,
}

/// A divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivergentTarget {
    /// Under a dense source, a quiescent node, from which time diverges by
    /// the idle tail.
    Quiescent(usize),
    /// Under a tick source, a terminal node, whose stutter step counts as
    /// a tick (FR-231 "Digital time").
    Terminal(usize),
    /// A cycle of edges from and back to the path's last node, each
    /// repetition of which lets time advance.
    Cycle(Vec<usize>),
}

/// FR-245's budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateCheckLimits {
    /// The most certificate edges a check reads.
    pub max_certificate_edges: u64,
    /// The most certificate bytes a check reads.
    pub max_certificate_bytes: u64,
    /// The most states of the claim automaton the checker rebuilds itself
    /// (ADR-018 LA-5): reaching it stops the check as EN-6's own
    /// `max_automaton_states` stops the search (SR-1647001 FND-002).
    pub max_automaton_states: u64,
}

impl Default for CertificateCheckLimits {
    fn default() -> Self {
        Self {
            max_certificate_edges: 1 << 24,
            max_certificate_bytes: 1 << 30,
            max_automaton_states: crate::model_check::limits::DEFAULT_MAX_AUTOMATON_STATES,
        }
    }
}

/// What the check needs beside the certificate: the subject, re-admitted
/// by the checker itself, the model whose timed reading the proof is over,
/// and the item.
pub struct ZoneCertificateRequest<'a> {
    /// FR-120's request of the subject.
    pub simulation: ModelSimulationRequest<'a>,
    /// The alias of the model whose time source and timed model the
    /// subject is read under (FR-230).
    pub model: &'a str,
    /// The bound on a state's live clocks the search ran under (QSpec
    /// FR-415, `model_check.max_clocks`).
    pub max_clocks: std::num::NonZeroU64,
    /// The item.
    pub item: CertifiedItem<'a>,
}

/// The item a zone certificate proves, as the checker recompiles it.
#[derive(Clone, Copy)]
pub enum CertifiedItem<'a> {
    /// An authored clause, its atoms read through `letters`.
    Clause {
        /// The checked clause.
        clause: &'a qsl_semantics::check::CheckedTemporalClause,
        /// The seam to the one clause evaluator for its `holds` atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's derived deadlock-freedom item, read over time (FR-232).
    DeadlockFreedom,
    /// FR-232's derived time-lock-freedom item.
    TimeLockFreedom,
    /// FR-249's checked schedulability claim over a task automaton. The
    /// occurrence binds duplicate uses of the same checked claim to the
    /// exact authored claim node named by the obligation.
    TaskSchedulability {
        /// The checked claim, whose task set and scheduler policy the
        /// reader recompiles from the admitted package.
        claim: &'a CheckedScheduleClaim,
        /// The claim node occurrence that binds the certificate identity.
        occurrence: &'a OccurrenceKey,
    },
    /// The safety half of a timed refinement (QSpec FR-430), whose concrete
    /// subject is `simulation` and whose concrete model is `model`.
    Refinement {
        /// The checked refinement.
        refinement: &'a CheckedRefinement,
        /// The package holding the abstract model.
        abstract_package: &'a CheckedPackage,
        /// The abstract model's initial states.
        abstract_initial: &'a [DocumentRef],
    },
}

/// The item an obligation names, as the checker compares it with the
/// recompiled item (FR-245): an authored clause by its `claim` occurrence,
/// or a derived item by its kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateItem<'o> {
    /// An authored clause.
    Clause(&'o OccurrenceKey),
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// An authored schedulability claim, by its checked-package occurrence.
    TaskSchedulability(&'o OccurrenceKey),
    /// A refinement's record, against the abstract initial states.
    Refinement {
        /// The refinement's requirement occurrence (FR-144).
        occurrence: &'o OccurrenceKey,
        /// The abstract initial states.
        abstract_initial: &'o BTreeSet<DocumentRef>,
    },
    /// Any other item, which no zone certificate proves.
    Other,
}

/// An item's identity, as FR-245's identity refusal names it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemIdentity {
    /// An authored clause, by its `claim` occurrence.
    Clause(OccurrenceKey),
    /// A clause its package does not list, which has no occurrence.
    UnlistedClause,
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// An authored schedulability claim, by its claim-node occurrence.
    TaskSchedulability(OccurrenceKey),
    /// A refinement's record, against the abstract initial states.
    Refinement {
        /// The refinement's requirement occurrence (FR-144).
        occurrence: OccurrenceKey,
        /// The abstract initial states.
        abstract_initial: BTreeSet<DocumentRef>,
    },
    /// Any other item.
    Other,
}

impl From<CertificateItem<'_>> for ItemIdentity {
    fn from(item: CertificateItem<'_>) -> Self {
        match item {
            CertificateItem::Clause(occurrence) => Self::Clause(occurrence.clone()),
            CertificateItem::DeadlockFreedom => Self::DeadlockFreedom,
            CertificateItem::TimeLockFreedom => Self::TimeLockFreedom,
            CertificateItem::TaskSchedulability(occurrence) => {
                Self::TaskSchedulability(occurrence.clone())
            }
            CertificateItem::Refinement {
                occurrence,
                abstract_initial,
            } => Self::Refinement {
                occurrence: occurrence.clone(),
                abstract_initial: abstract_initial.clone(),
            },
            CertificateItem::Other => Self::Other,
        }
    }
}

/// What [`check_zone_certificate`] returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateVerdict {
    /// Every check passed: the item settles `Proved{ZoneCertified,
    /// Certified}` (FR-235).
    Accepted,
    /// A rule failed: the item settles `inconclusive`,
    /// `CertificateRejected`.
    Rejected(CertificateRejection),
    /// A budget or a cancellation stopped the check (V-7).
    Stopped(IncompleteCause),
}

/// An invariant of the checker's own that a check broke.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckerFault {
    /// A zone operation met a zone of the wrong shape.
    ZoneShape,
    /// The fairness kernels refused the graph the checker built.
    Graph,
    /// A node read as reached has no model state.
    UnreachedNode,
    /// A valid edge has no successor it covers.
    EdgeWithoutSuccessor,
    /// A polled cancellation has no cause.
    CancellationWithoutCause,
    /// The claim automaton broke an invariant of its own.
    Automaton,
    /// `check_initial` or `check_step` returned an internal fault.
    RefinementStep,
}

/// Why [`check_zone_certificate`] checked nothing: the request, not the
/// certificate, is at fault. A refusal is never an acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateRefusal<O> {
    /// The certificate proves another item than `item`, or than the
    /// recompiled item: `stale_dependency`/`content-mismatch`, naming both.
    ObligationMismatch {
        /// The certificate's obligation.
        certificate: Box<O>,
        /// The item's.
        item: Box<O>,
    },
    /// The item the certificate names is not the recompiled item:
    /// `stale_dependency`/`content-mismatch`, naming both identities.
    IdentityMismatch {
        /// The identity the certificate's obligation names.
        certificate: Box<ItemIdentity>,
        /// The recompiled item's.
        recompiled: Box<ItemIdentity>,
    },
    /// The refinement part names another refinement than the
    /// recompiled one (FR-245 "Refinement form"):
    /// `stale_dependency`/`content-mismatch`, naming both.
    RefinementMismatch {
        /// The part's.
        certificate: Box<RefinementIdentity>,
        /// The recompiled.
        recompiled: Box<RefinementIdentity>,
    },
    /// A package of the refinement form does not emit.
    Package(Box<EmitRefusal>),
    /// FR-106's admission of the subject failed.
    Admission(Box<AdmissionFailure>),
    /// The model's timed reading does not build (FR-231).
    TimedSubject(Box<TimedSubjectRefusal>),
    /// The item is not one this checker reads yet (see the module).
    UnsupportedItem,
    /// The clause's instances are not finite.
    Unbounded,
    /// A reading the check needs has no value: a guard, invariant, urgency,
    /// expansion or atom.
    Undecided,
    /// The checker broke an invariant of its own.
    Fault(CheckerFault),
}

impl<O> ZoneCertificateRefusal<O> {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ObligationMismatch { .. }
            | Self::IdentityMismatch { .. }
            | Self::RefinementMismatch { .. } => Code::StaleDependency,
            Self::Package(refusal) => refusal.code(),
            Self::UnsupportedItem => Code::UnsupportedConstruct,
            Self::Fault(_) => Code::RuntimeInvariant,
            Self::Admission(_) | Self::TimedSubject(_) | Self::Unbounded | Self::Undecided => {
                Code::InvalidRuntimeInput
            }
        }
    }
}

/// How a check ends early.
enum Halt<O> {
    Verdict(ZoneCertificateVerdict),
    Refusal(ZoneCertificateRefusal<O>),
}

impl<O> From<ZoneError> for Halt<O> {
    fn from(_: ZoneError) -> Self {
        fault(CheckerFault::ZoneShape)
    }
}

type Step<T, O> = Result<T, Halt<O>>;

/// The verdict of a check whose rebuilt claim automaton reached
/// `max_automaton_states`, `limit`.
fn automaton_stop(limit: u64) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Stopped(IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxAutomatonStates,
        value: limit,
    })
}

fn fault<O>(fault: CheckerFault) -> Halt<O> {
    Halt::Refusal(ZoneCertificateRefusal::Fault(fault))
}

fn reject<T, O>(rule: CertificateRule, at: CertificateLocus) -> Step<T, O> {
    Err(Halt::Verdict(ZoneCertificateVerdict::Rejected(
        CertificateRejection { rule, at },
    )))
}

fn malformed<T, O>(member: CertificateMember) -> Step<T, O> {
    reject(
        CertificateRule::Malformed,
        CertificateLocus::CertificateMember(member),
    )
}

fn at_node(node: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: u64::try_from(node).unwrap_or(u64::MAX),
    }
}

fn undecided<T, O, E>(_: E) -> Step<T, O> {
    Err(Halt::Refusal(ZoneCertificateRefusal::Undecided))
}

fn malformed_verdict(member: CertificateMember) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Rejected(CertificateRejection {
        rule: CertificateRule::Malformed,
        at: CertificateLocus::CertificateMember(member),
    })
}

fn at_edge(edge: usize) -> CertificateLocus {
    CertificateLocus::ZoneEdge {
        edge: u64::try_from(edge).unwrap_or(u64::MAX),
    }
}

/// Why a refinement's subjects do not open, as the check's refusal.
fn subject_refusal<O>(refusal: SubjectRefusal) -> ZoneCertificateRefusal<O> {
    match refusal {
        SubjectRefusal::Admission(failure) => ZoneCertificateRefusal::Admission(failure),
        SubjectRefusal::NoUniverse => ZoneCertificateRefusal::Unbounded,
        SubjectRefusal::NoModels
        | SubjectRefusal::ProtocolSubject
        | SubjectRefusal::ProtocolAbstractSide => ZoneCertificateRefusal::UnsupportedItem,
    }
}

/// A refinement form's check (FR-245 "Refinement form"): `check_step` in
/// place of the claim automaton, over the positions the part lists.
struct Refined<'c, 'a> {
    refinement: &'c CheckedRefinement,
    abstract_system: &'c ModelSystem<'a>,
    part: &'c RefinementPart,
}

/// FR-245: check `certificate` for `request`'s item, whose obligation is
/// `item` and names the item `item_of` reads from it, under `limits`,
/// polling `cancel` throughout.
///
/// # Errors
///
/// [`ZoneCertificateRefusal`] when the certificate names another item than
/// `item` or the recompiled one, the subject does not admit, its timed
/// reading does not build, the item is not supported, or a reading has no
/// value.
pub fn check_zone_certificate<O: Clone + Eq>(
    request: ZoneCertificateRequest<'_>,
    item: &O,
    item_of: impl Fn(&O) -> CertificateItem<'_>,
    certificate: &ZoneCertificate<O>,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<O>> {
    let mismatch = || ZoneCertificateRefusal::ObligationMismatch {
        certificate: Box::new(certificate.obligation.clone()),
        item: Box::new(item.clone()),
    };
    if certificate.obligation != *item {
        return Err(mismatch());
    }
    // Task-product evidence is admitted only with the independently supplied
    // checked schedulability claim; ordinary model-subject items cannot use
    // the optional member to change what is being proved.
    let task_item = matches!(request.item, CertifiedItem::TaskSchedulability { .. });
    if task_item != certificate.task_product.is_some() {
        return Ok(malformed_verdict(CertificateMember::TaskProduct));
    }
    // The recompiled item's identity (FR-245): a clause's `claim`
    // occurrence in the package, or the derived item's kind.
    let named = ItemIdentity::from(item_of(&certificate.obligation));
    let recompiled = match request.item {
        CertifiedItem::Clause { clause, .. } => request
            .simulation
            .package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause() == clause)
            .map_or(ItemIdentity::UnlistedClause, |node| {
                ItemIdentity::Clause(node_occurrence(node))
            }),
        CertifiedItem::DeadlockFreedom => ItemIdentity::DeadlockFreedom,
        CertifiedItem::TimeLockFreedom => ItemIdentity::TimeLockFreedom,
        CertifiedItem::TaskSchedulability { claim, occurrence } => {
            let admitted_claim = request
                .simulation
                .package
                .graph()
                .task_sets()
                .claims()
                .iter()
                .any(|candidate| {
                    candidate == claim && candidate.occurrence().as_ref() == Some(occurrence)
                });
            if admitted_claim {
                ItemIdentity::TaskSchedulability(occurrence.clone())
            } else {
                ItemIdentity::Other
            }
        }
        CertifiedItem::Refinement {
            refinement,
            abstract_initial,
            ..
        } => refinement
            .requirement_occurrence()
            .map_or(ItemIdentity::Other, |occurrence| ItemIdentity::Refinement {
                occurrence: occurrence.clone(),
                abstract_initial: abstract_initial.iter().cloned().collect(),
            }),
    };
    if named != recompiled || recompiled == ItemIdentity::Other {
        return Err(ZoneCertificateRefusal::IdentityMismatch {
            certificate: Box::new(named),
            recompiled: Box::new(recompiled),
        });
    }
    let mut admitted_task_system = None;
    let mut admitted_fixed_task = None;
    if task_item {
        let Some(task_product) = certificate.task_product.as_ref() else {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        };
        if !task_product.has_valid_shape() {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        }
        let CertifiedItem::TaskSchedulability { claim, .. } = request.item else {
            return Err(ZoneCertificateRefusal::UnsupportedItem);
        };
        let Some(task_set) = request
            .simulation
            .package
            .graph()
            .task_sets()
            .task_set_of(claim)
        else {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        };
        let expected = match task_product.time_progress.method {
            TaskTimeProgressMethod::ClosedNoReleaseProductV1 => no_release_queue_bounds(task_set),
            TaskTimeProgressMethod::ClosedFiniteProductV1 => {
                let ScheduleClaim::Schedulable(policy) = &claim.claim else {
                    return Ok(malformed_verdict(CertificateMember::TaskProduct));
                };
                fixed_work_queue_bounds(task_set, *policy)
            }
        };
        let Some(expected) = expected else {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        };
        if task_product.queue_bounds != expected {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        }
        let task_releases = request
            .simulation
            .package
            .graph()
            .task_sets()
            .releases()
            .iter()
            .filter(|release| release.task_set() == claim.task_set)
            .cloned()
            .collect::<Vec<_>>();
        let task_system = ModelSystem::new(request.simulation.clone())
            .map_err(|failure| ZoneCertificateRefusal::Admission(Box::new(failure)))?;
        let no_release =
            task_product.time_progress.method == TaskTimeProgressMethod::ClosedNoReleaseProductV1;
        if !task_product_controls_match(
            certificate,
            &expected,
            task_set,
            &task_releases,
            &task_system,
            no_release,
        ) {
            return Ok(malformed_verdict(CertificateMember::TaskProduct));
        }
        if task_product.time_progress.method == TaskTimeProgressMethod::ClosedFiniteProductV1 {
            let Some((ceilings, diagonals)) = fixed_work_clock_metadata(
                &expected,
                &certificate.scale,
                &task_product.clock_ceilings,
                &task_product.diagonal_constraints,
            ) else {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            };
            if task_product.clock_ceilings != ceilings
                || task_product.diagonal_constraints != diagonals
            {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            }
            let Some(subtractions) =
                fixed_work_subtractions(certificate, &expected, &certificate.scale)
            else {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            };
            if task_product.subtractions != subtractions {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            }
            if !fixed_work_source_release_edges_match(certificate, &task_releases, &task_system)
                || !fixed_work_task_control_edges_match(certificate, &expected, task_set)
            {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            }
            let ScheduleClaim::Schedulable(policy) = &claim.claim else {
                return Ok(malformed_verdict(CertificateMember::TaskProduct));
            };
            admitted_fixed_task = Some((expected, task_set.clone(), task_releases, *policy));
        }
        admitted_task_system = Some(task_system);
    }
    let no_release_task = task_item;
    // FR-245 "Refinement form": both packages recompiled, the part's
    // identity compared before any system is opened.
    let checked_task_releases = if no_release_task {
        let CertifiedItem::TaskSchedulability { claim, .. } = request.item else {
            return Err(ZoneCertificateRefusal::UnsupportedItem);
        };
        request
            .simulation
            .package
            .graph()
            .task_sets()
            .releases()
            .iter()
            .filter(|release| release.task_set() == claim.task_set)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let opened;
    let system;
    let (concrete, refined) = match request.item {
        CertifiedItem::Refinement {
            refinement,
            abstract_package,
            abstract_initial,
        } => {
            let Some(part) = &certificate.refinement else {
                return Ok(malformed_verdict(CertificateMember::Refinement));
            };
            // The refinement node is the recompiled record's occurrence's,
            // which the identity check above found.
            let ItemIdentity::Refinement { occurrence, .. } = &recompiled else {
                return Err(ZoneCertificateRefusal::IdentityMismatch {
                    certificate: Box::new(named),
                    recompiled: Box::new(recompiled),
                });
            };
            let emitted = |package: &CheckedPackage| {
                qsl_package::emit_checked(package)
                    .map(|emission| emission.package().clone())
                    .map_err(|refusal| ZoneCertificateRefusal::Package(Box::new(refusal)))
            };
            let recompiled = RefinementIdentity {
                refinement_node: occurrence.node().clone(),
                abstract_package: emitted(abstract_package)?,
            };
            let listed = RefinementIdentity {
                refinement_node: part.refinement_node.clone(),
                abstract_package: emitted(request.simulation.package)?,
            };
            if listed != recompiled {
                return Err(ZoneCertificateRefusal::RefinementMismatch {
                    certificate: Box::new(listed),
                    recompiled: Box::new(recompiled),
                });
            }
            opened = open_systems(
                refinement,
                &request.simulation,
                abstract_package,
                abstract_initial.to_vec(),
            )
            .map_err(subject_refusal)?;
            (
                &opened.concrete,
                Some(Refined {
                    refinement,
                    abstract_system: &opened.abstract_system,
                    part,
                }),
            )
        }
        _ => {
            system = match admitted_task_system.take() {
                Some(system) => system,
                None => ModelSystem::new(request.simulation)
                    .map_err(|failure| ZoneCertificateRefusal::Admission(Box::new(failure)))?,
            };
            (&system, None)
        }
    };
    let timed = TimedSystem::with_max_clocks(concrete, request.model, request.max_clocks)
        .map_err(|refusal| ZoneCertificateRefusal::TimedSubject(Box::new(refusal)))?;
    if let Some((rows, task_set, releases, policy)) = admitted_fixed_task.as_ref() {
        match fixed_product_closure(
            &timed,
            concrete,
            certificate,
            rows,
            task_set,
            releases,
            *policy,
            cancel,
        ) {
            Ok(()) => {}
            Err(FixedProductClosureFailure::Unsupported) => {
                return Err(ZoneCertificateRefusal::UnsupportedItem);
            }
            Err(FixedProductClosureFailure::Rejected(rejection)) => {
                return Ok(ZoneCertificateVerdict::Rejected(rejection));
            }
            Err(FixedProductClosureFailure::Stopped(cause)) => {
                return Ok(ZoneCertificateVerdict::Stopped(cause));
            }
        }
        let Some(layout) = ReaderTaskClockLayout::new(rows, timed.places().len() + 1) else {
            return Err(ZoneCertificateRefusal::UnsupportedItem);
        };
        if fixed_product_idle_witness(
            &timed,
            concrete,
            certificate,
            task_set,
            rows,
            &layout,
            *policy,
        )
        .is_some()
        {
            return Ok(ZoneCertificateVerdict::Accepted);
        }
        return Err(ZoneCertificateRefusal::UnsupportedItem);
    }
    let task_release_operations = if no_release_task {
        checked_task_releases
            .iter()
            .filter_map(|release| {
                concrete
                    .operation_text(release.operation().declaring(), release.operation().name())
                    .map(str::to_owned)
            })
            .collect()
    } else {
        BTreeSet::new()
    };
    let mut live = None;
    let (claim, window) = match request.item {
        CertifiedItem::Clause { clause, letters } => {
            if !matches!(
                clause.profile(),
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed
            ) {
                return Err(ZoneCertificateRefusal::UnsupportedItem);
            }
            let instances = concrete
                .instances(&clause.over().1)
                .map_err(|_| ZoneCertificateRefusal::Unbounded)?;
            let atom = qsl_eval::automaton::invariant_atom(clause);
            // A TT-2 window nesting no interval operator under another has
            // FR-242's horizon automaton; any other clause is read on the
            // liveness product.
            let flat = if atom.is_none()
                && matches!(
                    clause.timed_form(),
                    Some(TimedPropertyForm::BoundedWindow { .. })
                ) {
                match WindowClaim::new(
                    clause.formula(),
                    instances.len(),
                    timed.tick_period().cloned(),
                    limits.max_automaton_states,
                ) {
                    Ok(claim) => Some(claim),
                    Err(WindowRefusal::NestedWindow) => None,
                    Err(_) => return Err(ZoneCertificateRefusal::UnsupportedItem),
                }
            } else {
                None
            };
            let window = if atom.is_some() {
                None
            } else if let Some(claim) = flat {
                let pieces = match timed.tick_period() {
                    None => claim.pieces(timed.places().len()),
                    Some(_) => vec![(Vec::new(), Rational::from_integer(Integer::zero()))],
                };
                Some(Window {
                    claim: RefCell::new(claim),
                    pieces,
                })
            } else {
                // FR-240: any other clause, on the liveness product.
                let mut automaton = match position_automaton(clause, limits.max_automaton_states) {
                    Ok(automaton) => automaton,
                    Err(AutomatonError::StateLimit { limit }) => return Ok(automaton_stop(limit)),
                    Err(
                        AutomatonError::ProfileNotTranslated { .. }
                        | AutomatonError::RepeatedPunctualInterval
                        | AutomatonError::ClockLimit { .. },
                    ) => return Err(ZoneCertificateRefusal::UnsupportedItem),
                    Err(_) => return Err(ZoneCertificateRefusal::Fault(CheckerFault::Automaton)),
                };
                let start = match automaton.initial() {
                    Ok(start) => start,
                    Err(AutomatonError::StateLimit { limit }) => return Ok(automaton_stop(limit)),
                    Err(_) => return Err(ZoneCertificateRefusal::Fault(CheckerFault::Automaton)),
                };
                let mut operations = Vec::new();
                for constraint in clause.fairness() {
                    let operation = constraint.operation();
                    let text = concrete
                        .operation_text(operation.declaring_type(), operation.name())
                        .ok_or(ZoneCertificateRefusal::Undecided)?;
                    operations.push(text.to_owned());
                }
                live = Some(product::LiveClaim::new(
                    automaton,
                    start,
                    letters,
                    instances.clone(),
                    clause.capture_site(),
                    clause.fairness().to_vec(),
                    operations,
                ));
                None
            };
            (
                Some(Claim {
                    atom,
                    letters,
                    instances,
                }),
                window,
            )
        }
        CertifiedItem::DeadlockFreedom
        | CertifiedItem::TimeLockFreedom
        | CertifiedItem::TaskSchedulability { .. }
        | CertifiedItem::Refinement { .. } => (None, None),
    };
    let time_lock = matches!(request.item, CertifiedItem::TimeLockFreedom);
    let count = certificate.nodes.len();
    let mut checker = Checker {
        timed: &timed,
        claim,
        window,
        live,
        time_lock,
        no_release_task,
        task_release_operations,
        refined,
        certificate,
        cancel,
        cover: certificate.lu_bounds.clone(),
        leaving: vec![Vec::new(); count],
        states: vec![None; count],
        successors: (0..count).map(|_| None).collect(),
        covers: vec![None; certificate.edges.len()],
        bound: BTreeMap::new(),
        binds: BTreeMap::new(),
        known: Vec::new(),
        refinement_failures: BTreeMap::new(),
        read: Cell::new(0),
        max_edges: limits.max_certificate_edges,
        max_automaton_states: limits.max_automaton_states,
    };
    match checker.verify(limits) {
        Ok(()) => Ok(ZoneCertificateVerdict::Accepted),
        Err(Halt::Verdict(verdict)) => Ok(verdict),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

/// A clause: its TT-1 atom, or `None` for a TT-2 clause its automaton
/// reads; each atom read through `letters` for each instance.
struct Claim<'a> {
    atom: Option<FormulaNodeId>,
    letters: &'a dyn ModelLetters,
    instances: Vec<ModelInstance>,
}

/// A TT-2 clause's automaton, qsl-eval's own (FR-242), and the pieces of
/// the horizon axis its positions are read on.
struct Window {
    claim: RefCell<WindowClaim>,
    pieces: Vec<(Vec<PlaceConstraint>, Rational)>,
}

/// One recomputed symbolic successor of a node.
struct Successor {
    /// The step; `None` for an idle-tail stutter position.
    step: Option<GuardedStep>,
    part: Option<usize>,
    /// The horizon piece a dense TT-2 position lies in.
    horizon: Option<Vec<PlaceConstraint>>,
    /// Whether the successor frees the horizon clock: past the point where
    /// the claim is decided.
    frees: bool,
    state_key: qsl_eval::simulation::StateKey,
    /// The checker's automaton state.
    q: usize,
    zone: Zone,
    /// A liveness product's silent move.
    silent: Option<SilentMove>,
    /// A liveness product successor's instance and entry.
    mark: Option<ProductMark>,
    /// A liveness product successor's cell, over every clock of a node.
    cell: Option<Zone>,
    /// A liveness product position's claim-automaton transition.
    read: Option<ClockedEdge>,
}

fn equivalent_successor(left: &Successor, right: &Successor) -> bool {
    left.step.as_ref().map(GuardedStep::transition)
        == right.step.as_ref().map(GuardedStep::transition)
        && left.part == right.part
        && left.horizon == right.horizon
        && left.state_key == right.state_key
        && left.q == right.q
        && left.zone == right.zone
        && left.silent == right.silent
        && left.mark == right.mark
        && left.cell == right.cell
        && left.read == right.read
}

struct Checker<'c, 'a, O> {
    timed: &'c TimedSystem<'c, 'a>,
    claim: Option<Claim<'c>>,
    window: Option<Window>,
    /// A clause decided on FR-240's liveness product.
    live: Option<product::LiveClaim<'c>>,
    /// Whether the item is time-lock freedom, with its divergence part.
    time_lock: bool,
    /// The task certificate is the no-release source-closure special case.
    no_release_task: bool,
    /// Source operation identities that would activate a checked release.
    task_release_operations: BTreeSet<String>,
    /// A refinement form's check.
    refined: Option<Refined<'c, 'a>>,
    certificate: &'c ZoneCertificate<O>,
    cancel: &'c Cancel,
    /// The bounds coverage is read under: the certificate's, or for the
    /// deadlock-freedom item `L = U = max(L, U)` per clock, the aM
    /// bisimulation, since the aLU simulation does not preserve deadlocks.
    cover: LuBounds,
    /// The edges leaving each node, by index, in edge order.
    leaving: Vec<Vec<usize>>,
    /// The model state of each node an edge reaches from an initial node.
    states: Vec<Option<ModelSimState>>,
    /// Each reached node's recomputed successors.
    successors: Vec<Option<Vec<Successor>>>,
    /// For each valid edge, the successor of its source it covers.
    covers: Vec<Option<usize>>,
    /// The checker's automaton state each certificate automaton state
    /// names, and back: a certificate numbers states in its own order.
    bound: BTreeMap<usize, usize>,
    binds: BTreeMap<usize, usize>,
    /// A refinement form's position at each index of `positions`, once a
    /// passing initial state or step yields it.
    known: Vec<Option<RefinementPosition>>,
    /// A refinement form's first rejection out of each node, by node,
    /// reported in node order before the node's own checks.
    refinement_failures: BTreeMap<usize, CertificateRejection>,
    /// The certificate edges the check has read, against
    /// `max_certificate_edges`: each listed edge once, and each edge of a
    /// divergence cycle again at each round of its repetition check.
    read: Cell<u64>,
    max_edges: u64,
    max_automaton_states: u64,
}

impl<O> Checker<'_, '_, O> {
    // Not named `check`: CK-1's checked-input gate reads any `.check(..)`
    // call in a stage crate as the S3 stage constructor (TC-745).
    fn verify(&mut self, limits: CertificateCheckLimits) -> Step<(), O> {
        self.poll()?;
        self.budgets(limits)?;
        self.members()?;
        if self.claim.is_none() && self.refined.is_none() {
            // FR-232's deadlocks are not preserved by the aLU simulation:
            // coverage is read under the aM bisimulation.
            for (lower, upper) in self.cover.lower.iter_mut().zip(self.cover.upper.iter_mut()) {
                let most = lower.clone().max(upper.clone());
                lower.clone_from(&most);
                *upper = most;
            }
        }
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            self.leaving[edge.from].push(index);
        }
        self.initial()?;
        self.reach()?;
        let divergent = self.divergent()?;
        for node in 0..self.certificate.nodes.len() {
            self.poll()?;
            if let Some(rejection) = self.refinement_failures.remove(&node) {
                return Err(Halt::Verdict(ZoneCertificateVerdict::Rejected(rejection)));
            }
            if self.states[node].is_some() {
                self.node(node, &divergent)?;
            }
        }
        if self.live.is_some() {
            self.components()?;
        }
        if self.time_lock {
            self.divergence_part()?;
        }
        self.admitted()
    }

    /// Charge `count` edge reads against `max_certificate_edges`.
    fn charge(&self, count: u64) -> Step<(), O> {
        self.poll()?;
        let read = self.read.get().saturating_add(count);
        if read > self.max_edges {
            return Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::CertificateMaxEdges,
                    value: self.max_edges,
                },
            )));
        }
        self.read.set(read);
        Ok(())
    }

    fn poll(&self) -> Step<(), O> {
        if self.cancel.poll() {
            let Some(source) = self.cancel.cause() else {
                return Err(fault(CheckerFault::CancellationWithoutCause));
            };
            return Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
        Ok(())
    }

    /// `max_certificate_edges` and `max_certificate_bytes`, before any
    /// recomputation.
    fn budgets(&self, limits: CertificateCheckLimits) -> Step<(), O> {
        let stop = |limit, value| {
            Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )))
        };
        let edges = u64::try_from(self.certificate.edges.len()).unwrap_or(u64::MAX);
        self.read.set(edges);
        if edges > limits.max_certificate_edges {
            return stop(
                ModelCheckLimit::CertificateMaxEdges,
                limits.max_certificate_edges,
            );
        }
        if certificate_bytes(self.certificate) > limits.max_certificate_bytes {
            return stop(
                ModelCheckLimit::CertificateMaxBytes,
                limits.max_certificate_bytes,
            );
        }
        Ok(())
    }

    /// The zone dimension: the reference clock, the clock places and, for
    /// a TT-2 claim, the horizon clock.
    fn dim(&self) -> usize {
        let z = self.live.is_some() && self.timed.tick_period().is_none();
        self.timed.places().len()
            + 1
            + usize::from(self.window.is_some())
            + self.automaton_clocks()
            + usize::from(z)
    }

    /// The horizon clock's index in a zone.
    fn horizon_clock(&self) -> usize {
        self.timed.places().len() + 1
    }

    /// The `Malformed` members, in member order.
    fn members(&self) -> Step<(), O> {
        let certificate = self.certificate;
        if self.no_release_task
            && certificate
                .edges
                .iter()
                .any(|edge| matches!(edge.transition, Some(CertificateTransition::Task(_))))
        {
            return malformed(CertificateMember::TaskProduct);
        }
        if certificate.scale <= Integer::zero() {
            return malformed(CertificateMember::Scale);
        }
        let constants = self
            .window
            .as_ref()
            .map(|window| window.claim.borrow().constants().to_vec());
        let Some((mut lower, mut upper)) =
            computed_bounds(self.timed, constants.as_deref(), &certificate.scale)
        else {
            return malformed(CertificateMember::Scale);
        };
        if self.live.is_some() {
            // The liveness product's cells compare clocks with guard
            // constants from both sides, which the aLU simulation keeps
            // only under `L = U = max(L, U)`; `z` is compared with 1.
            for (low, high) in lower.iter_mut().zip(upper.iter_mut()) {
                let most = low.clone().max(high.clone());
                low.clone_from(&most);
                *high = most;
            }
            // Each claim-automaton clock is compared with its interval's
            // ends, which are exact at the scale (FR-242).
            let (constants, every) = self.live.as_ref().map_or_else(Default::default, |live| {
                let automaton = live.automaton.borrow();
                (automaton.constants(), automaton.all_constants())
            });
            for value in &every {
                if self.scaled(value).is_err() {
                    return malformed(CertificateMember::Scale);
                }
            }
            for constant in &constants {
                let Ok(bound) = self.scaled(constant) else {
                    return malformed(CertificateMember::Scale);
                };
                lower.push(Some(bound.clone()));
                upper.push(Some(bound));
            }
            if self.timed.tick_period().is_none() {
                lower.push(Some(certificate.scale.clone()));
                upper.push(Some(certificate.scale.clone()));
            }
        }
        let at_least = |listed: &[Option<Integer>], computed: &[Option<Integer>]| {
            listed.len() == computed.len()
                && listed
                    .iter()
                    .zip(computed)
                    .all(|(listed, computed)| match (listed, computed) {
                        (_, None) => true,
                        (None, Some(_)) => false,
                        (Some(listed), Some(computed)) => listed >= computed,
                    })
        };
        if !at_least(&certificate.lu_bounds.lower, &lower)
            || !at_least(&certificate.lu_bounds.upper, &upper)
        {
            return malformed(CertificateMember::Bounds);
        }
        // A refinement form's part: present exactly for a refinement item,
        // with its positions and one `taken` per edge.
        let positions = match (&self.refined, &certificate.refinement) {
            (Some(refined), _) => {
                if refined.part.positions.is_empty() {
                    return malformed(CertificateMember::Positions);
                }
                if refined.part.taken.len() != certificate.edges.len() {
                    return malformed(CertificateMember::Edges);
                }
                Some(refined.part.positions.len())
            }
            (None, Some(_)) => return malformed(CertificateMember::Refinement),
            (None, None) => None,
        };
        let nodes = certificate.nodes.len();
        let windowed = self.window.is_some();
        let live = self.live.as_ref();
        let node_ok = |node: &CertNode| {
            node.zone.dim() == self.dim()
                && match positions {
                    // Each node's position, or the locked region's index.
                    Some(reserved) => node.automaton_state <= reserved,
                    None => windowed || live.is_some() || node.automaton_state == 0,
                }
                && match (live, node.product) {
                    (Some(live), Some(mark)) => mark.instance < live.instances.len(),
                    (None, None) => true,
                    _ => false,
                }
        };
        // QSpec FR-418: `initials` is nonempty.
        if certificate.initials.is_empty() {
            return malformed(CertificateMember::Initials);
        }
        if !certificate.nodes.iter().all(node_ok)
            || certificate.initials.iter().any(|&node| node >= nodes)
            || certificate.admitted.start >= nodes
        {
            return malformed(CertificateMember::Nodes);
        }
        // The Büchi part for a liveness product, the divergence part for
        // the time-lock-freedom item, each shaped to the nodes. The
        // time-lock item carries no component part, which FR-244's Büchi
        // part text names (spec gap STD-493, comment 8eafddf9, item 1;
        // SR-1647002 FND-003): its exact fixpoint is the stronger check.
        match (&certificate.components, live) {
            (Some(components), Some(_)) if numbering_ok(components, nodes) => {}
            (None, None) => {}
            _ => return malformed(CertificateMember::Components),
        }
        let edges = certificate.edges.len();
        let dense = self.timed.tick_period().is_none();
        let edge_ok = |edge: &CertEdge| {
            let shaped = match (&edge.transition, edge.part, edge.silent) {
                (Some(_), Some(_), None) => true,
                (None, None, None) => windowed || live.is_some(),
                (None, None, Some(_)) => live.is_some(),
                _ => false,
            };
            edge.from < nodes
                && edge.to < nodes
                && shaped
                && (edge.horizon.is_none() || (windowed && dense))
        };
        let target_ok = match &certificate.admitted.target {
            DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => *node < nodes,
            DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
        };
        if !certificate.edges.iter().all(edge_ok)
            || certificate.admitted.path.iter().any(|&edge| edge >= edges)
            || !target_ok
        {
            return malformed(CertificateMember::Edges);
        }
        let witness_ok = |witness: &DivergenceWitness| {
            witness.start < nodes
                && witness.path.iter().all(|&edge| edge < edges)
                && match &witness.target {
                    DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => {
                        *node < nodes
                    }
                    DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
                }
        };
        match (&certificate.divergence, self.time_lock) {
            (Some(witnesses), true)
                if witnesses.len() == nodes && witnesses.iter().all(witness_ok) => {}
            (None, false) => {}
            _ => return malformed(CertificateMember::Divergence),
        }
        Ok(())
    }

    /// The scaled integer of `value`; every constant of the model and the
    /// claim was checked integral under the scale in [`Self::members`].
    fn scaled(&self, value: &Rational) -> Step<Integer, O> {
        match scaled(value, &self.certificate.scale) {
            Some(value) => Ok(value),
            None => malformed(CertificateMember::Scale),
        }
    }

    fn apply_clock_updates(&self, zone: &mut Zone, step: &GuardedStep) -> Step<(), O> {
        let carries: Vec<_> = step
            .carries()
            .iter()
            .map(|(target, source)| (target + 1, source + 1))
            .collect();
        let resets: Vec<_> = step
            .resets()
            .iter()
            .map(|(place, value)| Ok((place + 1, self.scaled(value)?)))
            .collect::<Step<_, O>>()?;
        zone.assign_clocks(&carries, &resets)?;
        Ok(())
    }

    fn preimage_clock_updates(&self, zone: &Zone, step: &GuardedStep) -> Step<Zone, O> {
        let carries: Vec<_> = step
            .carries()
            .iter()
            .map(|(target, source)| (target + 1, source + 1))
            .collect();
        let resets: Vec<_> = step
            .resets()
            .iter()
            .map(|(place, value)| Ok((place + 1, self.scaled(value)?)))
            .collect::<Step<_, O>>()?;
        Ok(zone.preimage_clocks(&carries, &resets)?)
    }

    fn constrain(&self, zone: &mut Zone, constraints: &[PlaceConstraint]) -> Step<(), O> {
        for constraint in constraints {
            let clock = constraint.place() + 1;
            let bound = self.scaled(constraint.bound())?;
            let below = bound.neg();
            match constraint.op() {
                ClockOp::Less => zone.constrain(clock, 0, &bound, true)?,
                ClockOp::LessOrEqual => zone.constrain(clock, 0, &bound, false)?,
                ClockOp::Equal => {
                    zone.constrain(clock, 0, &bound, false)?;
                    zone.constrain(0, clock, &below, false)?;
                }
                ClockOp::GreaterOrEqual => zone.constrain(0, clock, &below, false)?,
                ClockOp::Greater => zone.constrain(0, clock, &below, true)?,
            }
        }
        Ok(())
    }

    fn invariants(&self, state: &ModelSimState) -> Step<Vec<PlaceConstraint>, O> {
        self.timed.invariants(state).or_else(undecided)
    }

    /// Whether time may elapse at `state`.
    fn elapses(&self, state: &ModelSimState) -> Step<bool, O> {
        Ok(
            self.timed.tick_period().is_none()
                && !self.timed.is_urgent(state).or_else(undecided)?,
        )
    }

    /// `zone` within `state`'s invariants, time let elapse unless it is
    /// urgent, and within them again.
    fn settle(&self, state: &ModelSimState, zone: &mut Zone) -> Step<(), O> {
        let invariants = self.invariants(state)?;
        self.constrain(zone, &invariants)?;
        if !zone.is_empty() && self.elapses(state)? {
            zone.up();
            self.constrain(zone, &invariants)?;
        }
        Ok(())
    }

    fn state_key(&self, state: &ModelSimState) -> Step<qsl_eval::simulation::StateKey, O> {
        let source = self
            .timed
            .system()
            .state_key(state)
            .map(|(_, state_key)| state_key)
            .or_else(undecided)?;
        if self.no_release_task {
            TaskProductStateKey::new(source, TaskProductStateKey::empty_scheduler_control())
                .into_state_key()
                .or_else(undecided)
        } else {
            Ok(source)
        }
    }

    /// The automaton state a position at `state` reads: position 0 when
    /// `from` is `None`.
    fn read(
        &self,
        from: Option<usize>,
        state: &ModelSimState,
        stamp: Stamp<'_>,
        stutter: bool,
    ) -> Step<usize, O> {
        let (Some(window), Some(claim)) = (&self.window, &self.claim) else {
            return Ok(0);
        };
        let limits = self.timed.system().meter_limits();
        let mut atoms = |instance: usize, atom| {
            let instance = claim
                .instances
                .get(instance)
                .ok_or_else(|| "window-instance".to_owned())?;
            claim
                .letters
                .atom(atom, instance, state, &mut Meter::new(limits))
                .map_err(|stop| format!("{stop:?}"))
        };
        let mut automaton = window.claim.borrow_mut();
        let read = match from {
            None => automaton.initial(&mut atoms),
            Some(q) => automaton.step(q, stamp, stutter, &mut atoms),
        };
        match read {
            Read::State(q) => Ok(q),
            Read::Atom(_) => Err(Halt::Refusal(ZoneCertificateRefusal::Undecided)),
            Read::AutomatonStates => Err(Halt::Verdict(automaton_stop(self.max_automaton_states))),
            Read::Fault(_) => Err(fault(CheckerFault::Automaton)),
        }
    }

    /// Whether the certificate's automaton state `listed` names the
    /// checker's `q`, binding the two when neither is bound yet.
    fn binds(&mut self, listed: usize, q: usize) -> bool {
        if self.refined.is_some() {
            // A refinement form's second component is a position, which
            // `check_step` judges on the edge, not an automaton state.
            return true;
        }
        if self.window.is_none() && self.live.is_none() {
            return listed == 0 && q == 0;
        }
        match (self.bound.get(&listed), self.binds.get(&q)) {
            (Some(&known), _) => known == q,
            (None, Some(_)) => false,
            (None, None) => {
                self.bound.insert(listed, q);
                self.binds.insert(q, listed);
                true
            }
        }
    }

    /// Each initial symbolic state is covered by a named initial node of
    /// its discrete state and claim-automaton state.
    fn initial(&mut self) -> Step<(), O> {
        if self.live.is_some() {
            return self.live_initial_nodes();
        }
        if self.refined.is_some() {
            return self.refined_initial();
        }
        let certificate = self.certificate;
        for start in self.timed.initial() {
            self.poll()?;
            let state = start.discrete().clone();
            let state_key = self.state_key(&state)?;
            let zero = Rational::from_integer(Integer::zero());
            let q = self.read(None, &state, Stamp::At(&zero), false)?;
            let mut zone = Zone::zero(self.dim());
            self.settle(&state, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            let mut covered = None;
            for &node in &certificate.initials {
                let listed = &certificate.nodes[node];
                if listed.state_key == state_key
                    && zone.alu_covered_by(&listed.zone, &self.cover)?
                    && self.binds(listed.automaton_state, q)
                {
                    covered = Some(node);
                    break;
                }
            }
            let Some(node) = covered else {
                return reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(state_key.clone()),
                );
            };
            self.states[node].get_or_insert(state);
        }
        Ok(())
    }

    /// A refinement form's initial nodes (FR-245 "Refinement form"): each
    /// concrete initial state's symbolic state is covered by its listed
    /// node (`InitialMissing`), which carries the position `check_initial`
    /// yields, or, when it fails, is a `locked` entry node with the reserved
    /// index (`InitialMismatch`). Whether a `locked` node is time-locked is
    /// the `BadState` rule's, read with the other nodes.
    fn refined_initial(&mut self) -> Step<(), O> {
        let Some(Refined {
            refinement,
            abstract_system,
            part,
        }) = self.refined
        else {
            return Ok(());
        };
        let certificate = self.certificate;
        let reserved = part.positions.len();
        self.known = vec![None; reserved];
        let mut listed = certificate.initials.iter();
        for (index, start) in self.timed.initial().iter().enumerate() {
            self.poll()?;
            let state = start.discrete().clone();
            let state_key = self.state_key(&state)?;
            let mut zone = Zone::zero(self.dim());
            self.settle(&state, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            let covering = match listed.next() {
                Some(&node) => {
                    let entry = &certificate.nodes[node];
                    (entry.state_key == state_key
                        && zone.alu_covered_by(&entry.zone, &self.cover)?)
                    .then_some(node)
                }
                None => None,
            };
            let Some(node) = covering else {
                return reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(state_key.clone()),
                );
            };
            let entry = &certificate.nodes[node];
            let verdict = check_initial(
                refinement,
                self.timed.system(),
                abstract_system,
                index,
                state.state(),
            );
            let holds = match verdict {
                StepVerdict::Passes { post, .. } => {
                    let holds = !entry.locked
                        && entry.automaton_state < reserved
                        && self.position_is(entry.automaton_state, &post)?;
                    if holds {
                        self.know(entry.automaton_state, post);
                    }
                    holds
                }
                failed => {
                    self.unsound(&failed)?;
                    entry.locked && entry.automaton_state == reserved
                }
            };
            if !holds {
                return reject(CertificateRule::InitialMismatch, at_node(node));
            }
            self.states[node].get_or_insert(state);
        }
        Ok(())
    }

    /// A refinement form's judgement of edge `index` out of `node`, by
    /// `step`: out of a positioned node, `check_step` passes with the
    /// target's position and the edge's `taken` (`PostDiffers`), or fails
    /// into a `locked` entry node (`StepFails`); out of the locked region,
    /// the edge stays in it with no `taken` (`PostDiffers`). The first
    /// rejection out of a node is kept for the node's turn.
    fn refine_edge(&mut self, node: usize, index: usize, step: &GuardedStep) -> Step<(), O> {
        let Some(Refined {
            refinement,
            abstract_system,
            part,
        }) = self.refined
        else {
            return Ok(());
        };
        let certificate = self.certificate;
        let reserved = part.positions.len();
        let source = &certificate.nodes[node];
        let edge = &certificate.edges[index];
        let target = &certificate.nodes[edge.to];
        let taken = part.taken.get(index).and_then(Option::as_ref);
        let entry = target.locked && target.automaton_state == reserved;
        let rule = if source.automaton_state == reserved {
            // The locked region, on concrete steps only.
            (taken.is_some() || target.automaton_state != reserved)
                .then_some(CertificateRule::PostDiffers)
        } else {
            let Some(pre) = self.known.get(source.automaton_state).cloned().flatten() else {
                // A node entered by a rejected edge has no position; that
                // edge's rejection decides the check.
                return Ok(());
            };
            let verdict = check_step(
                refinement,
                self.timed.system(),
                abstract_system,
                &pre,
                &ConcreteStep {
                    position: u64::try_from(index).unwrap_or(u64::MAX),
                    transition: Some(step.transition()),
                    post: step.post(),
                },
            )
            .verdict;
            match verdict {
                StepVerdict::Passes {
                    post,
                    taken: yielded,
                } => {
                    let holds = !target.locked
                        && target.automaton_state < reserved
                        && taken == Some(&yielded)
                        && self.position_is(target.automaton_state, &post)?;
                    if holds {
                        self.know(target.automaton_state, post);
                    }
                    (!holds).then_some(CertificateRule::PostDiffers)
                }
                failed => {
                    self.unsound(&failed)?;
                    if !entry {
                        Some(CertificateRule::StepFails)
                    } else if taken.is_some() {
                        Some(CertificateRule::PostDiffers)
                    } else {
                        None
                    }
                }
            }
        };
        if let Some(rule) = rule {
            self.refinement_failures
                .entry(node)
                .or_insert(CertificateRejection {
                    rule,
                    at: at_edge(index),
                });
        }
        Ok(())
    }

    /// Whether the part's position `at` is `position`.
    fn position_is(&self, at: usize, position: &RefinementPosition) -> Step<bool, O> {
        let Some(Refined {
            abstract_system,
            part,
            ..
        }) = self.refined
        else {
            return Ok(false);
        };
        let Some(listed) = part.positions.get(at) else {
            return Ok(false);
        };
        position_matches(self.timed.system(), abstract_system, listed, position).or_else(undecided)
    }

    /// Record the position `at` holds, once.
    fn know(&mut self, at: usize, position: RefinementPosition) {
        if let Some(slot) = self.known.get_mut(at) {
            slot.get_or_insert(position);
        }
    }

    /// A failing verdict's end of the check when it is no finding: a run
    /// stop, a fault, or an `Undetermined` that only ran out of the
    /// evaluation meter, which stops the check (V-7), never rejects it.
    fn unsound(&self, verdict: &StepVerdict) -> Step<(), O> {
        let stop = |limit, value| {
            Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )))
        };
        match verdict {
            StepVerdict::Stopped(reached) => stop(reached.limit, reached.value),
            StepVerdict::Fault(_) => Err(fault(CheckerFault::RefinementStep)),
            StepVerdict::Undetermined(undetermined) => {
                match EvaluationOutcome::of_undetermined(undetermined).meter_counter() {
                    Some(counter) => stop(
                        ModelCheckLimit::EvaluationMeter { counter },
                        crate::meter_bound(&self.timed.system().meter(), counter),
                    ),
                    None => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    /// The liveness product's initial nodes: for each initial state and
    /// instance, each position-0 successor is covered by a named initial
    /// node of its discrete state, automaton state, mark and cell.
    fn live_initial_nodes(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        for start in self.timed.initial() {
            self.poll()?;
            let state = start.discrete().clone();
            let state_key = self.state_key(&state)?;
            let missing = || {
                reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(state_key.clone()),
                )
            };
            let Some(starts) = self.live_initial(&state)? else {
                return missing();
            };
            for (_, q, mark, zone) in starts {
                let cell = self.cell_zone_of(&state, &zone)?;
                let mut covered = None;
                for &node in &certificate.initials {
                    let listed = &certificate.nodes[node];
                    if listed.state_key == state_key
                        && listed.product == Some(mark)
                        && zone.alu_covered_by(&listed.zone, &self.cover)?
                        && cell.includes(&listed.zone)?
                        && self.binds(listed.automaton_state, q)
                    {
                        covered = Some(node);
                        break;
                    }
                }
                let Some(node) = covered else {
                    return missing();
                };
                self.states[node].get_or_insert(state.clone());
            }
        }
        Ok(())
    }

    /// The model state of every node an edge reaches from an initial node,
    /// each reached node's successors, and for each edge the successor it
    /// covers: of its transition identity, guard part and horizon piece,
    /// whose post-state, automaton state and zone its target holds.
    fn reach(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        let mut queue: Vec<usize> = (0..self.states.len())
            .filter(|&node| self.states[node].is_some())
            .collect();
        let mut next = 0;
        while let Some(&node) = queue.get(next) {
            next += 1;
            self.poll()?;
            let Some(state) = self.states[node].clone() else {
                continue;
            };
            let listed = &certificate.nodes[node];
            let q = self
                .bound
                .get(&listed.automaton_state)
                .copied()
                .unwrap_or_default();
            let successors = match listed.product {
                Some(mark) if self.live.is_some() => {
                    self.live_successors(node, &state, q, mark, &listed.zone)?
                }
                _ => self.successors_of(&state, q, &listed.zone)?,
            };
            for index in self.leaving[node].clone() {
                let edge = &certificate.edges[index];
                let target = &certificate.nodes[edge.to];
                for (position, successor) in successors.iter().enumerate() {
                    let same = successor
                        .step
                        .as_ref()
                        .map(|step| CertificateTransition::Source(step.transition().clone()))
                        .as_ref()
                        == edge.transition.as_ref()
                        && successor.part == edge.part
                        && successor.horizon == edge.horizon
                        && successor.state_key == target.state_key
                        && successor.silent == edge.silent
                        && successor.mark == target.product
                        && successor.zone.alu_covered_by(&target.zone, &self.cover)?
                        && match &successor.cell {
                            Some(cell) => cell.includes(&target.zone)?,
                            None => true,
                        };
                    if same && self.binds(target.automaton_state, successor.q) {
                        self.covers[index] = Some(position);
                        if let Some(step) = &successor.step {
                            self.refine_edge(node, index, step)?;
                        }
                        if self.states[edge.to].is_none() {
                            let post = successor
                                .step
                                .as_ref()
                                .map_or_else(|| state.clone(), |step| step.post().clone());
                            self.states[edge.to] = Some(post);
                            queue.push(edge.to);
                        }
                        break;
                    }
                }
            }
            self.successors[node] = Some(successors);
        }
        Ok(())
    }

    /// The symbolic successors of `(state, q, zone)`, the zone first held
    /// to the state's own invariants: one per enabled step, guard part and,
    /// for a TT-2 claim, horizon piece, and the claim's idle-tail stutter
    /// positions, as FR-239's search reads them.
    fn successors_of(
        &self,
        state: &ModelSimState,
        q: usize,
        zone: &Zone,
    ) -> Step<Vec<Successor>, O> {
        let mut zone = zone.clone();
        let invariants = self.invariants(state)?;
        self.constrain(&mut zone, &invariants)?;
        let mut found = Vec::new();
        if zone.is_empty() {
            return Ok(found);
        }
        let decided = self
            .window
            .as_ref()
            .is_none_or(|window| window.claim.borrow().decided(q));
        let tail = self.window.as_ref().is_some_and(|window| {
            window
                .claim
                .borrow()
                .state(q)
                .is_some_and(|state| state.tail)
        });
        let period = self.timed.tick_period().cloned();
        let mut positions = 0_usize;
        if !tail || decided {
            for step in self.timed.guarded_steps(state).or_else(undecided)? {
                let target = self.invariants(step.post())?;
                let state_key = self.state_key(step.post())?;
                let tick =
                    self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
                for (part, constraints) in step.parts().iter().enumerate() {
                    let mut next = zone.clone();
                    self.constrain(&mut next, constraints)?;
                    if next.is_empty() {
                        continue;
                    }
                    self.apply_clock_updates(&mut next, &step)?;
                    self.constrain(&mut next, &target)?;
                    if next.is_empty() {
                        continue;
                    }
                    positions += 1;
                    if decided {
                        if self.window.is_some() {
                            next.free(self.horizon_clock())?;
                        }
                        self.settle(step.post(), &mut next)?;
                        if !next.is_empty() {
                            found.push(Successor {
                                step: Some(step.clone()),
                                part: Some(part),
                                horizon: None,
                                frees: self.window.is_some(),
                                state_key: state_key.clone(),
                                q,
                                zone: next,
                                silent: None,
                                mark: None,
                                cell: None,
                                read: None,
                            });
                        }
                        continue;
                    }
                    let advance = match (&period, tick) {
                        (Some(period), true) => period.clone(),
                        _ => Rational::from_integer(Integer::zero()),
                    };
                    for (constraints, stamp) in self.pieces() {
                        let mut at = next.clone();
                        self.constrain(&mut at, &constraints)?;
                        if at.is_empty() {
                            continue;
                        }
                        let stamp_of = match period {
                            Some(_) => Stamp::After(&advance),
                            None => Stamp::At(&stamp),
                        };
                        let read = self.read(Some(q), step.post(), stamp_of, false)?;
                        self.settle(step.post(), &mut at)?;
                        if at.is_empty() {
                            continue;
                        }
                        found.push(Successor {
                            step: Some(step.clone()),
                            part: Some(part),
                            horizon: period.is_none().then_some(constraints),
                            frees: false,
                            state_key: state_key.clone(),
                            q: read,
                            zone: at,
                            silent: None,
                            mark: None,
                            cell: None,
                            read: None,
                        });
                    }
                }
            }
        }
        if decided {
            return Ok(found);
        }
        // An idle-tail stutter position: under a dense source at a
        // quiescent state, under a tick source at a terminal one (FR-231).
        let idles = match &period {
            Some(_) => tail || positions == 0,
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        };
        if idles {
            let state_key = self.state_key(state)?;
            for (constraints, stamp) in self.pieces() {
                let mut at = zone.clone();
                self.constrain(&mut at, &constraints)?;
                if at.is_empty() {
                    continue;
                }
                let stamp_of = match &period {
                    Some(period) => Stamp::After(period),
                    None => Stamp::At(&stamp),
                };
                let read = self.read(Some(q), state, stamp_of, true)?;
                self.settle(state, &mut at)?;
                if at.is_empty() {
                    continue;
                }
                found.push(Successor {
                    step: None,
                    part: None,
                    horizon: period.is_none().then_some(constraints),
                    frees: false,
                    state_key: state_key.clone(),
                    q: read,
                    zone: at,
                    silent: None,
                    mark: None,
                    cell: None,
                    read: None,
                });
            }
        }
        Ok(found)
    }

    /// The pieces of a TT-2 claim's horizon axis.
    fn pieces(&self) -> Vec<(Vec<PlaceConstraint>, Rational)> {
        self.window
            .as_ref()
            .map(|window| window.pieces.clone())
            .unwrap_or_default()
    }

    /// Node `node`'s successors are each covered by an edge, and it is bad
    /// only when it is `locked` and no time diverges after it.
    fn node(&self, node: usize, divergent: &[bool]) -> Step<(), O> {
        let count = self.successors[node].as_ref().map_or(0, Vec::len);
        let mut covered = vec![false; count];
        for &index in &self.leaving[node] {
            if let Some(position) = self.covers[index] {
                covered[position] = true;
            }
        }
        let successors = self.successors[node]
            .as_ref()
            .expect("a reached node has recomputed successors");
        if self.no_release_task
            && successors.iter().any(|successor| {
                successor.step.as_ref().is_some_and(|step| {
                    self.task_release_operations
                        .contains(step.transition().operation())
                })
            })
        {
            return reject(CertificateRule::SuccessorMissing, at_node(node));
        }
        // A set or bag assignment can produce several semantically equal
        // guarded steps (for example, equal element matches). The certificate
        // edge schema names their transition, guard part and resulting
        // symbolic state, so one matching edge covers all duplicate
        // recomputations with that same observable successor.
        let witnesses: Vec<usize> = covered
            .iter()
            .enumerate()
            .filter_map(|(position, &covered)| covered.then_some(position))
            .collect();
        for position in 0..successors.len() {
            if covered[position] {
                continue;
            }
            let successor = &successors[position];
            covered[position] = witnesses
                .iter()
                .any(|&other| equivalent_successor(successor, &successors[other]));
        }
        if covered.iter().any(|&covered| !covered) {
            return reject(CertificateRule::SuccessorMissing, at_node(node));
        }
        let listed = &self.certificate.nodes[node];
        if listed.locked {
            if divergent[node] {
                return reject(CertificateRule::BadState, at_node(node));
            }
        } else if self.bad(node)? {
            return reject(CertificateRule::BadState, at_node(node));
        }
        Ok(())
    }

    fn state(&self, node: usize) -> Step<&ModelSimState, O> {
        self.states[node]
            .as_ref()
            .ok_or(fault(CheckerFault::UnreachedNode))
    }

    /// Whether `node` violates the item at some valuation of its zone.
    fn bad(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        if self.time_lock || self.live.is_some() || self.refined.is_some() {
            // A refinement form's badness is `check_step`'s, judged on
            // each edge in `reach`.
            // Time-lock freedom is read from the divergence part, and a
            // liveness product's bad cycles from its components.
            return Ok(false);
        }
        let Some(claim) = &self.claim else {
            return self.deadlocked(node);
        };
        if let Some(window) = &self.window {
            let listed = self.certificate.nodes[node].automaton_state;
            let q = self
                .bound
                .get(&listed)
                .copied()
                .ok_or(fault(CheckerFault::UnreachedNode))?;
            let automaton = window.claim.borrow();
            return Ok(automaton.undefined(q).is_some()
                || automaton.violated(q).is_some()
                || automaton.insufficient(q).is_some());
        }
        let atom = claim.atom.ok_or(fault(CheckerFault::Automaton))?;
        for instance in &claim.instances {
            let mut meter = Meter::new(self.timed.system().meter_limits());
            match claim
                .letters
                .atom(atom, instance, state, &mut meter)
                .or_else(undecided)?
            {
                AtomReading::True => {}
                AtomReading::False | AtomReading::Undefined(_) => return Ok(true),
            }
        }
        Ok(false)
    }

    /// FR-232 over the zone held to its state's invariants: a quiescent
    /// node some of whose valuations enable no step after any delay. Read
    /// under the aM coverage, a deadlocked real valuation has a bisimilar
    /// deadlocked one in some node.
    fn deadlocked(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        if !self.timed.is_quiescent(state).or_else(undecided)? {
            return Ok(false);
        }
        let mut enabling = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            let target = self.invariants(step.post())?;
            // Pull post-state invariants through the simultaneous carries
            // and resets to the pre-state valuations that enable this step.
            let mut pulled_back = Vec::with_capacity(target.len());
            let mut impossible = false;
            for bound in target {
                if let Some((_, value)) = step
                    .resets()
                    .iter()
                    .find(|(place, _)| *place == bound.place())
                {
                    if !bound.op().holds(value, bound.bound()) {
                        impossible = true;
                        break;
                    }
                    continue;
                }
                let place = step
                    .carries()
                    .iter()
                    .find(|(target, _)| *target == bound.place())
                    .map_or(bound.place(), |(_, source)| *source);
                pulled_back.push(PlaceConstraint::new(
                    place,
                    bound.op(),
                    bound.bound().clone(),
                ));
            }
            if impossible {
                continue;
            }
            for part in step.parts() {
                let mut zone = Zone::universe(self.dim());
                self.constrain(&mut zone, part)?;
                self.constrain(&mut zone, &pulled_back)?;
                if !zone.is_empty() {
                    zone.down();
                    enabling.push(zone);
                }
            }
        }
        if enabling.is_empty() {
            return Ok(matches!(
                self.timed.system().terminal_class(state, false),
                Ok(TerminalClass::Deadlocked)
            ));
        }
        let mut zone = self.certificate.nodes[node].zone.clone();
        self.constrain(&mut zone, &self.invariants(state)?)?;
        let mut uncovered = vec![zone];
        for cover in &enabling {
            let mut rest = Vec::new();
            for piece in &uncovered {
                rest.extend(piece.minus(cover)?);
            }
            uncovered = rest;
            if uncovered.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The step a valid edge takes, `None` for a stutter, and the
    /// successor it covers.
    fn covered(&self, index: usize) -> Step<&Successor, O> {
        let edge = &self.certificate.edges[index];
        self.covers[index]
            .and_then(|position| {
                self.successors[edge.from]
                    .as_ref()
                    .and_then(|successors| successors.get(position))
            })
            .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))
    }

    /// For each node, whether over valid edges it reaches a node where a
    /// behaviour may idle forever, or a strongly connected set whose
    /// divergence the checker cannot rule out. The components are computed
    /// once, under the caller's `cancel`, and divergence is propagated back
    /// along the valid edges.
    fn divergent(&self) -> Step<Vec<bool>, O> {
        let count = self.certificate.nodes.len();
        let mut graph = Graph::new();
        let vertices: Vec<_> = (0..count)
            .map(|_| graph.add_vertex(std::iter::empty::<Identity>()))
            .collect();
        let mut into: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            if self.covers[index].is_some() {
                graph
                    .add_edge(vertices[edge.from], vertices[edge.to], EdgeLabel::Stutter)
                    .map_err(|_| fault(CheckerFault::Graph))?;
                into[edge.to].push(edge.from);
            }
        }
        let within: BTreeSet<_> = (0..count)
            .filter(|&node| self.states[node].is_some())
            .map(|node| vertices[node])
            .collect();
        let found = components(&graph, &within, self.cancel).map_err(|error| match error {
            FairnessError::Cancelled => match self.cancel.cause() {
                Some(source) => Halt::Verdict(ZoneCertificateVerdict::Stopped(
                    IncompleteCause::Cancelled { source },
                )),
                None => fault(CheckerFault::CancellationWithoutCause),
            },
            _ => fault(CheckerFault::Graph),
        })?;
        let mut divergent = vec![false; count];
        for (node, divergent) in divergent.iter_mut().enumerate() {
            *divergent = self.states[node].is_some() && self.idles(node)?;
        }
        for component in found.iter().filter(|component| component.has_edge()) {
            self.poll()?;
            let members: BTreeSet<usize> = component
                .vertices()
                .iter()
                .map(|vertex| vertex.index())
                .collect();
            let inside: Vec<usize> = members
                .iter()
                .flat_map(|&node| self.leaving[node].iter().copied())
                .filter(|&index| {
                    self.covers[index].is_some()
                        && members.contains(&self.certificate.edges[index].to)
                })
                .collect();
            if !self.zeno(&members, &inside)? {
                for &member in &members {
                    divergent[member] = true;
                }
            }
        }
        let mut queue: Vec<usize> = (0..count).filter(|&node| divergent[node]).collect();
        while let Some(node) = queue.pop() {
            for &from in &into[node] {
                if !divergent[from] {
                    divergent[from] = true;
                    queue.push(from);
                }
            }
        }
        Ok(divergent)
    }

    /// Whether a behaviour may idle at `node` forever: under a dense source
    /// a quiescent state where time elapses, under a tick source one that
    /// enables no step, whose stutter ticks (FR-231).
    fn idles(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        Ok(match self.timed.tick_period() {
            Some(_) => self.successors[node].as_ref().is_none_or(|successors| {
                successors.iter().all(|successor| successor.step.is_none())
            }),
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        })
    }

    /// Whether an edge lets time advance under a tick source: a tick step,
    /// or a terminal state's stutter, which counts as a tick (FR-231).
    fn ticks(&self, index: usize) -> Step<bool, O> {
        Ok(match &self.covered(index)?.step {
            Some(step) => {
                self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero())
            }
            None => true,
        })
    }

    /// Whether no time diverges inside the strongly connected `members`
    /// with the `edges` among them. Under a dense source: every member is
    /// urgent, or some clock that none of `edges` assigns is bounded above
    /// by the time invariants of every member's own state, which every
    /// real valuation there satisfies. Under a tick source: none of
    /// `edges` ticks.
    fn zeno(&self, members: &BTreeSet<usize>, edges: &[usize]) -> Step<bool, O> {
        if self.timed.tick_period().is_some() {
            for &index in edges {
                if self.ticks(index)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let mut urgent = true;
        let mut bounded: Option<BTreeSet<usize>> = None;
        for &member in members {
            let state = self.state(member)?;
            urgent &= !self.elapses(state)?;
            let here: BTreeSet<usize> = self
                .invariants(state)?
                .iter()
                .filter(|bound| {
                    matches!(
                        bound.op(),
                        ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal
                    )
                })
                .map(PlaceConstraint::place)
                .collect();
            bounded = Some(match bounded {
                None => here,
                Some(earlier) => earlier.intersection(&here).copied().collect(),
            });
        }
        if urgent {
            return Ok(true);
        }
        let mut reset = BTreeSet::new();
        for &index in edges {
            if let Some(step) = &self.covered(index)?.step {
                reset.extend(step.resets().iter().map(|(place, _)| *place));
                reset.extend(
                    step.carries()
                        .iter()
                        .filter_map(|(target, source)| (target != source).then_some(*target)),
                );
            }
        }
        Ok(bounded.is_some_and(|bounded| bounded.iter().any(|place| !reset.contains(place))))
    }

    /// `admitted`: a path of valid edges from an initial node to a
    /// divergent target, recomputed in exact zones from the initial state,
    /// each held to its state's own invariants, so the path is one some
    /// behaviour takes. It proves that a time-divergent behaviour exists,
    /// as FR-244 TD-6 says, not that a fair one does: FR-240's vacuity
    /// under fairness has no certificate part (spec gap STD-493, comment
    /// 8eafddf9, item 3; SR-1647002 FND-002).
    fn admitted(&self) -> Step<(), O> {
        let witness = &self.certificate.admitted;
        let mut at = witness.start;
        let fails = |at: usize| reject(CertificateRule::WitnessFails, at_node(at));
        if !self.certificate.initials.contains(&at) || self.states[at].is_none() {
            return fails(at);
        }
        // A refinement form's witness starts at a positioned initial node: a
        // locked one has no admitted behaviour (FR-245).
        if let Some(refined) = &self.refined {
            let listed = &self.certificate.nodes[at];
            if listed.locked || listed.automaton_state >= refined.part.positions.len() {
                return fails(at);
            }
        }
        let Some(mut zone) = self.initial_zone(at)? else {
            return fails(at);
        };
        for &index in &witness.path {
            let edge = &self.certificate.edges[index];
            if edge.from != at || self.covers[index].is_none() {
                return fails(at);
            }
            zone = self.post(index, &zone)?;
            at = edge.to;
            if zone.is_empty() {
                return fails(at);
            }
        }
        let holds = match &witness.target {
            DivergentTarget::Quiescent(node) => {
                *node == at && self.timed.tick_period().is_none() && self.idles(at)?
            }
            DivergentTarget::Terminal(node) => {
                *node == at && self.timed.tick_period().is_some() && self.idles(at)?
            }
            DivergentTarget::Cycle(cycle) => {
                self.advancing_cycle(at, cycle)? && self.repeats(cycle, &zone)?
            }
        };
        if holds {
            Ok(())
        } else {
            fails(at)
        }
    }

    /// The exact zone of an initial state the initial node `node` holds.
    fn initial_zone(&self, node: usize) -> Step<Option<Zone>, O> {
        let listed = &self.certificate.nodes[node];
        for start in self.timed.initial() {
            let state = start.discrete();
            if self.state_key(state)? != listed.state_key {
                continue;
            }
            let mut zone = Zone::zero(self.dim());
            if self.live.is_some() {
                let cell = self.cell_zone_of(state, &zone)?;
                self.settle_within(state, &mut zone, Some(&cell))?;
                if !zone.is_empty() {
                    return Ok(Some(zone));
                }
                continue;
            }
            let zero = Rational::from_integer(Integer::zero());
            let q = self.read(None, state, Stamp::At(&zero), false)?;
            if self.bound.get(&listed.automaton_state) != Some(&q) && self.window.is_some() {
                continue;
            }
            self.settle(state, &mut zone)?;
            if !zone.is_empty() {
                return Ok(Some(zone));
            }
        }
        Ok(None)
    }

    /// The exact successor of `zone`, at a valid edge's source, along the
    /// edge.
    fn post(&self, index: usize, zone: &Zone) -> Step<Zone, O> {
        let edge = &self.certificate.edges[index];
        let source = self.state(edge.from)?;
        let successor = self.covered(index)?;
        let mut next = zone.clone();
        if let Some(cell) = &successor.cell {
            // A liveness product's move, within the target's cell.
            let post = successor.step.as_ref().map_or(source, GuardedStep::post);
            match (&successor.step, successor.silent) {
                (Some(step), _) => {
                    let part = successor
                        .part
                        .and_then(|part| step.parts().get(part))
                        .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
                    self.constrain(&mut next, part)?;
                    self.apply_clock_updates(&mut next, step)?;
                    self.constrain(&mut next, &self.invariants(post)?)?;
                }
                (None, Some(SilentMove::Wait)) => {
                    next.up();
                    self.constrain(&mut next, &self.invariants(source)?)?;
                }
                (None, Some(SilentMove::Diverge)) => {
                    let z = self.z_clock();
                    next.constrain(0, z, &self.certificate.scale.neg(), false)?;
                    if !next.is_empty() {
                        next.reset(z, &Integer::zero())?;
                    }
                }
                (None, None) => {}
            }
            if let Some(read) = &successor.read {
                self.read_at(&mut next, read)?;
            }
            self.settle_within(post, &mut next, Some(cell))?;
            return Ok(next);
        }
        let post = match &successor.step {
            Some(step) => {
                let part = successor
                    .part
                    .and_then(|part| step.parts().get(part))
                    .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
                self.constrain(&mut next, part)?;
                self.apply_clock_updates(&mut next, step)?;
                self.constrain(&mut next, &self.invariants(step.post())?)?;
                step.post()
            }
            None => source,
        };
        if let Some(horizon) = &successor.horizon {
            self.constrain(&mut next, horizon)?;
        }
        if successor.frees {
            next.free(self.horizon_clock())?;
        }
        self.settle(post, &mut next)?;
        Ok(next)
    }

    /// The valuations of `target`, at a valid edge's target, whose
    /// predecessors along the edge lie at its source: the inverse of
    /// [`Self::post`].
    fn pre(&self, index: usize, target: &Zone) -> Step<Zone, O> {
        let edge = &self.certificate.edges[index];
        let source = self.state(edge.from)?;
        let successor = self.covered(index)?;
        let post = successor.step.as_ref().map_or(source, GuardedStep::post);
        let invariants = self.invariants(post)?;
        let mut zone = target.clone();
        let hold = |zone: &mut Zone| -> Step<(), O> {
            self.constrain(zone, &invariants)?;
            if let Some(cell) = &successor.cell {
                intersect(zone, cell)?;
            }
            Ok(())
        };
        hold(&mut zone)?;
        if self.elapses(post)? && !zone.is_empty() {
            zone.down();
            hold(&mut zone)?;
        }
        match successor.silent {
            Some(SilentMove::Wait) => {
                if !zone.is_empty() {
                    zone.down();
                }
            }
            Some(SilentMove::Diverge) => {
                let z = self.z_clock();
                zone.constrain(z, 0, &Integer::zero(), false)?;
                zone.free(z)?;
                zone.constrain(0, z, &self.certificate.scale.neg(), false)?;
            }
            None => {}
        }
        if let Some(read) = &successor.read {
            self.read_before(&mut zone, read)?;
        }
        if let Some(horizon) = &successor.horizon {
            self.constrain(&mut zone, horizon)?;
        }
        if successor.frees {
            zone.free(self.horizon_clock())?;
        }
        if let Some(step) = &successor.step {
            zone = self.preimage_clock_updates(&zone, step)?;
            let part = successor
                .part
                .and_then(|part| step.parts().get(part))
                .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
            self.constrain(&mut zone, part)?;
        }
        self.constrain(&mut zone, &self.invariants(source)?)?;
        Ok(zone)
    }

    /// Whether `cycle` can be repeated forever from some valuation of
    /// `zone`, the exact zone the path reaches at its start. The greatest
    /// set of valuations at the start, within its invariants, each of which
    /// takes the cycle back into the set, is computed from the whole
    /// invariant zone down: each round keeps the valuations whose cycle
    /// lands in the last round's set. Each round is one zone, and each of
    /// its bounds only falls, so the rounds end, at a fixed point or the
    /// empty set. A valuation of `zone` in the fixed point is reached and
    /// repeats the cycle forever: a clock the cycle never resets may drift
    /// at each repetition. Each round reads the cycle's edges again, charged
    /// to `max_certificate_edges`.
    fn repeats(&self, cycle: &[usize], zone: &Zone) -> Step<bool, O> {
        if cycle.is_empty() {
            return Ok(false);
        }
        let start = self.state(self.certificate.edges[cycle[0]].from)?;
        let invariants = self.invariants(start)?;
        let mut held = Zone::universe(self.dim());
        self.constrain(&mut held, &invariants)?;
        let length = u64::try_from(cycle.len()).unwrap_or(u64::MAX);
        loop {
            self.charge(length)?;
            let mut back = held.clone();
            for &index in cycle.iter().rev() {
                back = self.pre(index, &back)?;
                if back.is_empty() {
                    return Ok(false);
                }
            }
            // The valuations at the start from which a delay reaches the
            // first step.
            self.constrain(&mut back, &invariants)?;
            if self.elapses(start)? && !back.is_empty() {
                back.down();
                self.constrain(&mut back, &invariants)?;
            }
            let mut next = held.clone();
            intersect(&mut next, &back)?;
            if next.is_empty() {
                return Ok(false);
            }
            if next.includes(&held)? {
                let mut met = next;
                intersect(&mut met, zone)?;
                return Ok(!met.is_empty());
            }
            held = next;
        }
    }

    /// Whether `cycle` is a cycle of valid edges from and back to `at`
    /// each repetition of which lets time advance: it ticks, under a tick
    /// source; under a dense source, some clock guarded on it by a lower
    /// bound above the value its last reset before the guard set it.
    fn advancing_cycle(&self, at: usize, cycle: &[usize]) -> Step<bool, O> {
        let mut node = at;
        for &index in cycle {
            let edge = &self.certificate.edges[index];
            if edge.from != node || self.covers[index].is_none() {
                return Ok(false);
            }
            node = edge.to;
        }
        if cycle.is_empty() || node != at {
            return Ok(false);
        }
        // The divergence clock resets only once a unit has passed.
        if cycle
            .iter()
            .any(|&index| self.certificate.edges[index].silent == Some(SilentMove::Diverge))
        {
            return Ok(true);
        }
        if self.timed.tick_period().is_some() {
            for &index in cycle {
                if self.ticks(index)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let mut steps = Vec::with_capacity(cycle.len());
        for &index in cycle {
            let successor = self.covered(index)?;
            steps.push((successor.step.as_ref(), successor.part));
        }
        let length = steps.len();
        for (position, (step, part)) in steps.iter().enumerate() {
            let Some(constraints) = step
                .zip(*part)
                .and_then(|(step, part)| step.parts().get(part))
            else {
                continue;
            };
            for constraint in constraints {
                let strict_lower = match constraint.op() {
                    ClockOp::Greater => true,
                    ClockOp::GreaterOrEqual | ClockOp::Equal => false,
                    ClockOp::Less | ClockOp::LessOrEqual => continue,
                };
                // The last reset of the clock before this guard, going back
                // around the cycle; this edge's own resets come after it.
                let reset = (1..=length).find_map(|back| {
                    let (earlier, _) = steps[(position + length - back) % length];
                    earlier?
                        .resets()
                        .iter()
                        .rev()
                        .find(|(place, _)| *place == constraint.place())
                        .map(|(_, value)| value.clone())
                });
                let Some(reset) = reset else {
                    continue;
                };
                let bound = constraint.bound();
                if bound > &reset || (strict_lower && bound == &reset) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// `zone` intersected with `other`, cell by cell.
fn intersect(zone: &mut Zone, other: &Zone) -> Result<(), ZoneError> {
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            if let Some((value, strict)) = other.bound(i, j)? {
                zone.constrain(i, j, &value, strict)?;
            }
        }
    }
    Ok(())
}

/// `value` in units of `1 / scale`; `None` when the scale does not make it
/// an integer.
fn scaled(value: &Rational, scale: &Integer) -> Option<Integer> {
    let product = value.mul(&Rational::from_integer(scale.clone()));
    (product.denominator().as_big() == &num_bigint::BigInt::from(1))
        .then(|| Integer::from_big(product.numerator().as_big().clone()))
}

/// The LU bounds the checker computes from the model and, for a TT-2
/// claim, its automaton's `constants` for the horizon clock: per clock,
/// the largest constant a guard, invariant or reset compares it with from
/// below and from above, scaled; `None` when the scale makes any constant
/// not an integer.
#[allow(
    clippy::type_complexity,
    reason = "one list of lower and one of upper bounds"
)]
fn computed_bounds(
    timed: &TimedSystem<'_, '_>,
    constants: Option<&[Rational]>,
    scale: &Integer,
) -> Option<(Vec<Option<Integer>>, Vec<Option<Integer>>)> {
    let model = timed.model();
    let fields = model.clocks().len();
    let mut lower: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut upper: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut record = |clock: usize, op: ClockOp, bound: &Rational| {
        if clock >= fields {
            return;
        }
        if matches!(
            op,
            ClockOp::Greater | ClockOp::GreaterOrEqual | ClockOp::Equal
        ) {
            lower[clock].push(bound.clone());
        }
        if matches!(op, ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal) {
            upper[clock].push(bound.clone());
        }
    };
    let mut parts: Vec<_> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .collect();
    while let Some(part) = parts.pop() {
        for constraint in part.clocks() {
            record(
                constraint.clock().index(),
                constraint.op(),
                constraint.bound(),
            );
        }
        parts.extend(part.families().iter().flat_map(|family| family.parts()));
    }
    for invariant in model.invariants() {
        for constraint in invariant.bounds() {
            record(
                constraint.clock().index(),
                constraint.op(),
                constraint.bound(),
            );
        }
    }
    for resets in model.resets() {
        for reset in resets.resets() {
            record(reset.clock().index(), ClockOp::Equal, reset.value());
        }
    }
    // Every constant, not only the largest, must be exact at the scale: a
    // checker zone reads each one.
    let all = lower
        .iter()
        .chain(&upper)
        .flatten()
        .chain(constants.into_iter().flatten());
    for value in all {
        scaled(value, scale)?;
    }
    let max = |values: &[Rational]| -> Option<Option<Integer>> {
        match values.iter().max() {
            Some(value) => scaled(value, scale).map(Some),
            None => Some(None),
        }
    };
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    for place in timed.places() {
        let clock = place.clock().index();
        lows.push(max(lower.get(clock)?)?);
        highs.push(max(upper.get(clock)?)?);
    }
    if let Some(constants) = constants {
        // The horizon clock is compared with every interval end, from both
        // sides.
        lows.push(max(constants)?);
        highs.push(max(constants)?);
    }
    Some((lows, highs))
}

/// The size a check charges against `max_certificate_bytes`: each node's
/// key and zone cells, each edge, the LU bounds, the witnesses and the
/// component numbering, at their integers' magnitudes. It bounds the certificate's in-memory reading,
/// not an encoding: FR-418's byte encoding of the certificate does not
/// exist yet (spec gap STD-493).
fn certificate_bytes<O>(certificate: &ZoneCertificate<O>) -> u64 {
    let integer = |value: &Integer| value.magnitude_bits() / 8 + 1;
    let mut bytes = integer(&certificate.scale);
    for bound in certificate
        .lu_bounds
        .lower
        .iter()
        .chain(&certificate.lu_bounds.upper)
        .flatten()
    {
        bytes = bytes.saturating_add(integer(bound));
    }
    for node in &certificate.nodes {
        bytes = bytes.saturating_add(40);
        let dim = node.zone.dim();
        for i in 0..dim {
            for j in 0..dim {
                let cell = match node.zone.bound(i, j) {
                    Ok(Some((value, _))) => integer(&value) + 1,
                    _ => 1,
                };
                bytes = bytes.saturating_add(cell);
            }
        }
    }
    let edges = u64::try_from(certificate.edges.len()).unwrap_or(u64::MAX);
    let mut parts = witness_bytes(&certificate.admitted);
    if let Some(components) = &certificate.components {
        parts = parts.saturating_add(components_bytes(components));
    }
    for witness in certificate.divergence.iter().flatten() {
        parts = parts.saturating_add(witness_bytes(witness));
    }
    if let Some(part) = &certificate.refinement {
        let mut refined = 32_u64;
        for position in &part.positions {
            let history = u64::try_from(position.history.len()).unwrap_or(u64::MAX);
            let states = u64::try_from(position.abstract_states.len()).unwrap_or(u64::MAX);
            refined = refined
                .saturating_add(40)
                .saturating_add(history)
                .saturating_add(states.saturating_mul(40));
        }
        let taken = u64::try_from(part.taken.len()).unwrap_or(u64::MAX);
        parts = parts
            .saturating_add(refined)
            .saturating_add(taken.saturating_mul(8));
    }
    bytes
        .saturating_add(edges.saturating_mul(48))
        .saturating_add(parts)
}

/// [`certificate_bytes`] of a divergence witness: its edges, each 8.
fn witness_bytes(witness: &DivergenceWitness) -> u64 {
    let cycle = match &witness.target {
        DivergentTarget::Cycle(cycle) => cycle.len(),
        DivergentTarget::Quiescent(_) | DivergentTarget::Terminal(_) => 1,
    };
    u64::try_from(witness.path.len().saturating_add(cycle).saturating_add(1))
        .unwrap_or(u64::MAX)
        .saturating_mul(8)
}

/// [`certificate_bytes`] of a component numbering: each node's index and
/// each reason, 8 per entry, nested numberings included. Read without
/// recursion, however deep the nesting.
fn components_bytes(components: &Components) -> u64 {
    let mut bytes = 0_u64;
    let mut pending = vec![components];
    while let Some(components) = pending.pop() {
        let entries = components
            .numbering
            .len()
            .saturating_add(components.reasons.len());
        bytes = bytes.saturating_add(u64::try_from(entries).unwrap_or(u64::MAX).saturating_mul(8));
        for reason in &components.reasons {
            if let ComponentReason::Strong {
                constraints,
                nested,
            } = reason
            {
                let listed = u64::try_from(constraints.len()).unwrap_or(u64::MAX);
                bytes = bytes.saturating_add(listed.saturating_mul(8));
                pending.push(nested);
            }
        }
    }
    bytes
}

/// Whether `components` numbers `nodes` nodes into its reasons, and each
/// strong reason's nested numbering does too, numbering a strict subset of
/// that reason's component: the engine nests a strong reason only below a
/// node that enables one of its constraints, which the nested numbering
/// leaves out. Each level then numbers fewer nodes, so the nesting is no
/// deeper than the node count (SR-1647001 FND-001). Read without
/// recursion.
fn numbering_ok(components: &Components, nodes: usize) -> bool {
    let mut pending = vec![components];
    while let Some(components) = pending.pop() {
        if components.numbering.len() != nodes {
            return false;
        }
        let mut sizes = vec![0_usize; components.reasons.len()];
        for &index in components.numbering.iter().flatten() {
            match sizes.get_mut(index) {
                Some(size) => *size += 1,
                None => return false,
            }
        }
        for (index, reason) in components.reasons.iter().enumerate() {
            let ComponentReason::Strong { nested, .. } = reason else {
                continue;
            };
            if nested.numbering.len() != nodes {
                return false;
            }
            let mut numbered = 0_usize;
            for (inner, outer) in nested.numbering.iter().zip(&components.numbering) {
                if inner.is_some() {
                    if *outer != Some(index) {
                        return false;
                    }
                    numbered += 1;
                }
            }
            if numbered >= sizes[index] {
                return false;
            }
            pending.push(nested);
        }
    }
    true
}

#[cfg(test)]
mod task_product_control_tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::Span;
    use qsl_semantics::check::{Arrival, Criticality, Task, Wcet};

    fn transition(operation: &str) -> ModelTransition {
        ModelTransition::new(
            serde_json::to_string(operation).expect("quoted operation identity"),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1024),
        )
        .expect("canonical model transition")
    }

    fn put_bytes(target: &mut Vec<u8>, bytes: &[u8]) {
        target.extend_from_slice(
            &u64::try_from(bytes.len())
                .expect("test input fits")
                .to_be_bytes(),
        );
        target.extend_from_slice(bytes);
    }

    fn put_text(target: &mut Vec<u8>, text: &str) {
        put_bytes(target, text.as_bytes());
    }

    fn pending_release(source: &ModelTransition) -> Vec<u8> {
        let mut control = TASK_CONTROL_PREFIX.to_vec();
        control.extend_from_slice(&0_u64.to_be_bytes()); // jobs
        control.push(0); // running
        control.extend_from_slice(&0_u64.to_be_bytes()); // activated task sets
        control.extend_from_slice(&1_u64.to_be_bytes()); // pending releases
        put_text(&mut control, "Ctl");
        put_text(&mut control, "t1");
        put_bytes(
            &mut control,
            &source
                .canonical_bytes(quire_canonical::Limits::new(1024))
                .expect("canonical transition bytes"),
        );
        control
    }

    fn ready_job(release: &ModelTransition) -> Vec<u8> {
        let mut control = TASK_CONTROL_PREFIX.to_vec();
        control.extend_from_slice(&1_u64.to_be_bytes()); // jobs
        put_text(&mut control, "Ctl");
        put_text(&mut control, "t1");
        control.extend_from_slice(&0_u64.to_be_bytes()); // rank
        control.extend_from_slice(&0_u64.to_be_bytes()); // slot
        put_bytes(
            &mut control,
            &release
                .canonical_bytes(quire_canonical::Limits::new(1024))
                .expect("canonical transition bytes"),
        );
        control.push(1); // ready
        put_text(&mut control, "1/1"); // exact demand
        control.push(1); // fixed priority is present
        control.extend_from_slice(&1_u64.to_be_bytes());
        control.push(0); // running
        control.extend_from_slice(&1_u64.to_be_bytes()); // activated task sets
        put_text(&mut control, "Ctl");
        put_text(&mut control, "t1");
        control.push(1);
        control.extend_from_slice(&0_u64.to_be_bytes()); // pending releases
        control
    }

    fn running_with_suspended_job(release: &ModelTransition) -> Vec<u8> {
        let mut control = TASK_CONTROL_PREFIX.to_vec();
        control.extend_from_slice(&2_u64.to_be_bytes()); // jobs
        for (rank, slot, phase) in [(0_u64, 0_u64, 3_u8), (1, 1, 2)] {
            put_text(&mut control, "Ctl");
            put_text(&mut control, "t1");
            control.extend_from_slice(&rank.to_be_bytes());
            control.extend_from_slice(&slot.to_be_bytes());
            put_bytes(
                &mut control,
                &release
                    .canonical_bytes(quire_canonical::Limits::new(1024))
                    .expect("canonical transition bytes"),
            );
            control.push(phase);
            put_text(&mut control, "1/1");
            control.push(1);
            control.extend_from_slice(&1_u64.to_be_bytes());
        }
        control.push(1); // running job
        put_text(&mut control, "Ctl");
        put_text(&mut control, "t1");
        control.extend_from_slice(&1_u64.to_be_bytes());
        put_bytes(
            &mut control,
            &release
                .canonical_bytes(quire_canonical::Limits::new(1024))
                .expect("canonical transition bytes"),
        );
        control.extend_from_slice(&1_u64.to_be_bytes()); // activated task sets
        put_text(&mut control, "Ctl");
        put_text(&mut control, "t1");
        control.push(1);
        control.extend_from_slice(&0_u64.to_be_bytes()); // pending releases
        control
    }

    fn node(control: Vec<u8>) -> CertNode {
        let source = StateKey::from_canonical_bytes(b"source-state".to_vec());
        CertNode {
            state_key: TaskProductStateKey::new(source, control)
                .into_state_key()
                .expect("joint state key"),
            automaton_state: 0,
            zone: Zone::universe(1),
            locked: false,
            product: None,
        }
    }

    fn certificate(
        from: Vec<u8>,
        to: Vec<u8>,
        template: TaskTransitionTemplate,
    ) -> ZoneCertificate<()> {
        ZoneCertificate {
            obligation: (),
            scale: Integer::one(),
            lu_bounds: LuBounds {
                lower: Vec::new(),
                upper: Vec::new(),
            },
            nodes: vec![node(from), node(to)],
            edges: vec![CertEdge {
                from: 0,
                transition: Some(CertificateTransition::Task(Box::new(template))),
                part: None,
                horizon: None,
                to: 1,
                silent: None,
            }],
            initials: vec![0],
            components: None,
            divergence: None,
            admitted: DivergenceWitness {
                start: 0,
                path: Vec::new(),
                target: DivergentTarget::Quiescent(0),
            },
            refinement: None,
            task_product: None,
        }
    }

    fn task_set() -> TaskSet {
        TaskSet {
            name: "Ctl".to_owned(),
            unit: None,
            tasks: vec![Task {
                name: "t1".to_owned(),
                arrival: Arrival::Periodic {
                    period: Rational::from_integer(Integer::from(4_i64)),
                },
                deadline: Rational::from_integer(Integer::from(4_i64)),
                bcet: Rational::from_integer(Integer::one()),
                wcet: Wcet::Single(Rational::from_integer(Integer::one())),
                priority: Some(1),
                criticality: Criticality::Lo,
                jitter: Rational::from_integer(Integer::zero()),
                blocking: Rational::from_integer(Integer::zero()),
            }],
            span: Span { start: 0, end: 0 },
        }
    }

    fn row() -> TaskQueueBound {
        TaskQueueBound {
            taskset: "Ctl".to_owned(),
            task: "t1".to_owned(),
            arrival: TaskArrivalKind::Periodic,
            interarrival: Rational::from_integer(Integer::from(4_i64)),
            relative_deadline: Rational::from_integer(Integer::from(4_i64)),
            max_jitter: Rational::from_integer(Integer::zero()),
            work_lower: Rational::from_integer(Integer::one()),
            work_upper: Rational::from_integer(Integer::one()),
            max_jobs: 2,
        }
    }

    #[trace("FR-418-AC-14")]
    #[test]
    fn activation_edge_drains_the_bound_pending_release() {
        let release = transition("Call::tick");
        let certificate = certificate(
            pending_release(&release),
            ready_job(&release),
            TaskTransitionTemplate {
                action: TaskAction::Activate,
                taskset: "Ctl".to_owned(),
                task: "t1".to_owned(),
                job_rank: 0,
                slot: 0,
                release: release.clone(),
                source_transition: Some(release),
                priority: Some(1),
            },
        );
        assert!(fixed_work_task_control_edges_match(
            &certificate,
            &[row()],
            &task_set()
        ));
    }

    #[trace("FR-418-AC-14")]
    #[test]
    fn dispatch_edge_cannot_create_a_running_job_without_a_ready_one() {
        let certificate = certificate(
            TaskProductStateKey::empty_scheduler_control(),
            TaskProductStateKey::empty_scheduler_control(),
            TaskTransitionTemplate {
                action: TaskAction::Dispatch,
                taskset: "Ctl".to_owned(),
                task: "t1".to_owned(),
                job_rank: 0,
                slot: 0,
                release: transition("Call::tick"),
                source_transition: None,
                priority: Some(1),
            },
        );
        assert!(!fixed_work_task_control_edges_match(
            &certificate,
            &[row()],
            &task_set()
        ));
    }

    #[trace("FR-418-AC-11")]
    #[test]
    fn completion_subtraction_is_recomputed_for_every_suspended_service_clock() {
        let release = transition("Call::tick");
        let certificate = certificate(
            running_with_suspended_job(&release),
            TaskProductStateKey::empty_scheduler_control(),
            TaskTransitionTemplate {
                action: TaskAction::Complete,
                taskset: "Ctl".to_owned(),
                task: "t1".to_owned(),
                job_rank: 1,
                slot: 1,
                release: release.clone(),
                source_transition: None,
                priority: Some(1),
            },
        );
        assert_eq!(
            fixed_work_subtractions(&certificate, &[row()], &Integer::one()),
            Some(vec![TaskSubtraction {
                transition: TaskTransitionTemplate {
                    action: TaskAction::Complete,
                    taskset: "Ctl".to_owned(),
                    task: "t1".to_owned(),
                    job_rank: 1,
                    slot: 1,
                    release,
                    source_transition: None,
                    priority: Some(1),
                },
                clock: "task:Ctl:t1:slot0:service".to_owned(),
                constant: Integer::one(),
            }])
        );
    }
}
