---
id: FR-170
title: "Replay state-graph evidence through ModelSystem"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
---
# FR-170: Replay state-graph evidence through ModelSystem

## Description

QSL SHALL replay state-graph evidence through `qsl-replay`'s replay facade
(FR-098, ADR-029 CB-2) at ADR-011 E9: `qsl_replay::replay_model_graph` takes FR-098's
request with the subject's inputs and a `WitnessEnvelope<GraphEvidence>`
whose source is `ReplaySource::ModelGraph` (ADR-022 GX-1). It re-executes
each recorded path through FR-120's `ModelSystem` with FR-101 `replay` and
evaluates the claim's predicates on the replayed states (ADR-022 GX-2 to
GX-4). For a trap it explores the trap's forward closure again, so it trusts
no engine claim about what the trap reaches. Replay needs no model checker,
no sampler and no solver.

## Use case

An auditor with the source and the initial snapshots replays a `proved`
`possible` claim: each witness path re-executes and ends where the target
holds. They replay a `refuted` `always possible` claim: the path to the trap
re-executes, and a fresh exploration from the trap finds no target state.
Evidence edited by hand, or produced by an engine that disagrees with the
evaluator, does not count.

## Inputs

- FR-098's request: package reference, byte provision (the subject's
  initial snapshots by `sha256-jcs` digest) and limits, with the subject's
  universes.
- A `WitnessEnvelope<GraphEvidence>` (FR-070) whose obligation identity
  binds the subject, the claim and its instance:

```rust
pub struct ModelPath {
    pub initial: u32,                   // index into the subject's initial states
    pub steps: Vec<ModelStep>,          // FR-128: transition identity, post-state digest
}

pub struct ModelLasso {
    pub path: ModelPath,                // stem and loop, FR-128's step shape
    pub loop_entry: u32,                // index of the loop's first state in `path`
}

pub enum WitnessPath {
    Path(ModelPath),                    // a state-graph `possible` witness
    Lasso(ModelLasso),                  // an HP-5 witness (FR-181)
}

pub enum TrapClosure {
    StateGraph,                         // the forward closure in the model's state graph
    Product,                            // one initial state's part of an HP-5 product (FR-181)
}

pub enum GraphEvidence {
    Witness { paths: Vec<(WitnessPath, WitnessSource)>, over: Option<OverBinding> },
    Trap { stem: ModelPath, closure: TrapClosure, over: Option<OverBinding> },
    PathPair { stem: ModelPath, first: Vec<ModelStep>, second: Vec<ModelStep>,
               over: Option<OverBinding> },
    Undefined { stem: ModelPath, undefined: UndefinedEvaluation,
                over: Option<OverBinding> },
}
```

- `GraphEvidence` implements `FamilyPayload` (FR-070-AC-5).

## Outputs

- An FR-072 replay result on the `ModelGraph` arm carrying the evaluated
  predicate values, as FR-128 carries its evaluated value; or a typed
  `ReplayRefusal` with no partial result.
- For a trap, the replay result also carries the closure exploration's
  node count.

## Behavior

### Common

- The executor SHALL recompile and check the package, resolve the claim by
  its `clause_node` and `occurrence_key`, and build the subject's
  `ModelSystem` by FR-128's rules and in FR-128's order, with FR-128's
  refusals.
- The executor SHALL re-execute each `ModelPath` from its indexed initial
  state with FR-101 `replay`, refusing as FR-128 refuses on an `initial`
  index outside the subject, a step that is not enabled, or a post-state
  digest that differs from the recomputed one.
- The executor SHALL evaluate predicates under the evidence's `over`
  binding through the one clause evaluator (FR-107).

### Witness

- If the `Witness` arm does not hold exactly one path per initial state of
  the subject, then the executor SHALL refuse
  `invalid_runtime_input`/`invalid-value`.
- The executor SHALL replay each path from its recorded steps whatever its
  source; a `Sampled` path SHALL replay without running the sampler.
- A `WitnessPath::Lasso` SHALL replay by FR-128's step, loop and fairness
  rules, as FR-181 states, so an unfair lasso refuses
  `invalid_runtime_input`/`invalid-value` and FR-169 settles it V-6
  `ReplayRefused`, as FR-127 settles an unfair counterexample lasso. The
  rest of this section is for `WitnessPath::Path`.
- The executor SHALL evaluate the target at each path's last state. True at
  every last state SHALL settle `reproduced-with-evaluated-witness`; false
  at one SHALL settle `inconclusive`, `Verdicts`.

### Trap

- A `TrapClosure::Product` trap SHALL replay as FR-181 states. The rest of
  this section is for `TrapClosure::StateGraph`.
- The executor SHALL re-execute the stem and evaluate `from` (for
  `always possible` and `unique path`) at its last state; a `possible` trap
  SHALL have an empty stem. A `from` that is false there SHALL settle
  `inconclusive`, `Verdicts`.
