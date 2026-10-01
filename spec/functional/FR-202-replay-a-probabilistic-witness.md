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
    pub scheduler: Option<WitnessScheduler>,     // absent under a workload
    pub evidence: WitnessEvidence,
}
pub struct WitnessScheduler { pub entries: Vec<(ProductKey, WitnessChoice)> }
pub enum WitnessChoice { Identity(SchedulerChoice), PostState(Vec<(Vec<Value>, Digest)>), Delay }
pub enum WitnessEvidence { Paths(Vec<WitnessPath>), Subsystem(ProbabilityCertificate),
    Undefined { path: WitnessPath, undefined: UndefinedEvaluation } }   // ADR-028 XV-8
pub struct WitnessPath { pub steps: Vec<WitnessStep> }  // CX-2 step content plus the drawn vector

pub fn replay_probabilistic_witness(
    request: &ReplayRequest,                     // FR-098
    envelope: &WitnessEnvelope<ProbabilisticCounterexample>,
    limits: &ExactProbLimits,
) -> Result<ReplayOutcome, ReplayRefusal>;
```

## Behavior

### Emission

- EN-5 SHALL emit a path set when at most `max_witness_paths` paths, most
  probable first, reach the bound; otherwise subsystem evidence: a `Lower`
  certificate on the induced chain or under the workload, for `Pr(not E)`
  under a `>= θ` bound or `Pr(E)` under `<= θ`, with value past the bound,
  or an `Upper` certificate on `Pr(E)` below `θ`. Long-run and
  expected-reward refutations SHALL use certificate evidence.
- For a quantile claim, the paths SHALL witness XF-2's transform: their
  activated, violating mass SHALL exceed `1 − q`.
- The witness scheduler SHALL be defined on the evidence's states only:
  memoryless and deterministic, or memoryless and randomized for a claim
  with a fairness set (FR-200).
- Evidence on a partial product SHALL settle the item when it is a path set
  or a `Lower` certificate refuting a `<= θ` bound, or a `Lower` proof of a
  `>= θ` bound, found before a limit stopped exploration (XV-5).

### Replay

- `replay_probabilistic_witness` SHALL recompile the package (FR-098) and
  re-admit the subject's initial state and universes from the provision, as
  FR-128 does.
- For a path set it SHALL re-execute each path through `ModelSystem`:
  select each step's successor by its post-state digest; check that each
  drawn value lies in its support; over every scheduler check that each step
  takes a choice the witness scheduler gives positive probability at its
  product state; recompute each path's probability exactly, as the product
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
- With a fairness set it SHALL check that every bottom strongly connected
  component of the induced chain meets FR-200's fair end-component
  condition.
- Agreement SHALL settle `reproduced-with-evaluated-witness` and the item
  `refuted`. A sum at or below the bound, paths that are not prefix-free, an
  event that evaluates the other way, or an unfair witness SHALL settle
  `inconclusive`, `ReplayParity`.
- A digest with no matching successor SHALL refuse `stale_dependency`/
  `revision-mismatch`; a step that is not enabled, a value outside its
  support, or a choice the scheduler gives probability 0 SHALL refuse
  `invalid_runtime_input`/`invalid-value`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-202-AC-1 | §15.4's `Deliver`: a one-path set (`send_b` lost, `send_b` lost) with scheduler `send_b` at both live states replays `refuted` with path probability `1/25 > 3/100`. §15.2 at `2 ms`: the one-path set (`request`, `attempt` with `d = 3 ms`, `outcome = Ok`) replays with probability `343/5000 > 1/20`. | Test (TC-637) |
| FR-202-AC-2 | §15.4's path with its second step changed to `send_a` refuses `invalid_runtime_input`/`invalid-value` (not the scheduler's choice); with `lost` drawn as a value outside `{true, false}` it refuses the same; with a post-state digest altered it refuses `stale_dependency`/`revision-mismatch`; the path listed twice settles `inconclusive`, `ReplayParity` (not prefix-free). | Test (TC-637) |
| FR-202-AC-3 | §15.3's per-window claim is refuted with subsystem evidence: a dyadic `Lower` certificate for `Pr(not E)` with value above `1/100`, which replay accepts through FR-201's checker. FR-200-AC-2's `Coin2` witness replays `refuted` and passes the fairness check; the same witness with the scheduler taking `wait` at the live state forever fails the fairness check and settles `inconclusive`, `ReplayParity`. | Test (TC-637) |
| FR-202-AC-4 | With `max_witness_paths` 0, §15.4's refutation carries subsystem evidence instead of a path set and still replays `refuted`. Replaying one envelope twice gives equal outcomes. | Test (TC-637) |
| FR-202-AC-5 | FR-196-AC-5's `Undefined` evidence replays `refuted`; the same evidence with `where` set to 0 settles `inconclusive`, `ReplayParity`. | Test (TC-641) |

## Dependencies

- ADR-028 WS-1 to WS-6, XV-3, XV-5, FS-9; ADR-018 CX-2, CX-3.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md),
  [FR-201](FR-201-check-a-probability-certificate.md).

## References

- QSpec half, which owns the witness wire and its replay rules: Linear
  STD-137.
- Owning ticket: Linear QSL-371.
