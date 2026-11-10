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
  - target: ix://agent-ix/quire-spec-language/FR-199
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

### Closed cyclic DTMC eventual document profile

The codec SHALL admit `"quire.probability-objective/cyclic-dtmc-eventual/v1"`
only for untimed infinite-trace unbounded `eventually holds(P)` over a checked
state predicate. This is a separate closed profile: it SHALL NOT change the
admission, canonical bytes or meaning of any existing profile. Its objective
has exactly `profile`, `targets`, `optimization`, `fairness`; the existing
reachability profile's exact document body, strict canonical decoding,
rational representation, full-key ordering/coverage and identity rules apply.
No policy, component partition, formula AST or additional body field is added.

- Admission SHALL independently enumerate the finite original reachable model
  graph, permitting real cycles, with one initial state, at most one actual
  S3 binding, no clocks, rewards or authored fairness, and FR-196's current
  single-post Markov restrictions. Over every scheduler, every original live
  state SHALL have exactly one semantic enabled distribution; merely finding
  a good policy, equal certified action values or equal action distributions
  SHALL NOT admit residual choices. A workload SHALL be fully resolved by its
  authored semantics to one distribution per live state, with no residual
  scheduler choice. Admission applies even beyond an early target. General
  MDPs, other formulas, multiple initials/bindings and non-exact kinds remain
  explicitly unsupported. Minimum AtLeast, maximum AtMost and either workload
  bound are admitted; optimization is independently derived from the claim.
- The checker SHALL independently read P at position 0 and each reached
  original state using the actual checked occurrence, infinite-trace profile
  and binding, then construct the stopped reachable graph by not expanding
  targets. Targets are exactly its reached P states and have value 1.
  Non-target model terminals have value 0: infinite terminal stutter cannot
  make P true later. For graph analysis only, these terminal boundaries and
  targets are absorbing; no synthetic model/product key is introduced.
- The checker SHALL compute positive-support SCCs and closed bottom SCCs on
  that stopped graph using its own bounded graph algorithms. All states in
  non-target closed bottom SCCs SHALL have value 0, including non-target
  terminals. It SHALL independently derive Prob0 as states with no
  positive-support path to a target and require value 0 there too. It SHALL
  check exact harmonic equality `x(s) = sum(p(s,t) * x(t))` at every other
  non-target live state, with normalized positive semantic edge masses and
  all values in [0,1]. These boundaries remove every recurrent non-target
  degree of freedom; the remaining finite transient system has a unique
  solution. The checker SHALL check equations, not solve them or run value
  iteration, and SHALL trust no producer graph, SCC labels or qualitative set.
- Ranking SHALL cover the same stopped graph as values. Targets and Prob0
  states SHALL have rank 0; every other state SHALL have the independently
  derived shortest positive-support edge distance to any target. This
  satisfies CE-3: each positive non-target state has a positive-valued
  lower-rank successor. It does not require every edge to decrease: a
  self-loop may retain its rank. Acyclic edge-decreasing heights and SCC
  condensation heights SHALL NOT substitute for these distances. Wrong
  distances reject `RankBroken` at the first differing complete key;
  missing/extra coverage uses the existing malformed-coverage check.
- Full keys SHALL use FR-196's authoritative model-key and real binding
  owners through `ProbabilityProductKey::for_model_state`, with actually
  absent monitor, accumulator and pending components null. Model-only keys,
  local ids, invented transport or substituted bindings are not aliases.
  Identity mismatch refuses before enumeration; independently changed target
  or optimization refuses objective mismatch. Wrong values reject
  `InequalityFails`; an unmet exact threshold rejects `ThresholdMissed`.
  Equivalent source changes may still accept if all independently derived
  contracts and equations hold; source spelling is not a new identity field.
- Original/stopped exploration, SCC/reverse reachability, distance queues,
  canonical-key materialization, parsing and exact sums SHALL obey caller
  byte, state, edge, storage, work, scalar/rational and cancellation controls.
  Graph passes SHALL use O(V+E) graph work/storage, with representation bytes
  and exact arithmetic charged separately before allocation/operation;
  rank overflow stops under the actual named limit. Pre-cancellation and
  cancellation during any pass preserve typed nonacceptance without truth
  values. The raw-model-key owner budget prerequisite remains open; this
  profile does not invent an upstream API or establish that prerequisite.
  Proof coverage alone is extended: FR-202 native cyclic refutations,
  QSpec FR-413 wire, fairness, timed, long-run and reward coverage remain open.

### Closed cyclic MDP eventual document profile

For checked untimed infinite-trace `probability op c [eventually holds(P)]`
claims over every scheduler, the codec SHALL additionally admit
`"quire.probability-objective/cyclic-mdp-eventual/v1"`. This separate proof
profile SHALL retain exactly the reachability objective's `profile`,
`targets`, `optimization` and `fairness` members and the existing exact
`identity`, `objective`, `kind`, `values` and `ranking` document body.
`kind` SHALL be `"exact"`; `fairness` SHALL be empty. No policy, action
ordinal, action-key transport, qualitative-set field or formula AST is added.

- Admission SHALL require a finite untimed Markov subject, one initial
  state, at most one actual S3 binding and the existing single-post-per-draw
  restrictions. The checker SHALL reconstruct EVERY enabled semantic action
  distribution from the checked subject rather than select a producer
  policy or merge different actions into one workload distribution. Multiple
  choices and real cycles are admitted. Source `Every` AtLeast SHALL select
  `"minimum"`; `Every` AtMost SHALL select `"maximum"`. This profile adds
  no minimum AtMost or maximum AtLeast source mode. Workload, fairness,
  rewards, clocks, other temporal forms, pending multi-post choice, interval
  kinds and refutation documents remain outside this profile.
- Preserve full original-subject identity, action/draw and Markov admission,
  including beyond an early P, under the actual caller controls. The checker
  SHALL independently evaluate the checked P occurrence and binding while
  traversing the initial graph BEFORE first P, stopping outgoing expansion
  at reached P. Objective targets, values and ranks SHALL cover precisely
  this stopped graph. A state reachable only after P contributes no row;
  query-P evaluation solely after P is not required. Earlier subject
  admission refusals remain authoritative. Full binding-bearing model keys
  SHALL use the existing FR-196 owner, with absent monitor, accumulator,
  fairness context and pending components actually null, not invented ids.
- Every value SHALL be an exact rational in [0,1]. A target SHALL have
  value 1 and a non-target terminal value 0. At each other state s and
  independently rederived action a, compute
  `Q_a(v,s) = sum_t p(s,a,t) * v(t)` with exact nonnegative probabilities
  whose complete action mass is 1. For minimum, check
  `v(s) = min_a Q_a(v,s)`; for maximum, check
  `v(s) = max_a Q_a(v,s)`. A tight action means exact equality
  `Q_a(v,s) = v(s)`, not closeness under a tolerance. All source actions
  SHALL enter these comparisons, including nonselected and tied actions.
- Let Z be the offered zero-valued non-target states. Every target and
  every state in Z SHALL have rank 0; every positive-valued non-target
  state SHALL have positive natural-number rank. Lower-rank successors
  MAY have value 0: the progress boundary is targets UNION Z, not targets
  alone and not positive-valued successors only. A probability-zero edge
  SHALL NOT establish progress or positive-support closure.
- For minimum, EVERY enabled action at EVERY positive-valued non-target
  state SHALL have SOME positive-probability successor of strictly smaller
  rank. Every live state in Z SHALL admit SOME enabled action whose entire
  positive-probability support lies in Z. This zero-support action also
  follows from exact min Bellman equality and nonnegative values, but the
  checker SHALL establish that condition rather than trust an asserted
  `Prob0` label. A false terminal needs no enabled action; its implicit
  stutter remains in Z. No universal progress obligation applies to Z.
- For maximum, EVERY positive-valued non-target state SHALL admit SOME
  TIGHT maximizing action with SOME positive-probability lower-rank
  successor. Progress under a nontight action SHALL NOT satisfy this rule.
  Exact max Bellman equality at a live state in Z forces ALL enabled
  actions to remain in Z; the checker SHALL retain that consequence.
  These ranks certify progress and need not equal producer BFS distances
  or SCC heights. Other edges may retain or increase rank.
- The checker SHALL verify these conditions without solving equations,
  importing producer values/classifications, policy iteration or a policy
  witness. In a finite MDP, minimum progress makes target-or-Z absorption
  almost sure under every policy; choose tight minimizing actions outside
  Z and zero-support actions in Z to realize v. Maximum progress supplies
  a proper tight policy outside Z, realizing v, while Bellman inequalities
  bound every other policy from above, including improper policies. Thus
  both values are exact extrema, not arbitrary Bellman fixed points.
  The required finite progress rankings exist for the genuine optimal
  values; a tie to a waiting self-loop is not an attaining-policy proof.
- Independently check the admitted initial threshold inclusively. Existing
  S3 probability endpoint admission remains unchanged: source thresholds
  0 and 1 are refused, although transported values 0 and 1 are valid.
  Wrong Bellman values or zero-support conditions SHALL reject
  `InequalityFails` at the first failing complete key; wrong boundary or
  progress ranks SHALL reject `RankBroken` there. A valid exact vector
  missing its initial threshold SHALL reject `ThresholdMissed`. Identity
  conflict refuses before enumeration; changed targets, optimization,
  fairness or profile refuses objective mismatch. Malformed/duplicate/
  missing/out-of-order data and foreign-binding or model-only keys retain
  existing typed rule/locus behavior, including `Malformed` at `values` or
  `ranking` for structural coverage errors. No failure or stop yields an
  accepted token or a refutation.
- Original/stopped expansion, all action/draw reconstruction, internal
  raw/current/candidate/post/full keys, exact action sums, zero support,
  progress passes and retained rows SHALL preserve the real caller's byte,
  state/edge, representation/arithmetic and cumulative work/storage limits
  and actual cancellation cause. Undefined P or source evaluation SHALL
  retain typed nonacceptance rather than false, zero or a missing action.
  Zero direct `over` instances retain `UnsupportedBinding` and unsupported
  disposal without invented evidence. Existing profiles retain their
  bytes, admission and verdicts; an eligible existing acyclic or cyclic
  DTMC producer SHALL retain its existing profile preference. No completed
  runtime, full repository/all-expansion FR-201-AC-30 assurance or shared
  FR-413 witness transport is claimed by this specification.

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

### Closed cyclic DTMC expected-reward document profile

For checked `expected accumulate R until holds(B) op c` claims under a
fully resolved workload, the codec SHALL additionally admit
`"quire.probability-objective/dtmc-expected-reward/v1"`. This finite-only
proof profile SHALL retain exactly the acyclic reward objective's
`profile`, `targets`, `reward`, `unit`, `optimization` and `fairness`
members; `optimization` SHALL be `"workload"` and `fairness` empty. The
document SHALL retain the existing exact `identity`, `objective`, `kind`,
`values` and `ranking` members, with kind `"exact"`. No policy, component,
action transport, second reward expression or new unit algebra is added.

