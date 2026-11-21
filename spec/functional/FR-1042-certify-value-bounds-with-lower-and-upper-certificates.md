---
id: FR-1042
title: "Certify value bounds with lower and upper certificates"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1041
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1043
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
---
# FR-1042: Certify value bounds with lower and upper certificates

## Description

When EN-5 proves a probability or expected-reward claim from a dyadic
interval
(`ValueBounds`, FR-203), it SHALL certify the interval with two documents in
the closed profile `"filament.probability-objective/value-bounds/v1"`: a
`lower` document whose initial value is the interval's lower end and an
`upper` document whose initial value is its upper end. Each wraps, as its
`base`, the objective of the exact profile the claim would otherwise be
written in, and the checker SHALL check each by FSpec FR-412's `lower` or
`upper` conditions over the product that base defines. A value whose exact
rational is too long to carry (§15.1's has 23,254 bits) is then still
certified on both sides, rather than written as a rational it is not.

## Use case

§15.1's no-fault probability `(9999999/10000000)^1000` exceeds
`max_rational_bits` 1,024. EN-5 reruns backward induction over 64-bit dyadic
intervals and settles `proved`, `ValueBounds{…, BackwardInduction{64}}`. The
item carries a `lower` document, ranked by remaining horizon, whose initial
value is above 0.999, and an `upper` document; `fsl-probability-check`
accepts both, so the reported interval is certified at both ends.

## Inputs

- An EN-5 `Bounds` result (FR-197 dyadic backward induction, or FR-198
  interval iteration) with its lower and upper vectors and, over every
  scheduler, its policy.

## Outputs

- Two documents, `lower` and `upper`; FR-201's check result for each.

## Behavior

### Bases

- `base` SHALL be the complete objective, `profile` member included, of
  one of these exact profiles, admitted by that profile's own rules:
  FR-201's `reachability/v1` (both subsets, with the bounded event forms of
  FR-201's bounded subset), `state-predicate/v1`, `cyclic-dtmc-eventual/v1`,
  `cyclic-mdp-eventual/v1` and `bounded-fair-eventual/v1`; FR-1041's
  `cyclic-mdp-fair-eventual/v1`; FR-1043's `finite-horizon-measure/v1`
  with any of its payoffs; and FR-201's expected-reward profiles
  `expected-reward/v1`, `dtmc-expected-reward/v1` and
  `cyclic-mdp-expected-reward/v1`, whose bounds FR-198's interval iteration
  produces. The checker SHALL compare every member of `base`
  with its own derivation, as that profile does, before reading a value.
- A timed proof keeps FR-920's profile, which admits `lower` and `upper`
  itself. Long-run values are solved exactly (FR-199) and have no dyadic
  method; when such a value exceeds `max_rational_bits`, EN-5 SHALL settle
  the item
  `Incomplete(LimitReached{MaxRationalBits, value, exact_prob.max_rational_bits})`,
  which raising that setting resolves, with no certificate and no truth
  value.

### Document

- The objective SHALL have exactly `profile` and `base`. The document SHALL
  have exactly `identity`, `objective`, `kind` (`"lower"` or `"upper"`),
  `values`, the `ranking` the table below requires, and optional `policy`
  hints for its existential choices, in
  FR-201's strict canonical JSON, `Rational` and `Count` encodings with
  complete product keys as the base profile writes them. A dyadic value is
  an ordinary reduced `Rational` whose denominator is a power of two; no
  precision member is written, since `ValueBounds.method` records it.
- `values` of an `upper` document SHALL cover exactly the base's stopped
  graph. A `lower` document SHALL name the initial key and any subset of
  the stopped graph; an unnamed successor counts as the base's least value
  (FSpec FR-412): 0 for a probability or reward base, `−level` for a
  measure base whose payoff is `shifted`, so an unnamed state never raises
  the bound. A `lower` document may cover only the part of the product explored
  before a limit stopped exploration (FSpec FR-411 "Evidence on a partial
  product"). `ranking` rows SHALL cover exactly the states the table
  names among the covered states; `policy` hints MAY omit any such state.
- A present `policy` hint SHALL use the existing `{state, choice}` row and
  an enabled action in the base profile's existing `TransitionIdentity`,
  reset or randomized-choice vocabulary, only where that base admits the
  form. It SHALL NOT invent an operation identity for an implicit terminal
  stutter or a post-state choice. A nonexistent or disabled hinted action
  SHALL reject `InequalityFails` at that state, even if another action
  would satisfy the bound. Duplicate, extraneous or structurally malformed
  rows SHALL retain FR-201's `Malformed` rejection and member locus.
- Targets and accepting decided states SHALL carry 1, non-target terminals
  and rejecting decided states 0, the states of FR-1041's `U` 0, and a
  measure base's decided states their payoff (FR-1043), and a reward base's
  target states 0. Every value of a probability base SHALL lie in [0,1];
  a reward base's values are nonnegative rationals in its comparison unit.

### Conditions

`Q_a(x, s)` is FR-201's one-step value of action `a`; under a workload, the
single resolved distribution. "Progress" at `s` means a positive-probability
successor of strictly smaller rank whose value is positive (FSpec FR-412
`lower`), or, for the fair-minimum upper, whose value is below 1.

| Base optimization | `lower` (`x` at most the value) | `upper` (`y` at least the value) |
| --- | --- | --- |
| workload | `x(s) <= Q(x, s)`; `ranking`; progress at each named non-target state with `x > 0` | `Q(y, s) <= y(s)` |
| minimum | for every action `a`, `x(s) <= Q_a(x, s)` and progress under `a`; `ranking` | selected action `σ`: `Q_σ(y, s) <= y(s)` |
| maximum (with or without fairness) | selected action `σ`: `x(s) <= Q_σ(x, s)` and progress under `σ`; `ranking` | for every action `a`, `Q_a(y, s) <= y(s)` |
| expected reward, any optimization | `x(s) <= c(s) + Q(x, s)`, under every action for a minimum, under selected action `σ` for a maximum; no `ranking` | `c(s) + Q(y, s) <= y(s)`, under every action for a maximum, under selected action `σ` for a minimum; `ranking` for properness: a successor of positive probability and smaller rank under the workload, `σ`, or every action for a maximum |
| fair minimum (FR-1041) | for every action `a`, `x(s) <= Q_a(x, s)`, `x = 0` on `U`; no `ranking` | selected action `σ`: `Q_σ(y, s) <= y(s)` and progress under `σ` toward `U`, the targets and the states with `y = 1`; `ranking` |

- For an `upper` on a minimum or a `lower` on a maximum, the checker
  SHALL independently rebuild every enabled action at each undecided
  covered state. A present hint SHALL select the action to check, and that
  action SHALL satisfy every applicable per-state condition together: its
  inequality and any progress, properness or rank condition required by
  the base. A failing hint SHALL reject with the applicable
  `InequalityFails` or `RankBroken` at that state even when another action
  satisfies all conditions; the checker SHALL NOT replace the hint.
- With no hint at that state, the checker SHALL accept the existential
  choice if and only if some rebuilt action satisfies every such condition,
  selecting and retaining the first satisfying action in canonical
  full-product-action order in its independently checked policy. If no
  action satisfies the inequality, it SHALL reject `InequalityFails` at
  that state; if some satisfy the inequality but none meets the required
  progress or rank conditions, it SHALL reject `RankBroken` there.
  Terminal-stutter-only states and intermediate states with multiple
  post-state choices SHALL use their existing semantic actions in this
  search without a policy row or a new wire action/carrier. The checker
  SHALL report the selected policy through its accepted check outcome;
  this adds no certificate field. Universal-action conditions elsewhere
  in the table SHALL still check every action.
- Over a bounded base (FR-201's bounded subset, the bounded fair profile,
  FR-1043's measure product) the undecided product is acyclic and decided
  states are absorbing. There the inequalities alone bound the value by
  induction on the remaining horizon, and a `lower` document's `ranking`
  SHALL instead equal the independently rederived remaining horizon, as
  FR-201's bounded rank rule states; no positive-value progress rule
  applies, so a measure's negative shifted payoffs are admitted.
- Why the conditions are sound: the value is the least fixed point of the
  one-step operator with the boundary values above. An `upper` vector is a
  pre-fixed point of the operator the table names (every action for a
  maximum, a policy's for a minimum, whose value is at least the minimum),
  so it dominates that least fixed point. A `lower` vector is a post-fixed
  point whose ranking makes the policy or every action reach the targets
  from every positive state with positive probability within its rank,
  which excludes the greatest-fixed-point solutions a cycle admits
  (FSpec FR-412, Funke, Jantsch and Baier). For the fair minimum the roles
  swap through `w = 1 − v` (FR-1041). For an expected reward they swap
  again: an improper scheduler has value `+∞` (FR-198), so a post-fixed
  point bounds every proper policy's unique fixed point from below with no
  ranking, while an upper vector needs the properness ranking, because a
  closed non-target class of cost 0 has the finite pre-fixed point 0
  although its value is `+∞` (FR-201's cyclic reward profiles use the same
  ranks).
- A `Lower` vector from interval iteration from below, or from dyadic
  backward induction rounded down, satisfies its inequalities, and its rank
  is the first iteration or position at which a state's value becomes
  positive; an `Upper` vector rounded up satisfies its own. EN-5 SHALL check
  both documents before emitting them, and on a rejection raise the
  precision as FR-201 states for any rejected certificate.

### Pairing, side and results

- The document on the claim's side SHALL meet the threshold inclusively at
  the initial key: `lower` for `AtLeast` (and for the `<= c` quantile's
  shifted payoff, at least 0), `upper` for `AtMost` (and for the `>= c`
  quantile's shifted payoff, below 0, or 0 with FR-201's tight-action
  search finding no activated state). FR-201's checker checks one document
  at a time, told whether it is the claim side: the claim-side document
  meets its conditions and the threshold, and a claim-side document of the
  wrong kind is rejected `Malformed` at `kind`; the counterpart meets its
  conditions and no threshold.
- EN-5's settlement (FR-203), not the checker, SHALL check the pairing: a
  `ValueBounds` proof settles `proved` only with an accepted claim-side
  document and an accepted counterpart of the other kind carrying the same
  identity, except for a `lower` proof found on a partial product, which
  has no counterpart. EN-5 builds both documents, so a proof that reaches
  settlement without its counterpart is a broken invariant and SHALL settle
  `failed` with `runtime_invariant`/`established-invariant-broken`, never
  `proved`. A rejected counterpart SHALL settle `inconclusive`,
  `CertificateRejected{rule, at}` with that document's rule and locus.
- Both accepted SHALL settle `proved`,
  `ValueBounds{lower, upper, method, reductions}`, certified, with `lower`
  and `upper` the two documents' initial values. A `lower` proof found on a
  partial product (FSpec FR-411) SHALL settle `ValueBounds` with
  `exploration: Partial`, which FSpec FR-331 writes as
  `"exploration": "partial"`, and with `upper` the base's least upper
  bound: 1 for a probability base, `1 − level` for a `shifted` measure base,
  1 for a `fraction` payoff, and `+∞` for a reward base, written `"infinity"`
  as FSpec FR-418 writes an absent bound. The marker, not the value of
  `upper`, is what exempts the proof from its counterpart, so a fully
  explored `lower` whose upper end is 1 still carries one. The claim-side document missing its
  threshold SHALL make EN-5 raise the precision, up to
  `max_precision_bits`, and then settle `Incomplete(PrecisionBudget{lower,
  upper})` (FR-203). A rejection SHALL settle `inconclusive`,
  `CertificateRejected{rule, at}` with FR-201's rules and loci. The checker
  SHALL obey the caller's byte, state, edge, work, storage, representation
  and arithmetic limits and cancellation; a stop SHALL settle
  `incomplete` with its limit and no truth value.
- Identity SHALL be FR-201's structural `{node_id, model, ordinal}`; both
  documents carry the item's identity, and a mismatch in either SHALL
  refuse before enumeration. A changed `base` member SHALL refuse objective
  mismatch.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1042-AC-1 | Over the cyclic `link` chain (from the undelivered state `s`, `1/3` each to `s`, delivered `g` and the stopped terminal `f`; value `1/2`) under `Even`, `probability >= rational(2,5) [eventually holds(m.delivered)]` with `max_rational_bits` 1 settles `proved`, `ValueBounds{lower, upper, …}` with `lower <= 1/2 <= upper`, and its `lower` and `upper` documents, `base.profile` `cyclic-dtmc-eventual/v1`, are each accepted by `fsl-probability-check` in a separate process and rejected with one value altered. | Test (TC-2065) |
| FR-1042-AC-2 | Over AC-1's chain, offered documents: `lower` `s` `7/16`, rank 1, with counterpart `upper` `s` `9/16` is accepted for `>= rational(2,5)`; `lower` `5/8` and `upper` `7/16` are rejected `InequalityFails` at `s`; `lower` `7/16` for `>= rational(1,2)` is rejected `ThresholdMissed` at `s`; `lower` without `ranking` is rejected `Malformed` at `ranking`; an `upper` document offered as the claim side of that `AtLeast` proof is rejected `Malformed` at `kind`; `f` at `1/16` in the `upper` document is rejected `InequalityFails` at `f`. | Test (TC-3151) |
| FR-1042-AC-3 | Over `W` (FR-1041-AC-1) with no fairness set, maximum `1/2`: for `probability <= rational(3,5)`, `upper` `s` `3/5` (both actions satisfy `Q_a(y) <= y`) with counterpart `lower` `s` `7/16`, policy `go`, rank 1, settles `proved`; `lower` `s` 1 with policy `wait` is rejected `RankBroken` at `s`; the `lower` `s` `7/16`, rank 1, without `policy` is accepted with independently selected `go`, because `wait` fails progress while `go` satisfies inequality and rank. For the minimum (0), `upper` `s` `1/4` with policy `wait` is accepted and with policy `go` rejected `InequalityFails` at `s`. | Test (TC-3151) |
| FR-1042-AC-4 | Over `W` under `fair strong go` (fair minimum `1/2`), `lower` `s` `2/5` with no ranking and counterpart `upper` `s` `3/5` with policy `go` and rank 1 prove `probability >= rational(2,5)`; `upper` `s` `1/4` with policy `wait` is rejected `RankBroken` at `s`. | Test (TC-3151) |
| FR-1042-AC-5 | A `Health` chain whose `ok` state stays `ok` with `9/10` per step (otherwise a fault): `probability >= 0.75 [always[0,2] holds(ok)]` (exact `81/100`) is proved by `lower` values `25/32` and `7/8` at the undecided keys of remaining horizon 2 and 1, ranks 2 and 1, with counterpart `upper` `7/8` and `15/16`, settling `ValueBounds{25/32, 7/8}`; the `lower` rank 2 changed to 3 is rejected `RankBroken` at that key, and `upper` `13/16` at horizon 2 is rejected `InequalityFails` there (`(9/10)(15/16) = 27/32 > 13/16`). | Test (TC-3151) |
| FR-1042-AC-6 | §15.1 with `max_rational_bits` 1,024 and `precision_bits` 64 (FR-197-AC-3, FR-203-AC-1) settles `proved`, `ValueBounds{…, BackwardInduction{64}}`, whose `lower` document (base `reachability/v1`, bounded `always`) ranks every undecided key by its remaining horizon and whose initial lower value is above `999/1000`; both documents are accepted by `fsl-probability-check` from the emitted package. | Test (TC-2066) |
| FR-1042-AC-7 | FR-201-AC-27's `Steady` long-run claim (exact `1800/1801`) with `max_rational_bits` 4 settles `Incomplete(LimitReached{MaxRationalBits, 4, exact_prob.max_rational_bits})` with no certificate, and not `proved`, `unsupported` or `failed`. A settlement handed a `lower` proof with no counterpart over a fully explored product settles `failed`, `runtime_invariant`/`established-invariant-broken`. | Test (TC-3151) |
| FR-1042-AC-9 | Over FR-201-AC-35's retry chain (`s` to itself and to `B` with `1/2` each, cost 1 on both; value 2), base `dtmc-expected-reward/v1`: `lower` `s` `15/8` with no ranking and counterpart `upper` `s` `9/4` with ranks `s` 1, `B` 0 prove `expected accumulate R until holds(B) >= 3/2` (`15/8 <= 1 + (1/2)(15/8)`, `1 + (1/2)(9/4) = 17/8 <= 9/4`); `upper` `s` `7/4` is rejected `InequalityFails` at `s`; `upper` without `ranking` is rejected `Malformed` at `ranking`. Over a closed non-`B` zero-cost self-loop reachable beside `B`, an `upper` row 0 there is rejected `RankBroken` at that state. | Test (TC-3151) |
| FR-1042-AC-10 | Over FR-1043-AC-2's `fast`/`slow` model at level 0.5 (shifted minimum `−1/16`), a `lower` document naming only the initial key at value 0 is rejected `InequalityFails` there: each unnamed successor counts as `−1/2`, so both actions give `−1/2`. | Test (TC-3151) |
| FR-1042-AC-11 | A `lower` proof found before `max_states` stopped exploration settles `ValueBounds` with `exploration: Partial` and no counterpart: over AC-1's chain with `upper` 1, over FR-1043-AC-2's level-0.4 `shifted` base with `upper` `3/5`, and over AC-9's reward chain with `upper` `+∞`, written `"infinity"`; each result passes the backend-provider schema. A fully explored `lower` proof over a base whose interval's upper end is 1 settles `proved` only with its counterpart. | Test (TC-3151) |
| FR-1042-AC-8 | Pre-cancellation, cancellation during either document's check, and exhaustion of each caller limit settle `incomplete` with the actual limit and no truth value; either document with another identity refuses before enumeration, and either with a changed `base` member refuses objective mismatch. | Test (TC-3151) |
| FR-1042-AC-12 | For a bounded fraction mean over an initial terminal state with a false fraction predicate and window 2, the base's only continuation is terminal stutter and its value is 0. An `upper` minimum document with every value 0 and no policy rows, and a `lower` maximum document with every value 0, the rederived remaining-horizon ranks and no policy rows, are accepted. The checker retains the existing stutter action at each undecided key; no fabricated operation identity or new wire form is required. | Test |
| FR-1042-AC-13 | At an intermediate state with two enabled post-state choices `a` and `b`, their successors reach event success with probability `1/4` and `3/4`, respectively. With correct successor values and ranks, a minimum `upper` initial bound `1/2` with no hint is accepted selecting `a`, and a maximum `lower` bound `1/2` with no hint is accepted selecting `b`. Initial bounds `1/8` for the upper and `7/8` for the lower reject `InequalityFails` there because no action satisfies. Post-state selection needs no fabricated TransitionIdentity row. | Test |
| FR-1042-AC-14 | In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails` although `wait` satisfies, and the same document without that hint accepts selecting `wait`. A nonexistent hint rejects `InequalityFails` even when an enabled action satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing the hint does not accept it, since no action satisfies both inequality and progress. When several actions satisfy every condition, an omitted hint selects the first in canonical full-product-action order. | Test |

## Dependencies

- [FR-201](FR-201-check-a-probability-certificate.md) supplies the base
  profiles and the rejection rules; [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
  and [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md)
  the dyadic results; FSpec FR-412 the `lower` and `upper` conditions.

## References

- F. Funke, S. Jantsch and C. Baier, farkas certificates and minimal
  witnesses for probabilistic reachability constraints, TACAS 2020;
  S. Haddad and B. Monmege, interval iteration, 2018.
- Ticket: Linear QSL-2158.
