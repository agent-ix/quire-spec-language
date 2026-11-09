---
id: FR-201
title: "Check a probability certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-201: Check a probability certificate

## Description

EN-5 SHALL emit every proof with a `ProbabilityCertificate` (ADR-028 CE-1).
QSpec FR-412 defines the certificate contract and the exact conditions its
checker verifies. The layer-6 facade entry
`qsl_replay::check_probability_certificate` SHALL apply that contract using
QSL's recompile, product enumeration and graph code, without an engine
present. It solves no equation and runs no iteration. The checker is part of
the qualified core (ADR-028 CE-7): an EN-5 `proved` SHALL count only after
this checker accepts its certificate.

## Use case

A reviewer receives a `proved` result for §15.4's claim at threshold 0.95.
The checker recompiles the model, rebuilds the product, checks that the
certificate's values `24/25` and `4/5` are the minimum over both actions at
each live state, and accepts. A certificate with one value altered is
rejected, and the item stays `inconclusive`.

## Inputs

```rust
pub struct ProbabilityCertificate {
    pub identity: ObligationIdentity,           // ADR-013 O-09
    pub objective: Objective,                   // targets or decided states, reward, min/max/workload, fairness set
    pub kind: CertificateKind,                  // Lower, Upper, Exact, LongRun
    pub values: Vec<(ProductKey, Rational)>,
    pub ranking: Option<Vec<(ProductKey, u64)>>,  // Lower and Exact
    pub policy: Option<Vec<(ProductKey, WitnessEntry)>>, // closed FR-202 choices/distributions
    pub components: Option<Vec<ComponentCertificate>>, // LongRun: value and gain–bias per component
}

pub fn check_probability_certificate(
    wire: ReplayRequestWire,                    // FR-098
    subject: &ModelTraceSubject,                // FR-128
    replay_limits: ReplayLimits,                 // FR-098 replay.input_bytes bound
    item: &ObligationIdentity,
    certificate: &ProbabilityCertificate,
    limits: &ExactProbLimits,
) -> Result<CertificateCheck, ReplayRefusal>;

pub enum CertificateCheck { Accepted { bound_side: Bound }, Rejected(CertificateRejection), Stopped(ExactProbLimit) }
```

The checker uses FR-338's `CertificateRejection{rule, at}`. FR-338's
`CertificateRule` gains `InequalityFails` and `ThresholdMissed`; the checker
also uses FR-169's `RankMissing` and `RankBroken` and FR-163's `Malformed`.
FR-338's `CertificateLocus` gains `ProbabilisticProductState(ProductKey)`,
the complete FR-196 product key; the checker also uses FR-163's
`CertificateMember`. QSpec FR-331 writes the first as its probabilistic
product state form `{product_key}` and the second as `{member}`.

| Failure | `rule` | `at` |
| --- | --- | --- |
| An `Upper`, `Lower`, `Exact`, expected-reward or `LongRun` condition fails at a state, a policy entry is missing, has the wrong mode or names a choice that is not enabled there, or a recurrent component under the policy lacks observation progress or fairness | `InequalityFails` | `ProbabilisticProductState` of the first failing state in canonical order; for a component, its first state |
| For a quantile claim with `y(s0) = 0`, the tight-action search reaches an activated state | `InequalityFails` | `ProbabilisticProductState` of the first reachable activated state |
| A non-target state with `x > 0` has no rank | `RankMissing` | `ProbabilisticProductState` of that state |
| A ranked non-target state with `x > 0` has no successor of positive probability, `x > 0` and a smaller rank | `RankBroken` | `ProbabilisticProductState` of that state |
| The conditions hold, but the initial state's value is not on the claim's side of the threshold | `ThresholdMissed` | `ProbabilisticProductState` of the initial state |
| A `Lower` or `Exact` certificate without `ranking`, a lower bound on a maximum without `policy`, or a `LongRun` certificate without `components` | `Malformed` | `CertificateMember` naming `ranking`, `policy` or `components` |

## Behavior

### Emission

- EN-5 SHALL build a certificate for every `proved` outcome, check it with
  this checker before emitting it, and on rejection raise precision or move
  to its exact method; it SHALL emit no proof without an accepted
  certificate.
- From an exact value it SHALL emit an `Exact` certificate; from a dyadic
  interval a `Lower` and an `Upper` certificate; from a long-run result a
  `LongRun` certificate with the gain–bias pairs of FR-199.

### Closed reachability document profile

For untimed unbounded `eventually` reachability and the bounded subset below, the
document codec SHALL implement the following QSL-owned profile of QSpec
FR-412's open, nonempty `objective` object. This profile does not narrow the
other objective obligations above and below; unsupported forms SHALL remain
explicitly unsupported rather than be encoded as this profile.

