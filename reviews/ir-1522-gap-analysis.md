---
id: SR-2026111204485700000002
title: "gap analysis of PR 2686 PathGuard constructor and path-condition test seam"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/claims.rs, qsl-semantics/src/check/claims/tests.rs"
review_set: subset
relationships:
  - type: references
    target: "ix://agent-ix/quire-spec-language/FR-062"
---

## Summary

Ticket: IR-1522. Gap analysis of PR agent-ix/quire-spec-language#2686, diff origin/main...HEAD, checked against the ticket's acceptance and the spec that owns the record surface (FR-062 "path condition", ADR-011 X-6 on `test-support`). Planless.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Verdict

PASS. Ticket acceptance, re-measured on the head:

- "Both public and documented": `PathGuard::new` is `pub`, ungated and documented. `RequirementRecord::with_path_condition` is `pub` and documented, under `test-support`, the same gate as `with_owner`, which ADR-011 X-6 prescribes for test-only items. A downstream test enables it with a dev-dependency feature, as qsl-replay already does. Met.
- "A downstream crate can build a record whose path_condition holds a guard at an occurrence that is not a checked node": the unit test does this in-crate, and the cfg gate is identical to `with_owner`, which qsl-replay uses downstream. quire-driver still needs its own dev-dependency line (noted in the code-review SR). Met for QSL's part.
- "No behavior change in checking": the diff adds only a constructor, a gated builder and a test. The checker's guard construction (claims.rs:1125) is untouched. Met.

Matrix: the diff adds and removes no criteria and no trace tags, so the computed Test Matrix does not change. Reverse gap: FR-062 owns what `check` records in a path condition, and a caller-built guard does not change that. The seam's purpose is owned by the ticket and the downstream quire-driver FR-020-AC-14, so it is not unowned behaviour. No stubs, no placeholder returns, no coverage inflation.

## Coverage

- Changed source items: 2 (both documented, both exercised by the new test).
- Changed tests: 1 (`a_record_can_carry_a_guard_at_a_non_checked_node`, untagged; no QSL criterion owns the seam).
- Semantic review: skipped (not requested).
- Plan completion: not assessed