- Admission SHALL require an untimed finite DTMC subject, one initial
  state, at most one actual S3 binding, fully resolved workload and the
  existing one-post-state-per-draw Markov restrictions. General MDP or
  every-scheduler, authored fairness, timed, multi-post, interval and
  refutation/infinity documents remain outside this profile. Preserve full
  original-subject validity, identity and Markov/workload admission; this
  profile SHALL NOT inherit the acyclic reward profile's original-DAG or
  all-original-terminals-are-B restrictions. Original graph admission and
  its caller budgets are not waived by an early B. Existing model-admission
  refusals, including FR-187's negative-reward refusal, SHALL retain their
  actual cause before profile evaluation; post-admission unsupported or
  undefined evaluation SHALL retain typed nonacceptance, not reward zero.
- The checker SHALL independently rebuild the graph reachable from the
  admitted initial state BEFORE the first B, evaluating B on that stopped
  traversal. A reached B SHALL absorb, with V zero and rank zero, without
  expanding its outgoing steps for the claim. A state reachable only after
  B SHALL have no value/rank row or contribution to the accumulated reward;
  a post-B cycle alone SHALL NOT invalidate this profile. Objective targets
  SHALL be exactly the B keys in this stopped graph. Query-expression
  evaluation solely after B is not required; earlier original-subject
  admission refusals remain authoritative.
- `reward` and `unit` SHALL use the acyclic profile's actual checked
  FR-196 model/reward identity, selected comparison unit, exact conversion
  graph and null/declared/compound UnitId descriptor. Independently
  evaluate every relevant semantic transition and typed draw BEFORE
  coalescing, preserving operation/receiver/non-random-argument identity.
  Missing R contributes zero; admitted costs are defined, finite and
  nonnegative integers or quantities. At every non-B state s check
  `V(s) = sum_e p(e) * (r_R(e) + V(to(e)))` under the actual single
  workload distribution. Include the cost on the step entering B exactly
  once, and no cost on a step leaving B. Values SHALL be nonnegative exact
  rationals without an upper bound of 1; they are expressed in the checked
  comparison unit, not probabilities.
- Values and natural-number ranks SHALL cover exactly the stopped graph
  in existing canonical complete-key order. Every non-B state SHALL have
  positive rank and SOME independently rederived positive-probability edge
  to a strictly smaller rank. This applies EVEN when its value and every
  step reward are zero. It does not require a positive successor VALUE,
  and it does not require every edge to decrease: retry self-loops and
  other cyclic edges may retain or increase rank. These are properness
  ranks, not exact acyclic heights. The checker SHALL verify this condition
  rather than trust producer `Prob1` labels or reuse the generic rank
  exemptions for reward or zero-value states.
- On a finite DTMC, the rank condition excludes every closed non-B class:
  a minimum-rank member of such a class would have a positive edge leaving
  it to a lower rank. Thus B is reached almost surely, expected hitting
  time and finite admitted reward are finite, and reward Bellman equality
  has a unique finite solution. This is the independent finiteness check;
  the checker SHALL solve no linear system, run no value/policy iteration
  or import producer graph/cost/properness classifications. FR-198 still
  assigns positive infinity to nonreachability, including zero-cost
  nontermination. An offered V=0/rank=0 finite document for a closed
  non-B zero-cost class SHALL reject `RankBroken` at its first complete key,
  not accept through its otherwise true Bellman equations. A producer
  encountering infinity SHALL emit no document of this finite-only profile
  and preserve the existing typed unsupported outcome; infinity SHALL NOT
  become a finite rational, `broken()` result or FR-202 refutation.
- Either workload bound side SHALL be checked inclusively at the initial
  complete key under existing S3 expected-reward admission. In particular,
  zero is a valid expected-reward threshold; the probability-threshold
  endpoint policy SHALL NOT be imposed on reward bounds. Wrong values
  SHALL reject `InequalityFails`, wrong ranks `RankBroken`, and an otherwise
  valid document missing its bound `ThresholdMissed`, at the first
  applicable complete key. Conflicting identity SHALL refuse before graph
  enumeration; changed reward/unit/targets/optimization/profile SHALL
  refuse objective mismatch. Malformed, duplicate, missing, out-of-order,
  foreign-binding or model-only key/member data retain existing typed
  rule/locus behavior, including `Malformed` at `values` or `ranking` for
  coverage errors. No rejected/stopped check yields an accepted token.
- Original/stopped expansion, actual key construction, reward/unit and B
  evaluation, graph/rank passes, retained rows and exact sums SHALL retain
  the real caller's byte/state/edge, representation/arithmetic,
  cumulative work/storage and cancellation controls with typed causes.
  Zero direct `over` instances retain `UnsupportedBinding` and unsupported
  EN-5 disposal with no invented evidence. Existing profiles retain their
  canonical bytes, admission and verdicts; the producer SHALL prefer the
  existing acyclic reward profile when it already applies. No completed
  implementation, full FR-201-AC-30/all-expansion memory qualification or
  shared FR-413 witness contract is claimed by this specification.

### Closed unweighted DTMC long-run document profile

For a checked unweighted `long-run fraction holds(P)` claim under an
authored workload in the untimed infinite-trace profile, the codec SHALL
admit the separate objective profile
`"quire.probability-objective/dtmc-long-run/v1"`. The objective SHALL have
exactly `profile`, `predicate_states`, `optimization` and `fairness`;
`optimization` SHALL be `"workload"` and `fairness` SHALL be the empty
array. `predicate_states` SHALL be the complete ascending set of reachable
complete product keys where the checked state predicate P holds. It is
extensional evidence, not a target boundary: the checker SHALL evaluate P
independently on the entire original reachable graph and compare the set.
It SHALL rederive the workload from the checked claim and admitted subject,
not from that set, values, component rows or a producer's graph.

