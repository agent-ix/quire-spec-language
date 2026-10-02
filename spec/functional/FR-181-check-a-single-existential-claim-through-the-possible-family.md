---
id: FR-181
title: "Check a single-existential hyper claim through the possible family"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
---
# FR-181: Check a single-existential hyper claim through the possible family

## Description

QSL's layer-A `qsl-analyze` engine EN-1 (ADR-029 CB-3) SHALL check an HP-5 claim, a `hyper` clause over
`behaviours` whose prefix is exactly one `exists trace b of M`, as a
possibility claim with a temporal body (ADR-023 SE-1 to SE-5): from every
initial state of the subject, some behaviour fair under the clause's
fairness set satisfies the body. It is settled by the possible family's
verdict map and evidence shapes (ADR-022 GV-1, GV-2, GV-5, GV-7, GX-1, RU-5):
a witness lasso per initial state proves it once exploration of the product
has completed with no open node and no undefined evaluation of the body, a
closed part of the product with no fair accepting cycle refutes it, an
undefined evaluation of the body at a reachable position refutes it, and
anything else stays undecided. The
possible family's requirements (FR-166, FR-169 and FR-170) apply as stated
here.

## Use case

A verification operator asks whether a vault can settle into a public
output of 1 for good, from every start. The engine finds, per initial
state, a lasso of `step(1)` steps on which the claim holds, and the result
is `proved` with those lassos. Asked whether the output can reach 2, which
its type never allows, the engine examines each start's whole product and
returns `refuted` with basis `closed-scope`, since no single trace can show
that no trace exists.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-5 clause (FR-173), with
  its subject, the clause's fairness set and `ModelCheckLimits`, including
  `witness_samples` and the seed (FR-166).

## Outputs

- FR-168's `StateGraphOutcome` shapes: `Witnessed` with one lasso per
  initial state and its source, `WitnessedUnchecked` with the same lassos,
  `Trapped` naming an initial state with an empty stem, `Undefined`,
  `NoDecision`, or `NoInitialState`; settled by FR-169's map.
- `GraphEvidence::Witness` whose paths are FR-170 `ModelPath`s with a loop
  entry set, so each is a lasso in ADR-018 CX-2's step and loop shape; a
  state-graph witness has no loop entry.
- `GraphEvidence::Trap` with scope `Product`, naming the initial state whose
  part of the body-automaton product is closed and holds no fair accepting
  cycle; a state-graph trap has scope `Model` (FR-170). Replay re-explores
  the product for scope `Product` and the model closure for scope `Model`.
  Each is replayed as stated below.

## Behavior

### Order

- The engine SHALL classify every root of the subject as FR-176's
  pre-check does, and SHALL run no separate exploration of the subject for
  an HP-5 item. Phase 0 SHALL run next, then the product exploration below,
  whose `max_states` counts product states (ADR-023 HC-8).

### Check

- The engine SHALL build the product of the subject with the generalized
  Büchi automaton for the body itself, not its negation, and explore it by
  FR-126's first phase, retaining every edge.
- For each initial state, the engine SHALL search that initial state's part
  of the product for a fair accepting cycle by FR-126's SCC phase, with the
  clause's fairness set in FR-126's fairness filter.
- A product node the engine did not fully expand SHALL be open, with the
  causes FR-167 names.

### Phase 0

- Before exploring, the engine SHALL draw FR-166's walks per initial state,
  with FR-166's trace indices and seed. A walk that revisits a state SHALL
  give a candidate lasso. A candidate SHALL count as a witness only when its
  loop is fair under the clause's fairness set and the body evaluates `true`
  on it by FR-125.
- Phase 0 SHALL NOT settle the item: the product exploration SHALL run
  whatever phase 0 found.

### Outcomes

- An initial state with a fair accepting cycle in its part, or a sampled
  witness, SHALL have a witness lasso: the canonical lasso of FR-126 for an
  explored one, source `Explored`; the candidate, source `Sampled`, for a
  sampled one.
- When every initial state has a witness, the engine SHALL return
  `Witnessed` if the product exploration completed with no open node, and
  `WitnessedUnchecked` with the run's end and open causes otherwise, since a
  reachable undefined evaluation of the body is then not ruled out
  (ADR-022 RU-5).
- `Witnessed` SHALL carry the product-closure certificate of the explored
  product (FR-163); the item SHALL settle `proved` only after its witnesses
  replay and the certificate check accepts (FR-182, ADR-023 HX-7).
- If a letter of the body evaluates `Undefined` at a reachable product
  state, the engine SHALL return `Undefined` with FR-126's canonical
  breadth-first path to the first such product state, `where` naming that
  position and `cause` the evaluator's undefined cause (ADR-023 HV-8,
  ADR-018 UE-4). `Undefined` SHALL take the place of `Witnessed` and
  `WitnessedUnchecked`; between it and a trap, the first in canonical
  breadth-first order SHALL be returned.
