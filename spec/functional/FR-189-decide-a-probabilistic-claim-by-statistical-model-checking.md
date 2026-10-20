---
id: FR-189
title: "Decide a probabilistic claim by statistical model checking (EN-4)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: depends_on
---
# FR-189: Decide a probabilistic claim by statistical model checking (EN-4)

## Description

QSL's `statistical` module in layer A, crate `qsl-analyze` (ADR-029
CB-3), SHALL decide a probabilistic claim under
a workload by sampling behaviours with FR-188's sampler, deciding the
claim's event or measure on each sample with the layer-5 evaluator (ADR-018
SM-1), and deciding the claim from the counts with the method the request
names: Okamoto's fixed-sample bound or Wald's sequential probability ratio
test (ADR-024 ST-1 to ST-6, ST-9, ST-10), or the regenerative method for a
long-run fraction (FR-190). It runs at stage S6c over E10 after
`simulation`. Every count, estimate, threshold and interval end SHALL be an
exact rational, so the decision is a function of the counts alone.

## Use case

A verification operator requests ADR-024 §7.2's `P95` claim with the
sequential method. EN-4 samples behaviours of `Service` under `Steady`,
evaluates `M <= 5 ms` on each, and stops when the log-likelihood sum crosses
a threshold. It returns Accepted with the sample count, and the same request
returns the same result on every platform.

## Inputs

```rust
pub struct StatisticalRequest<'a> {
    pub subject: ModelSubject<'a>,                  // FR-125
    pub claim: &'a CheckedProbabilisticClaim,       // FR-186, scheduler Workload, confidence Some
    pub method: StatisticalMethod,
    pub seed: Seed,
    pub sampler: DefinitionRef,                     // checked by identity: quire.simulation.sampler/v1 (FR-188)
    pub limits: StatisticalLimits,                  // FR-191
    pub max_witnesses: u32,                         // FR-193; default 8
}
pub enum StatisticalMethod { Okamoto, Sprt, Regenerative { min_cycles: u64 } }

pub fn check_statistical(
    request: StatisticalRequest<'_>,
    cancel: &Cancel,                    // FR-276
) -> Result<StatisticalOutcome, StatisticalRefusal>;
```

## Outputs

```rust
pub enum StatisticalOutcome {
    Completed(StatisticalVerdict),                  // FR-192
    Stopped { cause: IncompleteCause, limit: Option<ReachedStatisticalLimit>,
              partial: StatisticalBasis, provenance: StatisticalProvenance },
    Unsupported { cause: UnsupportedCause, provenance: StatisticalProvenance },
}                                                   // cause: NotMarkov, TotalAboveDrawRange
pub struct ReachedStatisticalLimit { pub limit: StatisticalLimit, pub value: u64,
    pub setting: &'static str }                     // `statistical.<member>`; None for Cancelled
pub struct TestResult { pub index: u32, pub initial: u32, pub binding: Option<Binding>,
    pub decision: Decision, pub estimate: Option<Rational>, pub interval: Option<(Rational, Rational)>,
    pub samples: u64, pub draws: u64,
    pub undefined: Option<UndefinedEvaluation> }   // ADR-024 SV-11; `where` names the sample
pub enum Decision { Accepted, Rejected, Undecided(UndecidedCause) }
pub enum UndecidedCause { IndifferenceRegion, UndecidedSuccessor }
```

`StatisticalRefusal` holds FR-120's `AdmissionFailure`, FR-101's
`RequiresBound` and `GeneratorMismatch`, `EveryScheduler` for a claim `under every scheduler` and
`MissingConfidence` for a claim without confidence parameters,
`NoInitialState` for a subject with no (initial state, binding) pair,
`ExactOnlyForm` for a claim form only EN-5 decides (QSpec FR-411, ADR-028
XF-5), or `NotStatistical` for a method
the claim's form does not admit (FR-190, SPRT below).

## Behavior

### Tests

- EN-4 SHALL run one statistical test per (initial state, `over` binding)
  pair of the subject, `m` tests in all, in subject order, and use
  `α' = α/m` and `β' = β/m` for each (Bonferroni).
- **Symmetry.** Under an admitted symmetry declaration (ADR-021 SYM-5), when
  the workload and every random parameter's distribution are invariant under
  its group, EN-4 SHALL run one test per orbit of (initial state, binding)
  pairs, on the least pair in key order, and `m` SHALL count orbits.
- Sample `i` of test `j` SHALL use trace index `i · m + j` and start at test
  `j`'s initial state.

### Per sample

- EN-4 SHALL sample `h + 1` positions for an event or bounded measure of
  horizon or window `h` (FR-188), and evaluate the event, or the measure and
  its comparison, with the layer-5 evaluator over those positions.