- The objective SHALL have exactly `profile`, `targets`, `optimization` and
  `fairness`. `profile` SHALL be
  `"quire.probability-objective/reachability/v1"`; `targets` SHALL be the
  complete set of reachable target product keys, transported using QSpec
  FR-331's `ProbabilityProductKey`; `optimization` SHALL be `"minimum"`,
  `"maximum"` or `"workload"`; `fairness` SHALL be the empty array. An empty
  target set is valid. Targets are extensional evidence, not an expression
  encoding or a trusted qualitative set: the checker SHALL independently
  evaluate the checked claim's target over its own reachable product and
  compare the exact set before acceptance. It SHALL rederive optimization
  from the checked claim, not from certificate values.
- Admission SHALL require one subject initial state, at most one admitted
  claim binding, no clocks, reward or authored fairness. The unbounded
  subset SHALL retain its acyclic continuation graph restriction apart
  from implicit terminal stutter. The bounded subset SHALL admit only
  origin-activated event-position `eventually[0,h] holds(P)` with a finite
  nonnegative position horizon and a checked state predicate P; its
  undecided monitor product SHALL terminate, even when the underlying model
  graph is cyclic. Other temporal forms remain unsupported by this profile.
  Both subsets retain FR-196's Markov successor restrictions.
  Workload admission SHALL require a single resolved
  distribution at each live state. Unsupported shape, profile version,
  optimization or certificate kind SHALL give an explicit unsupported
  result with no accepted certificate.
- The document SHALL have exactly `identity`, `objective`, `kind`, `values`
  and `ranking`, with `kind` equal to `"exact"`. Identity SHALL use the
  existing structural `{node_id, model, ordinal}` contract. Values SHALL
  use QSpec's existing `{state, value}` rows and `Rational` representation
  `{numerator, denominator}`, both canonical decimal integer strings with
  a positive denominator (zero is `"0"/"1"`);
  ranking SHALL be required and use its existing `{state, rank}` rows and
  `Count` (JSON integer 0 through 9007199254740991). No new top-level
  discriminator, package identity or field is introduced. Lower, upper,
  long-run, policy and component documents are outside this codec profile.
- The writer SHALL use RFC 8785 UTF-8 canonical JSON. The reader SHALL
  reject malformed UTF-8/JSON, duplicate or unknown members, missing
  members, noncanonical bytes, nonreduced rationals, nonpositive
  denominators and noncanonical integer strings rather than normalize
  them into accepted evidence. It SHALL use QSpec's existing unpadded
  canonical base64url transport for state keys. `targets`, `values` and
  `ranking` SHALL each be strictly ascending by decoded complete canonical
  key octets, with no duplicate keys. Values and ranking SHALL cover
  exactly the checker-rebuilt reachable states, including terminal states;
  probabilities SHALL lie in [0,1].
- Keys SHALL use FR-196's authoritative complete six-component basis,
  including the actual `binding`, through the shared
  `ProbabilityProductKey::for_model_state` helper for the unbounded subset.
  The bounded subset SHALL use the existing complete formula-monitor
  representation, with null accumulator and pending components. The stale
  five-component illustrative tuple in QSpec FR-412/schema documentation
  SHALL NOT cause binding to be dropped. Actual absent monitor,
  accumulator and pending components remain null. The reader SHALL admit
  keys against the checked subject and compare complete canonical octets;
  a model-only key, foreign binding or altered component is not an alias.
- The checker SHALL check Bellman equality for the independently selected
  operator at every live state, target value 1 and non-target terminal
  value 0, and CE-3's positive-value rank progress as well as threshold
  satisfaction. For the unbounded subset it MAY emit checker-verified acyclic height ranks, with
  terminal height 0 and live height one plus the maximum successor height;
  target ranks can be 0. Rank overflow SHALL stop under a named limit.
  Equality alone SHALL NOT discharge the required ranking condition.
  Every-scheduler `AtLeast` selects minimum and `AtMost` selects maximum;
  workload uses its resolved average. Exact evidence in these proof cases
  needs no crossing-bound policy. A case needing an upper-on-minimum or
  lower-on-maximum policy SHALL NOT fabricate or silently omit that policy.
- Identity mismatch SHALL refuse as conflicting identity before product
  enumeration. Objective mismatch SHALL refuse as invalid runtime input
  before acceptance; an internally consistent certificate for another
  target or optimization SHALL never establish this claim. Graph,
  arithmetic, parsing and source admission SHALL obey caller limits and
  `&Cancel`, preserving stopped/cancelled outcomes without truth values.

#### Bounded event-position checking

