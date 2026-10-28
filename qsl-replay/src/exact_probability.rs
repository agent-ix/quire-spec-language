// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-203's caller-raisable budgets for EN-5 exact probabilistic analysis.

use crate::ModelCheckLimits;

/// The default maximum interval-iteration sweeps (FR-203).
pub const DEFAULT_EXACT_PROB_MAX_ITERATIONS: u64 = 1_000_000;
/// The default initial dyadic precision in bits (FR-203).
pub const DEFAULT_EXACT_PROB_PRECISION_BITS: u32 = 64;
/// The default maximum dyadic precision in bits (FR-203).
pub const DEFAULT_EXACT_PROB_MAX_PRECISION_BITS: u32 = 4_096;
/// The default maximum bit length of an exact rational numerator or
/// denominator (FR-203).
pub const DEFAULT_EXACT_PROB_MAX_RATIONAL_BITS: u64 = 65_536;
/// The default maximum policy-improvement iterations (FR-203).
pub const DEFAULT_EXACT_PROB_MAX_POLICY_ITERATIONS: u64 = 10_000;
/// The default maximum number of witness paths (FR-203).
pub const DEFAULT_EXACT_PROB_MAX_WITNESS_PATHS: u64 = 1_024;

/// FR-203's finite set of caller-raisable EN-5 limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactProbLimits {
    /// Product exploration limits; its model members use FR-126 defaults.
    pub model: ModelCheckLimits,
    /// Maximum interval-iteration sweeps.
    pub max_iterations: u64,
    /// Starting dyadic precision.
    pub precision_bits: u32,
    /// Maximum dyadic precision.
    pub max_precision_bits: u32,
    /// Maximum numerator or denominator bit length for exact rationals.
    pub max_rational_bits: u64,
    /// Maximum policy-improvement iterations.
    pub max_policy_iterations: u64,
    /// Maximum emitted/replayed paths in a probabilistic witness.
    pub max_witness_paths: u64,
}

impl Default for ExactProbLimits {
    fn default() -> Self {
        Self {
            model: ModelCheckLimits::default(),
            max_iterations: DEFAULT_EXACT_PROB_MAX_ITERATIONS,
            precision_bits: DEFAULT_EXACT_PROB_PRECISION_BITS,
            max_precision_bits: DEFAULT_EXACT_PROB_MAX_PRECISION_BITS,
            max_rational_bits: DEFAULT_EXACT_PROB_MAX_RATIONAL_BITS,
            max_policy_iterations: DEFAULT_EXACT_PROB_MAX_POLICY_ITERATIONS,
            max_witness_paths: DEFAULT_EXACT_PROB_MAX_WITNESS_PATHS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_published_en5_budget_values() {
        let limits = ExactProbLimits::default();
        assert_eq!(limits.max_iterations, 1_000_000);
        assert_eq!(limits.precision_bits, 64);
        assert_eq!(limits.max_precision_bits, 4_096);
        assert_eq!(limits.max_rational_bits, 65_536);
        assert_eq!(limits.max_policy_iterations, 10_000);
        assert_eq!(limits.max_witness_paths, 1_024);
    }
}
