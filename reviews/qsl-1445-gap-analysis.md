---
id: SR-202610310154
title: "Gap analysis — checked equality emission rows"
type: SpecReview
analysis: gap-analysis
scope: "QSL-1445; QSpec FR-322-AC-288; QSL FR-390-AC-11; qsl-package/src/emit.rs, qsl-package/src/emit/tests/converted_equality.rs"
review_set: subset
---

## Summary

The QSpec enum identity and typed-source clauses were manually traced through the QSL emission code and focused tests because the installed Quoin gap-analysis method was unavailable. Production joins retain exact keys and refuse absent or ambiguous mappings, while two cases central to the ticket are not discriminated by tests.

## Coverage

**Plan completion: not assessed.** The reviewed source implements checker-held `ValueType` transfer, read-only producer data, and exact-key enum node lookups. Existing focused tests cover Text and primitive-bounded Int Plan operand facts and a local enum, but do not prove declaration-bearing Plan type retention or distinct source/lowered enum identity mapping. The full-root matrix could not be obtained: installed `quoin` 0.29.1 has no `gap-analysis` command, and `quire matrix --scope . --format tsv` / `quoin matrix --repo . --json` stalled without output. Manual review was used instead.

| Requirement unit | Expected evidence | Observed evidence | Coverage |
| --- | --- | --- | --- |
| QSpec-FR-322-AC-288: source types are preserved | Test proves Text/Plan source type identity and checker-held `ValueType`, including a declaration-bearing Plan type | `text_and_plan_equality_rows_retain_operand_type_identities` tests Text and `Int[-100,100]`; broader positive vector has `Point` but does not assert its emitted typed witness | Partial |
| QSpec-FR-322-AC-288: enum exact declaration/member identities, case, ordinal, ordered | Per-fact row has exact source/lowered declaration and member keys/IDs and test rejects swapped identity mappings | `enum_member_facts_join_source_and_lowered_declarations_to_nodes` checks keys and values; fixture is a local enum and only checks each ID occurs somewhere in the package node list | Partial |
| QSL-1445: producer data is read-only and usable | Public accessor returns borrowed producer data for consumer inspection | `producer_data()` returns `&EqualityProducerData`; tests inspect schedule and operand facts | Covered |
| QSL-FR-390-AC-11 | Repeated calls and emitted-package reads preserve result and charge log; refusal leaves meter unchanged | Unchanged evaluator/reader behavior; this PR changes the emission sidecar witnesses and does not alter evaluation paths | Not changed by this PR |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Declaration-bearing Plan ValueType is not asserted by the new typed-witness regression | qsl-package/src/emit/tests/converted_equality.rs:707-711 |
| FND-002 | medium | Enum regression cannot detect swapped source and lowered checked-node IDs | qsl-package/src/emit/tests/converted_equality.rs:792-865 |

## Findings detail

### FND-001 — Declaration-bearing Plan ValueType is not asserted

QSL-1445 explicitly requires checker-held declaration types for Plan equality. The row construction copies `producer_fact.value_type`, but the new focused typed-witness test uses only `Int[-100, 100]` for Plan. The separate `Point` fixture in `qspec_positive_equality_vectors_match_typed_producer_facts` asserts producer-side schedule/operands but does not inspect emitted `EqualityFactWitnessRow::value_type()`. A regression that loses or mismatches the nominal Plan `ValueType` can therefore pass the focused test. Exercise the `Point` Plan equality through the emitter and compare each row's `ValueType` with the exact corresponding checker fact.

### FND-002 — Enum test does not discriminate source/lowered node IDs

The enum test uses a local `Color` declaration and checks that `source_declaration_node`, `declaration_node`, and `member_node` each occur somewhere in emitted IDs. It never maps each structural key to its own expected emitted ID, and a local declaration does not demonstrate distinct source versus lowered declaration identities. A swapped or cross-wired source/lowered ID can pass. Use a selected/imported enum fixture with distinct source and lowered keys, then assert each row key maps to its exact checked node ID.

## Method note

`quoin gap-analysis` was unavailable (no such command in installed Quoin 0.29.1). Full-root Quire/Quoin matrix commands stalled without output. This artifact records a manual requirement-to-code-and-test check; it is not a Quoin-generated matrix analysis.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Point Plan equality now enters the emitted-row assertion loop and checker-held ValueTypes are compared. |
| FND-002 | fixed | Exact-key helper test uses distinct keys and selected model enum rows retain their exact structural keys and IDs. |