- Admission SHALL require one initial state, at most one actual S3
  binding, no clocks or authored fairness, and FR-196's existing Markov
  single-post successor restrictions. The finite original graph MAY
  cycle. At every live state the authored workload SHALL fully resolve
  semantic operation/receiver/non-random-argument choices and typed draws
  to one normalized distribution, without residual scheduler or post-state
  choice. A terminal SHALL use the existing infinite-trace stutter
  semantics: its same complete key has a probability-one self-loop and
  its P label is retained. No synthetic state/key or operation identity
  SHALL be invented for that implicit continuation. P states SHALL NOT
  absorb an enabled outgoing transition. The producer and checker SHALL
  preserve actual binding admission. With zero `over` instances, direct
  claim binding SHALL retain `UnsupportedBinding`, and the direct EN-5
  item SHALL retain its unsupported disposition without a terminal verdict,
  certificate or counterexample. This profile SHALL NOT invent a vacuous
  proof, binding, state or document, or prescribe higher-level planner
  behavior outside that direct entry.
- The document SHALL have exactly `identity`, `objective`, `kind`,
  `values` and `components`, with `kind` equal to `"long-run"`; `ranking`
  and `policy` SHALL be absent. Identity, exact rationals, full six-part
  binding-bearing model keys with null absent monitor/accumulator/pending,
  strict canonical JSON and unpadded base64url retain the existing owner
  contracts. `values` SHALL cover exactly the entire independently rebuilt
  original reachable graph, ordered by decoded complete key octets, with
  values in [0,1]. Reachability ranks or target labels SHALL NOT be used
  to substitute for long-run evidence.
- Each component SHALL use QSpec FR-331's existing exact
  `{value, gain, bias}` shape. `value` is `rho_C` in [0,1]; `gain` SHALL
  equal `rho_C`, the average original unweighted indicator reward in
  FR-199's result. `bias` SHALL have exactly the existing `{state, value}`
  rows: every member of that component once, sorted by decoded complete
  key, with signed exact rational bias values. Components SHALL be sorted
  by their first bias key. Bias keys identify the claimed membership;
  no component id, producer SCC number, trusted partition or additional
  top-level field is introduced. Empty components, duplicate/out-of-order
  rows, duplicate components, missing members and unknown members SHALL
  be rejected rather than normalized.
- The checker SHALL independently derive every positive-support SCC and
  closed bottom SCC (BSCC) of the original graph with its own bounded
  graph algorithms. The component list's bias-key sets SHALL match
  exactly those BSCCs, disjointly and completely; transient states SHALL
  occur in no component. This comparison SHALL NOT trust a producer's
  partition, even when its per-row equalities hold. There SHALL be at
  least one BSCC in the complete finite graph. In each BSCC C the checker
  SHALL require bias zero at C's first complete key and check, at every
  s in C, `gain_C = rho_C` and
  `h(s) = [P(s)] - rho_C + sum_t p(s,t) * h(t)`.
  Thus the centered reward `[P]-rho_C` has gain `gain_C-rho_C = 0`,
  exactly CE-3's condition. The transported gain remains FR-199's
  original-reward gain, not a second independently chosen component
  value. A different additive bias normalization SHALL NOT be silently
  rewritten into canonical evidence. The checker SHALL solve no equation,
  compute no stationary distribution and run no numerical iteration.
- Every state in C SHALL have `V(s) = rho_C`. At each transient state the
  checker SHALL check `V(s) = sum_t p(s,t) * V(t)` using the original
  workload distribution. These are the exact expected-component-value
  conditions: after all independently derived BSCCs are fixed, the
  remaining finite states are transient and the harmonic extension is
  unique. No reachability positive-value rank is required for this
  `long-run` kind; neither a nested certificate nor a new transport member
  is introduced. Exact `AtLeast` and `AtMost` proofs SHALL compare the
  initial V against the checked threshold inclusively, and SHALL be
  emitted only after independent acceptance. Thresholds and S3 constant
  evaluation SHALL use the existing exact checked conversion and caller
  limits. A valid document does not by itself become an FR-202 refutation.
- Conflicting identity SHALL refuse before enumeration. A changed
  predicate-state set, optimization or profile for an otherwise admitted
  claim SHALL refuse as objective mismatch before acceptance. Structural
  coverage/ordering/member failures SHALL reject `Malformed` at member
  `values` or `components` as appropriate. A component partition mismatch
  SHALL reject `InequalityFails` at the smallest complete key whose
  claimed component membership differs from its independently rederived
  BSCC membership, treating a transient state's membership as absent.
  An entirely missing component therefore uses its first key. A wrong
  component gain, normalization or
  state equation SHALL reject `InequalityFails` at the first failing key
  in complete-key order; a component-wide gain failure uses its first key.
  A wrong initial threshold SHALL reject `ThresholdMissed` at the initial
  complete key. No altered model-only key or foreign binding is an alias.