- The executor SHALL then explore the forward closure of the stem's last
  state with FR-101 `explore_request`, that state as the only initial state,
  unreduced, under the request's limits, evaluating the target at every
  node: `possible`'s and `always possible`'s target, or `unique path`'s `to`.
- Exploration that completes with no target node SHALL settle
  `reproduced-with-evaluated-witness`. A target node SHALL settle
  `inconclusive`, `Verdicts`. A node of the closure where the target
  evaluates `Undefined` SHALL settle `inconclusive`, `Verdicts`: the engine
  would have reported it as `Undefined` evidence (QSpec FR-391), not as a
  trap. Exploration a limit stops SHALL return the replay result stopped, naming
  the limit, its value and its setting, which FR-169 settles V-7
  `incomplete`, `LimitReached` (ADR-018 V-7).

### Path pair

- The executor SHALL re-execute the stem, evaluate `from` at its last
  state, and re-execute `first` and `second` from that state.
- It SHALL check that each sequence ends at a state where `to` holds, that
  `to` is false at every earlier state of each, and that the two sequences
  differ. All checks true SHALL settle `reproduced-with-evaluated-witness`;
  any false SHALL settle `inconclusive`, `Verdicts`.

### Undefined

- The executor SHALL re-execute the stem and evaluate the claim's
  predicates at each of its states in order (ADR-018 UE-5). When the first
  undefined evaluation is at the stem's last state, of the predicate
  `where.predicate` names, at the expression `where.locus` names, with an
  undefined cause equal to the payload's, the executor SHALL settle
  `reproduced-with-evaluated-witness`, with the `UndefinedEvaluation` as
  the value; otherwise `inconclusive`, `Verdicts`.

### Engine independence

- Evidence found under a reduction SHALL be concretised before it is
  enveloped, so no evidence carries a canonical state, a permutation or a
  reduction.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-170-AC-1 | ADR-022 §7.1's `ReachesTwo` witnesses replay to `reproduced-with-evaluated-witness`, for a `Sampled` witness and an `Explored` one alike, with no sampler in the replay request. | Test (TC-595) |
| FR-170-AC-2 | `ReachesThree`'s trap replays by exploring 9 nodes and reproduces; §7.2's `CanStillWin` trap re-executes `play, lose`, explores the closure `{Lost}`, and reproduces. §7.3's path pair reproduces. | Test (TC-595) |
| FR-170-AC-3 | Disagreements settle `inconclusive`, `Verdicts`: a `ReachesTwo` witness truncated to end at `(1, 0)`; a `CanStillWin` trap whose stem ends at `Mid` (its closure reaches `Won`); a path pair whose two sequences are equal; the `CanStillWin` trap in an envelope for its `from (x.phase != Lost)` variant, both claims in one unit (the stem's last state fails `from`). | Test (TC-595) |
| FR-170-AC-4 | Refusals settle no result: a witness with one post-state digest altered; a witness step replaced by a transition that is not enabled; a witness with no path for one of two initial states; an `initial` index of 1 over a one-snapshot subject. `ReachesThree`'s trap replayed with `max_states` 2 returns the replay result stopped at `max_states`, value 2. Replaying one envelope twice gives equal results. | Test (TC-595) |
| FR-170-AC-5 | FR-168-AC-5's `Undefined` evidence replays to `reproduced-with-evaluated-witness`; the same stem with its last step removed settles `inconclusive`, `Verdicts`, and so does the evidence with `where.locus` naming another expression, with the same cause. Over the same subject, `always possible PDiv using v over (c: Config::ConfigVersion) from (c.versionNumber >= 1) { 2 / (2 - c.versionNumber) = 2 }` for `c = a`, a claim with two predicates, returns `Undefined` evidence with the same stem whose `where.predicate` names the body; it replays to `reproduced-with-evaluated-witness`, and the same evidence with `where.predicate` naming the `from` predicate `c.versionNumber >= 1`, same cause, settles `inconclusive`, `Verdicts`. | Test (TC-612) |
| FR-170-AC-6 | A `Witness` arm holding a `WitnessPath::Lasso` over §7.1's subject whose loop is `upd(a)` from `(2, 0)` back to `(2, 0)`, for `possible c.versionNumber = 2`, replays the loop by FR-128's rules; the same lasso with its loop entry moved so the loop does not close refuses as FR-128 refuses. A `Trap` with `TrapClosure::Product` is routed to FR-181's product replay and never explored as a state-graph closure. | Test (TC-595) |

## Dependencies

- ADR-022 §5 GX-1 to GX-5; §6 GR-2 to GR-4 (concrete evidence under
  reductions); ADR-018 §5 CX-2 and CX-3; ADR-013 O-25 to O-27.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (`replay`, `explore_request`),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (shared refusals and
  `ModelStep`).
- QSpec owns the `GraphEvidence` wire and its replay rules, and exhaustive
  exploration from a recorded state (ADR-022 QS-5, QS-6).

## References

- ADR-022. QSpec half: QSpec FR-393 and FR-394 (Linear STD-135; ADR-022
  QS-5, QS-6).
