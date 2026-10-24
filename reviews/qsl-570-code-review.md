---
id: SR-12215
title: "Code review (Rust lane) of QSL-570 E17c part 1 EN-4 statistical decide (PR #1079)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@149fdd4b70d4ea10512bbde90c055f1d23032618; qsl-analyze/src/lib.rs, qsl-analyze/src/statistical.rs, qsl-analyze/src/statistical/bounds.rs, qsl-analyze/src/statistical/claim.rs, qsl-analyze/src/statistical/method.rs, qsl-analyze/src/statistical/regenerative.rs, qsl-analyze/src/statistical/tests.rs, qsl-replay/src/lib.rs, qsl-replay/src/statistical.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-190
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: reviews
---
# SR-12215: Code review (Rust lane) of QSL-570 E17c part 1 EN-4 statistical decide (PR #1079)

## Summary

Ticket: QSL-570 (E17c), part 1 of 2. PR quire-spec-language#1079 at head 149fdd4b7. Only the `origin/main...HEAD` diff was reviewed (9 files). This is a code review with the rust-review lane folded in. QSpec FR-408 was read from a local read-only checkout and is cited by id only.

Narrow tests were run through the build lane. `cargo test -p qsl-analyze --lib statistical` passed 30 tests in 180 s, LongRun included. `cargo test -p qsl-replay --lib statistical` passed 2. No gates were run, and `target/` was deleted afterwards.

To measure precision, a throwaway probe test (reverted, never committed) compared the bounds against 50-digit mpmath values.

Soundness, checked rule by rule against QSpec FR-408 "Exact rational bounds":

- **`ln`.**
  - The mantissa is `y ∈ [1, 2)`. The bit-length difference gives `x / 2^e ∈ (1/2, 2)`, and one decrement fixes the case below 1. So `u = (y − 1)/(y + 1) ∈ [0, 1/3)`.
  - The atanh partial sum is the lower bound. The tail `u^(2n+3)/((2n+3)(1 − u²))` is a true geometric majorant.
  - The `k · ln 2` term takes `ln2.upper` when `k < 0`. That is outward.
- **`√x`.** `r = isqrt(⌊x·2^140⌋)` gives `r/2^70 ≤ √x`. `(r+1)² ≥ ⌊x·2^140⌋ + 1 > x·2^140` gives the upper bound. Negative `x` is guarded before `BigInt::sqrt`.
- **`π` (Machin).** The two alternating atan series both have terms decreasing from the start, since `x < 1`. The lower bound is `16·atan(1/5).lower − 4·atan(1/239).upper`, and the upper bound mirrors it. Outward.
- **`1/√(2π)`.** Takes the reciprocal of the opposite-end root. Outward.
- **`Φ`.**
  - The term recurrence `t_{n+1} = t_n · z²/(2(n+1))` is right.
  - The series stops only once `n + 1 ≥ z²`. From there the magnitude ratio `z²(2m+1)/((2m+2)(2m+3)) < 1`, so the alternating bound is valid.
  - The product with `c` picks its end by the sign of the series bound. Correct.
- **Quantile.**
  - The bisection keeps as its upper end only a point whose `Φ.lower ≥ 1 − δ`, so `upper ≥ z_{1−δ}` always. That is sound whatever error `Φ` has.
  - For `δ > 1/2` the result stays `≥ 0`, which is still an upper bound.
  - A `δ` too small to separate from 1 at `z = 16` is refused, not looped on.
- **Okamoto.** `N = ⌈ln(1/min(α′,β′)).upper / (2ι²)⌉`. The vectors 23,026, 26,492 and 9,210,341 are exact. An `N` beyond `u64` leaves only the budgets to end the test, so nothing overflows.
- **Interval rule.** `>=`, `<=` and the strict `<` (accepted only when strictly below; rejected as `<=` is) match FR-408 and FR-189 "Per sample".
- **SPRT.**
  - `p0/p1 = θ ± ι`, swapped for `<=` and `<`.
  - The lower sum uses each increment's lower bound, multiplied by exact counts, and is compared against `ln(1/α′).upper`. The upper sum uses each upper bound and is compared against `ln(β′).lower`. These are the two-sum rule with the thresholds away from zero.
  - `StatisticalClaim::new` refuses `θ − ι ≤ 0` or `θ + ι ≥ 1` (FR-186 `invalid-confidence`), so `ln` never sees a non-positive argument.
  - The 219-sample acceptance and the 12-failure rejection agree with the closed forms `⌈ln 100 / −ln(94/96)⌉` and `⌈ln 100 / ln 1.5⌉`.
- **Chow–Robbins.**
  - `S²` comes from exact running sums. The identity `Σ(Y − ÂL)² = ΣY² − 2ÂΣYL + Â²ΣL²` is checked against a two-pass computation.
  - The half-width takes `z.upper · √(S²+1/n).upper / (L̄ · √n.lower)`. All factors are positive, so the result is an upper bound.
  - The test stops only after a cycle closes, with `n ≥ min_cycles`, `n ≥ 2` and half-width `≤ ι`.
  - FR-408-AC-12's degenerate walk stops at exactly 11,632, and `min_cycles` 20,000 stops at 20,000.
- **Bonferroni and trace indices.**
  - `α′ = α/m` and `β′ = β/m` are exact. Sample `i` of test `j` uses `i·m + j`.
  - `max_draws · m` is checked for overflow before any draw. With `draws < max_draws`, that bounds every trace index.
  - Regenerative test `j` walks at trace `j`.
- **Undefined.** It rejects the test at once, records `{test, trace, evaluation}` and stops the run (`break`). The claim is Rejected and keeps the tests already run.
- **Hostile limits.**
  - Every counter is bounded by a budget check before its increment: `draws`, `counted`, `cycle_steps` and the cycle count.
  - The per-run totals saturate.
  - `max_samples`, `max_draws` or `max_cycle_steps` at 0 stops at once.
  - Non-test code has no `unwrap`, `expect`, indexing or `as` casts.

Rust lane:

- Validated newtypes (`UnitFraction`, `Weight`, `Confidence`, `StatisticalClaim`) keep invalid values out of the engine.
- Faults are typed `InternalFault` refusals, not panics.
- The seam (`BehaviourSampler` / `RegenerationWalk`) is narrow, and the tests drive the real `weighted_choice`.
- The tests assert exact vectors and could fail.

No major finding. FND-001 and FND-002 are medium. In both, a precision claim is not met, and neither makes a decision wrong.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The normal-quantile upper bound is not within `2^-64` of `z_{1−δ}` once `φ(z)` is small, which breaks QSpec FR-408 "each within `2^-64`" and FR-408-AC-7. The bisection is narrowed to `2^-64`, but its lower end is only known to have `Φ.lower < 1 − δ`, and `Φ`'s own bound is about `2^-69` wide. That width becomes a quantile error of about `2^-69 / φ(z)`. Measured against mpmath: at `δ = 1/100` the gap is under `2^-64`, but at `δ = 10^-10` it is about `4.0·10^6 · 2^-64` (≈ 2.2e-13). The result stays outward, so it is never unsound; the half-width is only slightly too large. The code follows QSpec's stated method exactly (a `2^-70` tail for `Φ`), so QSpec's precision claim cannot be reached with that method. Fix one of two ways. Either tighten `Φ`'s tail as `z` grows (by about `log2(1/φ(z))` extra bits) and add a `2^-64` assertion to the quantile test, or raise the conflict with QSpec FR-408. The existing test asserts only `< 10^-12`. | qsl-analyze/src/statistical/bounds.rs:245-316,378-389 |
| FND-002 | medium | The width of the `ln` bound grows with the binary exponent `k`. It is `|k| · w(ln 2) + 2 · 2^-70`, with `w(ln 2) ≈ 2 · 2^-70`, so it passes `2^-64` once `|k| ≳ 31`. Measured: the width of `ln(2^-40)` is between 1 and 2 times `2^-64`. `okamoto_sample_count` and `sprt_thresholds` take `ln(1/α′)`, so `α′ ≲ 2^-31` (for example `α = 10^-8`, or `α = 1/100` with about 2·10^7 tests) gets a bound looser than FR-408 states. It is still outward. Fix: sum `ln 2`'s atanh to a tail below `2^-(70 + bits(k))`, or record the limit in QSpec FR-408, and test one large `|k|`. | qsl-analyze/src/statistical/bounds.rs:117-164 |
| FND-003 | low | The `max_draws · m` trace-index refusal also applies to the regenerative method, which only ever uses trace index `j < m`. A regenerative request with `max_draws` near `u64::MAX` and `m ≥ 2` is refused `TraceIndexRange` although every trace it uses exists. Fix: run the check only for Okamoto and SPRT. | qsl-analyze/src/statistical.rs:641-651 |
| FND-004 | low | Only the SPRT path checks a sample's kind against the claim's form. Under Okamoto, a `Sample::Fraction` for a probability or quantile claim is silently summed as a fraction, and a `Sample::Event` for a mean of a fraction is summed as 0 or 1. A faulty sampler adapter (part 2) would go unnoticed. Fix: refuse a `Fraction` unless the form is `MeanOfFraction` (generalising `FractionForEvent`), and an `Event` for `MeanOfFraction`. | qsl-analyze/src/statistical.rs:400-426 |

## Verdict

**PASS: mergeable. No major or high finding.**

- Every bound is rounded outward: `ln`, `√`, `π`, `Φ`, the quantile's upper end, Okamoto `N`, both SPRT sums and thresholds, and the Chow–Robbins half-width. No rounding can produce a decision the true values would not make.
- Bonferroni, the trace-index assignment, activation, the undefined-rejection stop, the budget stops and the provenance follow FR-189 to FR-191.
- No path panics or overflows on hostile limits.
- The spec vectors (23,026, 26,492, 9,210,341, `±ln 100`, 219 samples, 11,632 cycles) are asserted exactly and pass.

FND-001 and FND-002 are precision gaps against QSpec's `2^-64` claim. The PR can fix them in code, or QSpec FR-408 can record the limit of its stated method. FND-003 and FND-004 are low.

## Dispositions

Round 1, reviewed at a9a693ac0e88a414cd5c067f016507356549657c (fix commit fda45c590, then a main merge and rustfmt). No gates were run. Narrow tests: `cargo test -p qsl-analyze --lib statistical` passed 38 tests.

A throwaway probe test (reverted, never committed) checked precision against 60-digit mpmath values:

- The quantile's upper end is at or above `z_{1−δ}` and within `2^-64` of it at `δ` = 1/100, 3/10, `10^-10` and `10^-30`.
- `ln(2^-1000)` and `ln(10^-30)` are bracketed, each within `2^-64`.

How each fix was checked:

- **`phi_series`.** The integer common-denominator series is right:
  - the term recurrence, the `lcm(1, 3, …, 2n+1)` rescaling and the alternating signs;
  - the stop test `next·2^bits < next_base·(2n+3)`;
  - `n + 1 ≥ ⌈z²⌉`, which is stronger than before.
- **`quantile_bits`.** `3z²/4 + 2` bits exceed `log2(1/φ(z)) = z²/(2 ln 2) + 1.33`. Bisection precision is taken at the bracket's upper end, where `φ` is least, so it covers the whole bracket. Soundness is unchanged, because the upper end still needs `Φ.lower ≥ 1 − δ`.
- **`ln`.** `ln 2`'s tail is below `2^-(71 + bits(|k|))`, so `k·ln 2` adds at most `2^-70` to the width.
- **The new SPRT boundary test can fail.** Four holding samples make the true sum exactly `ln(1/16)`. The outward upper sum `−4·ln2.lower` does not reach the threshold `−4·ln2.upper`. An inward rounding would accept at four.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fda45c590 |
| FND-002 | fixed | fda45c590 |
| FND-003 | fixed | fda45c590 |
| FND-004 | fixed | fda45c590 |
