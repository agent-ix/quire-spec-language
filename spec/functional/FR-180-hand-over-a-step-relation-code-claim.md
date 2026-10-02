---
id: FR-180
title: "Hand over a step relation's code claim and replay its Kani counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: depends_on
---
# FR-180: Hand over a step relation's code claim and replay its Kani counterexample

## Description

When every execution variable of an HP-1 step relation names an operation
bound to an implementation by the abstraction relation, S3 SHALL record a
second requirement, an `operation-contract` code claim, beside the model
claim (ADR-023 XC-1), and S4 SHALL emit what a two-call harness reads. The
harness is generated and run on the IR/CG side; QSL states what it hands
over and what the harness must prove (ADR-023 XC-2), and replays a Kani
counterexample's tuple of executions (ADR-023 XC-4). QSL names no harness,
backend or Kani construct.

## Use case

A verification operator states that a vault's `step` is deterministic, and
`step` is bound to a Rust function. They get two results: the model claim
over the reachable transitions (FR-179), and a code claim that runs the
function twice from any two pre-states satisfying the declared invariants.
A Kani counterexample to the code claim replays in QSL against the same
body, with no reachability path.

## Inputs

- A `CheckedModelRelation` classified `StepRelation` (FR-173).
- Each operation's abstraction-relation binding (ADR-017 AR-1 to AR-4), its
  effective precondition, frame and postcondition, and each execution
  model's declared invariants, as S3 has checked them.
- For replay: FR-098's request and a Kani counterexample entering E9 by
  ADR-013 C-09's path, giving per execution a pre-state, an argument
  vector, a result and a post-state.

## Outputs

- One `Requirements{kind: operation-contract, extent}` record for the code
  claim, its extent per ADR-014 §4, beside the model claim's
  `temporal-satisfaction` record, each with its own obligation identity.
- In the checked package, for the relation node: its execution variables in
  order, each naming its operation; the body lowered to FR-322 terms over
  each execution's pre-state, arguments, result and post-state; each
  execution model's declared invariants; each operation's effective
  precondition, frame and postcondition; and each operation's
  abstraction-relation binding.
- For replay: an FR-072 replay result, or a typed `ReplayRefusal`.

## Behavior

### Hand-over

- S3 SHALL record the code claim only when every execution variable's
  operation has an abstraction-relation binding; otherwise the relation has
  the model claim alone.
- S4 SHALL emit the relation node with the members listed under Outputs, so
  the harness generator reads every premise and the body from the checked
  package.
- The code claim's meaning, which the generated harness must prove, SHALL
  be: for all pre-states `s_1 … s_n` of the declared types, each satisfying
  every declared invariant of its model, and all argument vectors in the
  operations' parameter domains with each operation's effective
  precondition true at its `s_i`, running each operation's bound
  implementation once from its own `s_i`, the calls sharing no state, gives
  results and post-states on which the body holds, for every outcome of a
  nondeterministic implementation.
- The code claim and the model claim SHALL have different obligation
  identities, and neither SHALL settle the other.
- A vacuous harness proof SHALL settle `inconclusive`, `KaniVacuousProof`,
  as ADR-013 C-09 states.

### Replay of a code-claim counterexample

- The executor SHALL recompile the package and resolve the relation by
  FR-098's rules and order, with FR-098's refusals.
- The executor SHALL admit each execution's pre-state as an FR-106 snapshot,
  refusing as FR-106 refuses.
- If a pre-state violates a declared invariant of its model, then the
  executor SHALL refuse `invalid_runtime_input`/`invalid-value`, naming the
  execution and the invariant.
- The executor SHALL evaluate the body over the tuple of executions through
  the one clause evaluator (FR-107). `false` SHALL settle
  `reproduced-with-evaluated-witness`; `true` SHALL settle `inconclusive`,
  `ReplayParity`.
- Replay SHALL use no reachability prefix.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-180-AC-1 | `Det` over a vault whose `step` is bound to an implementation records a `temporal-satisfaction` model claim and an `operation-contract` code claim with different identities; the checked package's relation node holds `x` and `y` naming `step`, the lowered body, `Vault`'s declared invariants, `step`'s effective precondition, frame and postcondition, and its binding. `Det` over a vault with no binding records the model claim alone. | Test (TC-605) |
| FR-180-AC-2 | A code-claim counterexample with two executions of `step` from equal pre-states and equal inputs giving `l = 0` and `l = 1` replays to `reproduced-with-evaluated-witness`; the same counterexample with equal post-states settles `inconclusive`, `ReplayParity`. | Test (TC-605) |
| FR-180-AC-3 | Over a vault variant that declares the invariant `self.l <= self.h`, a counterexample with pre-state `(h, l) = (0, 1)` refuses `invalid_runtime_input`/`invalid-value` naming the execution and the invariant; one whose pre-state fails FR-106 admission refuses with FR-106's record. | Test (TC-605) |

## Dependencies

- ADR-023 §14 XC-1 to XC-5, §12 RU-2; ADR-017 AR-1 to AR-4; ADR-013 C-09;
  ADR-014 §4.
- [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-179](FR-179-check-a-step-relation-over-reachable-transitions.md) (the
  model claim), [FR-098](FR-098-execute-a-replay-request.md),
  [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md).
- Harness construction, unwinding and state representation are CG's; IR
  admits the obligation (ADR-023 XC-5, DS-2, DS-3).
- QSpec owns the claim-form row for a relation's code claim and its Kani
  counterexample replay (ADR-023 QS-4, QS-6).

## References

- ADR-023. QSpec half: QSpec FR-397 (Linear STD-136; ADR-023 QS-4, QS-6).
