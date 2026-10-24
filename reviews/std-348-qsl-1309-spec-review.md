---
id: SR-16340
title: "spec-review of FR-273 verdict table follows FR-401/FR-405 evaluator placement (quire-spec-language PR 1309)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@204cdc2b; spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md; spec/functional/FR-273-resolve-each-canonical-type-namesake.md; spec/test-cases/TC-748-every-canonical-type-namesake-is-deleted-or-renamed.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-272
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-273
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-748
    type: references
---

# Spec review of FR-273 verdict table follows FR-401/FR-405 evaluator placement (quire-spec-language PR 1309)

## Summary

Ticket: STD-348. One spec-review pass over `git diff origin/main...204cdc2b`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- Each verdict row 2..48 has exactly one AC (row n is checked by FR-273-AC-n), checked by script: AC text names the row's namesake.
- `Evaluation` -> `quire_eval::Evaluation`, `QualifiedName` one home in qsl-eval, FR-071-AC-3 reasoning (`FromStr`/`unqualified` are explicit fallible constructors) consistent with FR-071-AC-3.
- Ids FR-272-AC-5 and FR-273-AC-2..49 are on no other open head (open #1384 edits FR-273 and conflicts textually).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Narrowing by lead default: the two verdict rows that renamed `quire_semantic_value::location::Location` to `ExpressionLocation` and `Origin` to `ExpressionOwner` are removed from the table and replaced by AC-49 keeping both names. FR-401 and ADR-011 X-11 support keeping the names (evidence), but the standing rule makes any dropped or narrowed verdict a blocker even when flagged as a lead default. Peter must clear it (or the rows return) before merge. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md | missing-requirement |

## Verdict

Not mergeable: FND-001 (high, standing rule). No other defect; merge-tree against main is clean, but open #1384 conflicts in FR-273.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | high | Rows 49 and 50 and FR-273-AC-49/AC-50 make the gate report no `identifier` finding for `quire_semantic_value::location::Location`/`Origin`, but the same requirement says the table's namesakes are items of the QSL workspace members and that the `identifier` and `re-export` rules never apply inside the shared leaf `quire-semantic-value` (FR-272). The gate clause of AC-49/AC-50 therefore cannot fail, whether or not SV renames the items, and the requirement contradicts itself. Either drop the SV rows from the gate-checked table and state the rename as a separate SV-side requirement with a compile-resolution test (AC-51), or make the gate scan SV for these two items and say so in FR-272. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md | wrong-requirement |
| FND-003 | medium | The rename updates FR-401 only. ADR-011 (line 385 and the SV module text), ADR-012 (line 1615), ADR-013 (lines 398 and 567) and AD-001 (line 340) still name `quire_semantic_value::location::Location` as the evaluation interface's locus type, which after FR-273-AC-51 no longer resolves. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | fix verified at 2f09aa3f |

Round 1 verdict at 2f09aa3f: Not mergeable: FND-002 (high) and FND-003 (medium) are new. FND-001 is fixed (both rename rows restored, FR-401 uses the renamed types). Open #1384 also edits FR-273.