The bounded subset is a capability extension of the same
`quire.probability-objective/reachability/v1` document contract: its members,
extensional target meaning, optimization, fairness and complete-key transport
do not change. Existing unbounded documents retain their meaning and admission.
No second formula AST, opaque monitor id or public horizon field is introduced.

- The checker SHALL independently construct the model/monitor product using
  `qsl-eval`'s qualified property-automaton translation, initialization and
  transition semantics. It SHALL read position 0 during initialization and
  one subsequent position per observation step, including the inclusive
  position h. At a terminal model state it SHALL apply the existing FR-126
  profile false-extension closure, rather than invent model progress or
  infer acceptance from current predicate truth.
- `qsl-eval` SHALL own a canonical monitor-state key operation alongside its
  existing automaton key ownership and a terminal false-extension operation
  usable by the qualified checker. Those operations SHALL preserve FR-196's
  checked formula/profile/origin/binding context and complete parsed FR-338
  automaton key, including real remaining horizon/debts for running states
  and counter-free absorbing decided verdicts. Analyzer-local `StateId`
  values SHALL NOT become certificate identity; the checker SHALL NOT copy
  analyzer monitor semantics or trust its enumerated product.
- Targets SHALL be exactly the independently rederived accepting decided
  product states. Accepting states have value 1; rejecting decided states
  have value 0; neither is expanded. Bellman equality applies at every
  undecided state over actual model/monitor successors.
- Bounded ranks SHALL agree with the independently rederived remaining
  event-position horizon/debts under QSpec FR-412's finite-horizon rule:
  each undecided state's rank is its actual remaining future positions,
  and each decided state's rank is 0. Every undecided observation successor
  that remains undecided consumes one position. The checker SHALL reject
  a substituted acyclic-model height or altered remaining-horizon rank
  even if Bellman equalities and general positive-value progress hold.
  A wrong bounded rank is `RankBroken` at the first differing full product
  key in canonical order; missing rank coverage remains malformed coverage.
- Only FR-197 `Exact` results can emit this profile's exact document.
  `Bounds` and `PrecisionBudget` SHALL preserve their explicit outcomes;
  dyadic lower/upper documents remain unsupported by this closed profile
  until their own certificate profile is defined. An interval SHALL NOT be
  serialized as an exact rational certificate. This admission does not
  establish bounded refutation/witness replay. FR-202 separately specifies
  that native typed path-set extension; broader certificate forms required
  by this requirement remain outstanding.

### Closed acyclic state-predicate document profile

For infinite-trace untimed `holds(A) until holds(B)` and `always holds(P)`
over checked state predicates, the document codec SHALL admit a separate
`"quire.probability-objective/state-predicate/v1"` profile. This does not
reinterpret `reachability/v1`. The new objective SHALL contain exactly
`profile`, `event`, `targets`, `failures`, `optimization` and `fairness`.
`event` SHALL be `"until"` or `"always"`; optimization and empty fairness
SHALL retain the closed reachability profile's meanings. All other document
members, canonical byte rules, exact rational representation, complete-key
transport, required ranking, strict decoding, identity checks and caller
controls SHALL retain that profile's contracts. Both key sets SHALL be
strictly ascending complete-key sets, disjoint and independently rederived;
neither SHALL be trusted as a qualitative result supplied by EN-5.

- Admission SHALL require the existing unbounded acyclic model graph apart
  from implicit terminal stutter, one initial state, at most one actual S3
  claim binding, no clocks, rewards or authored fairness, and FR-196's
  existing Markov successor restrictions. Only exact minimum AtLeast,
  maximum AtMost and single-resolved-distribution workload proofs SHALL be
  admitted. Cyclic, nested temporal, bounded, event-predicate and other
  profile/optimization/kind cases SHALL remain explicitly unsupported.
- The checker SHALL bind the actual checked occurrence, infinite-trace
  profile, formula and S3 binding before evaluating A/B/P through qualified
  state-predicate evaluation. It SHALL independently build the original
  reachable model graph and establish its admission, then build the stopped
  graph reachable from the initial state by expanding only continuation
  states. Values and ranks SHALL cover exactly this stopped graph. Nodes
  occurring only after a success or failure are not certificate states.
- For until, B SHALL take precedence: targets are exactly reached B states;
  failures are exactly reached states satisfying neither A nor B. A state
  satisfying A and not B continues unless terminal. A terminal A-and-not-B
  state SHALL have value 0: its infinite stutter never reaches B. For
  always, failures are exactly reached not-P states; targets are exactly
  reached terminal P states. Every nonterminal P state continues; its
  current truth SHALL NOT establish always. The stopped graph SHALL absorb
  targets and failures, including a failure whose original successor could
  later satisfy B or P. Terminal means no model continuation apart from
  implicit terminal stutter, not a provider's unverified flag.
