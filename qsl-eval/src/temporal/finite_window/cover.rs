// SPDX-License-Identifier: AGPL-3.0-or-later
//! The clock binding's cover of a stamp and the comparisons it supports
//! (FR-501 "Counter cover"; QSpec FR-160 "Counter-tick intervals and
//! endpoints"; ADR-026 MN-2).
//!
//! Under [`ClockCover::Exact`] a stamp is the instant itself and a difference
//! of two stamps is an integer. Under [`ClockCover::Counter`] a counter
//! reading `k` covers the half-open interval `[k, k+1)` of the counter unit,
//! widened by the uncertainty `u` to `[k-u, k+1+u)`, so the difference of the
//! covers of two different readings is an open interval whose endpoints are
//! integers, and a comparison against a bound is decided only when every pair
//! of instants in the two covers agrees.

/// What a clock binding declares a stamp to cover (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClockCover {
    /// A stamp is the instant itself (the default).
    Exact,
    /// A counter reading `k` covers `[k, k+1)` widened by `uncertainty` ticks
    /// on each side.
    Counter {
        /// The widening `u`, in ticks.
        uncertainty: u64,
    },
}

/// The answer of a comparison over covers: every pair of instants agrees, or
/// the covers straddle the bound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Containment {
    /// Every pair of instants satisfies the comparison.
    True,
    /// No pair of instants satisfies it.
    False,
    /// Some pairs do and some do not: the covers straddle the bound.
    Indeterminate,
}

impl Containment {
    /// The conjunction of two comparisons: `False` when either is, `True`
    /// only when both are.
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            (Self::True | Self::Indeterminate, Self::Indeterminate)
            | (Self::Indeterminate, Self::True) => Self::Indeterminate,
        }
    }
}

/// A difference of two covers: `lo < d < hi` for an interval whose ends are
/// both open (a counter cover) or `d == lo == hi` for an exact difference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Difference {
    lo: i128,
    hi: i128,
    /// Whether the interval excludes its ends (a counter cover) or is the
    /// single point `lo == hi` (an exact difference).
    open: bool,
}

impl Difference {
    /// `d >= bound` for every `d`, for no `d`, or for some.
    fn at_least(self, bound: i128) -> Containment {
        if self.lo >= bound {
            Containment::True
        } else if self.hi < bound || (self.hi == bound && self.open) {
            Containment::False
        } else {
            Containment::Indeterminate
        }
    }

    /// `d <= bound` for every `d`, for no `d`, or for some.
    fn at_most(self, bound: i128) -> Containment {
        if self.hi <= bound {
            Containment::True
        } else if self.lo > bound || (self.lo == bound && self.open) {
            Containment::False
        } else {
            Containment::Indeterminate
        }
    }
}

impl ClockCover {
    /// The margin `m` an establishment adds to a clause's future reach:
    /// `2u` under a counter cover, 0 under an exact one (FR-501 "Established
    /// window").
    pub fn margin(self) -> i128 {
        match self {
            Self::Exact => 0,
            Self::Counter { uncertainty } => 2 * i128::from(uncertainty),
        }
    }

    /// The difference `later - earlier` of the covers of two readings.
    fn difference(self, earlier: i128, later: i128) -> Difference {
        match self {
            Self::Exact => Difference {
                lo: later - earlier,
                hi: later - earlier,
                open: false,
            },
            Self::Counter { uncertainty } => {
                let spread = 1 + 2 * i128::from(uncertainty);
                Difference {
                    lo: later - earlier - spread,
                    hi: later - earlier + spread,
                    open: true,
                }
            }
        }
    }

    /// QSpec FR-160's `within_bound`: whether `ack - request <= bound` holds
    /// for every pair of instants in the covers of the readings `request_tick`
    /// and `ack_tick`, for none, or for some.
    pub fn within_bound(self, bound: i64, request_tick: u64, ack_tick: u64) -> Containment {
        self.difference(i128::from(request_tick), i128::from(ack_tick))
            .at_most(i128::from(bound))
    }

    /// QSpec FR-160's `definitely_before`: whether every instant in the cover
    /// of the reading `a` lies below every instant in the cover of `b`.
    pub fn definitely_before(self, a: u64, b: u64) -> bool {
        match self {
            Self::Exact => a < b,
            Self::Counter { uncertainty } => {
                i128::from(a) + 1 + i128::from(uncertainty)
                    <= i128::from(b) - i128::from(uncertainty)
            }
        }
    }

    /// Whether the reading at `candidate` lies at distance `[lower, upper]`
    /// from the reading at `anchor`, forward when `forward` and backward
    /// otherwise: for every pair of instants, for none, or for some. A
    /// reading's distance to itself is exactly 0 (the same instant).
    pub(super) fn eligible(
        self,
        anchor: u64,
        candidate: u64,
        same_position: bool,
        forward: bool,
        lower: u64,
        upper: u64,
    ) -> Containment {
        let (earlier, later) = if forward {
            (anchor, candidate)
        } else {
            (candidate, anchor)
        };
        let difference = if same_position {
            Difference {
                lo: 0,
                hi: 0,
                open: false,
            }
        } else {
            self.difference(i128::from(earlier), i128::from(later))
        };
        difference
            .at_least(i128::from(lower))
            .and(difference.at_most(i128::from(upper)))
    }
}
