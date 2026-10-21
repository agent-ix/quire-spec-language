// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 IV-6, LA-5): EN-1's run limits and the limit a stopped
//! run names.

use qsl_eval::simulation::Limits;

use crate::statistical::StatisticalLimit;

/// FR-126's published default for `max_automaton_states`: 2^20.
pub const DEFAULT_MAX_AUTOMATON_STATES: u64 = 1_048_576;

/// FR-184's published default for `max_witness_set`: 2^16 members of one
/// HP-3 witness set.
pub const DEFAULT_MAX_WITNESS_SET: u64 = 65_536;

/// FR-184's published default for `max_relation_tuples`: 2^24 tuples of one
/// HP-1 run.
pub const DEFAULT_MAX_RELATION_TUPLES: u64 = 16_777_216;

/// FR-166's published default for `witness_samples`: 64 phase 0 walks per
/// initial state (QSpec FR-392).
pub const DEFAULT_WITNESS_SAMPLES: u64 = 64;

/// FR-166's published default for `max_walk_steps`: 4,096 steps per phase 0
/// walk (ADR-022 GE-2).
pub const DEFAULT_MAX_WALK_STEPS: u64 = 4_096;

/// FR-166's published default seed, which a request that names none runs
/// under (QSpec FR-392, ADR-022 QS-7).
pub const DEFAULT_WITNESS_SEED: u64 = 0;

/// The caller-raisable ADR-014 B-5 budgets of one EN-1 run. The search
/// horizon `max_depth` is a request member beside these, never one of them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelCheckLimits {
    /// FR-101's `max_states` and `max_transitions`.
    pub limits: Limits,
    /// The most distinct automaton states the run materializes.
    pub max_automaton_states: u64,
    /// The phase 0 walks drawn per initial state for a `possible` item
    /// (FR-166); 0 turns phase 0 off.
    pub witness_samples: u64,
    /// The most steps one phase 0 walk takes (FR-166).
    pub max_walk_steps: u64,
    /// The most members one HP-3 witness set may reach (FR-184).
    pub max_witness_set: u64,
    /// The most tuples one HP-1 run may evaluate (FR-184).
    pub max_relation_tuples: u64,
}

impl Default for ModelCheckLimits {
    /// FR-101's `Limits::default()`, `max_automaton_states` 1,048,576,
    /// `witness_samples` 64, `max_walk_steps` 4,096, `max_witness_set` 65,536
    /// and `max_relation_tuples` 16,777,216.
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            max_automaton_states: DEFAULT_MAX_AUTOMATON_STATES,
            witness_samples: DEFAULT_WITNESS_SAMPLES,
            max_walk_steps: DEFAULT_MAX_WALK_STEPS,
            max_witness_set: DEFAULT_MAX_WITNESS_SET,
            max_relation_tuples: DEFAULT_MAX_RELATION_TUPLES,
        }
    }
}

impl ModelCheckLimits {
    /// Each limit of [`ModelCheckLimits`] with the value the run used, in
    /// member order: what every terminal record of an EN-1 item states
    /// (FR-127).
    pub fn used(&self) -> [(ModelCheckLimit, u64); 3] {
        let widen = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
        [
            (ModelCheckLimit::MaxStates, widen(self.limits.max_states)),
            (
                ModelCheckLimit::MaxTransitions,
                widen(self.limits.max_transitions),
            ),
            (
                ModelCheckLimit::MaxAutomatonStates,
                self.max_automaton_states,
            ),
        ]
    }
}

/// The limit a stopped EN-1 run reached, by the request member that raises
/// it: a member of [`ModelCheckLimits`], or of the subject's FR-120 limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelCheckLimit {
    /// `ModelCheckLimits::limits.max_states`.
    MaxStates,
    /// `ModelCheckLimits::limits.max_transitions`.
    MaxTransitions,
    /// `ModelCheckLimits::max_automaton_states`.
    MaxAutomatonStates,
    /// The subject's evaluation meter budget.
    EvaluationMeter,
    /// The subject's `ExpansionLimits::max_candidates`.
    MaxCandidates,
    /// FR-250: a hybrid solver's step budget.
    SolverSteps,
    /// `ModelCheckLimits::witness_samples` (FR-166).
    WitnessSamples,
    /// `ModelCheckLimits::max_walk_steps` (FR-166).
    MaxWalkSteps,
    /// `ModelCheckLimits::max_witness_set` (FR-184).
    MaxWitnessSet,
    /// `ModelCheckLimits::max_relation_tuples` (FR-184).
    MaxRelationTuples,
    /// An EN-4 budget (FR-191, FR-192).
    Statistical(StatisticalLimit),
}

impl ModelCheckLimit {
    /// The limit's FR-331 spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxStates => "max-states",
            Self::MaxTransitions => "max-transitions",
            Self::MaxAutomatonStates => "max-automaton-states",
            Self::EvaluationMeter => "evaluation-meter",
            Self::MaxCandidates => "max-candidates",
            Self::SolverSteps => "solver-steps",
            Self::WitnessSamples => "witness-samples",
            Self::MaxWalkSteps => "max-walk-steps",
            Self::MaxWitnessSet => "max-witness-set",
            Self::MaxRelationTuples => "max-relation-tuples",
            Self::Statistical(limit) => limit.as_str(),
        }
    }

    /// The request member that raises this limit, as the request spells it.
    pub const fn setting(self) -> &'static str {
        match self {
            Self::MaxStates => "max_states",
            Self::MaxTransitions => "max_transitions",
            Self::MaxAutomatonStates => "max_automaton_states",
            Self::EvaluationMeter => "meter",
            Self::MaxCandidates => "max_candidates",
            Self::SolverSteps => "max_solver_steps",
            Self::WitnessSamples => "witness_samples",
            Self::MaxWalkSteps => "max_walk_steps",
            Self::MaxWitnessSet => "max_witness_set",
            Self::MaxRelationTuples => "max_relation_tuples",
            Self::Statistical(limit) => limit.setting(),
        }
    }
}

/// A limit a run reached and the value it was set to.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReachedLimit {
    /// The limit.
    pub limit: ModelCheckLimit,
    /// Its value in the run.
    pub value: u64,
}
