// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker's zone code against its reference, on seeded random zones:
//! every operation pointwise over a grid, and the aLU test against the
//! simulation's own definition, which the Kani harness's plain-bound
//! transcription does not reach.

use ix_trace_rs::trace;

use super::reference::{self, RawZone, RefBound};
use super::{Integer, Zone, ZoneError};

/// A small seeded generator (xorshift64*), so every run checks the same
/// zones.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn constant(&mut self, max: i64) -> i64 {
        let span = u64::try_from(2 * max + 1).expect("a positive span");
        i64::try_from(self.below(span)).expect("a small value") - max
    }

    fn bound(&mut self, max: i64) -> RefBound {
        match self.below(3) {
            0 => RefBound::Inf,
            1 => RefBound::Lt(self.constant(max)),
            _ => RefBound::Le(self.constant(max)),
        }
    }

    /// A raw zone whose clocks are non-negative.
    fn raw(&mut self, dim: usize, max: i64) -> RawZone {
        let mut bounds: Vec<RefBound> = (0..dim * dim).map(|_| self.bound(max)).collect();
        for bound in bounds.iter_mut().take(dim) {
            if !bound.tighter(RefBound::Le(0)) {
                *bound = RefBound::Le(0);
            }
        }
        RawZone { dim, bounds }
    }
}

/// Every grid point with coordinates in `[0, top]` at scale `s`.
fn grid(dim: usize, top: i64) -> Vec<Vec<i64>> {
    let mut points = vec![vec![0_i64]];
    for _ in 1..dim {
        points = points
            .into_iter()
            .flat_map(|p| {
                (0..=top).map(move |c| {
                    let mut q = p.clone();
                    q.push(c);
                    q
                })
            })
            .collect();
    }
    points
}

