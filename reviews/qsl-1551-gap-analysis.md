---
id: SR-2159002
title: "Gap analysis of the QSL-1551 main-red fix (quire-spec-language PR 2159)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-083-resolve-unique-most-specific-dispatch.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: references
---

# Gap analysis of the QSL-1551 main-red fix (PR 2159)

## Summary

Ticket: QSL-1551. The change restores three existing tests to green without a spec change:
FR-230-AC-21 (TC-685) and FR-083-AC-4 (TC-225) keep their existing `#[trace]` tags, and the changed
code (the point-span branch of `expression_spans_at`) is owned by FR-230 clock-guard emission (QSL-580).
No untagged criterion, ownerless code, stub or coverage inflation was found in the diff. Plan completion:
not assessed (no plan supplied). Semantic review was not run (no Jev client).

## Verdict

**PASS**: no gaps.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |
