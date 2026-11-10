---
id: SR-20261110090706890008
title: scope-boundary review of weighted DTMC long-run proof specification
type: SpecReview
analysis: scope-boundary
scope: agent-ix/quire-spec-language; QSL-1826 PR2655; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md, spec/decisions/ADR-028-exact-probabilistic-engine.md,
  spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
review_set: subset
---

## Summary

Reviewed frozen 88bc0719c3228b5a03fe9b4c9170abda6d588a2b against base bfebfd678f405a8778150843dd3bfaaefadc0f5b. Specification only; no implementation or complete qualification is claimed.

## Verdict

**PASS** — QSL FR201 owns native independent decoding, objective/subject derivation and weighted certificate checking; FR199 owns result semantics and producer solving; FR196 owns reward/model/unit identity and draw evaluation. Shared transport and exact unit/canonical primitives are consumed under their existing contracts. The spec guarantees independent checks through planned cross-process TC1402; it does not claim runtime qualification. General MDP, fairness, timed, refutation and shared-wire FR413 remain outside this profile.

## Assurance Context

No active AssuranceProfile applies in the local spec tree (anchored type search found none). Direct delegated subset review; no enforced workflow acceptance or exception is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For a checked `long-run fraction holds(P) weighted by R` claim under an

    authored workload, the codec SHALL admit the separate objective profile

    `"quire.probability-objective/dtmc-weighted-long-run/v1"`. Its objective

    SHALL have exactly `profile`, `predicate_states`, `weight`, `unit`,

    `optimization` and `fairness`. `optimization` SHALL be `"workload"` and

    `fairness` SHALL be empty. `predicate_states` retains the unweighted

    profile''s independently rederived complete original-graph key set.

    `weight` SHALL be FR-196''s actual `[model_node,identifier]` for checked R;

    `unit` SHALL be its actual null,'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- This profile SHALL retain the unweighted profile's untimed, finite,\n\
    \  one-initial/at-most-one-binding DTMC admission, fully resolving workload,\n\
    \  complete original graph, positive-support BSCC partition, canonical\n  component\
    \ ordering, bias normalization, strict transport and independent\n  threshold\
    \ checks. Predicate-true states SHALL NOT absorb their outgoing\n  transitions.\
    \ Zero direct `over` instances retain `UnsupportedBinding`\n  and the unsupported\
    \ EN-5 disposition with no invented evidence. General\n  MDP/every-scheduler,\
    \ fairness, timed, multi-post, interval and refutation\n  forms, incl"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The checker SHALL independently evaluate the named checked reward on\n\
    \  every reachable semantic transition and typed draw, using FR-196's\n  operation/receiver/non-random-argument\
    \ identity and the checked expression.\n  An operation without R contributes zero.\
    \ Admitted weights SHALL be\n  defined, finite, nonnegative integers or quantities\
    \ of R's checked type;\n  quantities use the checked weight's unit and existing\
    \ exact conversion\n  graph. The threshold and fraction remain dimensionless,\
    \ not quantities\n  in that unit. Integer representation SHALL use the existing\
    \ representation\n  limit; numeri"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Write `p(e)` for the independently rederived normalized probability\
    \ of\n  an outgoing draw e from s and `r(e)` for its independently rederived\n\
    \  magnitude in the checked weight unit. The checker SHALL compute\n  `w(s) =\
    \ sum_{e from s} p(e) * r(e)`. FR-199 weights the position by the\n  step that\
    \ leaves it: the centered immediate reward is\n  `w(s) * ([P(s)] - rho_C)`, using\
    \ the CURRENT state's predicate. It is\n  neither `[P(to(e))]` nor an unweighted\
    \ indicator. Implicit terminal\n  stutter retains the original key and P label,\
    \ contributes named reward\n  zero and invents no reward-bearing operatio"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Before checking component evidence, the checker SHALL establish that\n\
    \  every independently rederived reachable BSCC has at least one state\n  with\
    \ `w(s) > 0`. For a finite positive-support irreducible chain with\n  nonnegative\
    \ weights this is exactly positive stationary denominator;\n  every state has\
    \ positive stationary mass. A BSCC with all w zero has\n  no defined weighted\
    \ fraction and SHALL retain the existing\n  `Unsupported(ZeroWeightComponent)`\
    \ outcome, identifying that component\n  under the existing diagnostic contract.\
    \ This includes a reachable\n  terminal stutter, even when P holds there"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The document SHALL retain exactly `identity`, `objective`, `kind`,\n\
    \  `values` and `components`, with kind `\"long-run\"` and unchanged\n  `{value,gain,bias}`\
    \ component and `{state,value}` bias-row shapes. Values\n  and component values\
    \ SHALL be exact rationals in [0,1]. Transported\n  `gain_C` SHALL equal `rho_C`,\
    \ FR-199's weighted-ratio result, not the\n  numerator payoff `pi_C(R*[P])` or\
    \ the centered reward's zero mean.\n  Bias magnitudes SHALL be signed exact rationals\
    \ in the selected weight\n  unit, with zero at C's first complete key. At EVERY\
    \ s in C the checker\n  SHALL check\n  `h(s) = w(s) * ("
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Every state in C SHALL have V equal to rho_C. Transient V rows SHALL\n\
    \  satisfy the unweighted ORIGINAL transition harmonic equation\n  `V(s) = sum_t\
    \ p(s,t)*V(t)`, giving FR-199's expected component fraction,\n  not a ratio of\
    \ globally combined numerators and denominators. All BSCCs\n  are independently\
    \ matched before that check, so this extension is unique.\n  Finite transient\
    \ weights do not enter its recurrence. Inclusive AtLeast\n  and AtMost proof checks\
    \ SHALL use the checked dimensionless threshold\n  and existing S3 admission and\
    \ exact arithmetic; no endpoint-admission\n  policy is changed. A "
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Conflicting identity SHALL refuse before enumeration. Changed weight,\n\
    \  unit, predicate set, optimization or profile SHALL refuse as objective\n  mismatch.\
    \ Structural member, coverage and ordering errors SHALL retain\n  `Malformed`\
    \ at `values` or `components`; mismatched BSCC membership,\n  gain, normalization,\
    \ equation or V SHALL retain the unweighted profile's\n  first-complete-key `InequalityFails`\
    \ rule. Model-only or foreign-binding\n  keys are not aliases. Equivalent independently\
    \ rederived draw costs may\n  accept under a matching identity/objective; source\
    \ changes alone are not\n  a stalenes"
- id: FR-201-AC-31
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: FR-199-AC-3's Steady availability graph, with up-to-up 1999/2000, down-to-up
    9/10, outgoing duration 1 at up and 3 at down, accepts the weighted profile with
    component value/gain 600/601, both state values 600/601, bias 0 at up and -2000/601
    at down in that canonical key order. Both bound sides accept at 600/601; AtMost
    999/1000 accepts and AtLeast 999/1000 rejects ThresholdMissed, while AtMost 599/600
    rejects. Unweighted value/gain 1800/1801 or bias -2000/1801 fails.
- id: FR-201-AC-32
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A periodic chain alternates P state a and not-P state b; at a two equally
    probable draws to b have R magnitudes 1 and 3, and b's step to a has R 1. The
    weighted component accepts rho=gain=2/3 and biases (a,b)=(0,-2/3). Swapping the
    two a draw weights preserves acceptance; changing 3 to 5 falsifies that certificate.
    A zero-weight transient s entering this component and an all-P state c with probabilities
    1/4 and 3/4, where c has an explicit probability-one self-step of R 1, accepts
    V(s)=11/12; a globally pooled numerator/denominator substitute 8/9 rejects. A
    reachable all-zero-weight BSCC, alon
- id: FR-201-AC-33
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-31/32 documents round-trip canonical bytes through a fresh independent
    checker without analyzer/provider linkage. Wrong checked weight name/model or
    actual unit, changed predicate set/optimization/profile, foreign binding/model-only
    keys, malformed rational/JSON/key data and duplicate/missing/out-of-order component/value/bias
    members prevent acceptance with their specified typed rule/locus. Independently
    checked equivalent quantity expressions normalize through the actual weight UnitId;
    another same-dimension descriptor cannot replace it. Gain 0, a numerator payoff
    in gain, unscaled unweigh
- id: FR-201-AC-34
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and exhaustion/cancellation during actual admission, key
    construction, graph/draw expansion, predicate/weight/unit evaluation, SCC/membership
    and exact weighted sums retain the actual typed limit/cancellation cause with
    no acceptance. Undefined or negative reachable weights, undefined predicates/workload
    and unsupported reward types preserve typed nonacceptance rather than zero. Unsupported
    shape/kind/fairness/every-scheduler cases issue no weighted proof. Zero direct
    over instances preserve UnsupportedBinding/unsupported with no invented evidence.
    Integer representation and e
- id: FR-199
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: examined
  excerpt: 'For a weighted workload result, the same transported `gain` is the

    dimensionless ratio `rho = pi(R*[P])/pi(R)`, as retained by the existing

    `LongRunResult`, rather than the numerator payoff `pi(R*[P])`. Its bias

    uses the checked weight unit and the centered immediate reward

    `E[R | s] * ([P(s)]-rho)`, with zero stationary mean. FR-201''s separate

    weighted document checks those equations and positive component weight;

    the zero centered mean does not replace the transported ratio by zero.

    '
- id: FR-199-AC-3
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: context_only
  excerpt: '`long-run fraction holds(v.up) weighted by duration >= 0.999` over a variant
    of `Avail` with `duration` 1 on `tick` and 3 on `repair` weights each position
    by the step that leaves it, giving `π_up / (π_up + 3 π_down)`, exact. A variant
    whose only reachable cycle carries `duration` 0 settles `Unsupported(ZeroWeightComponent)`.'
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: Amounts use the checked claim's
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s separate `quire.probability-objective/dtmc-weighted-long-run/v1`

    retains the finite untimed workload boundary and existing component wire

    shape, adding only the checked weight/unit objective identity. LR-2 and

    FR-199-AC-3 weight CURRENT positions by outgoing draws. Thus each BSCC

    checks `h=w*([P]-rho)+Ph` and transported `gain=rho`, with the same canonical

    zero-bias normalization. The selected weight unit applies to w and h;

    rho and its threshold are dimensionless. Complete positive-support BSCC

    coverage and at least one positive w in each component establish a

    positive stationary den'
- id: TC-1402
  path: spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "1. Admit FR-199-AC-3's original Steady availability graph, with up before\n\
    \   down in canonical complete-key order. Define the checked named duration\n\
    \   as 1 on every outgoing tick and 3 on every outgoing repair. Check the\n  \
    \ AC-31 exact weighted document at both equality bounds and its three\n   strictly\
    \ interior beyond/control thresholds. Independently verify\n   `pi_up=1800/1801`,\
    \ `pi_down=1/1801`, denominator `1803/1801`, ratio\n   `600/601` and both centered\
    \ state equations. Change value, gain and bias\n   separately, including the unweighted\
    \ certificate and zero gain.\n2. Admit AC-32's periodi"
- id: TC-1402
  path: spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "- Step 1 accepts rho/gain `600/601` and normalized down bias `-2000/601`.\n\
    \  Both equality bounds and AtMost `999/1000` accept; AtLeast `999/1000`\n  and\
    \ AtMost `599/600` reject `ThresholdMissed` at the initial key.\n  All thresholds\
    \ lie strictly in (0,1). Altered equations, gain or values\n  reject at the specified\
    \ complete-key locus.\n- Step 2 accepts `2/3` with biases `0,-2/3`, preserves\
    \ that acceptance\n  after the equal-probability swap, and rejects the changed\
    \ mean weight.\n  It accepts transient V(s)=`11/12` and rejects `8/9`, the ratio\
    \ formed by\n  pooling component numerators and denominators"
```

## Evidence and limits

All four normative artifacts passed scoped Quire validation and requirement grammar summary (4/4 clean, zero findings). Manual exact rational analysis independently checked both component equations and the transient mixture. Existing source semantics were inspected at qsl-analyze/src/exact_probabilistic/unbounded.rs:645-751 and qsl-semantics/src/check/probabilistic.rs:900-918. New source/typed adversarial acceptance and cancellation procedures are authored in TC1402 but have not been executed. Runtime/Rust/gap-analysis, complete test matrix, evidence advisor and criterion-strength/Jev methods were not selected for this spec-only diff. No full FR201AC30/all-expansion or whole-repository assurance is granted.
