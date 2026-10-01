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

QSL's layer-5 `statistical` module SHALL decide a probabilistic claim under
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
    pub sampler: DefinitionRef,                     // quire.simulation.sampler/v1
    pub limits: StatisticalLimits,                  // FR-191
    pub max_witnesses: u32,                         // FR-193; default 8
}
pub enum StatisticalMethod { Okamoto, Sprt, Regenerative { min_cycles: u64 } }

pub fn check_statistical(
    request: StatisticalRequest<'_>,
    poll: impl FnMut() -> bool,
) -> Result<StatisticalOutcome, StatisticalRefusal>;
```

## Outputs

```rust
pub enum StatisticalOutcome {
    Completed(StatisticalVerdict),                  // FR-192
    Stopped { cause: IncompleteCause, limit: StatisticalLimit, partial: StatisticalBasis },
    Unsupported(UnsupportedCause),                  // NotMarkov
}
pub struct TestResult { pub index: u32, pub initial: u32, pub binding: Option<Binding>,
    pub decision: Decision, pub estimate: Rational, pub interval: Option<(Rational, Rational)>,
    pub samples: u64, pub draws: u64 }
pub enum Decision { Accepted, Rejected, Undecided(UndecidedCause) }
pub enum UndecidedCause { IndifferenceRegion, UndecidedSuccessor }
```

`StatisticalRefusal` holds FR-120's `AdmissionFailure`, FR-101's
`RequiresBound`, or `NotStatistical` for a claim `under every scheduler` or
without confidence parameters, which negotiation never routes here (FR-192).

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
  as `Pr(M <= c | activated) >= q`, and `quantile q of M >= c` as
  `Pr(M < c | activated) <= q`.

### Okamoto

- EN-4 SHALL take `N = ⌈ln(1/min(α', β')) / (2ι²)⌉` samples per test, with
  the logarithm replaced by QSpec's rational upper bound and `N` rounded up,
  and report the interval `[p̂ − ι, p̂ + ι]` for the estimate `p̂`.
- For a `>= θ` bound it SHALL decide Accepted when the interval's lower end
  is at least `θ`, Rejected when its upper end is below `θ`, and Undecided
  (`IndifferenceRegion`) otherwise; `<= θ` is symmetric. A mean of a
  fraction SHALL be decided the same way over the sample mean of the
  fraction.

### SPRT

- For `>= θ`, with `p0 = θ + ι` and `p1 = θ − ι`, each counted sample SHALL
  add `ln(p1/p0)` when the event holds and `ln((1 − p1)/(1 − p0))` when it
  fails; the test SHALL decide Rejected when the sum reaches `ln(1/α')` and
  Accepted when it reaches `ln(β')`. A `<= θ` bound swaps the roles.
- Each logarithm SHALL be a rational bound by QSpec's method, with the
  thresholds rounded away from zero and each increment rounded so the test
  never decides earlier than with the true values.
- A mean of a fraction SHALL be decided by Okamoto only; a request naming
  SPRT for it SHALL be refused as `NotStatistical`.

### Decision and stops

- The claim SHALL be Accepted when every test accepts, Rejected when any
  test rejects, and Undecided otherwise. The verdict SHALL keep each test's
  result.
- A contract conjunction evaluated undecided during a sampled step SHALL end
  that test Undecided with `UndecidedSuccessor`.
- A `SampleStop::NotMarkov` SHALL return `Unsupported(NotMarkov{…})`; an
  `ExpansionStop` SHALL return `Stopped` with its cause; a budget reached
  SHALL return `Stopped` naming it (FR-191).
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
| FR-189-AC-4 | A quantile claim over a variant of `Service` whose `request` sets `phase = Busy` with probability `1/2` (a random parameter) and otherwise stays `Idle` counts only activated samples: with Okamoto the result has `samples = 23,026` and `draws > samples`. A censored sample (no `Done` within the window) counts as exceeding the threshold. | Test (TC-624) |
| FR-189-AC-5 | The ADR-024 §7.1 `NoFault` claim with Okamoto at `indifference 0.0005` computes `N = 9,210,341`. A `Health` variant with a non-unique post-state returns `Unsupported(NotMarkov)`; a variant with an undecided contract conjunction settles that test Undecided, `UndecidedSuccessor`; a `mean of` claim with SPRT is refused `NotStatistical`. | Test (TC-624) |

## Dependencies

- ADR-024 ST-1 to ST-6, ST-9, ST-10, RX-1, RP-1; ADR-011 §1 and §6.1 (S6c,
  E10, layer 5) as ADR-024 amends them on acceptance; ADR-018 SM-1.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md),
  [FR-188](FR-188-draw-weighted-choices-with-the-revised-sampler.md).
- FR-190 adds the regenerative method, FR-191 the limits and
  reproducibility, FR-192 settlement, FR-193 sampled witnesses.

## References

- QSpec half, which owns the normative statistical methods and the rational
  bounds for logarithms, square roots and normal quantiles: Linear STD-137.
- Owning ticket: Linear QSL-371.
