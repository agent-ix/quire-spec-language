---
id: FR-1043
title: "Certify finite-horizon measure verdicts"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-877
    type: references
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-407
    type: depends_on
---
# FR-1043: Certify finite-horizon measure verdicts

## Description

EN-5 SHALL certify every exact numeric proof of an untimed claim that reads a
bounded measure (FR-186 `BoundedMeasure`: accumulate, steps or a fraction)
in the closed profile
`"filament.probability-objective/finite-horizon-measure/v1"`: a comparison
event `probability op θ [M op c]`, a quantile `quantile q of M op c` and a
`mean of F op θ`. A document states the backward-induction values of one
payoff over FR-196's measure product, and the checker SHALL rebuild that
product, compute each decided state's payoff from its control and
accumulator itself, and check FSpec FR-412's "Finite-horizon measure"
conditions. Without this profile a latency claim such as `P95` could be
computed exactly but not checked by anything but the engine that computed
it.

## Use case

`P95` (ADR-028 §15.2) under `Steady` asks whether 95% of requests finish
within 5 ms. EN-5 computes `Pr(M <= 5 ms and activated) = 24233/25000` and
`Pr(activated) = 1` by backward induction over states that carry the
accumulated `duration`, saturated above 5 ms. It emits a `conditional`
`ratio` certificate whose two parts are documents of this profile;
`fsl-probability-check` recomputes every decided state's payoff from the
accumulated amount in its key, checks each part's equations and the ratio
`24233/25000 >= 95/100`, and accepts.

## Inputs

- A checked claim with a bounded measure and EN-5's FR-197 result over
  FR-196's measure product.

## Outputs

- The document or documents below and FR-201's check result.

## Behavior

### Forms and documents

The table states proof evidence and proof threshold conditions. For every
conditional reading, including a quantile under a workload or every
scheduler with or without fairness, the checker SHALL first determine
whether activation is unreachable in the complete product. If so, EN-5
SHALL use [FR-203](FR-203-bound-an-exact-run-and-settle-its-verdict.md)'s
certified value-free `Exhaustive` settlement with an independently accepted
`conditional` `vacuous` certificate, not a numeric or shifted-value proof.
When the checker independently finds activation unreachable, it SHALL
reject any offered numeric quantile certificate, including an all-zero
`exact` or bound `shifted` document or a workload `ratio` with denominator
0, with `InequalityFails` at the initial complete product key. This
rejection SHALL settle `inconclusive`, `CertificateRejected{rule, at}`,
without numeric acceptance or an `ExactValue`; a numeric certificate
SHALL NOT substitute for the required `conditional` `vacuous` certificate.
The numeric quantile rows below apply only when activation is reachable;
workload ratios with both activating and never-activating initial keys
SHALL follow [FR-201](FR-201-check-a-probability-certificate.md)'s `Ratio`
per-key vacuity rule.
Mean refutation evidence SHALL follow the distinct role below.

| Claim form | Evidence |
| --- | --- |
| `probability op θ [M op c]` | one `exact` document, payoff `event`; its initial value meets `θ` |
| `quantile q of M <= c` or `>= c` under a workload | FR-201's `conditional` `ratio`: numerator payoff `satisfied`, denominator payoff `activated`, each an `exact` document of this profile; `numerator / denominator >= q` for `<= c`, `< q` (strict) for `>= c`; unreachable activation uses the value-free vacuity rule above |
| `quantile q of M <= c` or `>= c` under every scheduler, with or without fairness | with activation reachable in the complete product, one document, payoff `shifted`: `exact` or `lower` for `<= c` with initial value at least 0; `exact` or `upper` for `>= c` with initial value below 0, or 0 with no activated state reachable through tight actions while activation remains reachable in the full product (FR-201 "General checking conditions") |
| `mean of F op θ` | one `exact` document, payoff `fraction`; its initial value meets `θ` |

- An elapsed measure over a timed subject keeps FR-877's timed-product
  certificate. A value too long for `max_rational_bits` is certified by
  FR-1042 with this profile's objective as its base; a workload quantile is
  then certified through the `shifted` payoff, which under a workload that
  leaves no choice equals `Pr(act) · (ratio − q)` and so has the ratio's
  sign.