- An `accumulate` or `steps` measure SHALL find the first activation
  position `a <= h` and the first `b`, `a <= b <= h`, where the end
  predicate holds, and sum the reward of the steps into positions `a + 1` to
  `b`. A sample with no `a` is not activated; one with `a` and no `b` is
  censored and exceeds every finite threshold.
- For a quantile claim, a sample that is not activated SHALL be drawn and
  not counted: it advances the trace index and counts toward `max_draws`,
  never toward the sample count or the SPRT sum. A quantile claim SHALL be
  decided as the probability bound of ADR-024 PF-4: `quantile q of M <= c`
  as `Pr(M <= c | activated) >= q`, and `quantile q of M >= c` as the strict bound
  `Pr(M < c | activated) < q`. The tests SHALL decide the strict bound as
  `<= q`, whose boundary lies inside the indifference region.

- If the event, the measure or its comparison evaluates `Undefined` on a
  sample, then EN-4 SHALL decide that test Rejected at once, stop the
  run, as QSpec FR-408 does, and set the test's `undefined` to
  `UndefinedEvaluation{where, cause}`, `where` naming the test index, the
  trace index and the position at which the claim evaluated undefined
  (ADR-024 SV-11, ADR-018 UE-1).

### Methods

- EN-4 SHALL implement QSpec FR-408's fixed-sample method (Okamoto) and
  sequential method (SPRT) over each test's `α'`, `β'` and the claim's
  `ι`, with FR-408's rational bounds, conservative rounding and two-sum
  SPRT decision, in exact rationals, so the decision is a function of the
  counts alone.
- EN-4 SHALL decide a mean of a fraction by the fixed-sample method over
  the sample mean of the fraction. A request naming the sequential method
  for it SHALL be refused as `NotStatistical`, as FR-408 states.

### Decision and stops

- The claim SHALL be Accepted when every test accepts, Rejected when any
  test rejects, and Undecided otherwise. The verdict SHALL keep each test's
  result.
- A test's `estimate` SHALL be `Some(count / samples)` (or the method's
  estimate, FR-190) once it has counted a sample or a cycle, and `None` when
  it has counted none, as for a test that rejected on an undefined first
  sample.
- The **least favourable test** of a verdict SHALL be the first rejecting
  test in subject order when the claim is Rejected, the first Undecided test
  when it is Undecided, and, when it is Accepted, the accepting test with the
  smallest margin to the claim's threshold (for a lower bound `>= θ`, the
  smallest lower end minus `θ`; for an upper bound `<= θ`, `θ` minus the
  largest upper end; for a test with no interval, its estimate in place of
  the interval end), the lowest index breaking a tie. FR-192's `estimate`
  and `interval` are the least favourable test's.
- If the subject has no (initial state, binding) pair, then EN-4 SHALL refuse
  with `StatisticalRefusal::NoInitialState`, so an empty product is never
  Accepted vacuously.
- If a test rejects on an undefined sample and stops the run, then the
  verdict SHALL keep the results of the tests that finished before it and the
  rejecting test's own result, and no result for a test that did not start.
- If a quantile claim's samples are all non-activated, then the run SHALL
  continue until `max_draws` stops it (FR-191); it SHALL never decide on zero
  counted samples.
- A contract conjunction evaluated undecided during a sampled step SHALL end
  that test Undecided with `UndecidedSuccessor`.
- A `SampleStop::NotMarkov` SHALL return `Unsupported(NotMarkov{…})`; an
  `ExpansionStop` SHALL return `Stopped` with its cause; a budget reached
  SHALL return `Stopped` naming it (FR-191).
- If a sample stops with `SampleStop::TotalAboveDrawRange`, then the run
  SHALL stop and return `Unsupported(TotalAboveDrawRange{total})`, which
  FR-192 settles as `unsupported` with the kebab-case cause
  `total-above-draw-range{total}`, `total` a decimal string (QSpec
  FR-181-AC-14, FR-331).
- If a sample stops with `SampleStop::GeneratorMismatch`, then EN-4 SHALL
  refuse with `StatisticalRefusal`'s `GeneratorMismatch`.
- The outcome SHALL be a function of the subject, the claim, the method,
  `min_cycles`, the seed, the sampler and the limits.
- The EN-4 provider manifest SHALL advertise (`probabilistic-satisfaction`,
  evidence `statistical`) and register through FR-075.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-189-AC-1 | `P95` (ADR-024 §7.2) with Okamoto runs one test of `N = 23,026` samples, each of at most 11 positions, and settles `Completed` Accepted with an interval of half-width `1/100` whose lower end is at least `19/20`; the same claim at `<= 2 ms` settles Rejected with an interval whose upper end is below `19/20`. With SPRT, at `5 ms` it settles Accepted and at `2 ms` Rejected, each with a sample count below 23,026. | Test (TC-624) |