/// `close`, `constrain`, `reset`, `up`, `is_empty`, `includes` and the aLU
/// test agree with the reference on 600 seeded zones of dimension at most 3
/// with bounds in `[-4, 4]`, on every point of a `1/3` grid over `[0, 6]`;
/// the aLU test is checked against the simulation's definition.
#[trace("TC-700", "FR-245-AC-4")]
#[test]
fn tc_700_checker_zone_agrees_with_the_reference_on_a_grid() {
    const S: i64 = 3;
    const TOP: i64 = 6 * S;
    let mut rng = Seeded(0x5eed_0700);
    let mut outcomes = [0_u32; 2];
    for _ in 0..600 {
        let dim = usize::try_from(rng.below(3) + 1).expect("a small dimension");
        let points = grid(dim, TOP);
        let raw = rng.raw(dim, 4);
        let mut zone: Zone = raw.to_zone();
        zone.close();
        let in_zone: Vec<bool> = points.iter().map(|p| raw.contains(p, S)).collect();
        for (p, expected) in points.iter().zip(&in_zone) {
            assert_eq!(
                reference::zone_contains(&zone, p, S),
                *expected,
                "close {raw:?} at {p:?}"
            );
        }
        assert_eq!(
            zone.is_empty(),
            !in_zone.iter().any(|&inside| inside),
            "is_empty {raw:?}"
        );

        let (i, j) = (
            rng.below(dim as u64) as usize,
            rng.below(dim as u64) as usize,
        );
        let c = rng.constant(4);
        let is_strict = rng.below(2) == 0;
        let bound = if is_strict {
            RefBound::Lt(c)
        } else {
            RefBound::Le(c)
        };
        let mut constrained = zone.clone();
        constrained
            .constrain(i, j, &Integer::from(c), is_strict)
            .expect("indices in range");
        for (p, inside) in points.iter().zip(&in_zone) {
            assert_eq!(
                reference::zone_contains(&constrained, p, S),
                *inside && bound.holds(p[i] - p[j], S),
                "constrain {raw:?} by x{i} - x{j} {bound:?} at {p:?}"
            );
        }

        let mut elapsed = zone.clone();
        elapsed.up();
        let up = raw.up_constraints();
        for p in &points {
            assert_eq!(
                reference::zone_contains(&elapsed, p, S),
                reference::constraints_hold(up.clone(), p, S),
                "up {raw:?} at {p:?}"
            );
        }

        if dim > 1 {
            let x = 1 + rng.below(dim as u64 - 1) as usize;
            let v = rng.constant(2).abs();
            let mut reset = zone.clone();
            reset.reset(x, &Integer::from(v)).expect("a clock");
            let expected = raw.reset_constraints(x, v);
            for p in &points {
                assert_eq!(
                    reference::zone_contains(&reset, p, S),
                    reference::constraints_hold(expected.clone(), p, S),
                    "reset x{x} := {v} on {raw:?} at {p:?}"
                );
            }
        }

        let other_raw = rng.raw(dim, 4);
        let mut other: Zone = other_raw.to_zone();
        other.close();
        let in_other: Vec<bool> = points.iter().map(|p| other_raw.contains(p, S)).collect();
        let grid_included = in_other.iter().zip(&in_zone).all(|(o, z)| !o || *z);
        assert_eq!(
            zone.includes(&other).expect("equal dimensions"),
            grid_included,
            "{raw:?} includes {other_raw:?}"
        );
        assert_eq!(
            reference::includes(&reference::raw_of(&zone), &reference::raw_of(&other)),
            grid_included,
            "transcribed inclusion of {other_raw:?} in {raw:?}"
        );

        let lower: Vec<Option<i64>> = (1..dim)
            .map(|_| (rng.below(3) > 0).then(|| rng.constant(4)))
            .collect();
        let upper: Vec<Option<i64>> = (1..dim)
            .map(|_| (rng.below(3) > 0).then(|| rng.constant(4)))
            .collect();
        let covered = zone
            .alu_covered_by(&other, &reference::lu_of::<Integer>(&lower, &upper))
            .expect("matching shapes");
        // Every point of the zone is simulated by some point of the target;
        // the target's points range over a finer grid so a strict bound
        // never hides one.
        // Both grids reach past every closed bound (at most 8), so no
        // point that decides the answer lies outside them.
        let zone_points = grid(dim, 10 * S);
        let target_points: Vec<Vec<i64>> = grid(dim, 24 * S)
            .into_iter()
            .filter(|q| other_raw.contains(q, 2 * S))
            .collect();
        let by_definition = zone_points.iter().all(|p| {
            let doubled: Vec<i64> = p.iter().map(|c| 2 * c).collect();
            !raw.contains(p, S)
                || target_points
                    .iter()
                    .any(|q| reference::simulated(&doubled, q, &lower, &upper, 2 * S))
        });
        // The Kani harness's plain-bound transcription agrees too.
        assert_eq!(
            reference::alu_covered(
                &reference::raw_of(&zone),
                &reference::raw_of(&other),
                &lower,
                &upper
            ),
            by_definition,
            "transcribed aLU on {raw:?} and {other_raw:?}"
        );
        outcomes[usize::from(covered)] += 1;
        assert_eq!(
            covered, by_definition,
            "{raw:?} aLU-covered by {other_raw:?} under L {lower:?} U {upper:?}"
        );
    }
    // Both verdicts of the aLU test occur, so neither half is vacuous.
    assert!(
        outcomes.iter().all(|&count| count > 50),
        "aLU outcomes {outcomes:?}"
    );
}

/// Operations naming a clock outside the zone, the reference clock as a
/// reset target or zones of different shapes refuse.
#[trace("TC-700", "FR-245-AC-4")]
#[test]
fn tc_700_checker_zone_refuses_out_of_shape_operations() {
    let mut zone = Zone::zero(2);
    assert_eq!(
        zone.constrain(2, 0, &Integer::one(), false),
        Err(ZoneError::ClockOutOfRange { clock: 2, dim: 2 })
    );
    assert_eq!(
        zone.reset(0, &Integer::one()),
        Err(ZoneError::ResetReferenceClock)
    );
    assert_eq!(
        zone.includes(&Zone::zero(3)),
        Err(ZoneError::DimensionMismatch { left: 2, right: 3 })
    );
    assert_eq!(
        zone.alu_covered_by(&Zone::zero(2), &reference::lu_of::<Integer>(&[], &[])),
        Err(ZoneError::LuLength {
            lower: 0,
            upper: 0,
            clocks: 1
        })
    );
}
