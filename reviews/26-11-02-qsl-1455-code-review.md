---
id: SR-202611020539
title: "Code review — QSL-1455 operation effects emission"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering/model.rs; qsl-semantics/src/check/mod.rs; qsl-package/src/emit.rs; qsl-package/src/emit/tests/receiver_frames.rs; qsl-package/src/emit/tests/abstraction.rs; qsl-package/src/emit/tests/admission_corpus.rs; qsl-semantics/tests/it/refinement_protocol.rs; Cargo.lock; FR-496; TC-1286"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1286
    type: references
---

## Summary

Reviewed QSL-1455's FR-496 operation-effect projection, semantic-graph declaration witnesses, checked-package emission, and acceptance tests. The implementation preserves complete canonical effects and provenance, and the focused Rust review found no code or test defects.

## Verdict

**PASS** — changed behavior matches FR-496-AC-1..5 and the Ledger rows admit through IR's strict reader and selected-type projection.

## Rust review

The producer derives rows from admitted model records, rejects partial projection by omitting the whole optional carrier, sorts the effect members and row keys, and uses the original operation member identity for provenance. The lowerer retains the selected model declarations independently of the optional carrier, so an operation row's declaration witnesses exist without authored operation clauses and carrier presence does not determine graph identities. The main and timed emitters both forward the admitted carrier. No new unsafe, panic-prone input handling, concurrency, or unbounded retry path was introduced. No CI workflow files changed. No `AssuranceProfile` applies to the changed scope.

## Acceptance mapping

- FR-496-AC-1: `selected_operation_effects_are_complete_canonical_and_readable`, `relationship_effects_reference_the_declared_relationship_node`, and `ledger_operation_effects_read_back_with_selected_type_projection` cover field scopes, relationship writes, creates, deletes, scoped deletes, provenance, and strict IR admission.
- FR-496-AC-2: the carrier test checks the all-empty `noop` row and multiple operations; `same_operation_name_keeps_its_document_and_declaring_identity` checks same-name rows across owners and documents.
- FR-496-AC-3: the carrier test checks canonical member order and equality after operation/frame-entry permutation.
- FR-496-AC-4: `process_and_state_machine_operations_are_carried_without_persistence_rows`, `systems_interface_and_inherited_operations_are_not_effect_declarations`, and `inherited_operations_have_no_subtype_effect_row` cover included and excluded owners.
- FR-496-AC-5: the carrier test confirms the top-level sidecar does not alter the semantic graph. Lowering adds selected declaration witnesses independent of carrier availability. The full optional carrier is constructed as one `Option`; a missing identity returns `None` rather than a partial set of rows.
- QSL-1455's Ledger acceptance: the Ledger test emits `pay` with a nonempty frame and `payTaxed` with an empty frame, reads the checked package as `Verified`, retains both effects, and projects the selected Ledger operation declarations.

## Gap Analysis

The installed Quoin CLI is 0.29.1 and has no `gap-analysis` command (`quoin gap-analysis --help` exits 2). `quoin matrix --repo . --json` did not complete within 60 seconds (exit 124); the computed whole-repository matrix was unavailable. I manually mapped all five FR-496 criteria to the traced behavioral tests above and checked the changed implementation for unowned behavior and stubs. That scoped check found no acceptance gap. A full repository reverse-gap inventory was not produced.

## Verification

- `cargo fmt --all -- --check` — passed.
- `~/dev/worktrees/locked-build.sh /home/peter/dev/.cargo-targets/qsl-1455 cargo test --locked -p qsl-package emit::tests::receiver_frames:: -- --nocapture` — passed, 20 tests.
- `~/dev/worktrees/locked-build.sh /home/peter/dev/.cargo-targets/qsl-1455 cargo clippy --locked -p qsl-semantics -p qsl-package --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.
- `quire validate --scope . reviews/26-11-02-qsl-1455-code-review.md` — passed. The repository-wide `reviews/**/*.md` validation exits 3 on pre-existing malformed review artifacts (52 files); this artifact validates independently.
- `~/dev/worktrees/locked-build.sh /home/peter/dev/.cargo-targets/qsl-1455 cargo deny check` — license check failed because the repository's existing dependency graph includes `AGPL-3.0-or-later`, which `deny.toml` does not explicitly allow; advisories, bans, and source checks passed. No dependency manifest changed; `Cargo.lock` only advances `quire-contract-model` to IR commit `f88eade3`.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |
