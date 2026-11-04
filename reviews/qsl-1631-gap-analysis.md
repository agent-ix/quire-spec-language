---
id: SR-2285002
title: "Gap analysis of PR #2285: source-owned enum identity rows (QSL-1631)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@592acce0e; git diff origin/main...592acce0e (PR #2285): FR-092-AC-19, FR-251-AC-42 against qsl-package/src/emit.rs, qsl-package/src/emit/monitor_plan.rs and their tests; ticket QSL-1631"
review_set: subset
---

# Gap analysis of PR #2285 (QSL-1631)

## Summary

Ticket: QSL-1631. Planless, diff-scoped. Plan completion: not assessed. Compared the two changed
acceptance criteria with the tests tagged to them and the producing code.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-092-AC-19 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:180 | examined |
| FR-251-AC-42 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:504 | examined |
| a_source_enum_emits_its_declaration_and_member_rows | qsl-package/src/emit/tests/source_enum_rows.rs:318-383 | examined |
| a_model_variant_type_has_no_source_enum_row | qsl-package/src/emit/tests/source_enum_rows.rs:385-392 | examined |
| every_retained_row_kind_is_carried_with_its_node_ids_rekeyed | qsl-package/src/emit/tests/retained_rows.rs:216-316 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-092-AC-19 requires "its key is the node's key", but the test never asserts `declaration.node_key` or `member.member_key` equal the emitted node's key; it only asserts `member_key != node_key`. Swapping `node_key` for any other key (for example the member's) would still pass. Assert each key against the lowered graph key that maps to the row's `structural_node` (for example via the emission's key-to-id map or `SemanticGraph::source_enum_*`). | qsl-package/src/emit/tests/source_enum_rows.rs:346,381 |
| FND-002 | low | The FR-251-AC-42 test checks only the re-keyed node ids of the source enum rows, not that every other field (owner, qualified segments, case, keys) is carried unchanged ("no row is written anew"); a `carry_rows` that rebuilt the row with a wrong case would pass. Compare `got` with `was` after renaming the ids, as for whole-row equality. | qsl-package/src/emit/tests/retained_rows.rs:280-316 |

## Verdict

Conditional: FR-092-AC-19 has an unasserted clause (FND-001). Everything else in the AC is asserted:
one declaration row and two member rows, structural ids naming `scalar_type`/`enum` and
`value`/`enum_value` nodes, owner equal to the node's source owner, qualified declaration `["Status"]`,
member rows naming the declaration's emitted id and their own case, and no row for a model variant
type. FR-251-AC-42's re-keying clause is covered for both row kinds; the non-empty clause is covered.
No stub or untested production branch in the diff besides the `SourceMapInvariant` refusal in
`source_enum_rows`, which is an internal invariant (unreachable while dependencies close over the
declaration).
