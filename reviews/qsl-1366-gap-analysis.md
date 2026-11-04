---
id: SR-2275002
title: "gap-analysis of the QSpec protocol mutation manifest replay (quire-spec-language PR 2275)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-383-AC-20; TC-1063; FR-518 row 7"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

# Gap analysis of QSL-1366 (quire-spec-language PR 2275)

## Summary

Ticket: QSL-1366. quire matrix lists FR-383-AC-20 as tagged, now including qsl-package/src/checked_v2/tests/protocol_mutation_manifest.rs:487 (tagged FR-383-AC-20 and TC-1063); the restated tests keep their tags. Every AC clause is covered: admit rows admitted, refuse rows exact code/cause/locus at the row stage with a wire refusal never satisfying a reader row, property assertions checked over the read-back package, nonzero count per class reported, later/pending reported as not executed. No code without an owner. No stubs: all asserts are value-asserting. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Clean. The one residual (pending mapping and exact counts reported but not asserted) is recorded as SR-2275001 FND-002.
