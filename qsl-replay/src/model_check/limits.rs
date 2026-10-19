// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 IV-6, LA-5): EN-1's run limits and the limit a stopped
//! run names.

use qsl_eval::simulation::Limits;

/// FR-126's published default for `max_automaton_states`: 2^20.
pub const DEFAULT_MAX_AUTOMATON_STATES: u64 = 1_048_576;

/// The caller-raisable ADR-014 B-5 budgets of one EN-1 run. The search
/// horizon `max_depth` is a request member beside these, never one of them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelCheckLimits {
    /// FR-101's `max_states` and `max_transitions`.
    pub limits: Limits,
    /// The most distinct automaton states the run materializes.
    pub max_automaton_states: u64,
}

impl Default for ModelCheckLimits {
    /// FR-101's `Limits::default()` and `max_automaton_states` 1,048,576.
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            max_automaton_states: DEFAULT_MAX_AUTOMATON_STATES,
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
