---
id: FR-199
title: "Decide long-run fractions by bottom components"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: depends_on
---
# FR-199: Decide long-run fractions by bottom components

## Description

EN-5 SHALL decide a `long-run fraction` claim, unweighted or `weighted by
R`, exactly (ADR-028 XF-6, LR-1 to LR-5): under a workload by the
stationary distribution of each bottom strongly connected component (BSCC),
over every scheduler by each maximal end component's (MEC's) optimal
fraction through Dinkelbach's parametric method over exact policy
iteration, and in both cases by combining the component values with §7's
reachability. EN-5 SHALL settle the result as a proof or a refutation that
carries no coverage.

## Use case

EN-5 decides ADR-024 §7.3's `LongRun` claim. It finds one BSCC, solves for
its stationary distribution exactly, gets `1800/1801`, and proves the claim
with a gain–bias certificate the checker verifies by two equalities.

## Inputs

- FR-196's product for a `LongRunFraction` claim, and FR-203's limits.

## Outputs

```rust
pub struct LongRunResult {
    pub components: Vec<ComponentValue>,
    pub value: Vec<ExtRational>,                  // per initial state and binding
    pub policy: Option<Vec<ActionChoice>>,
    pub gain_bias: Vec<GainBias>,                  // per component, for the certificate
}
pub struct ComponentValue { pub states: Vec<ProductStateId>, pub rho: Rational }
pub struct GainBias { pub gain: Rational, pub bias: Vec<(ProductStateId, Rational)> }
```

## Behavior

- Under a workload, EN-5 SHALL decompose the DTMC into BSCCs and give each
  BSCC `C` the value `π_C(R · [P]) / π_C(R)`, with `π_C` its stationary
  distribution from an exact linear solve; unweighted, `R = 1`.
- Over every scheduler, it SHALL decompose the MDP into MECs and give each
  MEC its optimal fraction over the schedulers that stay in it: Dinkelbach's
  method, each round solving the mean-payoff problem for the reward
  `R · ([P] − ρ)` by exact policy iteration and updating `ρ`, until the
  optimal gain is 0.
- It SHALL combine the component values as an expected terminal reward with
  `ρ_C` at each component: `Σ_C Pr(reach C) · ρ_C` under a workload, and the
  minimum or maximum of it over every scheduler, by FR-198.
- A component in which every cycle has total weight 0 under `R` SHALL settle
  `Unsupported(ZeroWeightComponent{component})`.
- The result SHALL keep, per component, the gain–bias pair that certifies
  its value (FR-201), and over every scheduler the policy that attains it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-199-AC-1 | `LongRun` under `Steady` has one BSCC `{up, down}` with `π = (1800/1801, 1/1801)` and value `1800/1801`; its gain–bias pair is gain `1800/1801` at both states, bias 0 at `up` and `−2000/1801` at `down`. | Test (TC-634) |
| FR-199-AC-2 | An `Avail2` variant with two repairs, `repair_fast(ok ~ {true: 9, false: 1})` and `repair_slow(ok ~ {true: 1, false: 1})`, has over every scheduler the minimum `1000/1001` (always `repair_slow`) and the maximum `1800/1801`; under a workload weighting both repairs 1 its value is `1400/1401`. | Test (TC-634) |
| FR-199-AC-3 | `long-run fraction holds(v.up) weighted by duration >= 0.999` over a variant of `Avail` with `duration` 1 on `tick` and 3 on `repair` weights each position by the step that leaves it, giving `π_up / (π_up + 3 π_down)`, exact. A variant whose only reachable cycle carries `duration` 0 settles `Unsupported(ZeroWeightComponent)`. | Test (TC-634) |

## Dependencies

- ADR-028 XF-6, LR-1 to LR-5, RU-5; ADR-024 PF-6.
- [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md).

## References

- QSpec half, which owns the long-run semantics and `ZeroWeightComponent`:
  Linear STD-137.
- Owning ticket: Linear QSL-371.
