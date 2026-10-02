---
id: FR-283
title: "Monitor a supplied trace offline against temporal clauses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-283: Monitor a supplied trace offline against temporal clauses

## Description

`monitor` applies the layer-5 trace evaluator to a trace the caller supplies
(ADR-029 OP-3). It uses the semantics replay uses: ADR-014 A-4's finite-prefix
and lasso evaluation. It lives in layer A, crate `qsl-analyze`.

A runtime monitor generated from S5 IR is a `generate` target owned by the
driver with CG and RT (FR-280). It is held to the verdicts `monitor` gives on
a shared trace corpus.

## Inputs

- `&CheckedPackage`.
- One or more temporal clause selections.
- A trace document holding a finite trace, or a lasso: prefix positions
  followed by a non-empty loop that re-enters at its first position. Each
  position is a snapshot and an invocation.
- The evaluator limits.
- `&Cancel`.

## Outputs

`Staged<MonitorOutcome>`: one record per selected clause, holding its
verdict under ADR-014 A-4:

| Trace | Verdict | O-16 category | Exit (FR-285) |
| --- | --- | --- | --- |
| Finite | violation at a position, with the QSpec FR-351 separating witness | violation | 10 |
| Finite | pending | inconclusive | 0 |
| Lasso | violation | violation | 10 |
| Lasso | `tested` | success | 0 |
| Finite or lasso | evaluation undefined at a position: violation at that position, cause `UndefinedEvaluation{where, cause}` | violation | 10 |
| Finite or lasso, clause with a non-empty fairness set | no verdict: cause `unsupported_projection`/`missing-fairness-premise`, naming the first constraint of the fairness set | unsupported | 21 |

`tested` is evidence for that trace only, and is never `proved`. A clause
still pending when the trace ends observed no violation, so it exits 0, the
QSpec FR-301 code for a run completed without violation. A clause
record never carries the label `undefined` (FR-286).

A supplied trace carries states and steps and no enabledness, so it gives a
fairness constraint no premise to read. A clause with a non-empty fairness
set settles unsupported over it, as QSpec FR-362 states; a clause with an
empty fairness set admits every lasso.

## Behavior

- The `monitor` operation shall admit every trace position as a snapshot and
  an invocation (FR-106) before it evaluates any clause.
- If any position fails admission, then `monitor` shall refuse the whole
  request with that position's admission cause and its position index, with
  no clause record.
- The `monitor` operation shall evaluate each selected clause over the
  admitted trace with the layer-5 trace evaluator replay uses.
- When the trace is finite and ADR-014 A-4's finite-prefix rule makes the
  clause false for every extension, `monitor` shall report a violation.
- When the trace is finite and the finite-prefix rule does not decide the
  clause false, `monitor` shall report pending.
- If a lasso's loop is empty, then `monitor` shall refuse the request with
  `invalid_runtime_input`/`invalid-value`, with no clause record.
- If a selected clause's fairness set is non-empty, then `monitor` shall
  settle that clause's record unsupported, cause
  `unsupported_projection`/`missing-fairness-premise`, naming the first
  constraint of the fairness set, without evaluating the clause (ADR-014
  A-4, QSpec FR-362).
- When the trace is a lasso and the clause's fairness set is empty,
  `monitor` shall report a violation when the clause evaluates false and
  `tested` when it evaluates true.
- If a clause evaluates to undefined at a trace position, then
  `monitor` shall report a violation at that position, with cause
  `UndefinedEvaluation{where, cause}` naming the position and the undefined
  reason.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-283-AC-1 | Over a three-position finite trace whose `p` is false at position 2, `always p` reports a violation at position 2 with its separating witness, category violation. The same clause over a trace whose `p` is true at every position reports pending, category inconclusive, and FR-285 maps it to exit 0. | Test (TC-765) |
| FR-283-AC-2 | Over a lasso of a two-position prefix and a two-position loop, `always eventually q`, whose fairness set is empty, reports `tested` when `q` holds at a loop position and a violation when `q` holds at no loop position. | Test (TC-765) |
| FR-283-AC-3 | A trace whose second position's snapshot fails FR-106 admission is refused with that admission cause and position index 1, and the outcome holds no clause record. A lasso with an empty loop is refused with `invalid_runtime_input`/`invalid-value`, and the outcome holds no clause record. | Test (TC-766) |
| FR-283-AC-4 | For each AC-1 and AC-2 trace, the verdict `monitor` reports equals the verdict layer-6 replay's trace evaluation reports for the same clause and trace. | Test (TC-766) |
| FR-283-AC-5 | Over a three-position finite trace whose `x` is 0 at position 1, `always (10 / x > 0)` reports a violation at position 1, category violation, with cause `UndefinedEvaluation` whose `cause` is `division-by-zero`; the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-765) |
| FR-283-AC-6 | Over AC-2's lasso, `always eventually q` under `fair weak inc` settles its record unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`, and FR-285 maps the outcome to exit 21; in a request selecting it beside AC-2's clause without fairness, the second clause still reports `tested`. | Test (TC-766) |

## Dependencies

- ADR-029 OP-3: the operation.
- ADR-014 A-4: finite-prefix and lasso evaluation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): position admission.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- ADR-018 (draft): the temporal clauses.
- QSpec FR-300: the `monitor` operation; QSpec FR-362: the missing fairness
  premise over a supplied trace.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSpec FR-300 (STD-141): the QSpec half.
- QSpec FR-362: the missing fairness premise.
- QSL-366: an undefined evaluation settles as a violation with cause
  `UndefinedEvaluation`, and the team-leader decision ADR-029's References
  records applies it to `monitor`.
