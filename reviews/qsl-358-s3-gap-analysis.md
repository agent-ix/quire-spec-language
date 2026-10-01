---
id: SR-959
title: "QSL-358 slice 3 gap analysis of PR 567: slice claims against tests and code"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f0166db0e28e7cd401619ed93d6ea261cd895ae7; diff bcc946cd...f0166db0; QSL-358 slice 3 claims (structural EnumDeclaration in SV, EnumValue::admitted deleted, AdmittedEnumDeclaration with refusal order unchanged, definition refusal vocabulary in SV, invalid_package pub, ADR-011 rows); quire-semantic-value/src/enumeration.rs; qsl-semantics/src/value/enumeration.rs; tests/it/text_enum_identity.rs; qsl-semantics/tests/it/{complete_value_lock,integer_division}.rs; xtask/src/import_graph.rs; tests/it/family_outcome_layering.rs; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 3). Each slice claim, the test that backs it, and
whether that test fails if the claim breaks:

- `new()` refuses non-canonical before unsorted: SV
  `a_declaration_refuses_a_malformed_member_list_before_an_unsorted_one`.
  `["b","a","a"]` is both, so a reorder fails it. Backed.
- `member()` refuses an undeclared case, and carries position, ordered flag and
  case: SV `a_member_carries_its_declaration_position_and_case`. Backed.
- Member admission order (foreign, undeclared, stale): `t09` pins undeclared
  before stale (`CLOSED` with a `READY` key). `tc_409` pins the declaration key
  refused as a member key. Backed.
- Enum admission order (unsorted, owner, stale): owner before stale is pinned
  (t09 changes), and unsorted before stale is pinned. Unsorted before owner is
  not (FND-001).
- Definition vocabulary moved unchanged: `complete_value_lock` (all eight codes,
  set equality with `ALL`) and `integer_division` (every `PackageCause`) run
  against the SV types. Backed. The ci log shows both passing at f0166db0.
- TC-390 lists: the diff changes no TC-390 file, and none needs a change.
  `quire_semantic_value` is permitted as a whole crate in
  `LAYER_PERMITTED_MODULES` and `BELOW_CORE`. `value::definition` and
  `value::enumeration` still exist in qsl-semantics and stay listed. (The PR
  body does not claim a TC-390 change. The brief did.)
- RT deletability: RT `origin/main` `src/exact/definition.rs` holds exactly the
  four moved types. Its `src/exact/enumeration.rs` holds `EnumDeclaration`
  (`new`, `member(case, member)`) and `EnumValue`, which now match the SV API.
  The PR's "What RT can delete" list matches.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The slice claims that refusal order is unchanged, and the check that moved across the crate boundary is the first one (unsorted, now inside SV `EnumDeclaration::new`). No test gives a declaration that is both unsorted and owned by an unselected owner. Moving `EnumDeclaration::new` below the owner check in `admit` would pass every test. Add one case to `t09` (or to the TC-409 sweep at text_enum_identity.rs:1015-1046): an unordered, unsorted preimage with an unselected owner must refuse `UnsortedUnorderedMembers`. | qsl-semantics/src/value/enumeration.rs:261-272; tests/it/text_enum_identity.rs:692-710,1031-1044 |

## Verdict

Mostly backed. Every slice claim has a test that fails if it breaks, except
that unsorted-before-owner precedence is not pinned (FND-001, low). No missing
production behaviour.