- `Trapped` SHALL be returned for the first initial state, in subject
  order, whose part of the product is closed (no open node) and holds no
  fair accepting cycle.
- Every other case SHALL return `NoDecision` with the run's end and open
  causes.

### Replay

- A witness lasso SHALL replay by FR-128's step, loop and stutter rules and
  refusals, with fairness checked on its loop and the body evaluated by
  FR-125: `true` SHALL settle `reproduced-with-evaluated-witness`; `false`
  SHALL settle `inconclusive`, `Verdicts`.
- A trap SHALL replay by re-exploring that initial state's part of the
  product, unreduced, under the request's limits, and running the SCC phase
  with the fairness filter: no fair accepting cycle SHALL settle
  `reproduced-with-evaluated-witness`; one found SHALL settle
  `inconclusive`, `Verdicts`; a stopped exploration SHALL return the replay
  result stopped with its limit.
- An `Undefined` path SHALL replay as FR-170 replays undefined-evaluation
  evidence, re-executing the path and evaluating the body's letter at its
  last position.
- Each result SHALL state how it was settled, as FR-169 requires: the
  refutation's basis is `closed-scope` and its method exhaustive
  exploration of the trap's closure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-181-AC-1 | Over ADR-023 §8's secure vault, `exists trace b of V { eventually always holds(v.l @ b = 1) }` with `witness_samples` 0 returns `Witnessed` with one lasso per initial state, each ending in a loop of `step(1)` steps, source `Explored`; each replays to `reproduced-with-evaluated-witness`, and the item settles `proved`, `decisive-witness`. With the default `witness_samples` it settles `proved` too, and its record names each witness's source. | Test (TC-606) |
| FR-181-AC-2 | `exists trace b of V { eventually holds(v.l @ b = 2) }` returns `Trapped` for initial state 0 with an empty stem; replay re-explores its part of the product, finds no accepting cycle, and the item settles `refuted`, `closed-scope`. | Test (TC-606) |
| FR-181-AC-3 | Over the secure vault with FR-176-AC-3's `reset` operation, `exists trace b of V fair { weak V::Vault::reset } { eventually always holds(v.l @ b = 1) }` settles `refuted`, `closed-scope`, and the same clause with no fairness set settles `proved`. | Test (TC-606) |
| FR-181-AC-4 | AC-1's claim with `max_depth` 1 and `witness_samples` 0 returns `NoDecision` and settles `inconclusive`, `BoundReached{depth: 1}`. AC-1's witness with its loop's last step removed refuses as FR-128 refuses a loop that does not close. | Test (TC-606) |
| FR-181-AC-5 | Over the secure vault, `exists trace b of V { eventually always holds(1 / (1 - v.l @ b) = 1 and v.l @ b = 0) }`, whose division is evaluated first and has no value at `l = 1`, has a witness lasso looping `step(0)` at `(h, 0)` for each initial state, and still returns `Undefined` at the first product state with `l = 1`, cause `division-by-zero`, settling `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`. AC-1's claim with the default `witness_samples` settles `proved` only from a product exploration that completed with no open node. | Test (TC-615) |
| FR-181-AC-6 | AC-1's claim with the default `witness_samples` and `max_states` 2 gets a sampled witness lasso per initial state, the exploration stops at `max_states`, and the item returns `WitnessedUnchecked` and settles `inconclusive`, `WellDefinednessUnchecked`, never `proved`; with `witness_samples` 0 the same run settles `failed`, `resource-incomplete`, naming `max_states`. | Test (TC-616) |

## Dependencies

- ADR-023 §15 SE-1 to SE-5, §12 RU-3, HV-8; ADR-022 §3 GE-2, §4 GV-1, GV-2,
  GV-5, GV-7 and "Settlement method", §10 RU-5, §5 GX-1 and GX-3; ADR-018
  §3 EN-1, CX-2, CX-3.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product phases, fairness filter, canonical lasso),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (lasso replay rules),
  [FR-173](FR-173-classify-hyper-clauses-into-forms.md).
- The possible family's requirements: FR-166 (phase 0), FR-167 (open
  nodes), FR-168 (outcome shapes), FR-169 (verdict map and settlement
  method) and FR-170 (`GraphEvidence` replay).
- QSpec owns HP-5's routing to the possible family with its witness, trap
  and verdict rules (ADR-023 QS-3).

## References

- ADR-023, ADR-022. QSpec halves: QSpec FR-396's single-existential rows
  (Linear STD-136; ADR-023 QS-3) and QSpec FR-390 to FR-394, the possible
  family (Linear STD-135).
