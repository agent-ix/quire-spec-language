---
id: SR-20261103155258
title: "QSL-1556 r1 gap analysis of PR #2205 (FR-517 recipe R compensations)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2205 head vs origin/main; FR-517 new paragraph and recipe row 3; examples/checked-handoff/unit-event-position.native, examples/checked-handoff/model.semantic-ir.json, examples/protocol-handoff/workflow.body.native, tests/it/checked_handoff.rs; qsl-semantics spine (protocol_obligation.rs attempt_type/is_record) per the PR's claim"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
---

# QSL-1556 r1 gap analysis of PR #2205

## Summary

Ticket: QSL-1556. Compared the new FR-517 text with recipe R and the legacy
recipe. Plan completion: not assessed. No cargo build was run; `main` does not
compile (see below), so this is a read-only comparison.

- Binder types: the legacy uses `M::Workflow` for all three binders. R uses
  `M::Attempt` (field `tally`) and `M::Recovery` (field `receipts`), both
  `record` in the model IR. Matches the new text.
- Stages and bounds: `within [0,30]`, `attempts 3`, the activation, the commit
  and the recover stage are present in both templates. Matches.
- Retry relations: they do not match (FND-001).
- Recover conditions: R uses `size(...)` in place of the legacy `sum` against
  the target. QSL-1555 tracks this, and FR-517-AC-14 has an ignored test gated
  on it (tests/it/checked_handoff.rs:1550-1575). This is tracked, not a deletion.

## Verdict

Changes requested. The paragraph states that R keeps every retry relation of
the legacy recipe. R's retry relations have dropped the legacy conjunct
`targetFull >= 0` / `targetPartial >= 0`. The spine does not refuse it: the
`retry` stage may read the template's registration captures (FR-381 "Names a
stage may read"), and FR-383-AC-51 fixes that exact term in a retry relation.
No ticket tracks the dropped conjunct; QSL-1555 covers only the `recover` sum.
So a recipe element that the spine admits was deleted, and the PR text says
otherwise.

Fix it in this PR by restoring the conjunct, one line per template:

- `unit-event-position.native:44`: `and targetFull >= 0 and not activatedFull`
- `unit-event-position.native:64`: `and targetPartial >= 0 and not activatedPartial`

The edit can be checked by reading. `targetFull`/`targetPartial` are
registration captures, which R's `recover` already reads (lines 48, 69).
Nothing in tests/it/checked_handoff.rs or src/checked_handoff* depends on the
retry relation's shape or node count; FR-518 mutation pointers use `*` for
node indices. The new term is per template, so the test that Full and Partial
share only the effect node (checked_handoff.rs:440-462) is unaffected. The
focused `checked_handoff` test waits for `main` to compile, the same as every
other PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new paragraph says "The compensations keep every stage, bound and retry relation of the legacy recipe". R's retry relations drop the legacy conjunct `targetFull >= 0` / `targetPartial >= 0` (legacy workflow.body.native:19, 39). The spine admits that term (FR-381 retry row reads registration captures; FR-383-AC-51), and no ticket tracks it. So a recipe element was deleted, and the spec text claims it was kept. Restore the conjunct in both templates in this PR. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:149-151; examples/checked-handoff/unit-event-position.native:42-45, 62-65 |