| FR-189-AC-2 | The Okamoto sample count is computed exactly with conservative rounding: `N = 23,026` for `α = β = 1/100`, `ι = 1/100`, `m = 1`, and `N = 26,492` for `m = 2`. The SPRT thresholds for `α' = β' = 1/100` are rationals `T+ >= ln 100` and `T- <= -ln 100` within QSpec's stated precision of them. Two runs with the same request give equal outcomes. | Test (TC-624) |
| FR-189-AC-3 | A two-initial-state variant of `Service` runs `m = 2` tests, each with `α' = β' = 1/200`, sample `i` of test `j` at trace index `2i + j`. A variant with two symmetric servers under an admitted symmetry declaration and an invariant workload runs `m = 1`; with a workload that weights one server's operations differently it runs `m = 2`. | Test (TC-624) |
| FR-189-AC-4 | A quantile claim over a variant of `Service` whose `request` sets `phase = Busy` with probability `1/2` (a random parameter) and otherwise stays `Idle` counts only activated samples: with Okamoto the result has `samples = 23,026` and `draws > samples`. A censored sample (no `Done` within the window) counts as exceeding the threshold. `quantile 0.5 of (M) >= 3` (`M` is a placeholder for the claim's parenthesised measure) reduces to the event `M < 3` with the strict bound `< 1/2`; over a variant where `M` is 1 or 3 with probability 1/2 each, `Pr(M < 3) = 1/2`, so the reduced bound is false, as the claim is (its 1/2-quantile is 1). | Test (TC-624) |
| FR-189-AC-5 | The ADR-024 §7.1 `NoFault` claim with Okamoto at `indifference 0.0005` computes `N = 9,210,341`. A `Health` variant with a non-unique post-state returns `Unsupported(NotMarkov)`; a variant with an undecided contract conjunction settles that test Undecided, `UndecidedSuccessor`; a `mean of` claim with SPRT is refused `NotStatistical`. | Test (TC-624) |
| FR-189-AC-6 | Over a `Coin` model (object `c` with `v: Int[0, 1]`, initially 0; operation `flip(b: Int[0, 1])` with `random Coin::flip(b) ~ { 0: 1, 1: 1 };` and postcondition `self.v = b`; workload weight `flip` 1) and the claim `probability >= 0.5 [ always[0,3] holds(1 / (1 - c.v) = 1) ]` with `alpha 0.01, beta 0.01, indifference 0.1`, Okamoto and seed 7, the test decides Rejected at the first sample in trace-index order that reaches `v = 1`, draws no further sample, and carries `UndefinedEvaluation` naming that sample's test index 0, its trace index and the first position with `v = 1`, cause `division-by-zero`; the claim settles `Completed` Rejected. | Test (TC-640) |
| FR-189-AC-7 | Over a `Wide` model (object `w` with `v: Boolean`, initially false; operation `spin(x: Boolean)` with `random Wide::spin(x) ~ { true: 1, false: 115792089237316195423570985008687907853269984665640564039457584007913129639936 };` (that decimal is 2^256) and postcondition `self.v = x`; workload weight `spin` 1) and the claim `probability >= 0.5 [ always[0,3] holds(w.v or not w.v) ]` with `alpha 0.01, beta 0.01, indifference 0.1`, one initial state so `m = 1`, Okamoto and seed 7, the run stops at the first sample and returns `Unsupported(TotalAboveDrawRange{total})` with `total` `2^256 + 1`; the claim settles `unsupported` with cause `total-above-draw-range` and `total` `"115792089237316195423570985008687907853269984665640564039457584007913129639937"` (QSpec TC-354 SM-09). | Test (TC-624) |
| FR-189-AC-8 | Over `Service`, a test that counts no sample (the first sample undefined, FR-189-AC-6's `Coin`) has `estimate` `None`; a two-test run whose second test rejects has the first test's result kept and the verdict's least favourable test is test 1 (the first rejecting); an all-accepting two-test run's least favourable test is the one with the smaller margin to the threshold, the lower index on a tie; a subject with no initial state is refused `NoInitialState`; a quantile claim whose every sample is non-activated stops at `max_draws`. | Test (TC-624) |

## Dependencies

- ADR-024 ST-1 to ST-6, ST-9, ST-10, RX-1, RP-1; ADR-011 §1 and §6.1 (S6c,
  E10, layer A) as ADR-024 and ADR-029 CB-3 amend them; ADR-018 SM-1.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md),
  [FR-188](FR-188-draw-weighted-choices-with-the-qspec-sampler.md).
- FR-190 adds the regenerative method, FR-191 the limits and
  reproducibility, FR-192 settlement, FR-193 sampled witnesses.

## References

- QSpec half, which owns the normative statistical methods and the rational
  bounds for logarithms, square roots and normal quantiles: QSpec FR-408
  (Linear STD-137).
- Owning ticket: Linear QSL-371.
