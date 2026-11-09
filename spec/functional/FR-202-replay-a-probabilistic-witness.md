---
id: FR-202
title: "Replay a probabilistic witness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: depends_on
---
# FR-202: Replay a probabilistic witness

## Description

EN-5 SHALL refute a claim with a `ProbabilisticCounterexample` (ADR-028
WS-1 to WS-4): the initial state and binding, the bound, the witness
scheduler over every scheduler, and evidence that is either a prefix-free
set of finite paths whose exact probabilities sum past the bound, or a
`Lower` or `Upper` certificate on the chain the witness induces.
`qsl_replay::replay_probabilistic_witness`, a layer-6 facade entry beside
`replay_model_trace`, SHALL re-execute that evidence through `ModelSystem`
and settle the item `refuted` only when it agrees (WS-5).

## Use case

A protocol designer gets `refuted` for §15.4's `Deliver`. The witness says
an adversary that always picks `send_b` makes two losses in a row with
probability `1/25`, above the `3/100` the claim allows. Replay re-executes
the two steps, checks each step is the adversary's choice and each drawn
value is in its support, recomputes `1/25`, and reproduces the refutation.

## Inputs

```rust
pub struct ProbabilisticCounterexample {
    pub initial_state: u32,
    pub binding: Option<Binding>,
    pub bound: Bound,
        pub scheduler: Option<WitnessScheduler>,     // present over every scheduler and under a workload with free delays (QSpec FR-413)

    pub evidence: WitnessEvidence,
}
pub struct WitnessScheduler { pub entries: Vec<(ProductKey, WitnessEntry)> }
pub enum WitnessEntry { Choice(ProductAction), Randomized(Vec<(ProductAction, Rational)>) }
// FR-196's closed ProductAction, including FreeDelays: per drawn given-delay vector, every free racing identity's delay
pub enum WitnessEvidence { Paths(Vec<WitnessPath>), Subsystem(ProbabilityCertificate),
    Undefined { path: WitnessPath, undefined: UndefinedEvaluation } }   // ADR-028 XV-8
pub struct WitnessPath { pub steps: Vec<WitnessStep> }  // CX-2 content plus mandatory semantic choice, post-key, draw, elapsed

pub fn replay_probabilistic_witness(
    wire: ReplayRequestWire,                     // FR-098
    subject: &ModelTraceSubject,                 // FR-128
    replay_limits: ReplayLimits,                  // FR-098 replay.input_bytes bound
    envelope: &WitnessEnvelope<ProbabilisticCounterexample>,
    limits: &ExactProbLimits,
) -> Result<ReplayOutcome, ReplayRefusal>;
```

Each WitnessStep SHALL retain ADR-018 CX-2's actual step content and drawn
vector (a merged step, FR-196, and a terminal stutter step have none), and additionally name its exact `ProductAction`, complete canonical
FR-196 post-ProductKey and exact nonnegative elapsed Rational in the model's
time unit (zero on untimed/IdleObserve steps). The action is the actual
resolved edge choice, not `action: None`, a local action number, a synthetic
operation NodeId, or a placeholder standing in for tail control. The actual
model post-state content stays as CX-2 specifies.

A WitnessScheduler SHALL have at most one entry per full ProductKey, in
canonical key order; equality/order cannot use local product IDs. A Choice
names exactly one enabled semantic choice. A Randomized entry names distinct
enabled choices in canonical choice order with positive reduced exact
Rational probabilities summing to1; duplicates, zero/negative mass, missing
mass and disabled choices refuse. The canonical choice body is the FR-196
action body. A `FreeDelays` choice, `["free-delays",[[given,free],...]]`, is
enabled only at a mixed given/free workload race state (FR-204): its rows
cover the actual conditioned given-delay support once in FR-196's numeric
row order, each row's `given` and `free` name exactly the given and the free
racing identities sorted by full schedule bytes, and each free delay lies in
its identity's window; any other FreeDelays body is a disabled choice and
refuses. This race-policy form cannot name IdleObserve or change a fixed
workload weight. No guessed QSpec wire spelling is adopted by this semantic
contract.

