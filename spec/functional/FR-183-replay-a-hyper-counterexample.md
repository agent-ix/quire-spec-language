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
pass before it settles (ADR-023 HX-7). The crate also holds the
match-witness check that an HP-2 `k`-induction proof with a non-empty `μ_U`
must pass before FR-182 settles it `proved`.

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
- For the match-witness check: FR-098's request and a `MatchWitness`:

```rust
pub struct MatchWitness {
    pub instance: Vec<(Name, Key)>,  // object parameter binding
    pub traces: Vec<HyperTrace>,     // one per universal variable, in quantifier order
    pub loop_entry: u64,
}

pub enum MatchWitnessCheck {
    Accepted,
    Refused(ReplayRefusal),
}

pub fn check_match_witness(
    wire: ReplayRequestWire,           // FR-098's request
    subjects: &[ModelTraceSubject],    // one per alias, in `clause.aliases` order (FR-128)
    witness: &MatchWitness,
    replay_limits: ReplayLimits,
) -> MatchWitnessCheck;
```

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
  `initial` index outside the subject, a step that is not enabled, a loop that does not close at
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
  step refusals, then evaluate `μ_U`, `μ_E`, the body's letters and each
  universal component's contract conjunctions at the joint positions up to
  `trace_position` (ADR-018 UE-5, UE-6, ADR-023 HV-8).
- The executor SHALL read the joint positions in increasing order and, at
  one joint position `p`, the expressions in the order EN-1 reads them
  (FR-176 "Product" for HP-2, FR-177 "Ranking findings" for HP-3):
  1. for `p` past 0, `μ_U` over the joint step that enters `p`, then
     `μ_E` over that step;
  2. the body's letter at `p`, its atoms in clause-node order (ADR-018
     UE-2);
  3. the contract conjunctions of each universal component's FR-120
     expansion at `p`, the components in quantifier order and, within one
     component, the conjunctions in the order its expansion records them.
     Every component of an HP-2 clause is universal. An existential
     member's undefined contract conjunction is no undefined evaluation:
     FR-177 settles it `Undecided(UndecidedSuccessor)`, so the replay does
     not read it as a locus.

  The first expression in this order that evaluates undefined is the
  replay's undefined evaluation; the expressions after it are not read.
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

- An `Undefined` counterexample of an `align skip` clause holds its skipped
  steps, unmarked, so its paths differ in total length, and its position
  counts visible steps. The executor SHALL refuse it with
  `unsupported_construct`/`not-yet-implemented` and SHALL NOT read it in
  lockstep, which would pair a skipped step of one path with a visible step
  of another.

### StepTuple (HP-1)

- Each trace SHALL be a finite prefix from an initial state to the
  execution's pre-state followed by the execution's transition, with no
  loop entry. The executor SHALL re-execute each, check the final
  transition is enabled with the recorded post-state, and evaluate the body
  over the tuple of executions through the one clause evaluator (FR-107).
  `false` SHALL settle `reproduced-with-evaluated-witness`; `true` SHALL
  settle `inconclusive`, `Verdicts`.

### Match witness (HP-2 non-vacuity)

- `check_match_witness` SHALL return `Refused` with
  `invalid_runtime_input`/`invalid-value` when `witness.instance` is not an
  instance of the clause: not exactly one `(parameter, key)` pair per
  member of `clause.parameters` in parameter order, each key in its
  population's universe (FR-125).
- It SHALL re-execute each trace through its alias's `ModelSystem` as for
  `Lockstep`, with the same step, loop and step-count refusals.
- It SHALL then check `μ_U` at every joint step, the loop's closing step
  included, evaluated under `witness.instance`, and each trace's fairness
  set on its own loop (ADR-023 HX-2).
  It SHALL NOT evaluate the body.
- It SHALL return `Accepted` when every check passes. A joint step at which
  `μ_U` is false, undefined, refused or incomplete, or an unfair trace,
  SHALL return `Refused` with `invalid_runtime_input`/`invalid-value`.
- FR-182's `settle_hyper_smt` SHALL apply these match-witness rules itself,
  over the item's own clause and subjects and the instance its outcome's
  `HyperProductIdentity` names (FR-315), with no recompile from a replay
  request. An HP-2 SMT outcome's witness is checked there, never by a
  caller that hands a `MatchWitnessCheck` to settlement.
- `settle_hyper_smt` SHALL likewise replay an HP-2 SMT outcome's
  `Lockstep` or `Undefined` counterexample itself, by this requirement's
  rules for its kind, over the item's own clause and subjects and under the
  instance its outcome's identity names, with no recompile from a replay
  request and no caller-supplied replay. A counterexample whose `instance`
  is not that instance is not replayed (FR-182).
- An `Accepted` witness is a tuple of fair behaviours that `μ_U` matches at
  every joint step under `witness.instance`. It establishes a matched fair
  tuple for that instance only, so FR-182 does not settle that instance
  `VacuousMatch` (ADR-023 HM-7).
- Building a witness is the SMT backend's work, an EN-2 lasso query with no
  body (ADR-018 DS-3). This requirement states only the check.

### Engine independence

- A counterexample from EN-2, or found under symmetry or copy-swap, SHALL
  arrive concretised (FR-174), carrying no canonical state or permutation,
  and SHALL replay by the rules above.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope, or the same request and match witness.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-183-AC-1 | ADR-023 §8.1's leaky `Lockstep` counterexample replays to `reproduced-with-evaluated-witness` with `trace_position` 1. §8.2's leaky `WitnessExhausted{position: 1}` replays, recomputing `X_0 = {((1, 0), g)}` and an empty `X_1`. | Test (TC-608) |
| FR-183-AC-2 | FR-178-AC-2's `Projected` counterexample and FR-179-AC-2's `StepTuple` counterexample each reproduce. | Test (TC-608) |
| FR-183-AC-3 | Refusals settle no result: §8.1's counterexample with one trace one step shorter; with `b`'s inputs changed to 1 at both steps (`μ_U` false at joint step 1; changing only the first input leaves the loop open and refuses earlier); a `Projected` counterexample with a `step` marked skipped. An HP-2 counterexample under `fair { weak V::Vault::reset }` on `a` whose loop never takes or disables `reset` refuses as unfair. | Test (TC-608) |
| FR-183-AC-4 | A hand-built `Lockstep` envelope over the leaky vault with `a` and `b` both from `(0, 0)` taking `step(0)`, loop entry 1, on which `l` stays equal, settles `inconclusive`, `Verdicts`; a hand-built `WitnessExhausted{position: 1}` envelope for `Opaque` over the secure vault, with `a`'s lasso `(0, 0) -step(0)-> (0, 0)`, recomputes a non-empty `X_1` and settles `inconclusive`, `Verdicts`; §8.2's leaky counterexample replayed with `max_witness_set` 0 returns the replay result stopped with `max_witness_set` and value 0. Replaying one envelope twice gives equal results. | Test (TC-608) |
| FR-183-AC-5 | FR-179-AC-4's `Undefined` counterexample, with one path per execution variable, replays to `reproduced-with-evaluated-witness` with its `UndefinedEvaluation` as the value; the same payload with its cause changed to `precondition-false` settles `inconclusive`, `Verdicts`. | Test (TC-610) |
| FR-183-AC-6 | FR-177-AC-5's `Undefined` counterexample, whose traces include the existential variable's path to the state where `μ_E` has no value, replays to `reproduced-with-evaluated-witness`; with the existential trace removed the reader refuses it as `invalid_runtime_input`/`invalid-value` before replay. | Test (TC-602) |
| FR-183-AC-7 | FR-178-AC-9's `Undefined` counterexample, whose paths each hold a skipped `mix` and a visible `step(0)`, is refused with `unsupported_construct`/`not-yet-implemented` and settles no result; it is not replayed as a lockstep counterexample. | Test (TC-608) |
| FR-183-AC-8 | Over ADR-023 §8's secure vault and `NonInterference` at instance `v`, the matched witness, with `a` and `b` each from initial `(0, 0)` taking `step(0)` twice and `loop_entry` 0, is `Accepted` without evaluating the body. The unmatched witness, with `a` the same and `b` from `(0, 0)` taking `step(1)` then `step(0)` and `loop_entry` 0, re-executes, since each loop closes, and is `Refused` with `invalid_runtime_input`/`invalid-value` at joint step 1, where `μ_U` is false. A witness whose `instance` binds `v` to a key outside the `vaults` universe is `Refused` with `invalid_runtime_input`/`invalid-value` before any trace is re-executed. Checking either witness twice gives equal results. | Test (TC-608) |
| FR-183-AC-9 | `μ_U` before the letter. Over ADR-023 §8's leaky vault, `NonInterference` with FR-176-AC-4's added `match` conjunct and the body `always holds(1 mod (1 + v.l @ a - v.l @ b) = 0)`, a hand-built `Undefined` envelope with `a` from `(0, 0)` and `b` from `(1, 0)`, one joint step `(step(0), step(0))` and `trace_position` 1, at which the conjunct and the letter both have no value, replays to `reproduced-with-evaluated-witness` when its locus is the `match` conjunct with cause `division-by-zero`; the same envelope with its locus changed to the body atom settles `inconclusive`, `Verdicts`. | Test (TC-608) |
| FR-183-AC-10 | The letter before a contract conjunction. Over the leaky vault with the added `pre G using v on V::Vault::step { 1 mod (1 - self.l) = 0 }` and the AC-9 body, under the §8.1 `match`, the hand-built `Undefined` envelope of AC-9's paths, at whose position 1 the letter and `b`'s `G` both have no value, replays to `reproduced-with-evaluated-witness` when its locus is the body atom; with its locus changed to `G` with `b`'s `step` transition it settles `inconclusive`, `Verdicts`. | Test (TC-608) |
| FR-183-AC-11 | Contract conjunctions in quantifier order. Over the secure vault with AC-10's `G` and `NonInterference`'s body, a hand-built `Undefined` envelope with `a` and `b` from `(0, 0)`, one joint step `(step(1), step(1))` and `trace_position` 1, at which the letter holds and `G` has no value on both components, replays to `reproduced-with-evaluated-witness` when its locus is `G` with `a`'s transition and only `a` named; with `b`'s transition and only `b` named it settles `inconclusive`, `Verdicts`. | Test (TC-608) |
| FR-183-AC-12 | `μ_U` before `μ_E`. Over the secure vault, `Opaque` with FR-176-AC-4's conjunct added to its `match`, which reads `a` alone and so is a `μ_U` conjunct, and FR-177-AC-5's conjunct, which reads `b` and so is a `μ_E` conjunct: a hand-built `Undefined` envelope with `a` and `b` from `(0, 0)`, each taking `step(0)`, and `trace_position` 1, at which both conjuncts have no value, settles `inconclusive`, `Verdicts` when its locus is the `μ_E` conjunct, since `μ_U` over the same joint step is read first. The envelope whose `b` path ends at `(0, 0)` before the step, as FR-177 shapes an undefined `μ_U`, with its locus the `μ_U` conjunct and cause `division-by-zero`, replays to `reproduced-with-evaluated-witness`. | Test (TC-602) |

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
