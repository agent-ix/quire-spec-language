---
id: SR-1368
title: "Code review of quire-spec-language PR #651: the emitter writes the node owner (QSL-638)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b90d17b13227416d2110e5a68778ee6033a21caf; PR #651 diff against origin/main (git diff origin/main...b90d17b13): Cargo.lock, qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/owners.rs, qsl-semantics/src/library/package_identity.rs; context: quire-contract-ir@3ed1f7ce crates/quire-contract-model/src/checked_package/v2/derived_keys.rs, QSpec 65e816f0 proposals/checked-package-v2"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
# Code review of quire-spec-language PR #651

## Summary

Ticket: QSL-638. PR: quire-spec-language#651, frozen head b90d17b13. The diff
touches no `spec/` file, so the earlier spec review (SR-1349) still covers the
spec side. The Rust lane (rust-review) is folded into this file. Only the
final diff is judged. The revert and re-apply commits around the conformance
domain-package evidence net out to one helper, `supply_qspec_domain_package`.

**Production code is correct and small.**

- `emit.rs` `wire_owner` maps `Owner::Source` to `CheckedNodeOwner::Source
  {authority, identity}` and `Owner::Model` to `CheckedNodeOwner::Model
  {identity, node}`. The match is exhaustive and has no wildcard. The value
  comes from `SemanticNode::owner()`, which is the same `content.owner` the
  structural preimage hashes, and is `None` for a nominal node. So the wire
  owner is the preimage owner by construction.
- `checked_v2.rs` maps IR's new `UnsupportedConstruct` refusal code to
  `Code::UnsupportedConstruct`, with no wildcard.
- `package_identity.rs` adds `owner` to `NODE_OPTIONAL`. This reader only
  checks what it projects (it does not check `recursion_group`'s shape
  either), and the one non-test `LibraryPackage` constructor is the IR read
  path in `checked_v2.rs`, so the comment's claim that IR checks the shape
  first holds today.
- Cargo.lock moves `quire-contract-model` to 3ed1f7ce, as ruled. The
  `quire-verification-contracts` entry also moves (ec4563ff to 1fc0ff61), as
  part of the same update. That is a routine lock bump.

**The `rekey_stale` oracle question.** `rekey_stale` rewrites the frame
fixtures' node keys with `emit::tests::rebuilt_key`. That function is test
code that rebuilds the FR-092 / FR-322 preimage from the wire JSON alone. It
does not call the emitter or `qsl_semantics`' key derivation. Raising
`emit::tests` and `rebuilt_key` to `pub(crate)` is under `#[cfg(test)]`, so it
exposes nothing outside test builds. The tests are not tautological:

- At IR 3ed1f7ce the reader re-derives every owner-free, ungrouped structural
  key (`derived_keys.rs`). The frame node has no owner, so IR re-derives its
  key on its own and refuses `stale-node-key` before the frame step if the
  key is wrong. A wrong `rebuilt_key` makes the admitted test fail and makes
  the refusal tests report `stale-node-key`, not the expected frame code. So
  IR is the oracle, and the rekey is only fixture building.
- In `owners.rs`, `rebuilt_key` is checked against the emitter's keys, against
  IR's reader (`Verified`), and against QSpec's published two-owner fixtures
  when `QSPEC_DIR` is set. So it is calibrated from outside QSL.
- The FR-340 conformance test maps QSpec's recorded `expected_locus_digest`
  through `renames`. That is needed, because a frame mutation changes the
  frame's body and so its key, and IR now refuses a stale frame key first.
  The QSpec vectors still record the old key. This is a QSpec vector
  question, not a QSL defect.

**`keyable_type_node`.** The fixtures' `T` used to have a literal body whose
`type` names `T`'s own key, so its key would depend on itself and
`rekey_stale` would never settle. The new node is `scalar_type`/`text` with
the empty `aggregate` body. That is the shape QSpec's own self-typed scalar
types use in its fixtures (`positive-union-nodes.json` `text`,
`positive-control-operations.json` `boolean` and `integer`). `T` still carries
a declaration and a source owner, so IR does not re-derive its key. The
change is sound.

**Rust lane.** No new `unwrap`, `expect`, panic or `unsafe` on a production
path. No new limit, pin, compatibility layer, vendored file or ceremony.
`supply_qspec_domain_package` reads QSpec's document from `$QSPEC_DIR` by
reference and copies nothing.

**Tests run** at b90d17b13, with `QSPEC_DIR` set to QSpec 65e816f0, through
`locked-build.sh`. The results are in the Verdict.

## Verdict

Approve the code, with three low findings. Each is a comment or helper-level
issue with no wrong behaviour. The production change is correct, and the
frame-fixture rekey is a sound use of an external oracle (IR's key
re-derivation), not tests checking the code against itself.

Tests run: `cargo test -p qsl-package --lib` with `QSPEC_DIR` set gave
115 passed, 0 failed. That includes the four `owners` tests, the three frame
fixture tests, `conformance_fr340_frame_mutations_match_qspec_vectors`,
`conformance_dependency_selection_vectors` and
`conformance_i2_read_over_qspec_checked_package_v2_fixtures`. Clippy was not
re-run, because the build lock was busy (the coder reports it clean). The
`qsl-semantics` change is one constant, and the `checked_v2` read path
exercises it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new `rekey_stale` doc comment was inserted under the existing `refresh_frame_identity` doc comment. Rustdoc joins the two, so `rekey_stale` carries both texts ("Rebuilds `envelope`'s `identity_projection` ... Renames every stale node key ...") and `refresh_frame_identity` has none. Fix: move the five "Rebuilds ..." lines back above `fn refresh_frame_identity`. | qsl-package/src/checked_v2/tests.rs:2041-2051 |
| FND-002 | low | The `rebuilt_key` doc comment still says it rebuilds "FR-092's structural preimage under owner (`a`, `u`)". This PR changed it to read the owner from the wire node's own `owner` member, and it is now used for `pkg`/`src` and `agent-ix`/`example-*` owners. Fix: say the owner is the wire node's own `owner`. | qsl-package/src/emit/tests.rs:412-415 |
| FND-003 | low | `emit()` in `emit/tests.rs` now builds the same closure as `owners.rs`' `emit_under` (place every occurrence at the whole of a given source), with the source taken from `package.graph().source()`. That is two copies of one helper. Fix: make `emit_under` the shared helper in `emit/tests.rs` and define `emit(p)` as `emit_under(p, p.graph().source())`. | qsl-package/src/emit/tests.rs:174-182; qsl-package/src/emit/tests/owners.rs:31-37 |

## Dispositions

Round 1, reviewed at 6f9f8e72cd34981a82734249e32b929e6f3cf346 (fix commits
6c8208363 and 6f9f8e72c after the review-records commit d91fd5530;
`git diff d91fd5530 6f9f8e72c`). The round adds no new finding. The fix
commits touch only the three test files the findings name.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6c8208363 |
| FND-002 | fixed | 6c8208363 |
| FND-003 | fixed | 6c8208363 |