EN-5 SHALL form the WitnessScheduler from the optimizer's attaining policy on
the evidence's states: each entry is the complete pre-choice ProductKey and the
ProductAction the policy takes there, chosen by FR-198's witness policy rules
(the MEC exit rule of a maximum first, then the attaining choice with ties
broken by canonical transition order). At a FreeDelays state the entry is the
whole action FR-198 chose, one row choice per row. A path step through a FreeDelays action is a merged step
(FR-196): it names the action, the post-key and the elapsed winning delay, and
no single given vector; replay recomputes its probability as the sum over the
action's rows of the row's given-vector probability times the mass the row's
race gives that post-key and elapsed amount.

## Behavior

### Emission

- EN-5 SHALL emit a path set when at most `max_witness_paths` paths, most
  probable first, reach the bound; otherwise subsystem evidence: a `Lower`
  certificate on the induced chain or under the workload, for `Pr(not E)`
  under a `>= θ` bound or `Pr(E)` under `<= θ`, with value past the bound,
  or an `Upper` certificate on `Pr(E)` below `θ`. Long-run and
  expected-reward refutations SHALL use certificate evidence.
- For a quantile claim, replay SHALL classify each path as violating or
  not activated, sum their exact probabilities into `V` and `N`, and apply
  QSpec FR-413's two-mass rule: `quantile q of M <= c` refutes when
  `V > (1 − q) · (1 − N)`; `quantile q of M >= c` refutes when `V > 0` and
  `V >= q · (1 − N)`. A path set that fails the rule SHALL settle
  `inconclusive`, `ReplayParity`. Subsystem evidence for a quantile claim
  SHALL carry a `Lower` certificate for `V` and one for `N`, and the same
  rule SHALL apply to their values.
- The witness scheduler SHALL be defined on the evidence's states only:
  memoryless and deterministic, or memoryless and randomized for a claim
  with a fairness set (FR-200), or for the derived TS-5 observation
  admissibility of a digital timed subject (ADR-028 §13a) even without an
  authored fairness set. The policy covers its reachable recurrent
  continuation support as well as the finite quantity evidence; no implicit
  default choice or absorbing goal self-loop establishes admissibility.
- Evidence on a partial product SHALL settle the item when it is a path set
  or a `Lower` certificate refuting a `<= θ` bound, or a `Lower` proof of a
  `>= θ` bound, found before a limit stopped exploration (XV-5).

### Replay

#### Checked-subject path-set entry

In addition to the wire facade below, layer 6 SHALL expose a checked-subject
path-set replay entry for untimed acyclic unbounded reachability claims, the
acyclic infinite-trace state-predicate subset and the bounded event-position
subset below, with no fairness,
one initial state, at most one bound instance, and a deterministic memoryless
witness scheduler (or the bound workload). It SHALL take the `CheckedPackage`,
its admitted `ModelSystem`, the claim occurrence and binding, typed path-set
evidence, caller-configured expansion/work/storage limits and `&Cancel`.
This entry decodes no wire and recompiles nothing, as the checked-subject
certificate entry in FR-201 does. It SHALL bind the claim from the package,
rather than accept an engine-supplied predicate or threshold.

Typed evidence SHALL retain actual semantic action identities, resolved
`ModelTransition` content and draws, initial and post-state content, and complete
canonical product keys. The entry SHALL reuse the qualified semantic types
(`ScheduledIdentity` and `ModelTransition`) without importing `qsl-analyze`
product/action types or treating local state/action numbers as identities.
The core SHALL independently re-enumerate enabled transitions, evaluate target
predicates, check the scheduler, and recompute exact path mass and prefix freedom
under the Replay rules below. An asserted transition probability, target label,
terminal marker or optimizer result SHALL NOT establish replay agreement.

For an `AtMost` bound `Pr(E) <= theta`, accepted paths SHALL end at their first
target hit and their mass SHALL exceed `theta`. For an `AtLeast` bound
`Pr(E) >= theta`, accepted paths SHALL avoid the target and end at leaves where
the core independently confirms no enabled continuation; their mass SHALL
exceed `1 - theta`. An initial target is a first hit with an empty path; a live
non-target prefix is not a terminal nonreach leaf. These are sufficient
refutations under the bound workload or a consistent deterministic witness
policy; an engine's assertion that a policy is optimal is not replay evidence.

