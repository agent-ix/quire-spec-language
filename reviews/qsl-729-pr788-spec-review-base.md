---
id: SR-7855
title: "QSL-729 PR 788 FR-056 native Integer spelling review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@908d4cadc37f9d8ee78f755af74612f996668466; spec/functional/FR-056-admit-domain-package-model-declarations.md"
review_set: subset
---

## Summary

Ticket: QSL-729. Spec review over PR 788's FR-056 edit, which renames the native type `Int` to `Integer` at lines 215, 218, 329 and in FR-056-AC-17. Checked against QSpec TC-235 at qspec 1498c1e: line 31 names the natives `UUID`, `Integer`, `String`, `Boolean` and `Timestamp`, line 50 types `Money/amount_minor` as `Integer`, and M04 run 5 binds `Instant` as `Integer`. All four edits match TC-235.

Other `Int` occurrences in spec/ were checked. `Int[lower, upper]` in FR-056 (lines 485, 535, AC-16) is the QSL value-type range spelling, not a native name, and stays. `Int` in FR-091, FR-215, FR-315, FR-333, ADR-014, TC-393, TC-480, TC-897 and TC-910 is QSL source-grammar type syntax or an SMT sort, not the native IRI name. No `ix://quire/native/Int` remains in spec/ or code.

## Verdict

PASS with one low finding: FR-056 line 643 still spells the native as `Int`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The native payload control list still reads "Mixed UUID/Timestamp/Int pairs give a typed refusal, not false, under FR-149". This list is about native payload identities, the same `UUID`, `Integer`, `String`, `Boolean`, `Timestamp` set the PR renames elsewhere. Spell it `Integer` for consistency with lines 215, 218, 329 and AC-17. | spec/functional/FR-056-admit-domain-package-model-declarations.md:643 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-056 native types paragraph | examined | The compiler SHALL retain the distinct native types `UUID`, `Integer`, `String`, `Boolean` and `Timestamp` required by QSpec TC-235's K2 domain package. |
| FR-056 Instant binding sentence | examined | A value type `Instant` bound to `Timestamp` SHALL resolve to that native type; binding it to `Integer` does not satisfy an event's `Timestamp` occurrence-field requirement under FR-208. |
| FR-056 ownership paragraph | examined | QSL owns ... binding K2's `Instant` to `Timestamp` and refusing its substitution by `Integer` under the event occurrence-field rule. |
| FR-056-AC-17 | examined | K2's `Instant` resolves to `Timestamp`; changing that binding to `Integer` gives TC-235 M04 run 5's `invalid_model_binding`/`malformed-declaration` at `OrderPlaced`'s `TypeDefinition.occurrenceField`, rather than an admitted substitute. |
| FR-056 native controls list | examined | Mixed UUID/Timestamp/Int pairs give a typed refusal, not false, under FR-149. |
| FR-056-AC-16 | context_only | A domain package whose scalar type `Wide` is bound to `Integer` ... resolves to `Int[0, 18446744073709551615]` exactly. |
| QSpec TC-235 K2 and M04 | context_only | `UUID`, `Integer`, `String`, `Boolean` and `Timestamp` are native value types; M04 run 5 binds `Instant` as `Integer`. |
