// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-693: the engine's DBM operations, and their agreement with a
//! reference that reads zones as sets of rational grid points.

use ix_trace_rs::trace;
use num_bigint::BigInt;

use super::{Bound, Dbm, DbmError};

fn le(value: i64) -> Bound {
    Bound::le(BigInt::from(value))
}

fn lt(value: i64) -> Bound {
    Bound::lt(BigInt::from(value))
}

const X: usize = 1;
const Y: usize = 2;

/// `0 <= x = y <= 3` over clocks `x, y`.
fn x_equals_y_up_to_three() -> Dbm {
    let mut zone = Dbm::zero(3);
    zone.up();
    zone.constrain(X, 0, le(3)).expect("x is a clock");
    zone
}

/// The closed matrix of `zone`, row by row.
fn rows(zone: &Dbm) -> Vec<Vec<Bound>> {
    (0..zone.dim())
        .map(|i| {
            (0..zone.dim())
                .map(|j| zone.bound(i, j).expect("in range").clone())
                .collect()
        })
        .collect()
}

/// FR-238-AC-2: `zero`, `up` and `constrain(x <= 3)` give `0 <= x = y <=
/// 3`; `reset(x, 0)` gives `x = 0, 0 <= y <= 3`; `y > 3` empties it and
/// `y >= 3` does not.
#[trace("TC-693", "FR-238-AC-2")]
#[test]
fn tc_693_zero_up_constrain_reset_give_the_stated_zones() {
    let zone = x_equals_y_up_to_three();
    assert_eq!(
        rows(&zone),
        [
            [le(0), le(0), le(0)],
            [le(3), le(0), le(0)],
            [le(3), le(0), le(0)],
        ]
    );

    let mut reset = zone.clone();
    reset.reset(X, &BigInt::from(0)).expect("x is a clock");
    assert_eq!(
        rows(&reset),
        [
            [le(0), le(0), le(0)],
            [le(0), le(0), le(0)],
            [le(3), le(3), le(0)],
        ]
    );
    assert!(!reset.is_empty());

    let mut above = reset.clone();
    above.constrain(0, Y, lt(-3)).expect("y is a clock");
    assert!(above.is_empty());

    let mut at = reset.clone();
    at.constrain(0, Y, le(-3)).expect("y is a clock");
    assert!(!at.is_empty());
    assert_eq!(at.bound(Y, 0), Ok(&le(3)));
    assert_eq!(at.bound(0, Y), Ok(&le(-3)));
}

#[trace("TC-685", "FR-230-AC-20")]
#[test]
fn carried_clock_updates_are_simultaneous_and_have_an_exact_preimage() {
    let mut before = Dbm::zero(4);
    before.reset(X, &BigInt::from(2)).expect("x is a clock");
    before.reset(Y, &BigInt::from(7)).expect("y is a clock");
    before.reset(3, &BigInt::from(9)).expect("z is a clock");

    // Slot 0 takes the old slot 1 value while slot 1 is fresh. Both outputs
    // are assigned from the same pre-update valuation.
    let carries = [(X, Y)];
    let resets = [(Y, BigInt::from(5))];
    let mut after = before.clone();
    after
        .assign_clocks(&carries, &resets)
        .expect("valid model clock updates");
    assert_eq!(after.bound(X, 0), Ok(&le(7)));
    assert_eq!(after.bound(0, X), Ok(&le(-7)));
    assert_eq!(after.bound(Y, 0), Ok(&le(5)));
    assert_eq!(after.bound(0, Y), Ok(&le(-5)));
    assert_eq!(after.bound(3, 0), Ok(&le(9)));

    let mut target = Dbm::zero(4);
    target.reset(X, &BigInt::from(7)).expect("x is a clock");
    target.reset(Y, &BigInt::from(5)).expect("y is a clock");
    target.reset(3, &BigInt::from(9)).expect("z is a clock");
    let predecessors = target
        .preimage_clocks(&carries, &resets)
        .expect("valid model clock updates");
    assert!(predecessors.includes(&before).expect("matching dimensions"));
}

#[trace("TC-685", "FR-230-AC-20")]
#[test]
fn a_carried_place_can_copy_one_pre_clock_to_two_post_places() {
    let mut before = Dbm::zero(3);
    before.reset(X, &BigInt::from(4)).expect("x is a clock");
    let mut after = before.clone();
    after
        .assign_clocks(&[(Y, X)], &[])
        .expect("valid model clock update");
    assert_eq!(after.bound(X, Y), Ok(&le(0)));
    assert_eq!(after.bound(Y, X), Ok(&le(0)));
}