### Objective

- The objective SHALL have exactly `profile`, `payoff`, `measure`,
  `optimization` and `fairness`. `payoff` SHALL be `"event"`,
  `"satisfied"`, `"activated"`, `"shifted"` or `"fraction"`.
  `optimization` and `fairness` SHALL have FR-201's meanings; fairness
  records are written as FR-201's bounded fair profile writes them and,
  as there, change no value (FR-200: a bounded form's fair optimum is its
  optimum).
- `measure` SHALL have exactly `kind`, `window`, `reward`, `unit`,
  `comparison`, `threshold` and `level`:
  - `kind`: `"accumulate"`, `"steps"`, `"fraction"` or
    `"weighted-fraction"`;
  - `window`: the `within` or `over [0, h]` bound `h` as a `Count`;
  - `reward`, `unit`: FR-196's `[model_node, identifier]` and UnitId
    descriptor of the accumulated reward or the fraction's weight, null for
    `steps` and an unweighted fraction;
  - `comparison`: `"le"` for `M <= c`, `"lt"` for `M < c`, `"ge"` for
    `M >= c`; the `<= c` quantile uses `"le"` and the `>= c` quantile
    `"lt"`; null for the payoffs `activated` and `fraction`;
  - `threshold`: `c` as a `Rational` in the claim's normalized comparison
    unit, null where `comparison` is;
  - `level`: `q` for the payoff `shifted`, null otherwise.
- The checker SHALL derive every member from the checked claim and compare;
  a changed member SHALL refuse objective mismatch. The measure's `from` and
  `until` predicates and its fraction predicate are read from the checked
  claim and are not written: the payoff of each decided state is a function
  of the claim and that state's complete key.

### Product and payoff

- Keys SHALL be FR-196's complete eight-member keys with the monitor
  `["measure",remaining,phase]` or `["fraction",remaining,phase]` and the
  accumulator alternative FR-196 gives the measure, the above-bound and
  censored amounts included. `fsl-eval` SHALL own the measure and fraction
  control's initialization, step, terminal-stutter continuation and
  canonical key operations, beside its bounded-monitor key ownership; the
  checker SHALL rebuild the product with them from the checked subject and
  claim, and SHALL NOT reuse an EN-5 product, monitor index or payoff table.
- A decided state (phase `complete`, `censored` or `unactivated`, or a
  completed fraction) SHALL take the payoff the checker computes from its
  key: `event`, FR-197's terminal value 1 or 0 of the comparison event;
  `satisfied`, 1 when activated and the amount meets `comparison` against
  `threshold`, else 0; `activated`, 1 when activated, else 0; `shifted`,
  `[activated] · ([amount comparison threshold] − level)`; `fraction`, the
  completed fraction (true count over position count, or true weight over
  total weight). A censored amount exceeds every finite threshold; an
  above-bound amount exceeds `threshold`.
- On a fraction's final departing step from position `h`, the evaluator
  SHALL first tentatively add that resolved step's weight to the total and,
  when the predicate held at the departing position, to the true sum, then
  evaluate their quotient before reading any predicate at position `h + 1`
  outside the window. If the total is 0, the evaluator SHALL report the
  quotient's existing `division-by-zero` undefined cause at the fraction
  expression, identified internally by the typed `FractionValue` read.
  The undefined location SHALL name position `h + 1` and the actual
  resolved post-state, while retaining the pre-step undecided fraction
  control and accumulator without committing the tentative contribution.
  It SHALL NOT fabricate a predicate read, a terminal decided control or
  an internal broken-invariant failure for that undefined quotient.
- Native undefined replay SHALL re-execute the actual step and its reward,
  repeat the same tentative contribution and quotient operation before
  any outside-window predicate, and compare the cause and complete
  pre-read location under
  [FR-202](FR-202-replay-a-probabilistic-witness.md)'s checked-subject
  undefined entry and general pre-read ordering. Agreement SHALL produce
  its opaque reproduced evidence; disagreement SHALL settle
  `inconclusive`, `ReplayParity`, without a token. The evidence SHALL reuse
  FSpec FR-413's existing generic undefined location and cause, with no new
  wire locus or evidence kind. Caller limits and cancellation SHALL retain
  their existing incomplete outcomes.
