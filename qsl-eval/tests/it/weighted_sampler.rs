// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-188 (TC-623): the `quire.simulation.sampler/v1` generator's weighted
//! choice -- QSpec TC-354's selection vectors, QSpec TC-210's uniform vector
//! as unit weights at choice index 0, the integer scaling of rational
//! weights, and the choice index in the preimage.

use ix_trace_rs::trace;
use qsl_eval::simulation::{weighted_choice, ChoicePoint, Weights, WeightsRefusal};
use quire_exact::{Integer, Rational};
use sha2::{Digest, Sha256};

const SEED: u64 = 424_242;

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn weights(values: &[(i64, i64)]) -> Weights {
    let values: Vec<Rational> = values.iter().map(|&(n, d)| rational(n, d)).collect();
    Weights::new(&values).expect("positive weights")
}

fn integers(values: &[u64]) -> Vec<Integer> {
    values.iter().copied().map(Integer::from).collect()
}

fn at(trace: u64, step: u64, choice: u32) -> ChoicePoint {
    ChoicePoint {
        seed: SEED,
        trace,
        step,
        choice,
    }
}

/// The generator definition's selection, computed here from its text alone:
/// SHA-256 of the hand-written JCS preimage, `v` big-endian, rejected when
/// `v >= 2^256 - (2^256 mod n)`, then the least index whose running weight
/// exceeds `v mod n`. `n` fits a `u64`.
fn reference(point: ChoicePoint, scaled: &[u64]) -> usize {
    let n: u64 = scaled.iter().sum();
    let modulo = |bytes: &[u8]| {
        bytes.iter().fold(0u64, |acc, byte| {
            ((u128::from(acc) * 256 + u128::from(*byte)) % u128::from(n)) as u64
        })
    };
    // `2^256 mod n`: 1 followed by 32 zero bytes.
    let mut power = vec![1u8];
    power.extend([0u8; 32]);
    let excess = modulo(&power);
    for draw in 0u64.. {
        let preimage = format!(
            r#"{{"choice":"{}","draw":"{draw}","seed":"{}","step":"{}","trace":"{}"}}"#,
            point.choice, point.seed, point.step, point.trace
        );
        let digest: [u8; 32] = Sha256::digest(preimage.as_bytes()).into();
        // `v >= 2^256 - excess` exactly when `(2^256 - 1) - v < excess`.
        let complement: Vec<u8> = digest.iter().map(|byte| !byte).collect();
        let high_zero = complement[..24].iter().all(|byte| *byte == 0);
        let low = u64::from_be_bytes(complement[24..].try_into().expect("8 bytes"));
        if high_zero && low < excess {
            continue;
        }
        let remainder = modulo(&digest);
        let mut cumulative = 0;
        for (index, weight) in scaled.iter().enumerate() {
            cumulative += weight;
            if remainder < cumulative {
                return index;
            }
        }
    }
    unreachable!("a draw is accepted with probability at least 1/2")
}

/// FR-188-AC-1 (TC-623 step 1): QSpec TC-354 SM-01, which is QSpec TC-210's
/// uniform vector, run as five unit weights at choice index 0: steps 0 to 4
/// select `1, 3, 3, 4, 4`.
#[trace("TC-623")]
#[test]
fn tc_210_uniform_vector_reproduces_as_unit_weights_at_choice_zero() {
    let unit = weights(&[(1, 1); 5]);
    assert_eq!(unit.scaled(), integers(&[1, 1, 1, 1, 1]));
    let selected: Vec<usize> = (0..5)
        .map(|step| weighted_choice(at(0, step, 0), &unit))
        .collect();
    assert_eq!(selected, [1, 3, 3, 4, 4]);
    assert_eq!(Weights::uniform(5), Ok(unit));
}

/// FR-188-AC-1 (TC-623 step 1): QSpec TC-354 SM-02 -- choice index 1,
/// weights `(1, 3)`, steps 0 to 4 select items `1, 0, 0, 1, 1` -- and SM-03,
/// weights `(1/2, 3/4)` scaled by 4 to `(2, 3)` with `n = 5`.
#[trace("TC-623")]
#[test]
fn tc_354_weighted_vectors_reproduce() {
    let one_three = weights(&[(1, 1), (3, 1)]);
    assert_eq!(one_three.total(), &Integer::from(4u64));
    let selected: Vec<usize> = (0..5)
        .map(|step| weighted_choice(at(0, step, 1), &one_three))
        .collect();
    assert_eq!(selected, [1, 0, 0, 1, 1]);

    let halves = weights(&[(1, 2), (3, 4)]);
    assert_eq!(halves.scaled(), integers(&[2, 3]));
    assert_eq!(halves.total(), &Integer::from(5u64));
}

/// FR-188-AC-2 (TC-623 step 2): `(1/3, 1/6, 1/2)` scales to `(2, 1, 3)` with
/// `N = 6`, and at 60,000 consecutive trace indices, step 0, choice 0, the
/// selection equals the generator definition's reference computation and
/// selects every index.
#[trace("TC-623", "FR-188-AC-2")]
#[test]
fn rational_weights_scale_and_select_as_the_reference_computes() {
    let thirds = weights(&[(1, 3), (1, 6), (1, 2)]);
    assert_eq!(thirds.scaled(), integers(&[2, 1, 3]));
    assert_eq!(thirds.total(), &Integer::from(6u64));

    let mut counts = [0u32; 3];
    for trace in 0..60_000 {
        let selected = weighted_choice(at(trace, 0, 0), &thirds);
        assert_eq!(
            selected,
            reference(at(trace, 0, 0), &[2, 1, 3]),
            "trace {trace}"
        );
        counts[selected] += 1;
    }
    assert!(counts.iter().all(|count| *count > 0), "{counts:?}");
}

/// FR-188-AC-2 (TC-623 step 2): choices 0 and 1 of one step read different
/// preimages, so among the first 100 points their selections differ.
#[trace("TC-623", "FR-188-AC-2")]
#[test]
fn two_choices_of_one_step_draw_independently() {
    let thirds = weights(&[(1, 3), (1, 6), (1, 2)]);
    let differing = (0..100)
        .filter(|trace| {
            weighted_choice(at(*trace, 0, 0), &thirds) != weighted_choice(at(*trace, 0, 1), &thirds)
        })
        .count();
    assert!(differing > 0);
    for trace in 0..100 {
        assert_eq!(
            weighted_choice(at(trace, 0, 1), &thirds),
            reference(at(trace, 0, 1), &[2, 1, 3])
        );
    }
}

/// A weight list that admits no choice is refused: empty, zero or negative.
#[trace("TC-623")]
#[test]
fn weights_that_are_not_positive_admit_no_choice() {
    assert_eq!(Weights::new(&[]), Err(WeightsRefusal::Empty));
    assert_eq!(
        Weights::new(&[rational(1, 1), rational(0, 1)]),
        Err(WeightsRefusal::NotPositive { index: 1 })
    );
    assert_eq!(
        Weights::new(&[rational(-1, 2)]),
        Err(WeightsRefusal::NotPositive { index: 0 })
    );
}
