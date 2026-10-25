// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 IV-6, LA-5): EN-1's run limits and the limit a stopped
//! run names.

use qsl_eval::simulation::Limits;
use quire_exact::{LimitKind, ScalarLimits};

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

/// The limit a stopped EN-1 or EN-5 run reached, by the request member that
/// raises it: a member of [`ModelCheckLimits`], or of the subject's FR-120
/// limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelCheckLimit {
    /// `ModelCheckLimits::limits.max_states`.
    MaxStates,
    /// `ModelCheckLimits::limits.max_transitions`.
    MaxTransitions,
    /// `ModelCheckLimits::max_automaton_states`.
    MaxAutomatonStates,
    /// EN-5's product bound `max_states` (FR-203), read from the exact
    /// run's `ModelCheckLimits::limits.max_states`: limit name `max_states`,
    /// setting `exact_prob.max_states`.
    ExactProbMaxStates,
    /// EN-5's product bound `max_transitions`, read from
    /// `ModelCheckLimits::limits.max_transitions`; setting
    /// `exact_prob.max_transitions`.
    ExactProbMaxTransitions,
    /// EN-5's product bound `max_automaton_states`, read from
    /// `ModelCheckLimits::max_automaton_states`; setting
    /// `exact_prob.max_automaton_states`.
    ExactProbMaxAutomatonStates,
    /// The subject's evaluation meter budget.
    EvaluationMeter {
        /// The denied accounting counter.
        counter: LimitKind,
    },
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
    /// The limit's name, snake_case per FR-255-AC-12
    /// (`^[a-z][a-z0-9_]*$`): the FR-331 member name for an EN-1 member,
    /// FR-255's engine-table limit name for an EN-5 product bound.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaxStates => "max_states",
            Self::ExactProbMaxStates => "max_states",
            Self::MaxTransitions => "max_transitions",
            Self::ExactProbMaxTransitions => "max_transitions",
            Self::MaxAutomatonStates => "max_automaton_states",
            Self::ExactProbMaxAutomatonStates => "max_automaton_states",
            Self::EvaluationMeter { counter } => match counter {
                LimitKind::IntegerBits => "integer_bits",
                LimitKind::DecimalDigits => "decimal_digits",
                LimitKind::ScaleExpansion => "scale_expansion",
                LimitKind::TextInputBytes => "text_input_bytes",
                LimitKind::TextScalars => "text_scalars",
                LimitKind::NormalizedScalars => "normalized_scalars",
                LimitKind::UnitEdges => "unit_edges",
                LimitKind::ValueOccurrences => "value_occurrences",
                LimitKind::WorkUnits => "work_units",
                LimitKind::ResultUnits => "result_units",
            },
            Self::MaxCandidates => "max_candidates",
            Self::SolverSteps => "max_solver_steps",
            Self::WitnessSamples => "witness_samples",
            Self::MaxWalkSteps => "max_walk_steps",
            Self::MaxWitnessSet => "max_witness_set",
            Self::MaxRelationTuples => "max_relation_tuples",
            Self::Statistical(limit) => limit.as_str(),
        }
    }

    /// The FR-255 setting name that raises this limit: the `model_check.*`
    /// row of an EN-1 member, `expansion.max_candidates`,
    /// `hybrid.max_solver_steps`, an `accounting.*` row for a meter, an
    /// EN-4 `statistical.*` row, or the `exact_prob.*` row of an EN-5 product
    /// bound.
    pub fn setting(self) -> &'static str {
        match self {
            Self::MaxStates => "model_check.max_states",
            Self::MaxTransitions => "model_check.max_transitions",
            Self::MaxAutomatonStates => "model_check.max_automaton_states",
            Self::ExactProbMaxStates => "exact_prob.max_states",
            Self::ExactProbMaxTransitions => "exact_prob.max_transitions",
            Self::ExactProbMaxAutomatonStates => "exact_prob.max_automaton_states",
            Self::EvaluationMeter { counter } => {
                crate::limits::AccountingLimits::setting_of(counter).name()
            }
            Self::MaxCandidates => "expansion.max_candidates",
            Self::SolverSteps => "hybrid.max_solver_steps",
            Self::WitnessSamples => "model_check.witness_samples",
            Self::MaxWalkSteps => "model_check.max_walk_steps",
            Self::MaxWitnessSet => "model_check.max_witness_set",
            Self::MaxRelationTuples => "model_check.max_relation_tuples",
            Self::Statistical(limit) => limit.setting(),
        }
    }
}