- At every undecided state the checker SHALL check Bellman equality: the
  workload's average, or over every scheduler the minimum (for `AtLeast`,
  the `<= c` quantile) or the maximum (for `AtMost`, the `>= c` quantile)
  over every enabled action, and at an FR-196 intermediate state the same
  optimum over its post-states. Every action and draw enters the
  comparison.
- Ranks SHALL equal the `remaining` count of the key's control at an
  undecided state and 0 at a decided one. The undecided product is acyclic
  because each model step lowers `remaining`, so these equalities with the
  decided payoffs determine the values by backward induction: they are the
  exact values, and no other fixed point exists. A rank that differs from
  the rederived count SHALL be rejected `RankBroken` at the first differing
  key.
- `values` and `ranking` SHALL cover exactly the product reachable from the
  initial key, decided states included and not expanded. Values SHALL lie
  in [0,1] except for `shifted`, in `[−level, 1 − level]`.

### Checking, bounds and results

- Strict canonical JSON, `Rational` and `Count` encodings, canonical row
  order, identity, reward and unit identity and conversion SHALL follow
  FR-201's reward profiles. Identity mismatch SHALL refuse before
  enumeration. A wrong value SHALL be rejected `InequalityFails`, a missed
  proof threshold or ratio `ThresholdMissed`, and a tight-action search that
  reaches an activated state `InequalityFails`, each at FR-201's locus.
- The product has at most `(h + 1)` positions times the reachable model
  states times the accumulator amounts, which are finite because reward
  supports are finite and amounts above `threshold` merge into one; fraction
  sums stay exact. Expansion, payoff evaluation, unit conversion and exact
  sums SHALL run under the caller's byte, state, edge, work, storage,
  representation and arithmetic limits and cancellation; a stop SHALL
  settle `incomplete` with its limit and no truth value.
- A vacuous conditional proof SHALL settle `proved`, certified,
  value-free `Exhaustive` under FR-203, with its accepted conditional
  `vacuous` certificate and no numeric entries. An accepted numeric proof
  SHALL settle `proved`, `ExactValue` (the ratio for a
  workload quantile, the probability or mean otherwise), certified; a
  quantile or comparison-event refutation SHALL keep FR-202's two-mass or
  path-set evidence. A mean refutation SHALL carry FSpec FR-413 subsystem
  evidence checked under the mean-refutation role below; an undefined
  reward or predicate SHALL settle by FR-202's `Undefined` evidence. EN-5 SHALL
  write the values and ranks it computed by FR-197's backward induction,
  which are exactly these.

### Mean-refutation role

- For `mean of F op θ`, EN-5 SHALL emit an `exact` document with payoff
  `fraction`, or a refuting `lower` or `upper` document under
  [FR-1042](FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md)
  with the same complete objective as `base` that a proof would use. The
  checker SHALL independently check the document's identity, objective or
  base, product, payoff and certificate conditions, including the required
  policy for a bound, before testing refutation. Under a workload, `exact`
  value `v < θ` or `upper < θ` refutes `>= θ`; `v > θ` or `lower > θ`
  refutes `<= θ`. Under every scheduler, the deciding extremum SHALL remain
  the minimum for `>= θ` and the maximum for `<= θ`: `exact` minimum or
  `upper` on that minimum must be strictly below `θ`, and `exact` maximum
  or `lower` on that maximum must be strictly above `θ`.
- The checker SHALL check these documents in their refutation role,
  applying the strict comparison to the independently checked initial
  value or bound, rather than requiring the proof claim-side kind or
  threshold of FR-1042. It SHALL NOT reject a valid refuting document
  because it misses the original claim's proof threshold. Equality SHALL
  NOT refute either non-strict mean claim. The counterexample SHALL reuse
  FSpec FR-413's subsystem evidence and
  [FR-202](FR-202-replay-a-probabilistic-witness.md)'s scheduler and
  checked-subject mean-certificate replay entry acceptance rules, with opaque accepted evidence for the same identity,
  base and deciding extremum required before settlement. A path set or
  probability-mass comparison SHALL NOT certify this expectation. The
  caller limits and cancellation obligations above apply to this check.

