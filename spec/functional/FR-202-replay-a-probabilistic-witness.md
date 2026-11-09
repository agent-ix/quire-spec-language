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
path-set replay entry for untimed acyclic reachability claims, with no fairness,
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

This entry SHALL report unsupported for cyclic subjects, subsystem evidence,
randomized schedulers, fairness, multiple initial states/instances, timed or
other claim forms outside this slice. It SHALL NOT project them onto this
slice. This additional entry does not change the wire facade obligation, the
broader replay rules, or any public JSON key or witness codec.

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
| FR-202-AC-11 | A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion of each configured expansion/work/storage limit returns incomplete without a token or a refuted record. Cyclic, subsystem, randomized-policy, fairness, multiple-initial/instance and timed inputs report unsupported without a token. A caller cannot construct a reproduced token or settle EN-5 refuted from a bare engine result. | Test |

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
