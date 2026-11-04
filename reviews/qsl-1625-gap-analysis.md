---
id: SR-20261104130236
title: 'Gap analysis of PR #2296 (QSL-1625): FR-390 reader-backed computed roots'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@52012fa99; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md (AC-15, AC-16, AC-24..27, AC-50, AC-60, AC-64), qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-package/src/reader_protocol_projection.rs; ticket QSL-1625'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
date: '2026-11-04'
---

## Summary

Ticket: QSL-1625. PR: quire-spec-language#2296, head 52012fa99. Plan completion: not assessed. Quoin `matrix` produced no output within its run, so a manual criterion-to-test check was done instead.

Results of the focused tests on the head:

- FR-390-AC-15 `reader_parity_matches_every_tc1070_slot_result_and_charge_log`: FAIL.
- AC-24 `reader_backed_projection_native_equality_preserves_scalar_identity_and_charges`: FAIL.
- AC-25 `reader_backed_projection_preserves_declared_and_compound_quantity_units`: pass.
- AC-25 `reader_backed_projection_preserves_compound_quantity_identity`: pass.
- AC-26 `reader_backed_projection_preserves_alias_target_behavior`: FAIL.
- AC-27 `reader_backed_projection_preserves_union_identity_and_case_payload_order`: FAIL.
- TC-1073 `reader_backed_tc1070_slots_match_the_checked_view_for_supported_roots`: FAIL.
- `reader_backed_shared_meter_sums_and_bounds_in_both_orders`: FAIL. It passes on origin/main.

The tests are not tautological. Each one compares the checked and reader results, values and charge logs.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-390-AC-15 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-16 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-24 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-25 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-26 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-27 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-50 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390-AC-64 | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |
| FR-390 location pairing (AC-33..38, AC-57) | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-390-AC-15, AC-24, AC-26 and AC-27 are tagged on un-ignored tests that fail on the head with a reader location fault, so these criteria are claimed but not met. | qsl-eval/tests/it/protocol_slots/acceptance.rs:3215,3775,3990,4039 |
| FND-002 | high | FR-390-AC-16 and AC-50 are not met. The plain-data reader root still evaluates the source CheckedBody through `ReaderRoot`, and the literal root takes its location from the CheckedBody. | qsl-eval/src/value/expression/mod.rs:1949-1953,1975-2003 |
| FND-003 | medium | FR-390 location pairing has no implementation and no tagged test. This covers visit k to row k by origin, and `MissingLocationProjection` for AC-33 to AC-38 and AC-57. The ticket's added scope names this pairing as QSL-1625 work. | qsl-package/src/reader_protocol_projection.rs:1050-1064 |
| FND-004 | medium | No test covers a typed refusal for an unsupported reader projection form, as FR-390 requires ("unsupported result ... is an unmet dependency"). | qsl-package/src/reader_protocol_projection.rs:384,818 |

## Verdict

CONDITIONAL. AC-25 is backed by two passing tests. AC-15, AC-24, AC-26 and AC-27 are tagged but red. AC-16 and AC-50 are violated in the code. Location pairing is missing.