- A valid document failing the strict refutation comparison, including
  equality, SHALL be rejected by the checker `ThresholdMissed` at the
  initial key; a `lower` for `>= θ` or `upper` for `<= θ` SHALL be rejected
  `Malformed` at `kind`. Failed certificate conditions SHALL retain
  FR-201's rule and locus. These rejections SHALL settle `inconclusive`,
  `CertificateRejected{rule, at}`. Witness-policy disagreement or failure
  to attain an exact extremum SHALL settle replay `inconclusive`,
  `ReplayParity`; identity and objective/base mismatch SHALL retain the
  existing typed refusal. The mean threshold `θ` SHALL NOT be written into
  `measure.threshold`, which remains null for payoff `fraction`.
- The checker SHALL rebuild the full stopped product for `exact` and
  `upper`; an FR-1042 `lower` MAY name a subset, including a partially
  explored product. For a `fraction` base, every unnamed successor SHALL
  count as 0, the least completed-fraction payoff. The checker SHALL expand
  the named states and their successors and check the lower conditions
  under its own limits. An accepted partial `lower > θ` SHALL refute
  `<= θ` without a counterpart, including a maximum under every scheduler.
  A live limit or cancellation during checking SHALL return incomplete
  without accepted evidence.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1043-AC-1 | `Retry`'s undelivered state `u` has `try`, delivering with `1/2` and otherwise staying; delivered is terminal. Under its workload, `quantile 0.5 of (steps from holds(true) until holds(delivered) within 4) <= 2` settles `proved`, `ExactValue{3/4}`, with a `conditional` `ratio` certificate whose numerator (payoff `satisfied`, `comparison` `"le"`, `threshold` 2) has values `3/4`, `1/2`, 0, 0 at `u` with active amounts 0, 1, 2 and above the bound and ranks 4, 3, 2, 1, and whose denominator (payoff `activated`) is 1 there; `fsl-probability-check` accepts it, and rejects it `RankBroken` with the rank 3 changed to 4. At level 0.8 the item settles `refuted`. | Test (TC-3152) |
