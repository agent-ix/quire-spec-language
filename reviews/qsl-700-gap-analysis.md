---
id: SR-4956
title: "QSL-700 gap analysis of PR #718: FR-075-AC-9"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9304c4cf90423813450bc89a9a797ec14bf62440; PR #718 diff from acad2f9bf2f700194b3225ad6d9a67428526ea28: FR-075-AC-9, TC-984, qsl-route/src/request.rs, tests/it/request_builder.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---
# QSL-700 gap analysis of PR #718

## Summary

Ticket: QSL-700. Plan completion: not assessed. I performed a change-scoped
manual AC-to-test and source-to-requirement review, and checked the computed
Quire matrix. `quire matrix --scope . --format json` and `quoin matrix --repo . --json` both
report FR-075-AC-9 as `tagged` by
`tests/it/request_builder.rs::tc_984_items_preserve_occurrence_path_conditions`.
Quoin reports `no run evidence` for the criterion in its evidence store; the
separate exact-head make-ci log records `CI_EXIT=0`.

The change has an owning requirement and test case; the tagged integration test
uses the real checked-package request-builder path. The route item copies the
source record's ordered `PathGuard` sequence, and the test verifies source
identity, opposite outcomes and the unguarded case. The changed behavior has
no unowned source path or hollow test coverage in the reviewed scope.

## Verdict

PASS for the reviewed change scope. No gap found between FR-075-AC-9, TC-984,
the tagged integration test and the implementation. This is not a whole-repo
gap-analysis claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- `FR-075-AC-9`: tagged by TC-984; TC-984's source fixture is implemented in
  the new integration test and reaches `items_from_requirements`.
- Reverse gap: inspected every changed production expression; the only new
  behavior is the FR-075-AC-9 path-condition output, backed by that criterion.
- Hollow evidence: no test doubles replace the request builder; the test
  derives guard expectations from the independently produced requirement
  records and also asserts the then/else Boolean values directly.
- Plan completion: not assessed.
- Semantic review was not requested as a separate Quoin pass; intent-to-code
  agreement for this changed criterion was checked manually.
