---
id: SR-7879
title: "QSL-737 PR 800 integrity analysis: FR-359, TC-998..TC-1002, index rows"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@67de856995d7f102f050fad3275c167e4f66b26d; spec/functional/FR-359-*, spec/functional/FR-056-*, spec/functional/FR-090-*, spec/test-cases/TC-998..TC-1002, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-737. This analysis checks the PR's structural integrity: id
collisions, index rows, the AC-to-TC mapping, whether expectations are typed,
and whether the three-dot diff removes anything it should not.

## Verdict

CONDITIONAL. These checks are clean:

- **Ids.** FR-359 and TC-998..TC-1002 exist only on `origin/spec/child-composite-at-parent-slot`. They appear on no other remote branch and in none of the 24 other open PRs.
- **Index rows.** `spec/spec.md` gains the FR-359 `contains` relationship and table row. `spec/tests.md` gains five TC rows whose AC lists cover FR-359-AC-1..AC-10.
- **Mapping.** Every AC names exactly one TC, and every TC lists its ACs in Scope.
- **Typed expectations.** Every TC asserts typed causes, values or member paths, never message text.
- **Removals.** The three-dot diff removes exactly three lines: the FR-056 line reflowed around the new pointer, and the two FR-090 cause rows that were intentionally reworded. No unrelated text is removed.
- **Validation.** `quire validate` (0.36.2) passes on all touched files.

Two testability defects remain.

## Examined Scope

```yaml
scope:
  - {id: FR-359-AC-1..AC-10, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: TC-998, path: spec/test-cases/TC-998-field-read-at-a-parent-slot-reads-by-key.md, role: examined}
  - {id: TC-999, path: spec/test-cases/TC-999-equality-of-a-child-at-a-parent-slot-is-unequal.md, role: examined}
  - {id: TC-1000, path: spec/test-cases/TC-1000-option-and-collection-payloads-admit-a-child.md, role: examined}
  - {id: TC-1001, path: spec/test-cases/TC-1001-model-operation-arguments-admit-a-child-at-a-parent-parameter.md, role: examined}
  - {id: TC-1002, path: spec/test-cases/TC-1002-replay-and-kernel-admit-a-child-at-an-ancestor-slot.md, role: examined}
  - {id: FR-056, path: spec/functional/FR-056-admit-domain-package-model-declarations.md, role: examined}
  - {id: FR-090, path: spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, role: examined}
  - {id: spec.md, path: spec/spec.md, role: examined}
  - {id: tests.md, path: spec/tests.md, role: examined}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-359-AC-3 requires that the package for "the same program with `ExpressOrderPlaced` removed has the same read and call nodes with the same node ids". TC-998 step 5 goes further and compares "every node id". As written, neither can be satisfied. The AC-1 program calls `f` with an `ExpressOrderPlaced` argument, so that call cannot exist once the type is removed. A package that contains `ExpressOrderPlaced` also has nodes the other package lacks. The AC should name the nodes compared: the `f`/`g` function, parameter and `record.project` nodes, and the `OrderPlaced` record node. | FR-359-AC-3; TC-998 step 5 |
| FND-002 | low | TC-998 step 6 hand-builds a package whose field read names an absent field and expects `field-slot-unresolved`. That is FR-359-AC-10, which TC-998's Scope and its `tests.md` row do not list. TC-1002 step 5 repeats the same case. Either drop step 6 or add AC-10 to TC-998's scope and row. | TC-998 step 6; FR-359-AC-10; spec/tests.md TC-998 row |
