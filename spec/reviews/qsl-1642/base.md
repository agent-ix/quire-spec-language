---
id: SR-164201
title: "Base specification review — QSL-1642 timed population scope"
type: SpecReview
analysis: base
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-391-name-a-population-in-a-state-clause.md; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md; spec/test-cases/TC-1334-refuse-named-populations-in-timed-model-predicates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

The requirement keeps timed model predicates self-scoped and makes named-population reads an explicit typed refusal, while preserving FR-391 scope in state clauses and `holds` atoms. IDs and links are valid, and TC-1334 gives a concrete test procedure.

## Verdict

**CONDITIONAL** — the authored requirements are testable and the planned test case is concrete. The new acceptance criterion has no source test binding yet because implementation follows this specification review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Bind FR-230-AC-58 to the TC-1334 implementation test before the code PR is accepted. | FR-230-AC-58, TC-1334 |

## Checklist

- FR and AC identifiers use the repository's existing format; FR-230-AC-58 and TC-1334 continue the current sequences.
- TC-1334 verifies both timed predicate forms, a self-only control, and a unit-level precondition control.
- The new requirement contains no hash, digest, pin, manifest, or file-tracking instruction.
- The full Quire validation command passed. Existing repository-wide style warnings remain outside the changed artifacts.