/// FR-238-AC-2: `includes` holds for `x <= 3` over `x < 3` and not the
/// reverse; a strict and a non-strict bound stay distinct; out-of-shape
/// operations refuse and leave the DBM unchanged.
#[trace("TC-693", "FR-238-AC-2")]
#[test]
fn tc_693_includes_orders_a_strict_bound_inside_its_non_strict_twin() {
    let mut non_strict = Dbm::zero(2);
    non_strict.up();
    non_strict.constrain(X, 0, le(3)).expect("x is a clock");
    let mut strict = Dbm::zero(2);
    strict.up();
    strict.constrain(X, 0, lt(3)).expect("x is a clock");
    assert_ne!(non_strict, strict);
    assert_eq!(non_strict.includes(&strict), Ok(true));
    assert_eq!(strict.includes(&non_strict), Ok(false));
    assert_eq!(
        strict.includes(&Dbm::zero(3)),
        Err(DbmError::DimensionMismatch { left: 2, right: 3 })
    );
    let before = strict.clone();
    assert_eq!(
        strict.constrain(2, 0, le(1)),
        Err(DbmError::ClockOutOfRange { clock: 2, dim: 2 })
    );
    assert_eq!(strict, before);
    assert_eq!(
        strict.reset(0, &BigInt::from(1)),
        Err(DbmError::ResetReferenceClock)
    );
    assert_eq!(strict, before);
}

/// The reference: a zone as a list of constraints, never closed, read by
/// membership of points `p / s` with `p[0] = 0`. Time elapse and reset are
/// Fourier-Motzkin eliminations of the delay and of the reset clock.
mod reference {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Plain {
        Lt(i64),
        Le(i64),
        Inf,
    }

    impl Plain {
        pub(super) fn plus(self, other: Self) -> Self {
            match (self, other) {
                (Self::Inf, _) | (_, Self::Inf) => Self::Inf,
                (Self::Le(a), Self::Le(b)) => Self::Le(a + b),
                (Self::Lt(a) | Self::Le(a), Self::Lt(b) | Self::Le(b)) => Self::Lt(a + b),
            }
        }

        pub(super) fn holds(self, diff: i64, s: i64) -> bool {
            match self {
                Self::Inf => true,
                Self::Le(c) => diff <= c * s,
                Self::Lt(c) => diff < c * s,
            }
        }

        fn shift(self, amount: i64) -> Self {
            match self {
                Self::Lt(value) => Self::Lt(value + amount),
                Self::Le(value) => Self::Le(value + amount),
                Self::Inf => Self::Inf,
            }
        }
    }

    pub(super) type Constraints = Vec<(usize, usize, Plain)>;

    pub(super) fn holds(constraints: &Constraints, p: &[i64], s: i64) -> bool {
        constraints
            .iter()
            .all(|&(i, j, bound)| bound.holds(p[i] - p[j], s))
    }

    pub(super) fn of_matrix(dim: usize, bounds: &[Plain]) -> Constraints {
        (0..dim)
            .flat_map(|i| (0..dim).map(move |j| (i, j, bounds[i * dim + j])))
            .collect()
    }

    /// `{p | ∃d >= 0. p - d ∈ Z}`.
    pub(super) fn up(dim: usize, bounds: &[Plain]) -> Constraints {
        let at = |i: usize, j: usize| bounds[i * dim + j];
        let mut out = vec![(0, 0, at(0, 0))];
        for i in 1..dim {
            out.push((0, i, at(0, i)));
            for j in 1..dim {
                out.push((i, j, at(i, j)));
                out.push((i, j, at(i, 0).plus(at(0, j))));
            }
        }
        out
    }

    /// `{p | p_x = v ∧ ∃y. p[x := y] ∈ Z}`.
    pub(super) fn reset(dim: usize, bounds: &[Plain], x: usize, v: i64) -> Constraints {
        let at = |i: usize, j: usize| bounds[i * dim + j];
        let mut out = vec![
            (x, x, at(x, x)),
            (x, 0, Plain::Le(v)),
            (0, x, Plain::Le(-v)),
        ];
        for i in (0..dim).filter(|&i| i != x) {
            for j in (0..dim).filter(|&j| j != x) {
                out.push((i, j, at(i, j)));
                out.push((i, j, at(i, x).plus(at(x, j))));
            }
        }
        out
    }