| FR-1043-AC-2 | With `try` split into `fast` (`1/2`) and `slow` (`1/4`), under every scheduler `quantile 0.4 of (…) <= 2` settles `proved` with an `exact` `shifted` document (`level` `2/5`) whose values are `3/80`, `−3/20`, `−2/5`, `−2/5`; `quantile 0.5 of (…) <= 2` settles `refuted` (minimum shifted value `−1/16`, `Pr(M <= 2) = 7/16` under `slow`); the level-0.4 document with `3/80` replaced by `fast`'s `9/40` is rejected `InequalityFails` at the initial key. | Test (TC-3152) |
| FR-1043-AC-3 | Over AC-2's model under every scheduler, `quantile 0.6 of (…) >= 2` (claim `Pr(M < 2 \| act) < 3/5`) settles `proved`: the `shifted` document (`comparison` `"lt"`) has maximum initial value `−1/10` (through `fast`) and `−3/5` at every other undecided state; `quantile 0.5 of (…) >= 2` settles `refuted`, its maximum 0 attained by `fast` with an activated state reachable. | Test (TC-3152) |
| FR-1043-AC-4 | `P95` over FR-185-AC-1's `Service` under `Steady` at `5 ms` (FR-741-AC-2, FR-203-AC-1) settles `proved`, `ExactValue{24233/25000}`, with a `conditional` `ratio` certificate whose parts are documents of this profile (`kind` `"accumulate"`, `reward` the checked `duration`, `threshold` 5 in its unit) and whose denominator is 1 at the initial key; `fsl-probability-check` accepts it from the emitted package and rejects it with one numerator value altered. At `2 ms` the item settles `refuted` with FR-202-AC-1's one-path set. | Test (TC-2066) |
| FR-1043-AC-5 | Under `Retry`'s workload, `mean of fraction holds(delivered) over [0, 2] >= rational(2,5)` settles `proved`, `ExactValue{5/12}`, with a `fraction` document; at `rational(1,2)` it settles `refuted` with FSpec FR-413 subsystem evidence containing an independently accepted `exact` `fraction` certificate of `5/12 < 1/2`, checked for refutation rather than the proof threshold. The comparison event `probability >= 0.7 [M <= 2]` over AC-1's measure settles `proved`, `ExactValue{3/4}`, with an `event` document. | Test (TC-3152) |
| FR-1043-AC-6 | FR-201-AC-51's finite-horizon quantile item over the cyclic `link` model under every scheduler (`Pr(M <= 3) = 13/27` for `steps from holds(true) until holds(m.delivered) within 4`), written at level 0.4, settles `proved` with shifted initial value `11/135` and a document `fsl-probability-check` accepts in a separate process; written at level 0.5 it settles `refuted`. | Test (TC-2065) |
| FR-1043-AC-7 | AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`, `level` or `payoff` changed refuses objective mismatch; a key whose accumulator amount or phase is altered is not an alias and fails coverage; a `shifted` value below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion of each caller limit during expansion, payoff evaluation or exact sums settle `incomplete` with the actual limit and no truth value. | Test (TC-3152) |
| FR-1043-AC-8 | With AC-1's `try` split into `fast` (`1/2`) and `slow` (`1/4`), `mean of fraction holds(delivered) over [0, 2]` has minimum `11/48` and maximum `5/12`. Under every scheduler, `>= 1/2` is refuted by an accepted exact minimum or FR-1042 `upper` on the minimum with initial bound `1/4` and policy `slow`; `<= 1/3` is refuted by an accepted exact maximum or `lower` on the maximum with initial bound `2/5` and policy `fast`. Both reuse subsystem evidence and are independently accepted by FR-202's checked-subject mean-certificate replay entry. | Test |
| FR-1043-AC-9 | AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload `upper` bound `1/2` at `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`. In AC-8, a valid `lower` minimum document with initial bound `1/5` at `>= 1/2` and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject `Malformed` at `kind` with the same settlement. Identity/base mismatch retains its typed refusal; an invalid value rejects `InequalityFails` with `CertificateRejected`; a disagreeing witness policy settles `inconclusive`, `ReplayParity`. A raw engine verdict or unchecked certificate cannot settle `refuted`. An accepted partial `lower` maximum bound `2/5 > 1/3`, with unnamed successors counted as 0, refutes without an upper counterpart; cancellation or a live checking limit remains incomplete. | Test |
| FR-1043-AC-10 | FR-203-AC-10's no-activation quantile under a resolved workload and under every scheduler, with and without an admitted fairness set, settles certified `Exhaustive` with an independently accepted conditional `vacuous` certificate, no `ExactValue`, shifted-value proof or numeric entry. Offering an all-zero `exact` `shifted` document under every scheduler, or a workload `ratio` with denominator 0, for that no-activation claim rejects `InequalityFails` at the initial complete product key and settles `inconclusive`, `CertificateRejected{rule, at}`, without numeric acceptance or `ExactValue`. A variant with reachable activation is not routed to vacuity; a forged `vacuous` certificate rejects `InequalityFails` under FR-201. The nonvacuous quantiles of AC-1 to AC-3 retain their numeric proof rules. | Test |
| FR-1043-AC-11 | `Coin` initially has bound object `c` with `c.v = 0`; its only operation `drop(b)` draws `b` uniformly from `{0,1}`, deleting `c` when `b = 0`, and declares reward `R = 0` for both supported draws. Under its resolved workload, `mean of fraction holds(c.v = 0) over [0,0] weighted by R >= 1/2` reaches undefined `division-by-zero` on the actual `drop(0)` step of probability `1/2`: the final tentative weighted sums are `(0,0)`, and the `FractionValue` location is position 1 with the actual post-state lacking `c`, pre-step undecided control `remaining = 1` and pre-step sums `(0,0)`. Native replay reproduces the same undefined result and settles `refuted` without reading outside-window `c.v`. Forging position 0, a decided control, a different accumulator, or the outside-window `absent-key` predicate cause yields `ReplayParity` without reproduced evidence. | Test |

## Dependencies

- [FR-196](FR-196-build-the-probabilistic-product.md) supplies the measure
  keys; [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
  the values; [FR-201](FR-201-check-a-probability-certificate.md) the
  document rules and the conditional certificates; FSpec FR-407 the measure
  semantics and FSpec FR-412 the conditions.

## References

- Ticket: Linear QSL-2158.
