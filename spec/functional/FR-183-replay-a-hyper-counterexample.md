---
id: FR-183
title: "Replay a hyper counterexample through ModelSystem"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
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
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: depends_on
---
# FR-183: Replay a hyper counterexample through ModelSystem

## Description

QSL SHALL replay a hyper counterexample through the layer-6 replay facade
(FR-098) at ADR-011 E9: `qsl_replay::replay_model_trace_tuple` takes
FR-098's request with one subject's inputs per alias and a
`WitnessEnvelope<HyperCounterexample>` whose source is
`ReplaySource::ModelTraceTuple` (ADR-023 HX-1). It re-executes every trace
through its subject's `ModelSystem`, checks the match and each trace's
fairness, and evaluates the body over the tuple (ADR-023 HX-2, HX-3, HX-5,
PA-7). For a `WitnessExhausted` counterexample it recomputes the witness
sets along the universal traces, so the absence of a matching existential
run is checked with no engine present. Replay needs no model checker and no
solver. The same crate, `qsl-replay` (ADR-029 CB-2), holds the
product-closure certificate checker of FR-163, which an EN-1 `proved` must
pass before it settles (ADR-023 HX-7).

## Use case

An auditor replays a refuted noninterference claim: both traces
re-execute from their initial snapshots, the inputs match at every joint
step, and the outputs differ at the named joint position. For a refuted
opacity claim, the auditor's replay recomputes, from the model alone, that
no matching partner run survives past the named position.

## Inputs

- FR-098's request, with each alias's subject: initial snapshots in the
  byte provision and the alias's universes.
- A `WitnessEnvelope<HyperCounterexample>` (FR-070):

```rust
pub struct HyperCounterexample {          // QSpec FR-400, member for member
    pub kind: HyperCounterexampleKind,
    pub instance: Vec<(Name, Key)>,       // object parameter binding
    pub traces: Vec<HyperTrace>,          // as QSpec FR-400 states for the kind
    pub trace_position: u64,
}

pub enum HyperCounterexampleKind {
    Lockstep { loop_entry: u64 },                       // HP-2
    WitnessExhausted { loop_entry: u64, position: u64 }, // HP-3
    Projected,                                          // HP-6
    StepTuple,                                          // HP-1
    Undefined(UndefinedEvaluation),       // any form; a trace per variable,
                                          // universal and existential
}

pub struct HyperTrace {
    pub variable: Name,
    pub alias: Name,
    pub initial: u32,
    pub steps: Vec<HyperStep>,            // FR-128 ModelStep or the stutter marker;
                                          // Projected marks each skipped or visible
}
```

- `HyperCounterexample` implements `FamilyPayload` (FR-070-AC-5); its
  `trace_position` names the failing joint position (ADR-014 TR-2).

## Outputs

- An FR-072 replay result on the `ModelTraceTuple` arm carrying the
  evaluated value and `trace_position`, as FR-128 does; or a typed
  `ReplayRefusal` with no partial result.

## Behavior

### Common

- The reader SHALL refuse an envelope that breaks QSpec FR-400's
  validation rules, with the code and member FR-400 names, before any
  replay runs.

- The executor SHALL recompile and check the package, resolve the clause,
  read each variable's fairness set from the recompile, and build each
  alias's `ModelSystem`, by FR-128's rules and order and with FR-128's
  refusals.
- The executor SHALL re-execute each trace through its alias's
  `ModelSystem` with FR-101 `replay`, refusing as FR-128 refuses on an
  `initial` index outside the subject, a step that is not enabled, a
  post-state digest that differs, a loop that does not close at
  `loop_entry`, or a stutter marker on a non-terminal state.
- The executor SHALL refuse with `invalid_runtime_input`/`invalid-value`:
  traces of unequal step counts for `Lockstep` and `WitnessExhausted`; a
  joint step at which `μ_U` is false; a trace that is unfair under its own
  fairness set on its own loop.

### Undefined evaluation

- For an `Undefined` counterexample, each trace SHALL be a finite path from
  an initial state, one per trace or execution variable of the clause,
  universal and existential, as QSpec FR-400 states. The executor SHALL
  re-execute each path through its alias's `ModelSystem` with FR-128's
  step refusals, then evaluate `μ_U`, `μ_E` and the body's letters at the
  joint positions up to `trace_position` (ADR-018 UE-5, ADR-023 HV-8).
- When the first undefined evaluation is at `trace_position`, with the
  payload's variables, state keys, locus and cause, the executor SHALL
  settle `reproduced-with-evaluated-witness`, the `UndefinedEvaluation`
  being the arm's value. Any other result SHALL settle `inconclusive`,
  `Verdicts`.

### Lockstep (HP-2)

- The executor SHALL check `μ_U` at every joint step, the loop's closing
  step included, and each trace's fairness on its loop, then evaluate the
  body over the tuple by FR-175. `false` SHALL settle
  `reproduced-with-evaluated-witness`; `true` SHALL settle `inconclusive`,
  `Verdicts`.

