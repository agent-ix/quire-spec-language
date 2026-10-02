---
id: FR-330
title: "Run a selected temporal clause over a supplied trace through the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
---
# FR-330: Run a selected temporal clause over a supplied trace through the spine

## Description

`qsl_replay::spine::run_clause` (FR-109) SHALL accept a `Temporal` selection
that names one temporal clause, a supplied trace and the clause's `over`
binding. It compiles the unit through the spine, admits each observation of
the trace (FR-106), evaluates the clause at S6a through the TemporalTrace
evaluator (FR-327, FR-328, FR-329), and returns one `ClauseRunReport`. With
it, every admitted clause kind executes through the spine, as the reference
runtime's exit asks (ADR-011 §5).

## Use case

An operator has recorded observations of a running system and a QSL unit
with temporal clauses. They run one clause by name over the recording, as
they already run a state clause, and get one report: held on this trace,
violated at a named position, pending, unsupported, or incomplete.

## Inputs

FR-109's `ClauseRunRequest`, with the selection
`Temporal { name, trace, over }`:

- `name`: a temporal clause's declared name;
- `trace`: `TraceInput::Finite(Vec<DocumentRef>)`,
  `TraceInput::Lasso{prefix: Vec<DocumentRef>, loop: Vec<DocumentRef>}`, or
  `TraceInput::Model{initial: DocumentRef, universes, prefix:
  Vec<ModelStep>, loop: Vec<ModelStep>}`; each `DocumentRef` names a
  snapshot in the request's snapshot provision by digest (FR-106);
- `over`: `{"reference": {"population": ..., "key": ...}}`, resolved in the
  observation at position 0, when the clause has an `over` parameter.

## Outputs

FR-109's `ClauseRunReport`: the disposition, the compiled `package_id` and
the usage, with no provenance record of the source, the selection or the
observations read. Its `category` adds `inconclusive` (a pending result)
and `unsupported` (a missing fairness premise), and its `disposition` for a
`violation` carries the failing `TemporalPosition`.

## Behavior

- The entry SHALL compile, check the expected `package_id`, and report
  compile results exactly as FR-109 states.
- The entry SHALL resolve the name in the compiled package's temporal
  clauses. When it resolves to none, the entry SHALL report stage `select`,
  `missing_declaration`/`missing-name`.
- The entry SHALL admit every observation of the trace by FR-106 before
  evaluation, in position order, and report the first admission failure at
  stage `admit`. For `TraceInput::Model`, it SHALL admit the initial state
  and reconstruct the positions as FR-329 states for `Lasso::Model`.
- When `over` resolves to no object of the observation at position 0, the
  entry SHALL report stage `admit`, `invalid_runtime_input`/
  `wrong-role-mapping`.
- The entry SHALL evaluate exactly once: FR-327 for a bounded-profile
  clause over `Finite`; FR-328 for an infinite-trace clause over `Finite`;
  FR-329 for an infinite-trace clause over `Lasso` or `Model`. A
  bounded-profile clause over `Lasso` or `Model` SHALL report stage
  `admit`, `invalid_runtime_input`/`invalid-value`: a bounded profile reads
  a closed finite trace.
- A refusal of FR-329 (malformed or unfair lasso) SHALL report stage
  `evaluate`, category `refusal`, with its record.
- The entry SHALL map `Completed(true)` to `success` (exit 0);
  `Completed(false)` to `violation` with the failing position; `Undefined`
  to `violation` with its position and its `UndefinedEvaluation` cause, so
  it reports category `violation`, never FR-109's `undefined` category;
  `Pending` to `inconclusive` (exit 0, completed without violation);
  `MissingFairnessPremise` to `unsupported` (exit 21) with cause
  `unsupported_projection`/`missing-fairness-premise`; and `Incomplete` to
  `incomplete` (exit 22), by QSpec FR-301's exit contract. Every
  `violation`, an undefined evaluation included, exits with the violation
  exit, 10.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-330-AC-1 | From source, the `Counter` unit's clause `Bounded` (`eventually[0,1] holds(c.value = 2)`, event-position false-extension) over `Finite` snapshots with `c.value` 0, 1, 2 reports stage `evaluate`, `violation`, position 0, exit 10. | Test (TC-840) |
| FR-330-AC-2 | The unit's infinite-trace clause `Reaches` (`eventually holds(c.value = 2)`) over the same `Finite` trace reports `inconclusive`, exit 0, with no `truth`; over `Lasso` with an empty prefix and loop 0, 1 it reports `violation` at position 0; over `Lasso` with loop 0, 1, 2 it reports `success`, exit 0. | Test (TC-840) |
| FR-330-AC-3 | A selection naming `Absent` reports stage `select`, `missing_declaration`/`missing-name`; `Bounded` over a `Lasso` reports stage `admit`, `invalid_runtime_input`/`invalid-value`; an `over` naming key `ghost` reports stage `admit`, `invalid_runtime_input`/`wrong-role-mapping`; a `Lasso` with an empty loop reports stage `evaluate`, `refusal`, `invalid_runtime_input`/`invalid-value`. | Test (TC-840) |
| FR-330-AC-4 | `Reaches` over the `Lasso` with loop 0, 1, 2 and a work budget of zero reports `incomplete`, exit 22; running any request of AC-1 to AC-3 twice gives equal reports. | Test (TC-840) |
| FR-330-AC-5 | From source, FR-327-AC-5's first clause over the `Finite` snapshots with `c.value` 0, 1, 2 reports `violation`, exit 10, position 2 and cause `UndefinedEvaluation` with `division-by-zero`. | Test (TC-847) |
| FR-330-AC-6 | From source, a clause `Fair` (`always eventually holds(c.value = 2)` under `fair weak inc`) over `Lasso` with an empty prefix and loop 0, 1, 2 reports stage `evaluate`, `unsupported`, exit 21, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`. | Test (TC-848) |

## Dependencies

- ADR-011 §5 (the spine replacement for native clause execution); ADR-014
  §5 A-4.
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md) (the run entry,
  its compile and report rules), [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-328](FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md),
  [FR-329](FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md).
- QSpec FR-301 (exit codes), FR-362 (the missing fairness premise).

## References

- Linear QSL-384 (spec ticket; the temporal share of the reference-runtime
  exit); QSL-43 (implementation).