- Source/document admission, exact key construction, complete graph
  expansion, predicate/workload evaluation, SCC and membership passes,
  bias/value storage and exact arithmetic SHALL obey actual caller byte,
  state/edge, work/storage and scalar/rational limits and cancellation.
  Exhaustion or cancellation SHALL retain its typed stop and actual
  control/consumption, without acceptance or truth values. Undefined
  reachable predicate/workload evaluation SHALL preserve the existing
  typed undefined/refusal path and SHALL NOT be coerced to false, zero
  or a missing component. Weighted forms are outside this profile;
  the existing `Unsupported(ZeroWeightComponent)` obligation for those
  forms remains in FR-199 and is not converted into an unweighted proof.
  General MDP/every-scheduler, fairness, timed, multi-post, weighted,
  interval and refutation profiles, including shared FR-413 wire, remain
  explicitly unsupported here. All earlier profiles retain their bytes,
  admission and verdicts. No upstream budget API is claimed delivered by
  this specification.

### Closed weighted DTMC long-run document profile

For a checked `long-run fraction holds(P) weighted by R` claim under an
authored workload, the codec SHALL admit the separate objective profile
`"quire.probability-objective/dtmc-weighted-long-run/v1"`. Its objective
SHALL have exactly `profile`, `predicate_states`, `weight`, `unit`,
`optimization` and `fairness`. `optimization` SHALL be `"workload"` and
`fairness` SHALL be empty. `predicate_states` retains the unweighted
profile's independently rederived complete original-graph key set.
`weight` SHALL be FR-196's actual `[model_node,identifier]` for checked R;
`unit` SHALL be its actual null, declared or compound UnitId descriptor.
The checker SHALL resolve both under the checked model and compare them
with the objective before accepting evidence. A display-name alias or
another same-dimension UnitId SHALL NOT substitute for that identity.

- This profile SHALL retain the unweighted profile's untimed, finite,
  one-initial/at-most-one-binding DTMC admission, fully resolving workload,
  complete original graph, positive-support BSCC partition, canonical
  component ordering, bias normalization, strict transport and independent
  threshold checks. Predicate-true states SHALL NOT absorb their outgoing
  transitions. Zero direct `over` instances retain `UnsupportedBinding`
  and the unsupported EN-5 disposition with no invented evidence. General
  MDP/every-scheduler, fairness, timed, multi-post, interval and refutation
  forms, including shared FR-413 wire, remain outside this profile.
- The checker SHALL independently evaluate the named checked reward on
  every reachable semantic transition and typed draw, using FR-196's
  operation/receiver/non-random-argument identity and the checked expression.
  An operation without R contributes zero. Admitted weights SHALL be
  defined, finite, nonnegative integers or quantities of R's checked type;
  quantities use the checked weight's unit and existing exact conversion
  graph. The threshold and fraction remain dimensionless, not quantities
  in that unit. Integer representation SHALL use the existing representation
  limit; numeric weight and weighted-row arithmetic SHALL still obey the
  existing scalar/rational arithmetic limits. Existing model-admission
  refusals, including FR-187's `invalid_probabilistic_model`/`negative-reward`,
  SHALL be preserved before profile evaluation; negative weights or
  unsupported reward types encountered after admission SHALL preserve
  explicit unsupported outcomes; undefined predicate, reward or workload
  evaluation SHALL preserve its typed nonacceptance, even in transient
  states. No invalid weight SHALL be coerced to zero. Complete actual
  draws SHALL be evaluated before any coalescing that could erase their
  different reward values.
- Write `p(e)` for the independently rederived normalized probability of
  an outgoing draw e from s and `r(e)` for its independently rederived
  magnitude in the checked weight unit. The checker SHALL compute
  `w(s) = sum_{e from s} p(e) * r(e)`. FR-199 weights the position by the
  step that leaves it: the centered immediate reward is
  `w(s) * ([P(s)] - rho_C)`, using the CURRENT state's predicate. It is
  neither `[P(to(e))]` nor an unweighted indicator. Implicit terminal
  stutter retains the original key and P label, contributes named reward
  zero and invents no reward-bearing operation.
- Before checking component evidence, the checker SHALL establish that
  every independently rederived reachable BSCC has at least one state
  with `w(s) > 0`. For a finite positive-support irreducible chain with
  nonnegative weights this is exactly positive stationary denominator;
  every state has positive stationary mass. A BSCC with all w zero has
  no defined weighted fraction and SHALL retain the existing
  `Unsupported(ZeroWeightComponent)` outcome, identifying that component
  under the existing diagnostic contract. This includes a reachable
  terminal stutter, even when P holds there, and any reachable zero-weight
  BSCC alongside a positive-weight one. No component is discarded, assigned
  value zero, or accepted through otherwise satisfiable zero equations.
  Zero-weight transient states alone SHALL NOT trigger this outcome.
- The document SHALL retain exactly `identity`, `objective`, `kind`,
  `values` and `components`, with kind `"long-run"` and unchanged
  `{value,gain,bias}` component and `{state,value}` bias-row shapes. Values
  and component values SHALL be exact rationals in [0,1]. Transported
  `gain_C` SHALL equal `rho_C`, FR-199's weighted-ratio result, not the
  numerator payoff `pi_C(R*[P])` or the centered reward's zero mean.
  Bias magnitudes SHALL be signed exact rationals in the selected weight
  unit, with zero at C's first complete key. At EVERY s in C the checker
  SHALL check
  `h(s) = w(s) * ([P(s)] - rho_C) + sum_t p(s,t) * h(t)`.
  Stationary averaging therefore gives
  `rho_C = pi_C(w*[P]) / pi_C(w)`: the checked denominator is positive.
  The checker SHALL compute no stationary distribution, solve no equation
  and reuse no producer partition, cost table, linear algebra or iteration.
  For weights multiplied by a positive scalar, rho stays the same while
  bias magnitudes scale; they SHALL NOT be copied unchanged from the
  unweighted profile.
