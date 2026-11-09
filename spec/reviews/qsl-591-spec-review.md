---
id: SR-20261108591002
title: "Spec review of run_with_backends placement and the EntrySelection shape"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-279, FR-294, ADR-029 (git diff origin/main...spec/l5-phase1-api-shape at 9a08b77df)"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: reviews
---

## Summary

Ticket: QSL-591. Lenses: integrity, EARS, evidence, scope boundary, over the
diff only. The code on origin/qsl-591-fr401 was checked against the spec:
`qsl_replay::backends::run_with_backends` takes `&SuppliedBackends<'_>` and
`prepare: &PrepareLimits` (qsl-replay/src/backends.rs:24-37), `spine::run`
takes no `qsl_eval` type, and `EntrySelection` is a struct `{ kind: EntryKind,
name: QualifiedName }` with `function`/`clause` constructors
(qsl-eval/src/execution.rs:162-199). The FR-279 and FR-294 text describes that
code accurately. No other spec file states the old shape: FR-296 and ADR-029
EB-1 use `EntrySelection` only as a type name, and FR-100, FR-285 to FR-288
name only `spine::run`. No id collisions; no AC changed. `quire validate` on the
three files passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ADR-029 Context addition puts `run_with_backends` into a list headed "Measured on QSL main on 2026-10-01", which it was not, and the same bullet then says "None takes a cancellation input", which `run_with_backends(.., cancel: &Cancel)` contradicts. The entry is unrequested and wrong; drop the hunk. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:72-76 |
| FND-002 | medium | `run_with_backends` is a public entry the spec now places and describes (chosen backend, explicit `PrepareLimits`), but only in FR-279 Description prose: no Behavior statement and no AC in FR-279, FR-296 or FR-100 names it. Its two tests trace to FR-279-AC-4 and FR-296-AC-4, whose text covers only `execute`. Nothing in the spec can fail if it ignores `prepare` or the chosen backend. | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:60-67 |
| FND-003 | medium | The FR-294 `EntrySelection` row says it "is not an enum whose variants hold a bare QualifiedName (FR-088-AC-6)". FR-088-AC-6 says nothing about enum variants. The struct `{kind, name}` has the same identity as the enum `Function(QN)/Clause(QN)` and passes TC-258's scan only through its stated blind spot ("a name beside a filler field"). Either selection by name is allowed here (then the enum was fine and the sentence is wrong) or it is not (then the struct breaks the AC as well). Drop the negative sentence and its citation. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:61 |
| FND-004 | low | FR-279 calls `run_with_backends` one of "the spine's function entries", then says two lines later that it lives outside `qsl_replay::spine`. | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:61-67 |

## Verdict

Not mergeable as is. FR-279 and FR-294 match the code. Fix FND-001 (revert the
ADR-029 hunk) and FND-003 (delete the negative sentence) before merge. FND-002
needs one Behavior line and one AC in FR-279 (or FR-296) for `run_with_backends`
that the existing tests at execution_tests.rs:1097 and :1396 can trace to.
FND-004 is a wording fix.
