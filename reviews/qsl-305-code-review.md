---
id: SR-784
title: "QSL-305/QSL-307 code review (with rust-review lane) of PR 522"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0e93ed8dbb225266de2b55e363075a09f755d154; Cargo.toml; qsl-package/Cargo.toml; Cargo.lock; qsl-package/src/emit.rs; qsl-package/src/checked_v2/tests.rs; qsl-replay/src/spine/clause/tests.rs; qsl-semantics/src/value/semantic_node.rs (unchanged, context); tests/it/text_enum_identity.rs (unchanged, context); quire-contract-ir@2a286437 checked_package/v2/{identity,operations,structural,frame}.rs (dependency, read only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
---
## Summary

Tickets: QSL-305, QSL-307. PR: quire-spec-language#522 at 0e93ed8d, base
origin/main 1a368fa4. Methods: code-review with the rust-review lane folded in.

The PR bumps the `quire-contract-ir` (`quire-contract-model`) pin from
48ab5dc to 2a286437 in both manifests and the lock. It fills IR's new required
`NominalOwner::Model.version` with an empty string in `emit.rs`. It re-spells
the FR-340 frame fixtures for QSpec's 30-vector `modifies` entry shape and the
`generated` frame occurrence role. It re-ignores TC-463 with a new reason.

Checks run, with results:

1. Pin. Both manifests carry 2a286437. The lock moves `quire-contract-model`
   to 2a286437 and, transitively, `quire-verification-contracts` from 61f4a44
   to 4c49706. Only `quire-contract-model` depends on that crate, so there is
   one lock entry and no split. `quire-contract-ir-historical` stays at
   04eb6f84 in `Cargo.toml` and `Cargo.lock`, unchanged from main.
2. `NodeOwner::Model` construction. No `NodeOwner::Model(` expression exists
   anywhere in the workspace. The `Owner::Model` hits in
   `qsl-semantics/src/check/lowering/model.rs` are `node_key::Owner`, a
   different type. But `NodeOwner` derives `Deserialize`, and the public
   `EnumDeclarationPreimage::from_json` admits `{"kind":"model","identity",
   "node"}`. `tests/it/text_enum_identity.rs:552-553` does exactly that. So
   the arm can be reached. What keeps `version: ""` off the wire is
   `owner_is_locked` (emit.rs:515-525): every non-`Source` owner is omitted
   as `UnlockedOwner` before `wire_node` runs (emit.rs:544, 861-876). The
   placeholder is safe today, for a different reason than the comment gives.
   See FND-001.
3. `ModelSubject` revert. The diff does not touch `qsl-semantics` or
   `tests/it/text_enum_identity.rs`. `ModelSubject` matches main.
   `enum_node_identity_vectors_reproduce_and_noncanonical_preimages_refuse`
   passes at head (part of the 885-pass `it` run).
4. IR-side claims, checked against the pinned checkout at 2a286437.
   `operations.rs:1087-1092` has `OperationConstraintKind::ReferenceEdge =>
   return ineligible(indices.first().copied())` with the quoted comment.
   `structural.rs:134` has `Self::Frame => Some(CheckedOccurrenceRole::
   Generated)`. There is no `FieldDeclaration`/`OperationDeclaration`/
   `ClauseMemberDeclaration` left in `checked_package/v2`, and `frame.rs:327`
   admits `ObjectType | Process` for field entries. The cargo checkout is
   clean (`git status` shows only `.cargo-ok`). IR-370's "instrumenting the
   vendored IR source" did not leave edits in the shared checkout.
5. Test helpers. `modifies_entries` reads `declaration`, `kind` and `name`
   straight from each QSpec vector entry and panics on an unmapped kind.
   `nodes_source_map` now reads each node's own occurrence role instead of
   hard-coding `declaration`. These are test-only unwraps and panics, which
   this repo allows.

## Verdict

Mergeable after FND-001, which is a comment fix. FND-002 is a low nit.
`make ci` exit 0 at 0e93ed8d (my own run, QSPEC_DIR unset). Full
`cargo test --locked --workspace` with QSPEC_DIR set to a fresh
quire-specification clone (e56756f) exit 0, with the main `it` suite at
885 passed / 6 ignored. `make conformance` against the same clone exit 0,
including "30 frame-body mutation vectors matched".

The informational prose at `integration/current-head/Cargo.toml:29` still says
the root pins 48ab5dc2. It is already labelled informational and nothing reads
it, so it is not a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new comment on the `version: ""` placeholder states the wrong invariant and names the wrong crate. It says nothing in this crate constructs the `Model` arm, because no enum/dimension/unit declaration is model-owned yet. In fact a model-owned `EnumDeclarationPreimage` can be built through the public `from_json`, and `tests/it/text_enum_identity.rs:552-553` builds one. The real guard is `owner_is_locked` (emit.rs:515-525), which omits every non-`Source` owner as `UnlockedOwner` before `wire_node` runs. The comment also says `quire_semantics::value::NodeOwner` and "this crate's own", but the type is `qsl_semantics::value::NodeOwner`, in qsl-semantics. The risk: when model owners become lockable, the empty string ships silently. QSpec's `ModelOwner.version` is `Nonempty`, and IR hashes it into the node id. Fix: rewrite the comment to name `owner_is_locked`/`UnlockedOwner` as the guard and fix the crate path. Optionally add a `debug_assert!` or test that ties the placeholder to that omission. | qsl-package/src/emit.rs:498-507; qsl-package/src/emit.rs:515-525; qsl-package/src/emit.rs:544 |
| FND-002 | low | TC-463's new `#[ignore]` reason and doc comment describe the IR gap but do not name its tracking ticket, IR-370. The old reason named QSL-307. Without the id, nobody can grep for the un-ignore trigger when IR-370 lands. Fix: put IR-370 in the `#[ignore = ...]` string. | qsl-replay/src/spine/clause/tests.rs:4913-4931 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 926873eb: the comment now names `qsl_semantics::value::NodeOwner::Model`, says a model-owned nominal node can be built, and names `owner_is_locked` / `UnlockedOwner` as the guard. Verified: `owner_is_locked` matches only `NodeOwner::Source` equal to the unit's source, `omissions` drops the node, `emit_package_inner` calls `wire_node` only on kept candidates, and `nominal_preimage` (the only caller of `nominal_owner`) is called only from `wire_node`. |
| FND-002 | fixed | 926873eb: the TC-463 `#[ignore]` reason and doc comment now name IR-370. |