- Every state in C SHALL have V equal to rho_C. Transient V rows SHALL
  satisfy the unweighted ORIGINAL transition harmonic equation
  `V(s) = sum_t p(s,t)*V(t)`, giving FR-199's expected component fraction,
  not a ratio of globally combined numerators and denominators. All BSCCs
  are independently matched before that check, so this extension is unique.
  Finite transient weights do not enter its recurrence. Inclusive AtLeast
  and AtMost proof checks SHALL use the checked dimensionless threshold
  and existing S3 admission and exact arithmetic; no endpoint-admission
  policy is changed. A document that misses its threshold rejects
  `ThresholdMissed` at the initial complete key and is not a refutation.
- Conflicting identity SHALL refuse before enumeration. Changed weight,
  unit, predicate set, optimization or profile SHALL refuse as objective
  mismatch. Structural member, coverage and ordering errors SHALL retain
  `Malformed` at `values` or `components`; mismatched BSCC membership,
  gain, normalization, equation or V SHALL retain the unweighted profile's
  first-complete-key `InequalityFails` rule. Model-only or foreign-binding
  keys are not aliases. Equivalent independently rederived draw costs may
  accept under a matching identity/objective; source changes alone are not
  a staleness rule. Source/document/key, graph, reward/unit evaluation,
  SCC, membership, value/bias storage and exact arithmetic SHALL retain
  actual caller byte/state/edge/work/storage/representation controls and
  cancellation with typed causes and no accepted truth value on a stop.
  Earlier profile bytes, meanings and verdicts SHALL remain unchanged.
  This contract is not a claim of completed implementation or whole-runtime
  allocation qualification.

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
| FR-201-AC-20 | A draw-dependent authored reward of 2 or 6 with equal probability on an edge entering B gives value 4, including the entry cost. Swapping costs 2 and 6 between those equally probable draws preserves the value 4; the same certificate accepts after independent recomputation when every equation, target, identity and rank remains valid. Changing the costs to 2 and 8 falsifies the certified initial value 4 and rejects; substituting a fixed unit-step cost also rejects because its value 1 fails that equation. Initial B has value 0 even with an enabled positive-cost operation. A reached B absorbs a later positive-cost edge; a reached non-B terminal, including a zero-cost terminal, is explicitly unsupported under FR-198's nonreachability rule. A raw reachable cycle beyond B is unsupported before stopping. | Test |
| FR-201-AC-21 | A checked quantity reward expressed as 2 ms and 6 ms under equal draws gives 4 ms and accepts a source threshold 0.004 s after exact checked-unit normalization. Wrong reward name/model, target set, optimization, binding or full key prevents acceptance; changing the claimed reward/unit identity with the same numeric magnitudes prevents objective acceptance. Equivalent independently rederived costs under the actual admitted subject remain valid; source changes alone do not imply a staleness rejection. A dimensionally wrong source bound preserves the typing refusal. Noncanonical, duplicate, omitted or oversized keys/rows/rationals refuse or stop under the actual caller control; missing/wrong ranks reject, including a zero-cost live state's wrong height. | Test |
| FR-201-AC-22 | The checker independently checks analyze's AC-19 to AC-21 exact documents with no qsl-analyze/provider dependency or reused solver graph/cost labels. Negative/undefined reward evaluation, unsupported profile/kind, exhausted graph/arithmetic/representation budgets, pre-cancellation and cancellation during expansion or reward evaluation preserve typed nonacceptance without truth values. Existing reachability/v1 and state-predicate/v1 canonical bytes/verdicts are unchanged. | Test |
| FR-201-AC-23 | A DTMC s with probabilities 1/3 each to s, target g and non-target absorbing f has exact eventual probability 1/2; values (s,g,f)=(1/2,1,0), ranks (1,0,0) accept minimum AtLeast and maximum AtMost at 1/2. Target g absorbs despite its original outgoing cycle. An initial target has exact value 1 and accepts minimum AtLeast 1/2; a non-target terminal has exact value 0 under infinite stutter and accepts maximum AtMost 1/2. Thresholds 3/5 AtLeast and 2/5 AtMost reject ThresholdMissed for the mixed case. A target-free two-state closed cycle has exact values/ranks 0 and accepts maximum AtMost 1/2; assigning its states 1/2 despite valid harmonic equalities rejects. | Test |
| FR-201-AC-24 | In a transient SCC, a goes to b with probability 1, b to a with 1/2, target g with 1/4 and closed failure f with 1/4. Values a=b=1/2, g=1, f=0 and ranks a=2,b=1,g=f=0 accept; both SCC members having condensation rank 1 rejects RankBroken. A fully authored workload at s choosing equally between deterministic return-to-s and a distribution g:1/3,f:2/3 resolves to s:1/2,g:1/6,f:1/3 and accepts exact value 1/3, rank 1 on either workload bound at equality; 2/5 AtLeast and 1/4 AtMost reject. | Test |
| FR-201-AC-25 | AC-23/24 documents round-trip canonically; wrong values, distances, missing/extra coverage, foreign binding, altered complete model key, model-only key, changed derived targets/optimization or mismatched identity prevent acceptance. Changed source probabilities that falsify equations reject; equivalent source changes may accept. A live state with stay and exit scheduler choices refuses as unsupported even if a producer supplies a policy or equal values, including when that state lies beyond an early target. Unresolved workload, fairness, clocks, rewards, other formulas, multi-post and multiple bindings/initials remain unsupported. Existing profile bytes and verdicts are unchanged. | Test |
| FR-201-AC-26 | AC-23/24 independently check with no solver dependency. Source/document/key byte, state/edge, graph work/storage and scalar/rational budgets are exercised at exhaustion, including SCC, reverse reachability and distance construction; actual typed limit and consumption are retained. Pre-cancellation and cancellation during each pass or exact sum stop without acceptance or truth values; no fabricated upstream key-budget API or cyclic refutation is claimed. | Test |
| FR-201-AC-27 | Under Steady, the original up/down chain with up-to-up 1999/2000 and down-to-up 9/10 accepts a long-run document with one component value and gain 1800/1801, bias 0 at up and -2000/1801 at down (choose the fixture's canonical key order up before down), and both state values 1800/1801. Both workload bound sides accept at equality; AtLeast 1801/1802 and AtMost 999/1000 reject ThresholdMissed. These comparison thresholds lie strictly between 0 and 1 under the existing S3 admission policy. Changing the bias, setting transported gain to 0, or replacing long-run evidence by reachability values prevents acceptance. No P state absorbs its original transitions. | Test |
| FR-201-AC-28 | A transient self-cycling s goes to s with 1/2, P-terminal g with 1/6 and not-P-terminal f with 1/3. Independent terminal stutter gives BSCCs {g} and {f}, respective component value/gain 1 and 0 and bias 0; values s=1/3,g=1,f=0 accept both workload bounds at 1/3. A two-state deterministic cycle alternates P and not-P and accepts rho=gain=1/2 with normalized biases 0,-1/2 in that key order, despite periodicity. Merging/splitting/omitting a BSCC, adding a transient bias member or changing transient V rejects; harmonic values alone cannot certify a wrong recurrent value. | Test |
| FR-201-AC-29 | AC-27/28 documents round-trip canonically through a fresh checker process with no analyzer/provider dependency. Noncanonical rational/JSON/key encodings, duplicate or incomplete rows/components, foreign bindings, model-only keys, wrong normalization or missing components prevent acceptance. Identity conflict refuses before enumeration; a changed derived predicate set/optimization refuses objective mismatch. Equivalent source changes that preserve every checked condition may accept. Earlier profile bytes and verdicts remain unchanged. | Test |
| FR-201-AC-30 | Pre-cancellation and exhaustion/cancellation during admission, full-key creation, workload/predicate evaluation, SCC/membership passes and exact sums retain typed nonacceptance without truth values under the actual caller control. Reachable undefined P/workload evaluation is never relabelled false/zero. Weighted claims emit no document of this profile; the separate existing FR-199 weighted zero-weight control retains Unsupported(ZeroWeightComponent). Unresolved workload, every-scheduler/MDP, fairness, clocks, multi-post, multiple initials/bindings and unsupported kind/profile remain explicitly unsupported. Zero over instances retain UnsupportedBinding at direct claim binding and an unsupported disposition at the direct EN-5 item, with no terminal verdict, certificate, counterexample or fabricated binding. | Test |
| FR-201-AC-31 | FR-199-AC-3's Steady availability graph, with up-to-up 1999/2000, down-to-up 9/10, outgoing duration 1 at up and 3 at down, accepts the weighted profile with component value/gain 600/601, both state values 600/601, bias 0 at up and -2000/601 at down in that canonical key order. Both bound sides accept at 600/601; AtMost 999/1000 accepts and AtLeast 999/1000 rejects ThresholdMissed, while AtMost 599/600 rejects. Unweighted value/gain 1800/1801 or bias -2000/1801 fails. | Test |
| FR-201-AC-32 | A periodic chain alternates P state a and not-P state b; at a two equally probable draws to b have R magnitudes 1 and 3, and b's step to a has R 1. The weighted component accepts rho=gain=2/3 and biases (a,b)=(0,-2/3). Swapping the two a draw weights preserves acceptance; changing 3 to 5 falsifies that certificate. A zero-weight transient s entering this component and an all-P state c with probabilities 1/4 and 3/4, where c has an explicit probability-one self-step of R 1, accepts V(s)=11/12; a globally pooled numerator/denominator substitute 8/9 rejects. A reachable all-zero-weight BSCC, alone or beside a positive-weight BSCC, and any reachable terminal stutter settle Unsupported(ZeroWeightComponent), with no certificate. | Test |
| FR-201-AC-33 | AC-31/32 documents round-trip canonical bytes through a fresh independent checker without analyzer/provider linkage. Wrong checked weight name/model or actual unit, changed predicate set/optimization/profile, foreign binding/model-only keys, malformed rational/JSON/key data and duplicate/missing/out-of-order component/value/bias members prevent acceptance with their specified typed rule/locus. Independently checked equivalent quantity expressions normalize through the actual weight UnitId; another same-dimension descriptor cannot replace it. Gain 0, a numerator payoff in gain, unscaled unweighted bias, false BSCC coverage or changed transient values reject. Older unweighted/reward/eventual profiles keep their bytes/verdicts. | Test |
| FR-201-AC-34 | Pre-cancellation and exhaustion/cancellation during actual admission, key construction, graph/draw expansion, predicate/weight/unit evaluation, SCC/membership and exact weighted sums retain the actual typed limit/cancellation cause with no acceptance. Undefined or negative reachable weights, undefined predicates/workload and unsupported reward types preserve typed nonacceptance rather than zero. Unsupported shape/kind/fairness/every-scheduler cases issue no weighted proof. Zero direct over instances preserve UnsupportedBinding/unsupported with no invented evidence. Integer representation and exact weight arithmetic retain their respective existing limits, without imposing a probability-value range on weights. No weighted claim is relabelled as an unweighted profile or an FR-202 refutation. | Test |
| FR-201-AC-35 | A DTMC retry state s has self and B-entering draws each of probability 1/2 and R=1 on both. The cyclic reward profile accepts V(s)=2, V(B)=0 and ranks (s,B)=(1,0), at both workload equality bounds. AtMost 3/2 and AtLeast 5/2 reject ThresholdMissed. Changing only entry-B R to 3 accepts V(s)=4; offering 2 rejects InequalityFails. Swapping equally probable draw costs preserves 4, while changing their mean falsifies it. Costs 2 ms on self and 6 ms on entry give 8 ms and accept an equivalent 0.008 s bound through actual checked conversion; declared/compound descriptors retain their actual identity rather than a fixed step cost or display alias. | Test |
| FR-201-AC-36 | A zero-cost retry with positive exit probability accepts V(s)=V(B)=0 and ranks (1,0), including either bound at 0. Giving s rank 0 rejects RankBroken despite zero values. A closed non-B zero-cost self-loop or cycle rejects otherwise Bellman-valid finite V=0 evidence as RankBroken; the producer preserves FR-198 infinity without a finite document. Initial B accepts only its zero value/rank even when its original outgoing operation has positive cost. An admitted A-to-B subject with a positive-cost cycle reachable solely after B accepts first-hit evidence with rows only for A and B; adding after-B value/rank/target keys rejects the specified coverage/objective rule. | Test |
| FR-201-AC-37 | AC-35/36 documents round-trip canonically through a fresh checker without analyzer/provider linkage. Changed actual reward name/model/unit, target set, optimization/profile or binding prevents acceptance; dimension-equivalent foreign UnitIds are not aliases. Noncanonical rational/JSON/key data, malformed or duplicate/missing/out-of-order rows, model-only keys, false Bellman values, and absent/wrong progress ranks retain precise rule/locus diagnostics. The old acyclic reward and probability/long-run profile bytes/verdicts remain unchanged; an eligible acyclic producer keeps its existing profile. | Test |
| FR-201-AC-38 | Pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic exhaustion during original/stopped admission, raw/internal/full keys, draw/B/reward/unit evaluation, progress checking and exact sums preserve the actual typed cause with no accepted truth. Earlier model refusals and deferred undefined/negative evaluation are not coerced to zero. Unsupported kind/shape/fairness/every-scheduler, multi-post or refutation/infinity requests emit no finite cyclic reward proof. Direct zero over instances retain UnsupportedBinding/unsupported without a fabricated binding, terminal verdict, certificate or counterexample. | Test |
| FR-201-AC-39 | A live state s has actions a with probabilities (self,target,fail)=(1/2,1/5,3/10) and b with (1/2,2/5,1/10). With target value 1 and false-terminal fail value 0, the new profile accepts minimum 2/5 under AtLeast 2/5 and maximum 4/5 under AtMost 4/5, with rank(s)=1 and boundary ranks 0. Minimum AtLeast 1/2 and maximum AtMost 3/4 reject ThresholdMissed. Changing an otherwise matching actual action probability, omitting a source action from the extremum or replacing separate actions by their averaged distribution cannot verify the old vector. Separately let S choose A=(1/2 fail,1/2 T) or B=(1/4 fail,3/4 T), and let T step to target with probability 2/3 and S with 1/3. Minimum (vS,vT)=(2/5,4/5) and maximum (2/3,8/9) accept with rank(S)=rank(T)=1 and boundary ranks 0: S progresses only to value-zero fail, while its positive successor T has equal rank. | Test |
| FR-201-AC-40 | For wait-to-self versus go-to-target, the actual minimum is 0. A bogus minimum v(s)=1/2 satisfies Bellman equality but rejects RankBroken because wait supplies no lower-rank successor, even though go does. For wait-to-self versus attempt to target/fail with probabilities 4/5 and 1/5, the maximum is 4/5: bogus v(s)=9/10 is a Bellman fixed point through wait but rejects RankBroken because only the nontight attempt progresses; at v(s)=4/5 the tight attempt and rank 1 accept under AtMost 19/20. Zero-valued live minimum states require a closed zero-support action; maximum zeros require all action supports to remain zero. Lower-rank value-zero successors count, but probability-zero successors do not. | Test |
| FR-201-AC-41 | AC-39/40 valid documents round-trip canonical bytes through a fresh checker without analyzer/provider linkage. Target/non-target-terminal boundaries, stopped targets/coverage and complete full-key bindings are independently checked. Wrong objective/identity, malformed rational/JSON/key data, duplicate/missing/out-of-order values or ranks, extra after-target rows, model-only keys, false Bellman values, zero support and rank progress prevent acceptance with the specified rule/locus. A valid post-target cycle alone does not add certificate rows. Earlier profile bytes/verdicts and eligible producer preferences remain unchanged. | Test |
| FR-201-AC-42 | Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic exhaustion during original/stopped admission, raw/internal/full keys, all-action and draw reconstruction, min/max sums, zero-support and progress checks retain their real typed cause without an accepted truth. Undefined predicates/workload or source refusals are not coerced to zero or dropped actions. Fairness, workload selection, rewards, timed/general temporal, pending multi-post, multiple-initial/binding, non-exact or refutation requests issue no new-profile proof. Zero direct instances retain UnsupportedBinding/unsupported without fabricated evidence. Existing source thresholds 0/1 stay refused and no new min-AtMost/max-AtLeast syntax is added. | Test |

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