- Target values SHALL be 1, failure and other terminal values 0. At each
  continuation state the checker SHALL verify exact Bellman equality with
  the independently selected minimum, maximum or resolved workload average.
  This checks the event probability directly; always's minimum is not the
  minimum probability of eventual not-P. If EN-5 uses complement solving,
  it SHALL exchange minimum/maximum and complement each value before
  issuing this direct-event certificate; workload complements its own
  resolved expectation. The checker SHALL solve no equations and reuse no
  EN-5 graph, labels or complement result as trusted evidence.
- Ranks SHALL satisfy CE-3's positive-value progress on the stopped graph;
  acyclic height ranks (absorbing and terminal rank 0) MAY be used. The
  checker SHALL use FR-196's full binding-bearing model-state keys with
  absent monitor, accumulator and pending components null, as ADR-028 PR-6
  specifies. No local monitor handle, surrogate formula AST or synthetic
  terminal model key SHALL be introduced. The qsl-eval qualified bounded
  monitor and false-extension closure are not the semantics of this
  infinite-trace subset; terminal stutter here has the same state predicates
  forever. Actual checked formula/profile/binding context SHALL determine
  every label, including at position 0. This terminal distinction follows
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)'s infinite-trace
  terminal rule and ADR-018 DL-6; the until/complement reductions are owned
  by [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md).
- Identity mismatch SHALL refuse before enumeration. A changed event,
  targets, failures or optimization SHALL refuse objective mismatch before
  acceptance. Value, rank and threshold failures SHALL use the existing
  rejection rules and canonical first-failure loci. A different formula
  with extensionally identical derived objective and valid values is not
  required to fail solely because its spelling differs; no invented formula
  identity field is added. Certificate proofs alone are extended here;
  this profile SHALL NOT authorize new FR-202 refutation semantics or
  complete the blocked QSpec FR-413 witness wire.

### General checking conditions

- Digital timed certificate objectives SHALL select ADR-028 §13a from the
  checked subject/claim, independently of the authored fairness vector.
  The checker SHALL rederive the original digital continuation graph,
  observation edges, observation-admissible components and R, target-avoiding
  admissible components, and pretarget positive-reward waiting components
  with admissible exits. A finite upper maximum bound SHALL fail if a
  checked positive waiting component makes the supremum +infinity.
  Existing qualitative +infinity states need no fabricated finite Rational
  row; their proof is graph-recomputed, never an asserted delay-forever
  attaining scheduler. These graph checks use checker-owned algorithms and
  actual budgets; they are not equation-solving/iteration.
- Policy entries SHALL be FR-202's closed WitnessEntry. Randomized entries
  use distinct enabled choices and positive reduced exact probabilities
  summing to1. The checker SHALL apply their actual weighted one-step
  operator and check every reachable recurrent induced-chain component for
  observation progress and the actual fairness set. Goal absorption cannot
  bypass checking an admissible continuation on the original subject.
  Missing choice, wrong mode, or a scheduled-identity alias for IdleObserve
  is rejected rather than projected from action None.
- For exact-only timed Reach/elapsed, goals are observation edges and the
  operator uses terminal continuation1/0 respectively, V(post-key)
  otherwise; pure delay does not read the predicate. The initial observed
  goal terminates before reward. A public post-key shared by delay and
  observation does not become a synthetic goal-state key. For finite
  expected elapsed, the Lower/Exact properness condition is a positive
  probability lower-rank successor **or goal observation edge of terminal
  rank0**, even when the target value is0. Every reachable pretarget
  recurrent component under the evidence policy must have such an exit.
  Upper/Exact value and rank coverage for this edge-goal objective is every
  reachable as-yet-unhit state, with no value row required for a post-goal
  visit or internal sink; every non-goal successor still requires its actual
  row. The original post-goal continuation remains fully covered by the
  checked admissibility policy, not a missing-value/default action.
  This replaces the positive-value successor proviso below only for this
  expected-reward use; Reach retains its original proviso.

- The checker has one set of conditions and two ways to obtain its subject.
  The wire form above recompiles the package a `ReplayRequestWire` names. The
  checked-subject form,
  `check_claim_exact_reachability(system, claims, item, certificate,
  max_states, max_edges)` for the reachability objective and its counterparts
  for the other objectives, takes the `ModelSystem` admitted over a
  `CheckedPackage` and its `claims`, binds the claim by the item's node, and
  re-enumerates the product through that system with the checker's own graph
  code and no `qsl-analyze` code; it decodes no wire and recompiles nothing,
  as FR-193's replay over a package S4 does not emit (FR-185). A package with
  a random parameter, a workload or a reward has no emitted form, so
  `analyze` (FR-281) runs the checked-subject form for every EN-5 item. Both forms apply every condition of this requirement
  (fairness end components, `LongRun` components, `Lower` ranking, `Upper`,
  `Exact`, quantile tight actions and expected-reward properness), the
  rejection table and the budgets; they differ only in how the subject is
  obtained.
- The checker SHALL refuse a certificate whose identity differs from the
  item's.
- It SHALL decode `wire` through FR-098's canonical `ByteProvision` and
  recompile path, passing `replay_limits` to `ReplayRequest::decode` so the
  encoded request is bounded by `replay.input_bytes` before provision access;
  it SHALL re-admit `subject` and re-enumerate the product through
  `ModelSystem` and the monitor, using
  `qsl-eval`'s property-automaton translation and enabledness (ADR-018
  LA-5) and its own product enumeration and graph algorithms, and no
  `qsl-analyze` code. It SHALL NOT accept or reuse an EN-5 solver product as
  trusted replay data: every
  reachable product state for an `Upper`, `Exact` or `LongRun` certificate,
  and the states the certificate names and their successors for a `Lower`
  one, an unexplored successor counting as value 0.
- It SHALL recompute the decided states, `Prob0` and `Prob1`, and components
  with its own graph code. When the objective has a fairness set, it SHALL
  derive the maximal fair end components using FR-200's refinement before
  applying QSpec FR-412's certificate conditions.
- It SHALL apply QSpec FR-412's exact-rational conditions and threshold-side
  rule, and QSpec FR-331's rejection vocabulary. It SHALL accept when the
  conditions hold and the initial state's value is on the claim's side of the
  threshold, and report the side; otherwise it SHALL reject with the rule and
  locus the table above gives the first failing condition and state in
  canonical order. QSL's internal rule and locus variants map to the published
  forms as the table above specifies. It SHALL check the `Malformed` members
  before any re-enumeration.
- For a `quantile q of M >= c` claim over every scheduler, proved by an
  `Upper` certificate `y` for `max E[Y]`, `Y = [activated] · ([M < c] − q)`
  (ADR-028 XF-2, CE-4): when `y(s0) < 0` the checker SHALL accept with side
  `AtLeast`. When `y(s0) = 0`, it SHALL compute the tight actions, those
  with `Q_y(s, a) = y(s)` in exact rationals, and search the product graph
  from the initial state using tight actions only; it SHALL accept when no
  activated state is reachable, and otherwise reject, naming the first
  reachable activated state in canonical order.
- A checker run stopped by a budget SHALL return `Stopped` naming it.

### Verdict path

- QSL's settlement map in `qsl-replay` SHALL run the checker on the
  certificate that left S6c over E11 and settle `proved` only on `Accepted`; `Rejected` SHALL settle
  `inconclusive`, `CertificateRejected`; `Stopped` SHALL settle V-7.
- The checker's arithmetic and graph algorithms SHALL be code separate from
  EN-5's iterative and linear-algebra methods.

### Closed acyclic expected-reward document profile

For checked `ProbForm::ExpectedReward` source claims `expected accumulate R
until holds(B) op c` in the untimed infinite-trace profile, the codec SHALL
admit the separate `"quire.probability-objective/expected-reward/v1"`
objective. It SHALL have exactly `profile`, `targets`, `reward`,
`unit`, `optimization` and `fairness`. `reward` SHALL use FR-196's existing
`[model_node,identifier]` identity: the actual declaring model NodeId and
checked reward Identifier R. `unit` SHALL use FR-196's actual descriptor:
null for plain numeric, `["declared",unit_key]` or
`["compound",unit_form]` for the admitted UnitId. The checker SHALL compare
both against the checked claim and its normalized comparison unit. Targets are the independently
rederived complete-key set where B holds in the stopped graph. Optimization
and empty fairness retain their existing meanings. This profile SHALL NOT
change the bytes or interpretation of either existing probability profile.

- Admission SHALL require one initial state, at most one actual S3 binding,
  no clocks or authored fairness, the existing Markov successor restrictions
  with one post-state per draw, and an acyclic original reachable graph
  apart from implicit terminal stutter. Establish original-graph acyclicity
  before stopping at targets, so a cycle hidden beyond B is unsupported.
  Every reachable original terminal SHALL satisfy B. A non-target terminal,
  even with zero reward, SHALL make this finite-only profile unsupported:
  FR-198 assigns positive infinity to nonreachability, rather than the
  finite cost of a failed prefix. No infinity is encoded as a rational.
