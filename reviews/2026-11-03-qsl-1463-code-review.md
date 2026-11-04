---
id: SR-2239004
title: "Code review of QSL-1463 PR #2257: retain acquisition witness rows on the plan-root emission, key the root by its call closure"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ede08267050c252deecec764fe7beb3c592b5e3b; git diff origin/main...HEAD (PR #2257): qsl-package/src/emit/monitor_plan.rs, qsl-package/src/emit.rs, qsl-package/src/lib.rs, qsl-analyze/src/monitor_root.rs, qsl-analyze/tests/it/monitor_root.rs, qsl-analyze/tests/it/monitor_entries.rs (rust-review lane folded in)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1081
    type: reviews
---
# SR-2239004: Code review of QSL-1463 PR #2257

## Summary

Ticket: QSL-1463. Tools: rust-review lane of code-review. Trial merge on origin/main 34de0684 (clean merge, no conflicts). Gates in the trial merge (exit codes): fmt 0; check --locked --workspace --all-targets 0; clippy -D warnings default 0 and --no-default-features 0; arch-lint qualified-core 0; arch-lint api-surface --qsl . --qsl-only 0; qsl-analyze it monitor_root + monitor_entries 45 passed 0 failed; qsl-package lib 605 passed 7 failed, the same 7 by name as clean main (a_node_a_state_clause_places_does_not_move_to_a_type_name, a_valid_attempt_does_not_let_garbage_content_through, an_attempt_and_its_clause_share_one_frame_node_through_emission, an_inherited_operation_binds_its_declaring_types_anchor_and_frame, an_operation_named_only_by_an_attempt_emits_its_anchor_frame_and_record, anchor_occurrence_ordinals_follow_source_order, protocol_view::an_unknown_or_non_clause_node_refuses_without_a_partial_result), no delta.

Removal of the public `emit_monitor_plan`: no other caller, doc, spec or example names it (grep over the tree: only monitor_plan.rs, lib.rs, emit.rs re-export, monitor_root.rs and monitor_entries.rs, all updated); the api-surface lint passes; pre-release no-compat rule applies. Allowed.

Mutation results on a scratch copy of the PR head (test run: the_root_key_covers_the_bodies_of_called_helpers and the_plan_root_retains_the_acquisitions_witness_rows, baseline both pass):

| Mutant | Result |
| --- | --- |
| drop closure tail from the root path | killed (the_root_key_covers_the_bodies_of_called_helpers) |
| drop the clause from the closure roots | killed (same test) |
| skip all re-keying of retained rows | SURVIVES |
| skip re-key: operation-member anchor | SURVIVES |
| skip re-key: equality producer rows | SURVIVES |
| skip re-key: equality fact rows | SURVIVES |
| skip re-key: tuple rows | SURVIVES |
| skip re-key: model enum member rows | SURVIVES |
| descending instead of ascending helper order | SURVIVES |
| omissions not carried (Vec::new()) | SURVIVES |
| temporal binding rows not carried | SURVIVES |
| source-unit rows emptied | not run (mutant did not compile); the test asserts the rows are non-empty, killed by inspection |

Root `it` (cargo test --locked --test it): 877 passed, 0 failed, 6 ignored, exit 0. The coder did not run the old code for red; not repeated. The re-keying logic is correct for every id field the writer can actually change on the fixtures (see FND-003 for the id fields it does not touch).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | the_plan_root_retains_the_acquisitions_witness_rows does not exercise the re-keying it is tagged for: skipping all re-keying, or any single row kind (operation-member, equality producer, equality fact, tuple, enum), leaves the test green, and no assertion reads the operation-member, tuple, enum, omission or temporal-binding rows at all (dropping omissions or temporal binding rows also stays green). The retained-row contract is only verified for the exact declaration/field rows (equal to the acquisition), non-empty source-unit/equality rows, and that source-unit/equality ids exist in the package. | qsl-analyze/tests/it/monitor_root.rs:468-553 |
| FND-002 | medium | the_root_key_covers_the_bodies_of_called_helpers checks only that the closure tail contains each helper's body root (tail.contains); it does not pin the ascending node-id order or once-each, both required by QSpec FR-487. Reversing the order (into_values().rev()) leaves the test green. The fixture also calls helpers only from the atom: a helper called by an entry or the activation guard is not exercised, though the closure is specified for all three. | qsl-analyze/tests/it/monitor_root.rs:395-466 |
| FND-003 | medium | The rows are copied then re-keyed only on the id fields the PR names; other CheckedNodeId fields in retained rows are neither re-keyed nor checked: SelectedExactDeclarationRow.structural_node and SelectedExactFieldRow.record_node (copied verbatim), EqualityFactWitness variants Unit.quantity_type, DimensionTerm.base_node, ConversionEdge.target_node, EnumMember.{source_declaration_node, declaration_node, member_node}, and OmittedNode.node. The ids the writer renames are record_value nodes of Instance/Activation/LimitReached and the four entry functions, which none of these fields can name today, so no row is wrong now; but the stated contract is that rows describe the plan-root package's own ids, and a future renamed id referenced from one of these fields would leave a stale id with no failing test. Cheapest fix: re-key them with the same helper (or assert in finish that no retained row id is in the old side of the change list). | qsl-package/src/emit/monitor_plan.rs:768-810 |

## Verdict

Not mergeable as is: the production change is correct and every gate is green, but two of the three tests the PR relies on do not fail under the mutations the review was asked to run (FND-001, FND-002), and FND-003 leaves part of the re-keying contract unexercised and unguarded. All three fixes are small and local to the tests and one helper.

## New findings (disposition pass 1)

Re-review at 81aa7e1c7 (merged on origin/main d421482f in a trial merge; all gates green, see Dispositions).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The writer's hand-off of its renames to carry_rows is not tested. carry_rows is tested only with synthetic rows and a synthetic rename; no real row names a renamed node (the Instance/Activation/LimitReached record_value nodes and the four entries), so the monitor_root end-to-end test cannot see the wiring. Mutants that call `finish(base, retained, &[], ...)`, or drop either `all_changes.extend(changes)` (step 1 or step 2), pass the whole package lib retained_rows tests and the whole qsl-analyze monitor_root/monitor_entries run (45 passed). An end-to-end assertion is needed: pass emit_monitor_plan_over a retained Emission with a row whose id is the pre-rename id of one entry function (and one renamed record_value node) taken from the base package, and assert the returned row names the new id. | qsl-package/src/emit/monitor_plan.rs:396-410, 454, 686 |

## Dispositions

Round 1, at 81aa7e1c7. Mutation set re-run on a scratch copy of the fix head (tests: qsl-package lib retained_rows, and qsl-analyze the_root_key_covers_the_bodies_of_called_helpers; baseline both pass):

| Mutant | Result |
| --- | --- |
| skip all re-keying (Rekey::id no-op) | killed (2 of 2 retained_rows tests) |
| skip re-key: operation-member anchor | killed |
| skip re-key: tuple structural node | killed |
| skip re-key: source-unit structural node | killed |
| skip re-key: model enum declaration node | killed |
| skip re-key: enum member structural node / declaration node (two mutants) | killed, killed |
| skip re-key: exact declaration node | killed |
| skip re-key: exact field record node | killed |
| skip re-key: equality producer checked node / source-map entry (two mutants) | killed, killed |
| skip re-key: equality fact application / structural node / fact node ids / source-map entry (four mutants) | killed x4 |
| skip re-key: source-map entry node (Rekey::entry) | killed |
| skip re-key: omitted node / omission cause node (two mutants) | killed, killed |
| skip re-key: temporal binding declaration | killed |
| drop carried omissions / drop carried temporal binding rows | killed, killed |
| closure order reversed | killed (the_root_key_covers_the_bodies_of_called_helpers) |
| entry roots dropped from the closure / clause dropped from the closure | killed, killed |
| finish called with no renames / step-1 renames not carried / step-2 renames not carried | ALL SURVIVE (FND-004) |

Gates in the trial merge on origin/main d421482f (exit codes): fmt 0; check --locked --workspace --all-targets 0; clippy default 0; clippy --no-default-features 0; arch-lint qualified-core 0; api-surface --qsl-only 0; qsl-analyze it monitor_root + monitor_entries 45 passed; qsl-package lib 608 passed, 6 failed, the same 6 failing names as clean origin/main (606 passed, 6 failed), no delta (current main has 6 pre-existing failures, not 7); root it 877 passed, 0 failed, 6 ignored.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | carry_rows is exercised by every_retained_row_kind_is_carried_with_its_node_ids_rekeyed and a_row_naming_a_node_no_rename_touches_is_carried_unchanged (TC-1081 / FR-251-AC-42); each re-key mutant and both retention mutants fail them. The remaining untested piece is the writer's wiring, tracked as FND-004. |
| FND-002 | fixed | The test now asserts assert_eq!(tail, expected_closure(wire)) against a closure computed independently from the wire, with atom-called, helper-through-helper and guard-called helpers; the reversed-order, entry-roots-dropped and clause-dropped mutants all fail. Entries reach authored helpers only through generated declared helpers, which the closure includes (tail longer than the three authored helpers); the duplicate-check in the test compares test-local vectors only (harmless: assert_eq on the tail already fails a duplicate). |
| FND-003 | fixed | carry_rows destructures every row type (and CheckedSourceMapEntry, EqualityFactWitness, OmissionCause) with no rest pattern, re-keys every node id field, and each field's re-key is pinned by a killed mutant. |
