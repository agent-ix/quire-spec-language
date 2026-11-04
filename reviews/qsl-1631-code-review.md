---
id: SR-2285001
title: "Code review of PR #2285: source-owned enum identity rows in Emission (QSL-1631)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@592acce0e; git diff origin/main...592acce0e (PR #2285): qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs, qsl-package/src/emit.rs, qsl-package/src/emit/monitor_plan.rs, qsl-package/src/emit/tests/source_enum_rows.rs, qsl-package/src/emit/tests/retained_rows.rs; ticket QSL-1631"
review_set: subset
---

# Code review of PR #2285 (QSL-1631)

## Summary

Ticket: QSL-1631. PR quire-spec-language#2285, head 592acce0e. Method: code-review with the
rust-review lane. The diff records `SourceEnumDeclaration`/`SourceEnumMember` side tables while
lowering each admitted enum, builds IR `SelectedEnumDeclarationRow`/`SelectedEnumMemberRow` from
them in `source_enum_rows` (emitted node ids only), exposes them on `Emission`, and re-keys them in
the plan-root `carry_rows`.

Row shape was checked field by field against the locked IR crate (quire-contract-model at
64de9ec, `checked_package/v2/exact_projection.rs`): declaration row = structural_node,
source_authority, source_identity, qualified_declaration, node_key; member row = structural_node,
source_authority, source_identity, declaration_node, declaration_qualified_name, case, member_key.
The producer passes each argument in constructor order with the right value (member's own key,
declaration's emitted id, enum owner, original qualified segments, exact case).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| SourceEnumDeclaration / SourceEnumMember side tables | qsl-semantics/src/check/lowering.rs:334-450 | examined |
| enum_nodes side-table recording | qsl-semantics/src/check/lowering.rs:2835-2900 | examined |
| source_enum_rows | qsl-package/src/emit.rs:5939-5988 | examined |
| Emission fields, accessors, DebugWith | qsl-package/src/emit.rs:478-490,669-710,927-945 | examined |
| carry_rows / finish re-keying | qsl-package/src/emit/monitor_plan.rs:729-735,862-890,958-962,1028-1032 | examined |
| source_enum_rows tests | qsl-package/src/emit/tests/source_enum_rows.rs | examined |
| retained_rows composite | qsl-package/src/emit/tests/retained_rows.rs:51-58,277-316 | examined |
| IR row types | quire-contract-model 64de9ec exact_projection.rs:244-324 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `enum_nodes` inserts into `source_enum_declarations` unconditionally (last binding wins), while the emitted node is first-binding-wins (`insert_nominal` leaves an existing key unchanged; the function's own comment says two bindings of one admitted declaration are one node). The owner is resolved per binding name (`source_owners.get(&binding.name)` falling back to the unit owner), so if two bindings of one declaration resolve different owners the row's source_authority/identity disagree with the emitted node's `owner`. Fix: record the side table only when `inserted` (or `entry().or_insert`), so row and node come from the same binding. | qsl-semantics/src/check/lowering.rs:2846-2896 |
| FND-002 | low | `source_enum_rows` re-collects `graph.source_enum_declarations()` (already a `BTreeMap` iterator) into a second `BTreeMap<&NodeKey, _>` only to look members' declarations up; a `SemanticGraph::source_enum_declaration(&NodeKey)` accessor over the existing map would avoid the copy. | qsl-package/src/emit.rs:5952-5953 |

## Verdict

Mergeable after FND-001 (small, one-line class of fix); FND-002 is optional.

What was checked and is correct:

- Field identity and constructor argument order match the locked IR row types exactly.
- Determinism: declaration rows iterate a `BTreeMap` by `NodeKey`, member rows iterate the
  graph's `BTreeMap` by member `NodeKey`; the `emitted` `HashMap` is only probed, never iterated.
  Output order is stable across runs.
- Rows are built from admitted enum facts and emitted ids only; a non-emitted enum yields no row;
  a member whose declaration is not emitted refuses `SourceMapInvariant` rather than inventing an id.
- `carry_rows` destructures both new IR structs exhaustively, so a new IR field fails to compile
  rather than silently skipping re-keying. No `..` struct spread builds `Emission` or `RetainedRows`
  anywhere in the workspace (grep), so every constructor names the new fields.
- The removal of "union" from the `selected_exact_declaration_rows` doc is correct: QSL produces
  no `SelectedExactDeclarationForm::Union` row (grep of qsl-semantics), and the locked IR
  `matches_admitted` has no Union arm, confirming the IR-1323 blocker.
- Round-trip: the locked IR (64de9ec) and IR main expose these two row types but have no reader or
  join that consumes them, so no IR round-trip exists to test; the test checks the rows against the
  emitted wire nodes instead, which is the strongest check currently available.

Evidence: `cargo test -p qsl-package --lib -- source_enum_rows retained_rows variant_types`
14 passed, 0 failed.