- The source contract is the actual named checked Reward expression over
  operation parameters, returning integer or quantity, not a Steps/Time enum
  or an invented expected-elapsed form. The checker SHALL resolve R under
  the checked model and rederive every step reward through qualified
  transition/draw evaluation. An operation without that reward contributes
  zero. Every reachable step reward SHALL be defined, finite and
  nonnegative; violations preserve the source refusal or explicit unsupported
  outcome without accepted evidence. The bound and costs SHALL use the
  checked claim's normalized comparison unit and existing exact conversion
  graph. FR-196 owns the actual UnitId descriptor (null for plain numeric;
  declared or compound identity for quantities); no display-name alias or second unit algebra is introduced.
- Values and ranking SHALL cover exactly the independently rebuilt stopped
  graph: expand only states where B is false. A target, including the
  initial state, has value zero before any outgoing cost is charged. At
  each continuation state s check `V(s) = opt_a sum_e p(e) *
  (r_R(e) + V(to(e)))` over the actual semantic choices and their draws;
  workload uses its single resolved distribution. This includes the cost
  of entering B, and excludes every cost after B. Costs, probabilities and
  action grouping SHALL be independently rederived, not read from solver
  values, labels or a trusted objective table. Full FR-187 ScheduledIdentity
  operation/receiver/non-random arguments and actual typed draws determine
  actions and edges; local offsets, action ordinals and model-only key
  aliases SHALL NOT substitute for them. No action/edge transport or formula
  AST is added to the certificate.
- Admit exact minimum AtLeast and maximum AtMost proofs, and either bound
  under a resolved workload; evaluate equality inclusively. Values SHALL
  be nonnegative exact rationals with no upper bound of 1. Retain the exact
  document's existing strict canonical JSON, rational, complete binding-bearing
  key, coverage, identity, ranking and threshold rules. Use full model keys
  with absent monitor/accumulator/pending null. Check acyclic height ranks
  exactly: target/terminal zero, each continuation one plus the maximum
  stopped-successor height. Thus zero-cost continuation is also checked;
  probability-target value 1 and probability range checks do not apply.
- Identity conflict SHALL refuse before enumeration. Wrong target, reward
  or optimization SHALL refuse as objective mismatch. A wrong value or rank
  SHALL reject at the first full key in canonical order under the existing
  inequality/rank rules; an otherwise valid document missing the requested
  threshold SHALL reject ThresholdMissed. Parsing, original/stopped graph
  expansion, expression evaluation, unit conversion and rational arithmetic
  SHALL obey caller byte/state/edge/arithmetic limits and cancellation,
  including metering full key, rank and value representations. No solver,
  equation solving, iteration or provider dependency is allowed in checking.
  Cyclic, fair, long-run, timed, multi-post, interval and reward-refutation
  extensions, including native FR-202 reward witnesses and FR-413 wire,
  remain outside this profile.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-201-AC-1 | §15.4 at threshold 0.95: the `Exact` certificate with values `24/25`, `4/5`, 1 at delivered states and 0 at the two-loss state is accepted with side `AtLeast`. The same certificate with `4/5` replaced by `9/10` is rejected, naming the first state in canonical order where the minimum over actions differs from the certified value; the item settles `inconclusive`, `CertificateRejected`. | Test (TC-636) |