    /// The image of a zone under `x := x - amount`, restricted to the
    /// source valuations satisfying `amount <= x <= ceiling`.
    pub(super) fn subtract_bounded(
        dim: usize,
        bounds: &[Plain],
        x: usize,
        amount: i64,
        ceiling: i64,
    ) -> Constraints {
        let mut out = Vec::new();
        for i in 0..dim {
            for j in 0..dim {
                let original = bounds[i * dim + j];
                let image = match (i == x, j == x) {
                    (true, false) => original.shift(-amount),
                    (false, true) => original.shift(amount),
                    _ => original,
                };
                out.push((i, j, image));
            }
        }
        // The guard is stated in the source valuation. Substituting
        // `source_x = image_x + amount` yields these image bounds.
        out.push((0, x, Plain::Le(0)));
        out.push((x, 0, Plain::Le(ceiling - amount)));
        out
    }
}

use reference::{Constraints, Plain};

fn to_bound(plain: Plain) -> Bound {
    match plain {
        Plain::Lt(c) => lt(c),
        Plain::Le(c) => le(c),
        Plain::Inf => Bound::Infinite,
    }
}

/// Whether the point `p / s` lies in the engine's matrix.
fn engine_holds(zone: &Dbm, p: &[i64], s: i64) -> bool {
    let s = BigInt::from(s);
    (0..zone.dim()).all(|i| {
        (0..zone.dim()).all(|j| {
            let diff = BigInt::from(p[i] - p[j]);
            match zone.at(i, j) {
                Bound::Infinite => true,
                Bound::Finite {
                    value,
                    strict: true,
                } => diff < value * &s,
                Bound::Finite {
                    value,
                    strict: false,
                } => diff <= value * &s,
            }
        })
    })
}

/// A seeded generator (xorshift64*), so every run checks the same DBMs.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("small")).expect("small")
    }

    fn constant(&mut self) -> i64 {
        i64::try_from(self.below(41)).expect("small") - 20
    }

    fn bound(&mut self) -> Plain {
        match self.below(3) {
            0 => Plain::Inf,
            1 => Plain::Lt(self.constant()),
            _ => Plain::Le(self.constant()),
        }
    }
}

/// Every grid point `p / s` with `p` in `corner + [0, span]` per clock.
fn window(corner: &[i64], span: i64) -> Vec<Vec<i64>> {
    let mut points = vec![vec![0_i64]];
    for &low in corner.iter().skip(1) {
        points = points
            .into_iter()
            .flat_map(|p| {
                (low..=low + span).map(move |c| {
                    let mut q = p.clone();
                    q.push(c);
                    q
                })
            })
            .collect();
    }
    points
}

/// The least point of a closed non-empty zone at scale `s`: each clock at
/// its lower bound.
fn least_corner(zone: &Dbm, s: i64) -> Vec<i64> {
    (0..zone.dim())
        .map(|k| match zone.at(0, k) {
            Bound::Finite { value, .. } => -i64::try_from(value).expect("a small bound") * s,
            Bound::Infinite => 0,
        })
        .collect()
}

/// FR-238-AC-3: on 10,000 seeded DBMs of dimension at most 5 with bounds in
/// `[-20, 20]`, `close`, `constrain`, `reset`, `up`, `is_empty` and
/// `includes` agree with the reference on every point of a `1/dim` grid in
/// one-unit windows at the least corner of each zone involved and at a
/// seeded anchor. The least-corner window of a non-empty zone always holds
/// one of its points, so emptiness is checked in both directions.
#[trace("TC-693", "FR-238-AC-3")]
#[test]
fn tc_693_operations_agree_with_the_grid_reference() {
    let mut rng = Seeded(0x0693_5eed);
    let mut non_empty = 0_u32;
    let mut included = 0_u32;
    for _ in 0..10_000 {
        let dim = 1 + rng.below(5);
        let s = i64::try_from(dim).expect("small");
        let raw_of = |rng: &mut Seeded| {
            let mut bounds: Vec<Plain> = (0..dim * dim).map(|_| rng.bound()).collect();
            // Clocks are non-negative.
            for bound in bounds.iter_mut().take(dim) {
                if !matches!(bound, Plain::Le(c) | Plain::Lt(c) if *c < 0) {
                    *bound = Plain::Le(0);
                }
            }
            bounds
        };
        let raw = raw_of(&mut rng);
        let matrix = reference::of_matrix(dim, &raw);
        let mut zone = Dbm {
            dim,
            bounds: raw.iter().copied().map(to_bound).collect(),
        };
        zone.close();

        let anchor: Vec<i64> = (0..dim)
            .map(|k| {
                if k == 0 {
                    0
                } else {
                    i64::try_from(rng.below(21)).expect("small") * s
                }
            })
            .collect();
        let mut points = window(&anchor, s);
        if !zone.is_empty() {
            points.extend(window(&least_corner(&zone, s), s));
        }
        let agree = |result: &Dbm, expected: &Constraints, points: &[Vec<i64>], what: &str| {
            for p in points {
                assert_eq!(
                    engine_holds(result, p, s),
                    reference::holds(expected, p, s),
                    "{what} of {raw:?} at {p:?}"
                );
            }
        };

        // close: the same points, at a fixpoint, with no shorter path.
        agree(&zone, &matrix, &points, "close");
        let mut again = zone.clone();
        again.close();
        assert_eq!(again, zone, "close is idempotent on {raw:?}");
        let found = points.iter().any(|p| reference::holds(&matrix, p, s));
        assert_eq!(zone.is_empty(), !found, "is_empty of {raw:?}");
        if zone.is_empty() {
            continue;
        }
        non_empty += 1;

        // constrain
        let (i, j) = (rng.below(dim), rng.below(dim));
        let bound = match rng.bound() {
            Plain::Inf => Plain::Le(rng.constant()),
            finite => finite,
        };
        let mut constrained = zone.clone();
        constrained
            .constrain(i, j, to_bound(bound))
            .expect("in range");
        let mut expected = matrix.clone();
        expected.push((i, j, bound));
        let mut at = points.clone();
        if !constrained.is_empty() {
            at.extend(window(&least_corner(&constrained, s), s));
        }
        agree(&constrained, &expected, &at, "constrain");
        assert_eq!(
            constrained.is_empty(),
            !at.iter().any(|p| reference::holds(&expected, p, s)),
            "is_empty after constrain of {raw:?}"
        );
        let mut closed = constrained.clone();
        closed.close();
        assert_eq!(closed, constrained, "constrain leaves {raw:?} canonical");

        // up
        let mut elapsed = zone.clone();
        elapsed.up();
        let mut at = points.clone();
        at.extend(window(&least_corner(&elapsed, s), s));
        agree(&elapsed, &reference::up(dim, &raw), &at, "up");
        let mut closed = elapsed.clone();
        closed.close();
        assert_eq!(closed, elapsed, "up leaves {raw:?} canonical");

        // reset
        if dim > 1 {
            let x = 1 + rng.below(dim - 1);
            let v = i64::try_from(rng.below(21)).expect("small");
            let mut reset = zone.clone();
            reset.reset(x, &BigInt::from(v)).expect("a clock");
            let mut at = points.clone();
            at.extend(window(&least_corner(&reset, s), s));
            agree(&reset, &reference::reset(dim, &raw, x, v), &at, "reset");
            let mut closed = reset.clone();
            closed.close();
            assert_eq!(closed, reset, "reset leaves {raw:?} canonical");
        }

        // includes: against the constrained zone (a subset) and a fresh one.
        assert_eq!(
            zone.includes(&constrained),
            Ok(true),
            "{raw:?} includes its constraint"
        );
        let other_raw = raw_of(&mut rng);
        let other_matrix = reference::of_matrix(dim, &other_raw);
        let mut other = Dbm {
            dim,
            bounds: other_raw.iter().copied().map(to_bound).collect(),
        };
        other.close();
        let mut at = points.clone();
        if !other.is_empty() {
            at.extend(window(&least_corner(&other, s), s));
        }
        let verdict = zone.includes(&other).expect("equal dimensions");
        let outside = at
            .iter()
            .any(|p| reference::holds(&other_matrix, p, s) && !reference::holds(&matrix, p, s));
        if verdict {
            included += 1;
            assert!(
                !outside,
                "{raw:?} includes {other_raw:?} yet a point of it is outside"
            );
        } else {
            // A refusal must be shown by a point of `other` outside the
            // zone. It lies past an entry where `other` is looser, so look
            // at the least corner of `other` beyond that entry's bound.
            let shown = (0..dim)
                .flat_map(|a| (0..dim).map(move |b| (a, b)))
                .filter_map(|(a, b)| match zone.at(a, b) {
                    Bound::Finite { value, strict } if other.at(a, b) > zone.at(a, b) => {
                        let mut beyond = other.clone();
                        let negated = if *strict {
                            Bound::le(-value)
                        } else {
                            Bound::lt(-value)
                        };
                        beyond.constrain(b, a, negated).expect("in range");
                        (!beyond.is_empty()).then(|| window(&least_corner(&beyond, s), s))
                    }
                    _ => None,
                })
                .flatten()
                .any(|p| {
                    reference::holds(&other_matrix, &p, s) && !reference::holds(&matrix, &p, s)
                });
            assert!(
                shown,
                "{raw:?} excludes {other_raw:?} with no point to show it"
            );
        }
    }
    // The generator reaches both outcomes often enough to matter.
    assert!(non_empty > 2_000, "{non_empty} non-empty zones");
    assert!(included > 100, "{included} inclusions");
}

