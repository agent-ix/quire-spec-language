---
id: SR-11753
title: "Code review of quire-spec-language PR #912: frame node scope and deletes_scoped emission"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@294414ccb60ac665ab0945bc459a8eef3266f609; qsl-semantics/src/check/lowering/state.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/model/domain_package.rs, qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests/receiver_frames.rs, qsl-replay/src/spine/clause/tests/receiver_frames.rs, qsl-semantics/tests/support/receiver_frames.rs, qsl-semantics/tests/it/model_population.rs, qsl-semantics/tests/it/model_receiver_frames.rs, Cargo.lock"
review_set: subset
---
# Code review of quire-spec-language PR #912

## Summary

Ticket: QSL-522. One pass of code-review with the rust-review lane folded in,
over `git diff origin/main...HEAD` (one commit, 19 files). Read-only: no build
or cargo run, because a gate is running on the worktree's target. Rule in
force: only major bugs block; mediums and lows are recorded.

The emitted frame body was checked against the IR V2 frame reader at the
locked quire-contract-ir revision (`checked_package/v2/frame.rs` and
`state.rs`): entry shape, order key, deletes_scoped order, the
conflicting-binding step and the state-step scoped-deletes check.

Examined:
- `Lowering::frame_node` (state.rs): `modifies` built from the every-object
  and the receiver-scoped entries as `(declaration, name, scope)`, sorted and
  deduplicated; `deletes_scoped` as receiver first, then parameter names
  sorted as UTF-8 bytes and deduplicated (examined)
- `FrameScope`, `FrameDeletesScoped`, `FrameField` wire and `FixedShape`,
  `FrameTerm.deletes_scoped` in the preimage walk, `map_keys` and the
  reference visitor (node_key/mod.rs) (examined)
- `OperationEffect::written_fields` and `is_empty`, and their three callers
  (checked_dispatch `is_query`, field_refinement, lowering/model) (examined)
- frame dependencies in `emit.rs` and `named_nodes` in lowering.rs: no
  dependency added for scoped entries, matching IR's join (context_only)
- IR V2 reader: `read_frame_body`, `ModifiesEntry::order_key`,
  `DeletesScopedEntry` ordering, `frame_defect` conflict step,
  `check_anchor` scoped-deletes check (context_only)
- intake receiver rule for record value types (`OperationReceiver::NoObject`
  at the record-value-type operation reader, already on main) against the
  FR-154 / TC-1043 I23 spec edit (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for code-review. No major bug.

Emission matches the IR V2 reader. QSL sorts field entries by
(declaring node key, name, scope) with `EveryObject < Receiver`, and a
relationship sorts as name `None`, which is IR's
`(digest, name, Option<scope>)` with a relationship's name empty and scope
absent. `deletes_scoped` is receiver first, then parameter names ascending by
`String` order (UTF-8 bytes), the same as IR's derived `Ord` on
`DeletesScopedEntry`. Both are deduplicated, so IR's repeat check is never
hit. QSL never emits both scopes of one field: intake refuses that pair
(`conflicting-binding`, normalize.rs). The IR reader's conflict step and its
frame-node-located scoped-deletes refusal are reached only by the edited
copies in X6, X8 and X9, which the tests read with the I2 reader. X6 also
asserts the locus is the re-keyed frame node. `deletes_scoped` carries no
node reference, so neither the dependencies nor the visitor change. That is
correct: IR checks parameters through the anchor's operation, not through the
frame's dependencies.

The `is_query` / `written_fields` changes fix a real latent defect: before
this PR, an operation with only receiver-scoped writes or scoped deletes
counted as a query, and a receiver-scoped writer was invisible to the
redefinition refinement checks.

Rust lane: no unwrap, expect or panic in non-test code. `FrameScope::as_wire`
is a const closed match. No new public surface beyond the two re-exported
enums and the `scope()` accessor. `pub(crate) mod tests` and the
`pub(crate) rekey_stale` are test-only (`#[cfg(test)]`).

Recorded, not blocking:
- low: the `register_frame_occurrences` doc still says two operations share
  a frame node when `modifies`/`creates`/`deletes` are equal. It should now
  also name `deletes_scoped` and the scope (state.rs:384-385).
- low: the test fixture is shared with `#[path]` includes from
  `qsl-semantics/tests/support/` into the `qsl-package` and `qsl-replay` unit
  tests. This works, but it couples three crates' test trees through relative
  paths.
