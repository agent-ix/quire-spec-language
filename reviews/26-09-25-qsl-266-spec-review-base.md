---
id: SR-636
title: "QSL-266 base checklist review of per-operation requirement records"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@64ee12700cd66bb17767a8e9090cbca114308364; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/functional/FR-057-admit-shared-capability-kinds.md; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/test-cases/TC-160-checked-family-contract-shape.md; spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---

## Summary

Ticket: QSL-266 (PR agent-ix/quire-spec-language#456, spec only). Base
checklist over the eight changed files: ID formats, AC testability, the
six coverage rules and the tests.md rows.

IDs are well formed and unique. TC-449 is new and not reused, and
FR-075-AC-8 is new. `quire validate` reports no structural error and no
grammar warning in any changed file. The six failures it does report are
the MP-003 to MP-006 MeasurementPlan frontmatter errors, which this diff
does not touch.

Each new or changed AC has a TC:
- FR-062-AC-4 and FR-062-AC-13 → TC-160 steps 5 and 10.
- FR-057-AC-10 → TC-160 step 10, plus TC-153.
- FR-075-AC-8 → TC-449 steps 1 to 5.

The TC expected results match the AC oracles clause by clause. The
tests.md TC-160 row adds FR-057-AC-10 to its trace. The TC-449 row traces
FR-075-AC-8 and is marked Planned under QSL-266. Both rows match their
files.

Two coverage gaps remain. The fixtures skip three extent rules the new
text defines, and several fixtures leave out the declared result type
their oracle depends on.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-062-AC-13 and TC-160 step 10 have no fixture for three rules the diff defines. (a) Query, `count`, `sum`, `fold` and `reduce` binders as extent roots (ADR-014 §4 names them): no fixture reads a binder. (b) A narrow whose operand is not a scalar operation application, for example FR-093-AC-4's `c2`, `3` narrowed into `Int[0, 9]`: the expected answer is no record at all, and nothing checks it. (c) Two `expression` occurrences of one application node whose extents or enclosing narrows differ (see SR-640 FND-002). Without these, an implementation that skips binders, panics on a narrow over a literal, or pairs occurrences by order still passes TC-160. | spec/functional/FR-062-implement-checked-family-contract.md:248; spec/test-cases/TC-160-checked-family-contract-shape.md:72-78, 116-128 |
| FND-002 | low | The FR-062-AC-13 fixtures give parameter types but not function result types, except `x + 1` → `Int[0, 10]`. The oracle depends on the result type. With result type `Int[0, 9]`, `-z` over `z: Int[0, 9]` does not check at all: the range [-9, 0] is not within [0, 9], so the Coerce range obligation refuses. With `Int[-9, 0]`, it checks with a narrow node. `x = y` needs `Boolean`. An implementer can pick types for which the stated oracle is wrong. | spec/functional/FR-062-implement-checked-family-contract.md:248; spec/test-cases/TC-160-checked-family-contract-shape.md:72-78, 116-128 |
| FND-003 | low | Disposition pass, at accbac38. FR-062-AC-13 now says each record has "the listed extent, result bound and path condition", but several fixtures do not list a result bound. RR-7, RR-8, RR-11 (`ge`, `le`), RR-12 (`ge`, `lt`), RR-14, RR-15 and RR-17 give no result bound, and RR-5 says only "its own result type". An implementation that writes the wrong result bound for those records (for example the narrow's range on a `+` that no narrow wraps) still passes TC-160 step 10 for them. Name the result bound in every row: `Integer` for RR-5, RR-7, RR-8, RR-14, RR-15 and RR-17, and `Boolean` for the comparisons. | spec/functional/FR-062-implement-checked-family-contract.md:294-306, 324 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@accbac3849a26f9f8206e655369105408b9f4a11` (fix commit `accbac38`, "QSL-266 spec: address spec review SR-636 to SR-640"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed accbac38 | The fixture table RR-1 to RR-17 adds the three missing cases. RR-14 reads a `forall` binder, with roots keyed by the binder. RR-10 is a narrow over a literal and gives no record. RR-15 (the sibling `let`s, `Bounded` and `Unbounded` at one node) and RR-16 (one `+` node under two narrows) are two occurrences with different extents or bounds. TC-160 step 10 checks each one, and RR-15 in both operand orders (FR-062:283-306, TC-160:72-76, 114-130). |
| FND-002 | fixed accbac38 | Every RR unit is a full declaration with its result type. RR-1 is `-z` into `Int[-9, 0]` and RR-4 is `x = y` into `Boolean` (FR-062:290-306). |

New in this pass: FND-003 (low), recorded in `## Findings` above. It has no outcome yet.