/// FR-238-AC-4 and QSpec FR-419-AC-6: bounded subtraction applies the exact
/// row/column diagonal update only within its lower and upper guard. The
/// result agrees with an independent constraint-set reference and remains
/// canonical; invalid inputs leave it unchanged.
#[trace("TC-693", "FR-238-AC-4", "QSpec-FR-419-AC-6")]
#[test]
fn bounded_subtraction_matches_reference_with_diagonal_constraints() {
    // Values are scaled by six, so amount=5 represents 5/6 and ceiling=24
    // represents 4. The source zone has x in [0, 5], y in [0, 3], and two
    // diagonal bounds. The update clips x to [5, 24], then subtracts 5.
    let dim = 3;
    let mut raw = vec![Plain::Inf; dim * dim];
    for clock in 0..dim {
        raw[clock * dim + clock] = Plain::Le(0);
        raw[clock] = Plain::Le(0);
    }
    raw[X * dim] = Plain::Le(30);
    raw[Y * dim] = Plain::Le(18);
    raw[X * dim + Y] = Plain::Le(6);
    raw[Y * dim + X] = Plain::Le(12);

    let mut result = Dbm {
        dim,
        bounds: raw.iter().copied().map(to_bound).collect(),
    };
    result.close();
    let expected = reference::subtract_bounded(dim, &raw, X, 5, 24);
    result
        .subtract_bounded(X, &BigInt::from(5), &BigInt::from(24))
        .expect("a valid task-product update");

    assert_eq!(result.bound(X, 0), Ok(&le(19)));
    assert_eq!(result.bound(0, X), Ok(&le(0)));
    assert_eq!(result.bound(X, Y), Ok(&le(1)));
    assert_eq!(result.bound(Y, X), Ok(&le(17)));
    let scale = 6;
    let anchor = [0, 0, 0];
    // All source clocks are bounded to [0, 5] in this fixture; checking a
    // 1/6-grid over that full rectangle distinguishes strict and non-strict
    // boundaries as well as every integral diagonal bound.
    let mut points = window(&anchor, 30 * scale);
    points.extend(window(&least_corner(&result, scale), 30 * scale));
    for point in &points {
        assert_eq!(
            engine_holds(&result, point, scale),
            reference::holds(&expected, point, scale),
            "bounded subtraction at {point:?}"
        );
    }
    let mut closed = result.clone();
    closed.close();
    assert_eq!(closed, result, "bounded subtraction leaves a canonical DBM");

    // A zero subtraction still enforces its declared ceiling.
    let mut zero_amount = Dbm {
        dim,
        bounds: raw.iter().copied().map(to_bound).collect(),
    };
    zero_amount.close();
    zero_amount
        .subtract_bounded(X, &BigInt::from(0), &BigInt::from(12))
        .expect("zero is a valid non-negative subtraction");
    assert_eq!(zero_amount.bound(X, 0), Ok(&le(12)));

    let mut disabled = Dbm::zero(2);
    disabled.up();
    disabled.constrain(X, 0, le(1)).expect("x is a clock");
    disabled
        .subtract_bounded(X, &BigInt::from(2), &BigInt::from(4))
        .expect("the product update has a valid range");
    assert!(disabled.is_empty(), "no valuation satisfies x >= amount");
}