The entry SHALL produce an opaque reproduced-witness token only after all
checks and a final cancellation poll. The token SHALL bind the package, claim
occurrence, subject binding and accepted evidence. Layer-6 EN-5 terminal
construction SHALL require the matching token to settle `refuted` and retain
the probabilistic counterexample evidence in the analyze record. A caller
SHALL NOT construct the token or turn a bare engine verdict into `refuted`.
Replay disagreement/refusal SHALL retain OP-2's `replay_parity`/`replay_refused`
settlement; exhaustion and cancellation SHALL remain `incomplete` without a
token. Claim, package or binding mismatch SHALL refuse before acceptance.

For the unbounded subset this entry SHALL report unsupported for cyclic subjects.
Both subsets SHALL report unsupported for subsystem evidence,
randomized schedulers, fairness, multiple initial states/instances, timed or
other claim forms outside this slice. It SHALL NOT project them onto this
slice. This additional entry does not change the wire facade obligation, the
broader replay rules, or any public JSON key or witness codec.

#### Acyclic infinite-trace state-predicate path sets

The checked-subject entry SHALL additionally admit `holds(A) until holds(B)`
and `always holds(P)` under `INFINITE-TRACE`, using FR-201's acyclic
state-predicate admission: one initial state, at most one actual S3 binding,
empty fairness, untimed subjects, existing Markov successor restrictions,
minimum `AtLeast`, maximum `AtMost`, or a single resolved workload distribution
with either comparison. Source bounds SHALL retain `MinAtLeast`/`MaxAtMost`
threshold admission in the open interval (0,1); event probabilities 0 and 1
remain valid. Nested/bounded temporal forms, event predicates, rewards,
unresolved workloads and residual post-draw choices outside that admission
SHALL remain unsupported. No horizon or bounded false-extension rule applies.

Before absorbing any outcome, the core SHALL independently enumerate the
original reachable model graph and establish acyclicity apart from implicit
terminal stutter. A cycle beyond an initial or intermediate decided outcome
SHALL still refuse admission. An engine's stopped graph, supplied labels or
terminal flags SHALL NOT establish admission or replay agreement.

Replay SHALL bind the actual checked occurrence, formula, infinite-trace
profile, threshold, scheduler/workload and S3 parameter node/selected instance
from the package and admitted subject. Qualified state-predicate evaluation
SHALL classify position 0 and each actual semantic successor independently.
For until, B decides acceptance even when A is false; neither A nor B decides
rejection; A and not B continues until a decision or a genuine model terminal,
where it rejects because infinite stutter never reaches B. For always, not-P
decides rejection immediately, including before a later P state; nonterminal P
continues, and terminal P decides acceptance because P holds on its infinite
stutter. A live P prefix SHALL NOT establish always. Paths SHALL stop at their
first decision. Terminal closure contributes probability 1 and no fabricated
transition, draw, elapsed time or synthetic state.

Evidence SHALL use actual initial/post-state content, semantic scheduled
identities, resolved model transitions and draws, and FR-196's complete
binding-bearing model-state keys with absent monitor, accumulator and pending
components null, as in FR-201's state-predicate profile. Formula-monitor keys,
local state/action numbers, surrogate predicates, fabricated S3 nodes or
synthetic terminal keys SHALL NOT substitute. This reuses the native evidence
types and independent transition re-enumeration, not the reachability-only
first-target/terminal-nonreach classifier or a trusted solver graph.

Each path SHALL have positive recomputed exact mass. The core SHALL check
pairwise prefix-free, disjoint execution/draw cylinders and deterministic
memoryless scheduler consistency at every represented full choice key.
Distinct supported draws reaching one state remain distinct cylinders;
duplicates SHALL NOT multiply mass. Workload evidence SHALL contain no witness
policy; replay SHALL rederive its exact action and draw weights. A witness
policy need cover only represented choice states and requires no optimality
assertion, because the admitted remaining choices can be extended consistently.

