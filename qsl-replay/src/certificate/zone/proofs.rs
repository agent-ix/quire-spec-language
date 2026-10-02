// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kani harnesses for the checker's zone code (ADR-026 CF-6, FR-245).
//!
//! Each harness takes a zone of dimension at most 3 with every bound
//! symbolic in `[-8, 8]` or `∞`, applies one operation, and proves the
//! operation free of overflow and panics and in agreement with the
//! [`reference`](super::reference): pointwise for `close`, `constrain`,
//! `reset` and `up`, where a symbolic point is in the result exactly when
//! it is in the reference set; entry by entry for `includes` and the aLU
//! test, against their plain-bound transcriptions.
//!
//! The harnesses run the checker's matrix code over `i64`, where Kani checks
//! every addition and subtraction for overflow; the checker runs the same
//! code over `quire_exact::Integer`. Bounds come from the harness inputs
//! only; the loop unwinding is a Kani setting, not a limit of the checker.

use super::reference::{self, RawZone, RefBound};
use super::Zone;

/// Largest dimension the harnesses cover: two clocks and the reference.
const MAX_DIM: usize = 3;
/// Bound magnitudes the harnesses cover.
const MAX_BOUND: i64 = 8;
/// The scale symbolic points are read at: a grid of `1 / SCALE`.
const SCALE: i64 = 6;
/// Largest scaled point coordinate magnitude: a closed bound is a sum of
/// at most `MAX_DIM - 1` input bounds, and `up` and `reset` add two.
const MAX_COORD: i64 = 4 * MAX_BOUND * SCALE;

fn any_dim() -> usize {
    kani::any_where(|d: &usize| (1..=MAX_DIM).contains(d))
}

fn any_constant() -> i64 {
    kani::any_where(|c: &i64| (-MAX_BOUND..=MAX_BOUND).contains(c))
}

fn any_bound() -> RefBound {
    match kani::any::<u8>() % 3 {
        0 => RefBound::Inf,
        1 => RefBound::Lt(any_constant()),
        _ => RefBound::Le(any_constant()),
    }
}

fn any_raw(dim: usize) -> RawZone {
    let mut bounds = Vec::with_capacity(dim * dim);
    for _ in 0..dim * dim {
        bounds.push(any_bound());
    }
    RawZone { dim, bounds }
}

/// A symbolic point with the reference clock at 0.
fn any_point(dim: usize) -> Vec<i64> {
    let mut point = vec![0_i64; dim];
    for coordinate in point.iter_mut().skip(1) {
        *coordinate = kani::any_where(|c: &i64| (-MAX_COORD..=MAX_COORD).contains(c));
    }
    point
}

/// A symbolic zone, closed by the checker.
fn any_closed(dim: usize) -> (RawZone, Zone<i64>) {
    let raw = any_raw(dim);
    let mut zone = raw.to_zone();
    zone.close();
    (raw, zone)
}

/// `close` keeps exactly the raw zone's points and reaches a fixpoint.
#[kani::proof]
#[kani::unwind(12)]
fn close_agrees_with_the_reference() {
    let dim = any_dim();
    let (raw, zone) = any_closed(dim);
    let point = any_point(dim);
    assert_eq!(
        reference::zone_contains(&zone, &point, SCALE),
        raw.contains(&point, SCALE)
    );
    let mut again = zone.clone();
    again.close();
    assert_eq!(again, zone);
}

/// `constrain` keeps exactly the points of the zone that satisfy the
/// constraint, and leaves the zone canonical.
#[kani::proof]
#[kani::unwind(12)]
fn constrain_agrees_with_the_reference() {
    let dim = any_dim();
    let (raw, mut zone) = any_closed(dim);
    let i = kani::any_where(|i: &usize| *i < dim);
    let j = kani::any_where(|j: &usize| *j < dim);
    let constant = any_constant();
    let is_strict: bool = kani::any();
    let point = any_point(dim);
    let bound = if is_strict {
        RefBound::Lt(constant)
    } else {
        RefBound::Le(constant)
    };
    zone.constrain(i, j, &constant, is_strict)
        .expect("indices are in range");
    assert_eq!(
        reference::zone_contains(&zone, &point, SCALE),
        raw.contains(&point, SCALE) && bound.holds(point[i] - point[j], SCALE)
    );
    let mut closed = zone.clone();
    closed.close();
    assert_eq!(closed, zone);
}

/// `reset` gives exactly the points of the eliminated reference, and
/// leaves the zone canonical.
#[kani::proof]
#[kani::unwind(12)]
fn reset_agrees_with_the_reference() {
    let dim = kani::any_where(|d: &usize| (2..=MAX_DIM).contains(d));
    let (raw, mut zone) = any_closed(dim);
    let clock = kani::any_where(|x: &usize| (1..dim).contains(x));
    let value = kani::any_where(|v: &i64| (0..=MAX_BOUND).contains(v));
    let point = any_point(dim);
    zone.reset(clock, &value)
        .expect("a clock other than the reference");
    let expected = reference::constraints_hold(raw.reset_constraints(clock, value), &point, SCALE);
    assert_eq!(reference::zone_contains(&zone, &point, SCALE), expected);
    let mut closed = zone.clone();
    closed.close();
    assert_eq!(closed, zone);
}

/// `up` gives exactly the points of the eliminated reference, and leaves
/// the zone canonical.
#[kani::proof]
#[kani::unwind(12)]
fn up_agrees_with_the_reference() {
    let dim = any_dim();
    let (raw, mut zone) = any_closed(dim);
    let point = any_point(dim);
    zone.up();
    let expected = reference::constraints_hold(raw.up_constraints(), &point, SCALE);
    assert_eq!(reference::zone_contains(&zone, &point, SCALE), expected);
    let mut closed = zone.clone();
    closed.close();
    assert_eq!(closed, zone);
}

/// `includes` agrees with the entrywise reference, and an included zone's
/// points are points of the including zone.
#[kani::proof]
#[kani::unwind(12)]
fn includes_agrees_with_the_reference() {
    let dim = any_dim();
    let (_, big) = any_closed(dim);
    let (_, small) = any_closed(dim);
    let point = any_point(dim);
    let included = big.includes(&small).expect("equal dimensions");
    assert_eq!(
        included,
        reference::includes(&reference::raw_of(&big), &reference::raw_of(&small))
    );
    if included && reference::zone_contains(&small, &point, SCALE) {
        assert!(reference::zone_contains(&big, &point, SCALE));
    }
}

/// The aLU test agrees with its plain-bound transcription.
#[kani::proof]
#[kani::unwind(12)]
fn alu_agrees_with_the_reference() {
    let dim = any_dim();
    let (_, zone) = any_closed(dim);
    let (_, target) = any_closed(dim);
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    for _ in 1..dim {
        lower.push(kani::any::<bool>().then(any_constant));
        upper.push(kani::any::<bool>().then(any_constant));
    }
    let covered = zone
        .alu_covered_by(&target, &reference::lu_of::<i64>(&lower, &upper))
        .expect("matching shapes");
    assert_eq!(
        covered,
        reference::alu_covered(
            &reference::raw_of(&zone),
            &reference::raw_of(&target),
            &lower,
            &upper
        )
    );
}