| FR-201-AC-2 | §15.3's `LongRun` certificate (component value `1800/1801`, reward `[up] − 1800/1801`, gain 0, bias 0 at `up` and `−2000/1801` at `down`) is accepted; with the bias at `down` changed to `−2001/1801` it is rejected. §15.1's dyadic `Lower` certificate at 64 bits, ranked by remaining horizon, is accepted with side `AtLeast`. | Test (TC-636) |
| FR-201-AC-3 | A `Lower` certificate for §15.4 with its ranking omitted is rejected; one whose identity names a different claim is refused before any re-enumeration; §15.6's `FairTerminates` `Exact` certificate is accepted only after the checker recomputes that no fair end component exists, and the same certificate with its fairness set removed is rejected. With no authored fairness, FR-196's geometric timed policy is checked for derived TS-5 progress and its exact values5/4/3; rank3/2/1 has a positive-probability goal edge to rank0. Missing observation progress, an Observe-as-Delay policy, a finite maximum3 or a fabricated infinite-wait infinity witness is rejected. | Test (TC-636) |
| FR-201-AC-4 | A proof whose certificate the checker accepts settles `proved`; with a checker `max_states` of 2 the check stops and the item settles `Incomplete(LimitReached{…})` naming `max_states`. The same applies while rederiving R/waiting components or checking randomized observation evidence, with the actual consumed value/setting retained. | Test (TC-636) |
| FR-201-AC-5 | An `Opt` model whose initial state has the actions `go`, which activates and then sets `M` to 1 with probability `1/4` and to 3 with `3/4`, and `skip`, which never activates. For `quantile 0.5 of (M) >= 3` (`M` is a placeholder for the claim's parenthesised measure) over every scheduler, the `Upper` certificate with `y = 0` at the initial state and `−1/4` after `go` is accepted with side `AtLeast`: only `skip` is tight, and no activated state is reachable through it. The same claim over a variant whose `go` sets `M` to 1 and 3 with `1/2` each, with `y = 0` at both states, is rejected, naming the activated state after `go`, since `go` is tight. | Test (TC-636) |
| FR-201-AC-7 | Given a `ReplayRequestWire` whose measured encoded size is `B`, when the checker receives caller-configured `ReplayLimits` with `input_bytes = B - 1`, it returns `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded { bound: B - 1, actual: B }))` before byte-provision lookup or recompile. | Test |
| FR-201-AC-6 | Rejection rules and loci: AC-1's certificate with `4/5` replaced by `9/10` is rejected `InequalityFails` at the `ProbabilisticProductState` of that first state; AC-2's `LongRun` certificate with the changed bias, `InequalityFails` at the `down` state; AC-3's `Lower` certificate with its ranking omitted, `Malformed` at `CertificateMember("ranking")` before any re-enumeration; AC-2's `Lower` certificate with one named non-target state's rank removed, `RankMissing` at it, and with that rank raised to the rank of its only positive successor, `RankBroken` at it; AC-1's accepted certificate offered for a `>=` threshold above its initial state's value, `ThresholdMissed` at the initial state; AC-5's rejected variant, `InequalityFails` at the activated state after `go`. Each rejection is written with QSpec FR-331's rule spelling at its `{product_key}` or `{member}` form. | Test (TC-1197) |
| FR-201-AC-8 | Over each of those ACs' own model, the checked-subject form gives the verdict, rule and locus that AC states: AC-1's certificate over §15.4's `Link`, including the `4/5` replaced by `9/10` rejection; AC-2's `LongRun` certificate over §15.3's model and its dyadic `Lower` certificate over §15.1's; AC-3's `FairTerminates` certificates over §15.6's `Coin` model, the fairness-removed one rejected; and AC-5's tight-action cases over its `Opt` model. | Test (TC-636) |
| FR-201-AC-9 | For an admitted acyclic eventual claim, writing and reading the closed profile preserves identity, exact rational values, complete keys, targets and required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator 0, missing ranking, extra fields, padded base64url and a model-only key are rejected. | Test |
| FR-201-AC-10 | A valid document offered for another identity refuses before enumeration; replacing its targets or optimization while retaining valid Bellman rows refuses objective mismatch. A same-model key with a different binding is rejected rather than merged. Altering a value fails equality; removing or breaking positive-value rank progress fails ranking even when equalities hold. | Test |
| FR-201-AC-11 | Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass the profile checker at an attained threshold; a threshold strictly beyond the certified initial value fails. Unbounded cyclic, timed, reward, fair, multi-initial and multi-binding cases, bounded forms outside the admitted subset and non-exact kinds return explicit unsupported with no accepted certificate. Pre-cancelled input and exhausted byte/state/edge/arithmetic budgets stop without a truth value. | Test |
| FR-201-AC-12 | ADR-028 §15.4's bounded Deliver at horizon 2 gives accepted exact minimum AtLeast at 24/25, maximum AtMost at 99/100 and Even workload at 391/400; after one loss the minimum is 4/5 and workload 17/20. Each threshold beyond the certified side fails. Existing unbounded profile documents retain their verdict and canonical bytes. | Test |
| FR-201-AC-13 | A model that stays undelivered with probability 1/2 and delivers with probability 1/2 per step has bounded horizon-2 probability 3/4 despite its raw self-cycle. The initial and one-loss visits share the model key but have distinct full formula-bearing keys and ranks 2 and 1. Horizon 0 accepts iff P holds at position 0. A terminal undelivered state closes to rejection by false extension without enumerating stutter positions. | Test |
| FR-201-AC-14 | For AC-13's valid certificate, replacing a running monitor with a local StateId, null, another remaining horizon, or another binding cannot certify the claim. Changing rank 2 to 3 while retaining valid values fails RankBroken at that full key. An exact document fabricated from a dyadic interval is never emitted; unsupported lower/upper documents and precision stops produce no accepted exact certificate. | Test |
| FR-201-AC-15 | For until, an initial A-and-not-B state has choices a and b reaching B with probability 1/4 and 3/4 respectively, otherwise a neither-A-nor-B state that later reaches B. Exact minimum AtLeast 1/4 and maximum AtMost 3/4 are accepted; an equal-weight resolved workload accepts both sides at 1/2. The failure state has value 0 and its later B successor is not expanded. Thresholds 1/3 for minimum AtLeast, 2/3 for maximum AtMost, and 3/5 AtLeast or 2/5 AtMost for that workload fail ThresholdMissed. A reached B-and-not-A state has value 1; initial B accepts at 1, initial neither-A-nor-B at 0, and terminal A-and-not-B at 0. | Test |
| FR-201-AC-16 | For always, an initial nonterminal P state has choices a and b reaching terminal P with probability 1/4 and 3/4 respectively, otherwise not-P followed by terminal P. Direct-event minimum AtLeast 1/4, maximum AtMost 3/4 and equal-weight resolved workload both sides at 1/2 are accepted. A prefix not-P has value 0 despite later P; initial terminal P has value 1 and initial not-P value 0. Treating the initial P as target, using 3/4 as minimum via an unswapped complement, or changing either boundary value fails checking. The beyond-side thresholds of AC-15 fail here too. | Test |
| FR-201-AC-17 | Canonical state-predicate documents for AC-15 and AC-16 round-trip byte for byte. Replacing until B by a different checked predicate that changes the derived target set, or always P by one that changes failures, prevents acceptance of the original document under that claim even with internally consistent old rows. Changed event/optimization/targets/failures refuses objective mismatch; a foreign S3 binding or altered complete key is not an alias. Altered value, missing coverage or broken positive-value rank fails its existing check. A reachability/v1 document cannot encode these events by relabelling its targets. | Test |
| FR-201-AC-18 | AC-15 and AC-16 obey pre-cancellation and byte/state/edge/arithmetic limits throughout independent admission, original/stopped graph construction and checking, with typed stop/cancellation and no accepted truth value. Cycles in the original graph even after an early success, nonempty fairness, clocks, rewards, multiple initials/bindings, nested temporal forms, unresolved workload and non-exact documents remain explicitly unsupported. No state-predicate proof document is treated as a refutation witness. | Test |

| FR-201-AC-19 | For an acyclic two-step source with named integer reward R, initial choices a (cost 1, then cost 2 to B) and b (cost 4 directly to B) have exact minimum 3 and maximum 4. The minimum AtLeast 3 and maximum AtMost 4 documents accept at equality; thresholds 4 and 3 respectively reject ThresholdMissed. An equal-weight resolved workload has value 7/2 and accepts either bound at 7/2. Values above 1 are accepted; a probability-style target value 1 rejects. | Test |
| FR-201-AC-20 | A draw-dependent authored reward of 2 or 6 with equal probability on an edge entering B gives value 4, including the entry cost. Swapping costs 2 and 6 between those equally probable draws preserves the value 4; the same certificate SHALL accept after independent recomputation when every equation, target, identity and rank remains valid. Changing the costs to 2 and 8 falsifies the certified initial value 4 and SHALL reject; substituting a fixed unit-step cost also SHALL reject because its value 1 fails that equation. Initial B has value 0 even with an enabled positive-cost operation. A reached B absorbs a later positive-cost edge; a reached non-B terminal, including a zero-cost terminal, is explicitly unsupported under FR-198's nonreachability rule. A raw reachable cycle beyond B is unsupported before stopping. | Test |
| FR-201-AC-21 | A checked quantity reward expressed as 2 ms and 6 ms under equal draws gives 4 ms and accepts a source threshold 0.004 s after exact checked-unit normalization. Wrong reward name/model, target set, optimization, binding or full key prevents acceptance; changing the claimed reward/unit identity with the same numeric magnitudes prevents objective acceptance. Equivalent independently rederived costs under the actual admitted subject remain valid; source changes alone do not imply a staleness rejection. A dimensionally wrong source bound preserves the typing refusal. Noncanonical, duplicate, omitted or oversized keys/rows/rationals refuse or stop under the actual caller control; missing/wrong ranks reject, including a zero-cost live state's wrong height. | Test |
| FR-201-AC-22 | The checker independently checks analyze's AC-19 to AC-21 exact documents with no qsl-analyze/provider dependency or reused solver graph/cost labels. Negative/undefined reward evaluation, unsupported profile/kind, exhausted graph/arithmetic/representation budgets, pre-cancellation and cancellation during expansion or reward evaluation preserve typed nonacceptance without truth values. Existing reachability/v1 and state-predicate/v1 canonical bytes/verdicts are unchanged. | Test |

## Dependencies

- ADR-028 SP-5, AR-4, CE-1 to CE-7 (CE-7 records the qualified-core
  placement), FS-9; ADR-013 O-09.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md).

## References

- QSpec half, which owns the certificate wire and the checker's conditions:
  QSpec FR-412 (Linear STD-137).
- ADR-029 (owning record of the qualified core and RU-2) is on its own draft
  branch: Linear QSL-390.
- Owning ticket: Linear QSL-371.
