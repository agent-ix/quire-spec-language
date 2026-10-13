---
id: SR-8725
title: "EARS conformance review of FR-360 (QSL-738)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@0e8fe78c290748d5e81ba2318514920aa846a9af; spec/functional/FR-360-*.md"
review_set: subset
date: 2026-10-13
---
# EARS conformance review of FR-360 (QSL-738)

## Summary

Ticket: QSL-738. PR: quire-spec-language#809 (spec only). EARS form of FR-360's statement and target-design items.

## Scope examined

- `FR-360` (examined)
- `FR-360 item 1` (examined)
- `FR-360 item 2` (examined)
- `FR-360 item 3` (examined)
- `FR-360 item 4` (examined)
- `FR-360 item 5` (examined)
- `FR-360 item 6` (examined)
- `FR-360 item 7` (examined)
- `FR-360 item 8` (examined)
- `FR-360 item 9` (examined)
- `FR-360 item 10` (examined)
- `FR-360 item 11` (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The statement is compound: one SHALL carries admit, type, check, evaluate, compare, replay and encode, and defers to 'as the target design below states'. This is acceptable as an umbrella only if each item carries its own SHALL (see FND-002). | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:37-41 |
| FND-002 | low | Items 3 to 8 state obligations in the indicative ('conforms', 'refuses', 'is true', 'follow') while items 1, 2 and 9 to 11 use SHALL. The modality is mixed. Use SHALL for the checker, evaluator and assembler obligations in items 3 to 8. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:77-134 |

## Verdict

Findings, low only. The statement is event-driven and well formed, but it is compound, and the items mix SHALL with indicative wording.