Let R sum the submitted rejecting-path masses and A sum the accepting-path
masses. For minimum `AtLeast theta` (or workload `AtLeast`), every submitted
path SHALL reject and replay SHALL require `R > 1 - theta`; the event probability
under that policy/workload is at most `1 - R < theta`. For maximum `AtMost theta`
(or workload `AtMost`), every path SHALL accept and replay SHALL require
`A > theta`; the event probability is at least A. These are sufficient bounds,
not an exact whole-event value. Unrepresented mass SHALL remain unknown.
Equality, insufficient mass, opposite-verdict paths, duplicate/overlapping
cylinders or continuation past a decision SHALL yield replay parity without
a reproduced token.

EN-5 SHALL emit this subset's path evidence only from exact results. `Bounds`
intervals and `PrecisionBudget` SHALL NOT be relabelled exact. The existing
opaque matching-token and retained-evidence settlement contract applies;
certificate acceptance, a bare engine verdict or a reachability witness alone
SHALL NOT settle these events refuted. Refused predicates and unsupported
inputs retain their typed outcomes. Expansion/work/storage/arithmetic exhaustion
and cancellation, including original-graph admission and the final acceptance
poll, SHALL yield no truth, reproduced token or accepted evidence in a refuted
record. This entry adds no wire spelling or shared FR-413 codec; that facade
remains blocked on the closed shared contract prerequisite.

#### Bounded checked-subject path sets

The same native entry SHALL additionally admit origin-activated untimed
`eventually[0,h] holds(P)`, with finite nonnegative event-position horizon h,
one initial state and at most one admitted binding, for minimum `AtLeast`,
maximum `AtMost`, and a bound workload with either comparison. Admission SHALL
retain FR-201's bounded subset's supported Markov successor restrictions and
single resolved workload distribution; unsupported residual post-draw choices
SHALL NOT be silently resolved by selecting a successor. Other temporal forms,
timing, rewards, fairness and randomized policies remain outside this increment.
The unbounded subset retains its existing semantics.

Replay SHALL independently bind the actual checked claim occurrence, threshold,
formula, profile and scheduler/workload from the `CheckedPackage` and admitted
`ModelSystem`. For an `over` claim the monitor context SHALL use the actual S3
parameter node and selected instance key, as FR-201's bounded checker does;
an alias, fabricated parameter node or engine-supplied predicate SHALL NOT
replace it. The existing item/package and binding checks and reproduced-token
settlement contract apply unchanged.

The core SHALL use `qsl-eval`'s qualified bounded-monitor initialization, step,
canonical key and terminal false-extension operations. It SHALL read position 0
once, advance one position per actual resolved model step, include position h,
and stop a path at its first decided verdict. Complete initial and post-product
keys SHALL carry FR-196's actual formula-monitor context, parsed automaton key
and remaining horizon/debts; decided keys SHALL use the counter-free absorbing
verdict. Local monitor indices, model-only keys, a null monitor and unbounded
reachability keys SHALL NOT substitute for them. An undecided state's rank
context is its independently rederived remaining future positions/debts, and a
decided state's is 0, as in FR-201; this adds no witness rank field. Revisits to
one model state at different remaining horizons SHALL remain distinct product
states. A raw model cycle SHALL NOT cause rejection when the finite undecided
monitor product terminates. Replay SHALL NOT run the unbounded raw-graph
acyclicity test or search beyond h to decide this bounded event.

At an undecided model leaf, replay SHALL independently establish that there is
no enabled continuation and apply qualified terminal false-extension closure.
This closure contributes mass factor 1 and no model step, draw or elapsed time;
the final supplied model-step key remains the actual post-observation key, and
the closed decision is derived by replay. It SHALL NOT require fabricated
stutter steps or treat a live non-target prefix as a rejecting leaf. Horizon
exhaustion may decide rejection at a live model state. Horizon 0 is decided by
the initial position alone, permitting an empty path with mass 1 when its
verdict is the required one.

