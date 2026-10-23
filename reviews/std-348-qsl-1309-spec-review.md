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