### WitnessExhausted (HP-3)

- The executor SHALL re-execute and check the universal traces as for
  `Lockstep`, without evaluating the body.
- It SHALL recompute `X_0` to `X_position` along the universal traces from
  the existential aliases' `ModelSystem`s and the body's safety automaton,
  by FR-177's rule, under the request's `max_witness_set` (FR-184).
- `X_position` empty with every earlier `X` non-empty SHALL settle
  `reproduced-with-evaluated-witness`. A non-empty `X_position`, or an
  earlier empty `X`, SHALL settle `inconclusive`, `Verdicts`. Reaching
  `max_witness_set` SHALL return the replay result stopped with that limit
  and its value, which FR-182 settles V-7, as FR-170 settles a stopped trap
  replay.

### Projected (HP-6)

- The executor SHALL check that each skipped mark names a listed operation
  and each visible mark an unlisted one, that the visible counts are equal,
  and that `μ_U` holds at every joint visible step, refusing otherwise with
  `invalid_runtime_input`/`invalid-value`.
- It SHALL evaluate the body three-valued over the joint projected prefix
  (FR-178). `false` SHALL settle `reproduced-with-evaluated-witness`; any
  other value SHALL settle `inconclusive`, `Verdicts`.

### StepTuple (HP-1)

- Each trace SHALL be a finite prefix from an initial state to the
  execution's pre-state followed by the execution's transition, with no
  loop entry. The executor SHALL re-execute each, check the final
  transition is enabled with the recorded post-state, and evaluate the body
  over the tuple of executions through the one clause evaluator (FR-107).
  `false` SHALL settle `reproduced-with-evaluated-witness`; `true` SHALL
  settle `inconclusive`, `Verdicts`.

### Engine independence

- A counterexample from EN-2, or found under symmetry or copy-swap, SHALL
  arrive concretised (FR-174), carrying no canonical state or permutation,
  and SHALL replay by the rules above.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-183-AC-1 | ADR-023 §8.1's leaky `Lockstep` counterexample replays to `reproduced-with-evaluated-witness` with `trace_position` 1. §8.2's leaky `WitnessExhausted{position: 1}` replays, recomputing `X_0 = {((1, 0), g)}` and an empty `X_1`. | Test (TC-608) |
| FR-183-AC-2 | FR-178-AC-2's `Projected` counterexample and FR-179-AC-2's `StepTuple` counterexample each reproduce. | Test (TC-608) |
| FR-183-AC-3 | Refusals settle no result: §8.1's counterexample with one trace one step shorter; with `b`'s first input changed to 1 (`μ_U` false); with one post-state digest altered; a `Projected` counterexample with a `step` marked skipped. An HP-2 counterexample under `fair { weak V::Vault::reset }` on `a` whose loop never takes or disables `reset` refuses as unfair. | Test (TC-608) |
| FR-183-AC-4 | A hand-built `Lockstep` envelope over the leaky vault with `a` and `b` both from `(0, 0)` taking `step(0)`, loop entry 1, on which `l` stays equal, settles `inconclusive`, `Verdicts`; a hand-built `WitnessExhausted{position: 1}` envelope for `Opaque` over the secure vault, with `a`'s lasso `(0, 0) -step(0)-> (0, 0)`, recomputes a non-empty `X_1` and settles `inconclusive`, `Verdicts`; §8.2's leaky counterexample replayed with `max_witness_set` 0 returns the replay result stopped with `max_witness_set` and value 0. Replaying one envelope twice gives equal results. | Test (TC-608) |
| FR-183-AC-5 | FR-179-AC-4's `Undefined` counterexample, with one path per execution variable, replays to `reproduced-with-evaluated-witness` with its `UndefinedEvaluation` as the value; the same payload with its cause changed to `precondition-false` settles `inconclusive`, `Verdicts`. | Test (TC-610) |
| FR-183-AC-6 | FR-177-AC-5's `Undefined` counterexample, whose traces include the existential variable's path to the state where `μ_E` has no value, replays to `reproduced-with-evaluated-witness`; with the existential trace removed the reader refuses it as `invalid_runtime_input`/`invalid-value` before replay. | Test (TC-602) |

## Dependencies

- ADR-023 §7 HX-1 to HX-6, §6 HV-1; ADR-029 CB-2; §13 PA-6 and PA-7, §16 CS-4; ADR-018 §5 CX-2 and
  CX-3; ADR-014 TR-2 and A-4; ADR-013 O-25 to O-27.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (shared refusals),
  [FR-175](FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md),
  [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md).
- QSpec owns the `HyperCounterexample` wire and its replay rules (ADR-023
  QS-6).

## References

- ADR-023. QSpec half: QSpec FR-400 and FR-401 (Linear STD-136; ADR-023 QS-6).
