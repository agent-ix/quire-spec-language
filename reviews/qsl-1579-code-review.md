---
id: SR-20261104002156
title: "Code review — QSL-1579 RoleSlot model read-back (PR #2226)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9605b39e1; PR #2226 diff: qsl-package/src/protocol_occurrence.rs, qsl-package/src/emit/tests/protocol_occurrence.rs, tests/it/composed_domain_models.rs, tests/it/emit_checked_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-384
    type: references
---

## Summary

Ticket: QSL-1579. Code review with the rust-review lane folded in, scoped to the four files PR #2226 changes at `9605b39e1`. `RoleSlot` gains a `model: NodeKey`, filled in `derive` from the role node's `model` reference by the existing `Raw::reference` helper. One derivation serves both the in-process schema and the IR read-back, so parity holds by construction. The reader's fixed Role layout `[model, replication]` refuses a role with no `model` argument before `derive` runs, so the new `?` does not add a new reachable refusal. A new unit test checks W's three roles against their authored object types and confirms each key is an emitted `object_type` node. The I2 integration test now reads the package back through IR, asserts parity, and records the current limit: a Port role reads back on its owning object type. Two FR-384 tests change their protocol profile from `v` to `p`. Focused runs: `cargo test -p qsl-package --lib protocol_occurrence` (15 passed). `cargo test --test it architecture_fixture_binds_references_through_spine_i2` (1 passed).

## Verdict

**PASS.** The change is small and idiomatic. The field is documented, the test helpers are local, and there is no new public API beyond the field itself. No panics, wrong results or unsafe code. QSpec is cited by ID only. The `pub(crate)` widening of `read_back` in `tests/it/emit_checked_v2.rs` is the smallest change that lets the I2 test share the strict reader.

## Scope

- `qsl-package/src/protocol_occurrence.rs` `RoleSlot.model` and its `derive` assignment: examined.
- `qsl-package/src/emit/tests/protocol_occurrence.rs` new `object_type` helper, `role_slots_carry_their_selected_model_type_through_read_back`, and the two `v`→`p` profile fixes: examined.
- `tests/it/composed_domain_models.rs` I2 read-back assertions: examined.
- `tests/it/emit_checked_v2.rs` `read_back` visibility: examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