/// The configured bound for one evaluation counter.
pub fn meter_bound(limits: &ScalarLimits, counter: LimitKind) -> u64 {
    match counter {
        LimitKind::IntegerBits => limits.integer_bits,
        LimitKind::DecimalDigits => limits.decimal_digits,
        LimitKind::ScaleExpansion => limits.scale_expansion,
        LimitKind::TextInputBytes => limits.text_input_bytes,
        LimitKind::TextScalars => limits.text_scalars,
        LimitKind::NormalizedScalars => limits.normalized_scalars,
        LimitKind::UnitEdges => limits.unit_edges,
        LimitKind::ValueOccurrences => limits.value_occurrences,
        LimitKind::WorkUnits => limits.work_units,
        LimitKind::ResultUnits => limits.result_units,
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

#[cfg(test)]
mod tests {
    use super::ModelCheckLimit;
    use crate::statistical::StatisticalLimit;
    use ix_trace_rs::trace;
    use quire_exact::LimitKind;

    #[test]
    fn exact_prob_product_bounds_pair_a_snake_case_limit_with_an_exact_prob_setting() {
        let rows = [
            (
                ModelCheckLimit::ExactProbMaxStates,
                "max_states",
                "exact_prob.max_states",
                ModelCheckLimit::MaxStates,
            ),
            (
                ModelCheckLimit::ExactProbMaxTransitions,
                "max_transitions",
                "exact_prob.max_transitions",
                ModelCheckLimit::MaxTransitions,
            ),
            (
                ModelCheckLimit::ExactProbMaxAutomatonStates,
                "max_automaton_states",
                "exact_prob.max_automaton_states",
                ModelCheckLimit::MaxAutomatonStates,
            ),
        ];
        for (exact, name, setting, model) in rows {
            assert_eq!((exact.as_str(), exact.setting()), (name, setting));
            assert_ne!(exact.setting(), model.setting());
        }
    }

    #[trace("TC-521", "FR-126-AC-19")]
    #[test]
    fn evaluation_meter_stop_names_the_accounting_setting() {
        assert_eq!(
            ModelCheckLimit::EvaluationMeter {
                counter: quire_exact::LimitKind::WorkUnits
            }
            .setting(),
            "accounting.work_units"
        );
        assert_eq!(
            ModelCheckLimit::EvaluationMeter {
                counter: quire_exact::LimitKind::IntegerBits
            }
            .as_str(),
            "integer_bits"
        );
        assert_eq!(
            ModelCheckLimit::MaxCandidates.setting(),
            "expansion.max_candidates"
        );
    }

    /// The literal FR-255 (limit name, setting name) row of a limit. The
    /// match is exhaustive with no wildcard arm, so a new variant, meter or
    /// statistical budget fails to compile until it has a row here.
    fn row(limit: ModelCheckLimit) -> (&'static str, &'static str) {
        match limit {
            ModelCheckLimit::MaxStates => ("max_states", "model_check.max_states"),
            ModelCheckLimit::MaxTransitions => ("max_transitions", "model_check.max_transitions"),
            ModelCheckLimit::MaxAutomatonStates => {
                ("max_automaton_states", "model_check.max_automaton_states")
            }
            ModelCheckLimit::ExactProbMaxStates => ("max_states", "exact_prob.max_states"),
            ModelCheckLimit::ExactProbMaxTransitions => {
                ("max_transitions", "exact_prob.max_transitions")
            }
            ModelCheckLimit::ExactProbMaxAutomatonStates => {
                ("max_automaton_states", "exact_prob.max_automaton_states")
            }
            ModelCheckLimit::EvaluationMeter { counter } => match counter {
                LimitKind::IntegerBits => ("integer_bits", "accounting.integer_bits"),
                LimitKind::DecimalDigits => ("decimal_digits", "accounting.decimal_digits"),
                LimitKind::ScaleExpansion => ("scale_expansion", "accounting.scale_expansion"),
                LimitKind::TextInputBytes => ("text_input_bytes", "accounting.text_input_bytes"),
                LimitKind::TextScalars => ("text_scalars", "accounting.text_scalars"),
                LimitKind::NormalizedScalars => {
                    ("normalized_scalars", "accounting.normalized_scalars")
                }
                LimitKind::UnitEdges => ("unit_edges", "accounting.unit_edges"),
                LimitKind::ValueOccurrences => {
                    ("value_occurrences", "accounting.value_occurrences")
                }
                LimitKind::WorkUnits => ("work_units", "accounting.work_units"),
                LimitKind::ResultUnits => ("result_units", "accounting.result_units"),
            },
            ModelCheckLimit::MaxCandidates => ("max_candidates", "expansion.max_candidates"),
            ModelCheckLimit::SolverSteps => ("max_solver_steps", "hybrid.max_solver_steps"),
            ModelCheckLimit::WitnessSamples => ("witness_samples", "model_check.witness_samples"),
            ModelCheckLimit::MaxWalkSteps => ("max_walk_steps", "model_check.max_walk_steps"),
            ModelCheckLimit::MaxWitnessSet => ("max_witness_set", "model_check.max_witness_set"),
            ModelCheckLimit::MaxRelationTuples => {
                ("max_relation_tuples", "model_check.max_relation_tuples")
            }
            ModelCheckLimit::Statistical(budget) => match budget {
                StatisticalLimit::MaxSamples => ("max_samples", "statistical.max_samples"),
                StatisticalLimit::MaxDraws => ("max_draws", "statistical.max_draws"),
                StatisticalLimit::MaxCycleSteps => {
                    ("max_cycle_steps", "statistical.max_cycle_steps")
                }
            },
        }
    }

    /// Every limit, one instance per variant, meter and statistical budget.
    fn every_limit() -> Vec<ModelCheckLimit> {
        let mut all = vec![
            ModelCheckLimit::MaxStates,
            ModelCheckLimit::MaxTransitions,
            ModelCheckLimit::MaxAutomatonStates,
            ModelCheckLimit::ExactProbMaxStates,
            ModelCheckLimit::ExactProbMaxTransitions,
            ModelCheckLimit::ExactProbMaxAutomatonStates,
            ModelCheckLimit::MaxCandidates,
            ModelCheckLimit::SolverSteps,
            ModelCheckLimit::WitnessSamples,
            ModelCheckLimit::MaxWalkSteps,
            ModelCheckLimit::MaxWitnessSet,
            ModelCheckLimit::MaxRelationTuples,
            ModelCheckLimit::Statistical(StatisticalLimit::MaxSamples),
            ModelCheckLimit::Statistical(StatisticalLimit::MaxDraws),
            ModelCheckLimit::Statistical(StatisticalLimit::MaxCycleSteps),
        ];
        all.extend(
            [
                LimitKind::IntegerBits,
                LimitKind::DecimalDigits,
                LimitKind::ScaleExpansion,
                LimitKind::TextInputBytes,
                LimitKind::TextScalars,
                LimitKind::NormalizedScalars,
                LimitKind::UnitEdges,
                LimitKind::ValueOccurrences,
                LimitKind::WorkUnits,
                LimitKind::ResultUnits,
            ]
            .map(|counter| ModelCheckLimit::EvaluationMeter { counter }),
        );
        all
    }

    /// FR-255-AC-12: every limit name matches `^[a-z][a-z0-9_]*$` and each
    /// limit pairs with its engine-table (limit name, setting name) row.
    #[trace("TC-1161", "FR-255-AC-12")]
    #[test]
    fn every_limit_spells_its_name_in_snake_case() {
        for limit in every_limit() {
            let (name, setting) = row(limit);
            assert_eq!((limit.as_str(), limit.setting()), (name, setting));
            assert!(
                name.starts_with(|c: char| c.is_ascii_lowercase())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{name} must match ^[a-z][a-z0-9_]*$"
            );
        }
    }
}