#[trace("TC-693", "FR-238-AC-4")]
#[test]
fn bounded_subtraction_refuses_malformed_ranges_and_clock_indices_transactionally() {
    let mut zone = x_equals_y_up_to_three();
    let before = zone.clone();
    assert_eq!(
        zone.subtract_bounded(3, &BigInt::from(1), &BigInt::from(3)),
        Err(DbmError::ClockOutOfRange { clock: 3, dim: 3 })
    );
    assert_eq!(zone, before);
    assert_eq!(
        zone.subtract_bounded(0, &BigInt::from(1), &BigInt::from(3)),
        Err(DbmError::SubtractReferenceClock)
    );
    assert_eq!(zone, before);
    for (amount, ceiling) in [(-1, 3), (4, 3), (1, 0)] {
        assert_eq!(
            zone.subtract_bounded(X, &BigInt::from(amount), &BigInt::from(ceiling)),
            Err(DbmError::InvalidSubtractionRange {
                amount: BigInt::from(amount),
                ceiling: BigInt::from(ceiling),
            })
        );
        assert_eq!(zone, before);
    }
}

/// FR-239 "Abstraction": `x = 7` is covered by `x = 6` under the aLU
/// simulation when `x` is compared with nothing above 5, and not when `U`
/// is 10; the empty zone is covered by any zone, and a zone of another
/// dimension is refused.
#[trace("TC-693")]
#[test]
fn alu_coverage_forgets_values_above_the_bounds() {
    let at = |value: i64| {
        let mut zone = Dbm::zero(2);
        zone.up();
        zone.constrain(X, 0, le(value)).expect("x is a clock");
        zone.constrain(0, X, le(-value)).expect("x is a clock");
        zone
    };
    let five = [Some(BigInt::from(5))];
    let ten = [Some(BigInt::from(10))];
    assert!(at(7)
        .alu_covered_by(&at(6), &five, &five)
        .expect("one shape"));
    assert!(!at(7).alu_covered_by(&at(6), &ten, &ten).expect("one shape"));
    assert!(at(6).alu_covered_by(&at(6), &ten, &ten).expect("one shape"));
    let mut empty = at(1);
    empty.constrain(X, 0, lt(1)).expect("x is a clock");
    assert!(empty
        .alu_covered_by(&at(6), &five, &five)
        .expect("one shape"));
    assert!(matches!(
        at(1).alu_covered_by(&Dbm::zero(3), &five, &five),
        Err(DbmError::DimensionMismatch { .. })
    ));
}

/// `down` of `1 <= x <= 2` is `x <= 2`; `universe` holds every valuation
/// and is canonical; `scale_by(2)` doubles every finite bound.
#[trace("TC-693")]
#[test]
fn down_universe_and_scaling() {
    let mut zone = Dbm::zero(2);
    zone.up();
    zone.constrain(X, 0, le(2)).expect("x is a clock");
    zone.constrain(0, X, le(-1)).expect("x is a clock");
    zone.down();
    assert_eq!(zone.bound(0, X).expect("in range"), &le(0));
    assert_eq!(zone.bound(X, 0).expect("in range"), &le(2));

    let universe = Dbm::universe(3);
    let mut closed = universe.clone();
    closed.close();
    assert_eq!(closed, universe);
    assert!(universe
        .includes(&x_equals_y_up_to_three())
        .expect("one shape"));

    let mut scaled = x_equals_y_up_to_three();
    scaled.scale_by(&BigInt::from(2));
    assert_eq!(scaled.bound(X, 0).expect("in range"), &le(6));
    assert_eq!(scaled.bound(X, Y).expect("in range"), &le(0));
}

/// QSpec FR-430: a renaming swaps clocks' rows and columns and keeps the
/// zone closed; one that maps two clocks to one is refused, so no row is
/// silently overwritten.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn a_renaming_must_be_a_permutation() {
    let mut zone = Dbm::zero(3);
    zone.up();
    zone.constrain(0, Y, le(-2)).expect("y is a clock");
    let swapped = zone.renamed(&[1, 0]).expect("a permutation");
    assert_eq!(swapped.bound(0, X).expect("x"), &le(-2));
    assert_eq!(swapped.renamed(&[1, 0]).expect("a permutation"), zone);
    assert_eq!(
        zone.renamed(&[0, 0]),
        Err(DbmError::NotAPermutation {
            first: 1,
            second: 2
        })
    );
    assert_eq!(
        zone.renamed(&[2, 0]),
        Err(DbmError::ClockOutOfRange { clock: 3, dim: 3 })
    );
}
