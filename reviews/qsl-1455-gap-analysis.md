---
id: SR-1455007
title: "Gap analysis — QSL-1455 FR-496 omission (PR 2093, re-review)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/operation_effects.rs; qsl-package/src/emit.rs; Ticket: QSL-1455"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1455. Head 7357d8fa6, base 347e9769e. Plan completion: not assessed.

## Verdict

**PASS** — every FR-496-AC-5 arm of the carried-node check now has a tagged test that fails when the arm is removed.

## Acceptance mapping

- FR-496-AC-1, AC-2: every_operation_emits_one_row_equal_to_its_frame_lift.
- FR-496-AC-3: permuting_source_order_emits_an_equal_member.
- FR-496-AC-4: state_machines_processes_and_object_types_emit_rows_and_a_subtype_does_not.
- FR-496-AC-5: an_unavailable_effect_omits_the_whole_member (relationship), each_node_an_effect_names_omits_the_whole_member_when_the_package_lacks_it (creates, deletes, field owner, declaring), the_member_is_the_only_difference_from_the_package_without_it.

No code without an owning requirement; no stubs.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |
