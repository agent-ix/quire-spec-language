---
id: SR-922
title: "QSL-343 spec review of PR 548 (FR-093-AC-17, TC-416 step 9, TC-421 step 4, tests.md)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3e95fb6a6f9d749b8fcd815f61d2814d7f41ea27; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (AC table, coverage note); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (Scope, step 9, Expected Results, Status); spec/test-cases/TC-421-package-source-map-carries-the-wire-source-map.md (step 4); spec/tests.md (TC-416 row); spec/spec.md (FR-093 row, read only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-343. PR: quire-spec-language#548 at 3e95fb6a.

FR-093-AC-17 is deleted along with its coverage note. TC-416 drops AC-17 from
its Scope, and drops step 9, its expected result and its Status line. The
tests.md TC-416 row drops AC-17 and its "passes locally (QSL-260)" clause.
TC-421 step 4 now says the fixtures are read "with each fixture's required
features as supported". That matches `read_fixture_wire`. A workspace grep
finds no remaining `FR-093-AC-17`, `diagnostics_catalog` or TC-416 step 9
reference in spec/. The spec.md FR-093 row never cited AC-17. Mentions in the
historical SR files under reviews/ are records, not live references. Nothing
was renumbered, so AC-1..16 and TC-416 steps 1..8 keep their ids.

The one spec-level consequence, that the catalog revision is now unowned, is
recorded in SR-921 FND-001 and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The deletions leave no dangling references.

## Dispositions

Round 1, reviewed at 985bb85bcfd85683f898a4131f3dceebc52678fb. There were no findings to dispose of. Re-checked after the rebase: the FR-093, TC-416 and tests.md conflict resolutions keep main's (#544) ticket-id strip, and the diff adds no `QSL-NNN` text to spec/. The new FR-093 paragraph, AC-17, TC-416 Scope, step 9, its expected result, its Status line and the tests.md row agree with one another and with the test. New AC-17 is testable, and it is tested.