Every represented path SHALL have positive independently recomputed exact
probability, rederived semantic steps/draws and keys, and pairwise disjoint
prefix-free execution cylinders. Duplicate paths and paths that continue past
a decided verdict SHALL fail replay agreement. Under every scheduler, the
deterministic witness policy SHALL agree at every represented full product key;
it need cover only the evidence's choice states. Remaining finite-horizon
choices can be extended arbitrarily to enabled choices, so no assertion of
optimality is required. Under a workload the policy SHALL be empty, and replay
SHALL independently rederive the workload's exact action/draw probabilities.
Distinct represented draws into one post-state retain disjoint cylinders;
their probabilities SHALL NOT be duplicated by naming the same draw twice.

Let R be the sum of represented rejecting-path probabilities, and A the sum of
represented accepting-path probabilities, under that consistent policy or
workload. For `AtLeast theta`, every submitted path SHALL decide rejection and
replay SHALL reproduce a refutation only if `R > 1 - theta`: the actual event
probability under that scheduler/workload is at most `1 - R < theta`, hence
the minimum also fails. For `AtMost theta`, every submitted path SHALL decide
acceptance and replay SHALL reproduce only if `A > theta`: the event probability
under that scheduler/workload is at least A, hence the maximum also fails.
Unrepresented mass SHALL NOT be assumed accepting or rejecting. These are
sufficient path-set bounds, not claims that the represented mass is the exact
whole event probability. Equality, insufficient mass or an opposite-verdict
path SHALL yield replay parity without a reproduced token.

This increment SHALL emit bounded path evidence only from FR-197 `Exact`
results. `Bounds`, `PrecisionBudget`, refused predicates, unsupported shapes,
unreplayed engine results and certificate acceptance alone SHALL NOT become a
bounded refutation or reproduced token. Expansion/work/storage/arithmetic
exhaustion and cancellation, including the final acceptance poll, SHALL preserve
their original non-truth outcomes with no token or accepted probabilistic
evidence in a refuted record. The global QSpec FR-413 witness wire still lacks
the closed bound, scheduler-choice and semantic product-action/key contracts
needed by this slice. Its facade remains blocked on that shared-owner work;
this increment specifies native typed overload behavior only and introduces no
JSON spelling, wire schema or transport capability.

#### Wire facade and shared evidence checks

- `replay_probabilistic_witness` SHALL decode `wire` through FR-098's
  canonical `ByteProvision` and recompile path, passing `replay_limits` to
  `ReplayRequest::decode` so the encoded request is bounded by
  `replay.input_bytes` before provision access. It SHALL re-admit the initial
  states, universes and expansion limits in `subject`, as FR-128 does. It
  SHALL NOT accept or reuse an EN-5 solver product as trusted replay data.
- For a path set it SHALL re-execute each path through `ModelSystem`:
  rederive the named semantic action and complete post-key, select each
  step's successor by its post-state; check that each
  drawn value lies in its support; over every scheduler, and under a workload
  at a state with a free delay, check that each step takes a choice the
  witness scheduler gives positive probability at its product state;
  recompute each path's probability exactly, as the product
  of its draw probabilities and the scheduler's choice probabilities, or of
  its step probabilities under the workload; evaluate the event with SM-1;
  check that the paths are pairwise prefix-free; and check the sum against
  the bound.
- For `Undefined` evidence it SHALL re-execute the path as a path of a
  path set, check that every step has positive probability, and evaluate
  the claim's letters along it by ADR-018 UE-5, with no sum against the
  bound. The first undefined evaluation at `where` with an equal cause
  SHALL settle `reproduced-with-evaluated-witness`; any other result
  SHALL settle `inconclusive`, `ReplayParity`.
- For subsystem evidence it SHALL rebuild the induced chain over the
  certificate's support by applying the witness scheduler, and run FR-201's
  checker on it.
- Digital replay SHALL distinguish UnitDelay in running, EnterIdleDelay
  from quiescent running, IdleDelay from idle-ready/idle-delayed and
  IdleObserve only from idle-delayed. It SHALL check their actual
  clock/deadline/tail post-state, elapsed1/scale versus0, and no-letter
  versus current-clock stutter letter. A pending discrete resolution applies
  only its already-delayed discrete relation. Target predicates are checked
  on observations only; a goal observation terminates the quantity, not
  the original subject continuation required for admissibility.
