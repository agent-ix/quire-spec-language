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

## Dispositions

Disposition pass 1, head 391cb15ed (trial merge with origin/main 0642337ef).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-15, AC-24, AC-25 (both tests), AC-26, AC-27, TC-1073 and the shared-meter test pass: 46/46 `protocol_slots::acceptance`, 0 ignored. The un-ignore only removed `#[ignore]`. The AC-24 test gained a same-shape `NativeTwin` capture, which makes it stronger. TC-1073 now asserts that all 20 listed roots go through the reader. |
| FND-002 | fixed | AC-16: one `Machine` path over `ReaderNode`, and `ReaderRoot` is deleted. AC-50: the slot root and origin come only from the selector, and no location is read from a `CheckedBody`. |
| FND-003 | fixed | Pairing is implemented. Tests tagged AC-31/52/53, AC-33/54, AC-35, AC-37, AC-38 and AC-57 are in qsl-package/src/emit/tests/protocol_locations.rs:700-883, and all pass (123 qsl-package focused tests). |
| FND-004 | fixed | `an_unprojected_slot_root_is_a_typed_refusal_at_its_node` covers a typed refusal with catalog pair, reason and locus. Its trace is wrong (FND-005), and the per-slot scope is code-review FND-011. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Wrong trace. `an_unprojected_slot_root_is_a_typed_refusal_at_its_node` is tagged FR-390-AC-17. AC-17 is `MissingTypeRepresentation` before any view exists. The test asserts `MissingOperationProjection` for `quire.op.control.if` while the view *is* created, so it shows the opposite of AC-17, and it shows AC-41's claim (no package reaches `MissingOperationProjection`) failing. Fix: drop the AC-17 tag and trace the test to the follow-up ticket's interim behaviour, or leave it untagged until the remaining kinds are projected. | qsl-package/src/emit/tests/protocol_locations.rs:889-918 |
| FND-006 | low | AC-15 requires location parity, but every AC-15/TC-1073 comparison strips locations (`without_location`). Only leaf halts justify `None` (AC-64). Application-located results now carry row locations and could be compared, but none is. Fix: compare `location` whenever the reader result's location is `Some`. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2873-2883,3243-3343 |

## Dispositions (pass 2)

| ID | Disposition |
| --- | --- |
| FND-005 | Fixed. The AC-17 tag is removed from `an_unprojected_slot_root_is_a_typed_refusal_at_its_node`; the test is untagged and its doc names QSL-1666 as the owner of the interim behavior. The assertions are unchanged. |
| FND-006 | Done as far as the code allows. `assert_location_parity` compares a halt's location whenever the reader's is `Some`, at both unavailable-halt comparison sites. The reader result carries a location only on `Unavailable`, which per AC-64 is `None` for a leaf read, so the comparison is a guard that holds today and bites once a located halt exists; `Evaluated` and `CallFailure` results are already compared in full. |
