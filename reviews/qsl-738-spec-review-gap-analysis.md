---
id: SR-8726
title: "Traceability review of FR-360 items, ACs and TC-1006..TC-1010 (QSL-738)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0e8fe78c290748d5e81ba2318514920aa846a9af; spec/functional/FR-360-*.md; spec/test-cases/TC-997, TC-1004, TC-1006..TC-1010; spec/tests.md"
review_set: subset
date: 2026-10-13
---
# Traceability review of FR-360 items, ACs and TC-1006..TC-1010 (QSL-738)

## Summary

Ticket: QSL-738. PR: quire-spec-language#809 (spec only). Planless spec-level traceability: target-design items against ACs, ACs against TC rows and steps. Every AC has a TC row (1-3 -> TC-1006, 4-5 -> TC-1007, 6-7 -> TC-1008, 8 -> TC-1009, 9-11 -> TC-1010), every TC row is marked as not yet built (🚧), and FR-056-AC-27/28/32 -> TC-997 and AC-31 -> TC-1004 still hold after the rewrite. Plan completion: not assessed.

## Scope examined

- `FR-360-AC-1` (examined)
- `FR-360-AC-2` (examined)
- `FR-360-AC-3` (examined)
- `FR-360-AC-4` (examined)
- `FR-360-AC-5` (examined)
- `FR-360-AC-6` (examined)
- `FR-360-AC-7` (examined)
- `FR-360-AC-8` (examined)
- `FR-360-AC-9` (examined)
- `FR-360-AC-10` (examined)
- `FR-360-AC-11` (examined)
- `TC-997` (examined)
- `TC-1004` (examined)
- `TC-1006` (examined)
- `TC-1007` (examined)
- `TC-1008` (examined)
- `TC-1009` (examined)
- `TC-1010` (examined)
- `FR-360 item 1` (examined)
- `FR-360 Behavior` (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Item 1 admits variant-typed fields on state machines and processes too, but no AC or TC step covers either. AC-1 and TC-1006 cover only an object, a record value type and an event. Add a state-machine context field and a process field typed `Tier` to AC-1 and TC-1006 step 1. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:54-58, :217 |
| FND-002 | low | TC-1008 step 4 expects only 'the two in the same order whatever the source order', so a wrong but consistent order passes. Name the expected order, which depends on SR-8723 FND-002. | spec/test-cases/TC-1008-equality-of-variant-values-is-by-case.md Expected Results step 4 |
| FND-003 | low | The Behavior rule 'a variant slot never holds `none` unless its type is `Option<V>`' has no AC vector. Add `none` at a `Config::Tier` slot (and the replay `{"absent":{}}` at a required variant field) to AC-5 or AC-10. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:210-211 |
| FND-004 | low | The AC-1 occurrence-field refusal names the path `TypeDefinition.occurrenceField` but not the node. FR-056-AC-17 locates it at the event type node. Name the event type node in AC-1 and TC-1006. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:217 |

## Verdict

Findings. The AC-to-TC mapping is complete, with no orphan ACs or TCs. Some item claims are not covered by any AC, and one TC step cannot fail.