- With a fairness set it SHALL check every reachable bottom strongly
  connected component of the induced chain meets FR-200's fair
  end-component condition. For a digital TS-5 subject it SHALL additionally
  rederive observation edges and require an observation edge in every
  reachable recurrent component on the original continuation graph,
  irrespective of authored fairness. Actual TA-5 zero-delay-cycle refusal
  and positive delays still apply. This is almost-sure admissibility; it
  does not exclude probability-zero non-progress draw sequences or impose
  a finite observation period. An absorbing analysis sink does not pass
  this check in place of a real continuation.
- Agreement SHALL settle `reproduced-with-evaluated-witness` and the item
  `refuted`. A sum at or below the bound, paths that are not prefix-free, an
  event that evaluates the other way, or an unfair/TS-5-inadmissible witness SHALL settle
  `inconclusive`, `ReplayParity`.
- A step that is not enabled, a value outside its
  support, or a choice the scheduler gives probability 0 SHALL refuse
  `invalid_runtime_input`/`invalid-value`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-202-AC-1 | §15.4's `Deliver`: a one-path set (`send_b` lost, `send_b` lost) with scheduler `send_b` at both live states replays `refuted` with path probability `1/25 > 3/100`. §15.2 at `2 ms`: the one-path set (`request`, `attempt` with `d = 3 ms`, `outcome = Ok`) replays with probability `343/5000 > 1/20`. | Test (TC-637) |
| FR-202-AC-2 | §15.4's path with its second step changed to `send_a` refuses `invalid_runtime_input`/`invalid-value` (not the scheduler's choice); with `lost` drawn as a value outside `{true, false}` it refuses the same; the path listed twice settles `inconclusive`, `ReplayParity` (not prefix-free). | Test (TC-637) |
| FR-202-AC-3 | §15.3's per-window claim is refuted with subsystem evidence: a dyadic `Lower` certificate for `Pr(not E)` with value above `1/100`, which replay accepts through FR-201's checker. FR-200-AC-2's `Coin2` witness replays `refuted` and passes the fairness check; the same witness with the scheduler taking `wait` at the live state forever fails the fairness check and settles `inconclusive`, `ReplayParity`. With no authored fairness, FR-196's capped x2 policy chooses IdleDelay3/4 and IdleObserve1/4, gives elapsed value5>3 and replays the threshold refutation; the same choice distribution after each observation supplies almost-sure continuation. A delay-only recurrent policy fails derived observation admissibility. Observe from idle-ready or its operation-identity alias refuses invalid-value; replacing Observe by Delay cannot reproduce its letter/post-key/elapsed. | Test (TC-637) |
| FR-202-AC-4 | With `max_witness_paths` 0, §15.4's refutation carries subsystem evidence instead of a path set and still replays `refuted`. Replaying one envelope twice gives equal outcomes. | Test (TC-637) |
| FR-202-AC-7 | Given a `ReplayRequestWire` whose measured encoded size is `B`, when the replay facade receives caller-configured `ReplayLimits` with `input_bytes = B - 1`, it returns `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded { bound: B - 1, actual: B }))` before byte-provision lookup or recompile. | Test |
| FR-202-AC-6 | Over the two-point model (`M` 1 or 3, probability 1/2 each), the path to `M = 1` (`V = 1/2`, `N = 0`) replays `refuted` for `quantile 0.5 of (M) >= 3` (`M` is a placeholder for the claim's parenthesised measure) (equality refutes), and settles `inconclusive`, `ReplayParity` for `quantile 0.9 of (M) >= 3`, which holds. Over its variant that activates with probability 1/2 (`M` 1 or 3 with probability 1/4 each), the path set of the `M = 1` path and the not-activated path (`V = 1/4`, `N = 1/2`) replays `refuted` for `quantile 0.5 of (M) >= 3`, and the `M = 1` path alone settles `inconclusive`, `ReplayParity`. | Test (TC-637) |
| FR-202-AC-8 | FR-204-AC-10's `Detour` maximum witness (`FreeDelays` rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 1, path set `A` at 1 ms and `B` at 1 ms followed by `G` at 2 ms) replays `refuted` with `3/4 > 7/10`, the first step's merged probability `1/2` recomputed from row `[1]` and the `B` step's `1/2` from row `[3]`. The same witness with its rows swapped in order, with the row given `[3]` removed, with `B` delay 4 (outside `[1, 3]`), or as a `Choice` naming `WorkloadResolve{B, 1}` at that state refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-637) |
| FR-202-AC-5 | FR-196-AC-5's `Undefined` evidence (`drop` with `b = 0`, `UndefinedEvaluation{where: 1, cause: absent-key}`) replays `refuted`; the same evidence with `where` set to 0 settles `inconclusive`, `ReplayParity`. | Test (TC-641) |
| FR-202-AC-9 | A checked-subject acyclic reachability fixture replays a target-first-hit path set of mass `2/3` against `AtMost 1/2` and a terminal-nonreach path set of mass `2/3` against `AtLeast 1/2`; each produces a token permitting only its matching EN-5 refuted record, with the accepted evidence retained. Repeating replay gives equal semantic outcomes. Neither replay needs an emitted package or analyzer graph types. | Test |
| FR-202-AC-10 | In AC-9, altering the claim occurrence, package or binding refuses; altering an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping paths, inconsistent scheduler choices, an earlier target on a first-hit path, and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant threshold yields replay parity. | Test |
| FR-202-AC-11 | A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion of each configured expansion/work/storage limit returns incomplete without a token or a refuted record. Unbounded cyclic, subsystem, randomized-policy, fairness, multiple-initial/instance and timed inputs report unsupported without a token. A caller cannot construct a reproduced token or settle EN-5 refuted from a bare engine result. | Test |
| FR-202-AC-12 | A cyclic undelivered model has action slow delivering with probability 1/2 and otherwise returning undelivered, and fast delivering with probability 3/4 and otherwise returning undelivered. For origin eventually[0,2] holds(delivered), slow at both full choice keys refutes minimum AtLeast 4/5 with the two-failure path of mass 1/4 greater than 1/5; fast at both keys refutes maximum AtMost 9/10 with first-hit paths of masses 3/4 and 3/16, sum 15/16. Equal-weight resolved workload has per-step delivery 5/8: all two-failure action/draw cylinders sum to 9/64 and refute AtLeast 7/8; all first-hit cylinders sum to 55/64 and refute AtMost 4/5. Each produces only its matching token and refuted record retaining evidence; repeat replay gives equal semantic outcomes. | Test |
| FR-202-AC-13 | AC-12 initial and one-failure states share model content but carry different complete formula keys with remaining contexts 2 and 1. Changing a supplied key to a model-only key, null/local-index monitor, another remaining horizon or binding prevents a token. Rebinding to a changed source package, horizon, threshold, claim occurrence, actual S3 over-parameter node or selected instance prevents reuse of the original token/evidence as an accepted refutation of that item. A checked source with two possible object values is restricted to one admitted over binding; replay uses that object's predicate and actual S3 node, never a fabricated binding node. | Test |
| FR-202-AC-14 | At horizon 0, initially true P gives an empty accepting path of mass 1 refuting AtMost 1/2, while initially false P gives an empty rejecting path refuting AtLeast 1/2; opposite-verdict empty paths yield parity. At horizon 2, a terminal initially false P closes by false extension with mass 1 and no fabricated stutter step, refuting AtLeast 1/2. A live undecided prefix after one failure does not refute; after two failures horizon rejection does, even though another model action is enabled. An initial true P path extended by an operation cannot reproduce. | Test |
| FR-202-AC-15 | In AC-12 a missing represented path cannot be counted, equal-threshold represented mass yields parity, and a changed action/draw/post-state, duplicated draw-cylinder, overlapping path or inconsistent choice at one full key cannot produce a token. First-hit masses under slow are only 3/4 and cannot refute AtMost 4/5. A policy may choose different enabled actions at the same model content with different remaining horizons; this is a consistent full-product policy. | Test |
| FR-202-AC-16 | AC-12 replay with pre-cancellation, cancellation at its final acceptance poll, or each configured expansion/work/storage/arithmetic budget exhausted returns its typed incomplete cause with no truth, token or accepted evidence in a refuted record. A refused predicate preserves its typed refusal; FR-197 Bounds and PrecisionBudget, unsupported bounded until/always/reward/timed/fair inputs, subsystem evidence and nonreplayed exact results produce no bounded reproduced token. The native entry needs neither S4 emission nor QSpec FR-413 document transport. | Test |
| FR-202-AC-17 | Until fixture: initial A-and-not-B has actions a and b reaching B-and-not-A with probabilities 1/4 and 3/4, otherwise neither-A-nor-B followed by terminal B. At threshold 1/2, a's rejecting cylinder of mass 3/4 refutes minimum AtLeast, and b's accepting cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved workload has accepting and rejecting mass 1/2 each: its rejecting cylinders refute AtLeast 3/5 and accepting cylinders refute AtMost 2/5. All four produce only their matching reproduced token and retain semantic evidence; repeat replay gives equal outcomes. The failure prefix stops before its later B; the B-and-not-A path accepts. | Test |
| FR-202-AC-18 | Always fixture: initial nonterminal P has actions a and b reaching terminal P with probabilities 1/4 and 3/4, otherwise not-P followed by terminal P. At threshold 1/2, a's not-P cylinder of mass 3/4 refutes minimum AtLeast and b's terminal-P cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved workload refutes AtLeast 3/5 with rejecting mass 1/2 and AtMost 2/5 with accepting mass 1/2. Later P cannot repair the not-P prefix; the initial live P alone cannot be accepted. | Test |
| FR-202-AC-19 | With admitted threshold 1/2, empty paths of mass 1 refute AtMost for initial B-and-not-A until and initial terminal-P always; empty paths refute AtLeast for initial neither-A-nor-B until, terminal A-and-not-B until, and initial not-P always even with a later terminal P successor. Terminal A-and-not-B has until probability 0 and terminal P has always probability 1 without a fabricated stutter step. Source threshold endpoints remain refused; probabilities 0 and 1 are admitted. | Test |
| FR-202-AC-20 | In AC-17/18, changed source/package, foreign claim occurrence or S3 binding refuses identity mismatch; a changed formula that changes the represented verdict, changed threshold to equality, changed actual action/draw/post-state/full key, fabricated binding node, missing binding component or a formula-monitor key prevents a token. Formula spelling alone need not fail if the actual bound claim has identical semantics and identity checks pass. A wrong-verdict path, live undecided prefix, path extended beyond first success/failure, duplicate cylinder or prefix overlap yields no reproduced token; insufficient/equal mass yields replay parity. | Test |
| FR-202-AC-21 | AC-17/18 two evidence paths demanding different actions at one complete choice key fail policy consistency; a policy under a resolved workload fails replay. Distinct supported draws reaching the same post-state contribute their separate exact masses once, while repeating a draw cannot increase mass. A raw reachable cycle beyond initial B for until or initial not-P for always is unsupported despite immediate absorption; acyclic counterparts remain admitted. Fairness, timed/reward/nested/bounded forms, multiple initials/bindings, unresolved workload, subsystem and randomized-policy evidence remain unsupported in this subset. | Test |
| FR-202-AC-22 | AC-17/18 pre-cancellation, cancellation at final acceptance, or exhaustion of each caller expansion/work/storage/arithmetic limit during original admission or replay preserves its typed incomplete outcome with no truth, token or accepted evidence in a refuted record. Refused predicates preserve refusal. Bounds, PrecisionBudget, certificate acceptance alone and unreplayed exact engine verdicts cannot settle refuted. Native replay needs no emitted package or witness wire codec and imports no analyzer graph types. | Test |

## Dependencies

- ADR-028 WS-1 to WS-6, XV-3, XV-5, FS-9; ADR-018 CX-2, CX-3.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md),
  [FR-201](FR-201-check-a-probability-certificate.md).

## References

- QSpec half, which owns the witness wire and its replay rules: QSpec
  FR-413 (Linear STD-137).
- Owning ticket: Linear QSL-371.
