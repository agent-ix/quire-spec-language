---
id: SR-17007
title: "Code review of PR #1437: bounded-solver plugin bases settle by item kind (QSL-1036), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@47d37b6c30436ba93810087b7fc1b07eddc89585; git diff origin/main...47d37b6c3 (PR #1437, branch qsl1036-bounded-basis): qsl-replay/src/hybrid_replay/tests.rs; qsl-replay/src/proof_result/plugin_tests.rs; qsl-replay/tests/terminal_record_facade.rs; production context qsl-replay/src/proof_result/plugin.rs, qsl-replay/src/proof_result/plugin_read.rs"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: reviews
---
## Summary

Ticket: QSL-1036. PR: quire-spec-language#1437. Tests-only change in
qsl-replay, following the spec ruling of PR #1435. The rust-review lane is
folded into this file.

Clause-by-clause check of the new test `plugin_bases_read_and_settle_by_item_kind`
(hybrid_replay/tests.rs:1129), tagged TC-774/FR-290-AC-5 and TC-773/FR-289-AC-9:

- **FR-290-AC-5 reader vectors.** `exhaustive`, `bounded-complete` depth 4,
  `inductive` depth 0, `bounded-solver` `delta-complete` and
  `flowpipe-unrounded` (with `exhaustive` `["time"]`, 5/2, 3) are each
  asserted by full equality. The `reduced` vector is asserted only as
  `Reduced` with two reductions (FND-001).
- **FR-290-AC-5 non-hybrid settlement.** Exhaustive, BoundedComplete,
  Inductive and the read Reduced each settle `proved`, `trusted`, with the
  reported basis and the plugin's `BackendId`. Complete.
- **FR-290-AC-5 / FR-289-AC-9 refusal.** `from_plugin_proof` with the read
  `BoundedSolver` returns `Malformed` at `/basis/type`. Complete.
- **Hybrid settlement.** On `SlowEnough` (10 s), a 10 s bounded-solver basis
  settles `proved`, `trusted`, naming the plugin, with the reported basis; the
  5/2 basis settles `inconclusive` `InsufficientHorizon{5/2, 3, {Time}}`, the
  reported coverage. Complete for both ACs.
- **FR-289-AC-9** is asserted in full: `proved`, `trusted`, `BackendId` and the
  reported basis for admitted bases, the hybrid coverage split, and the
  refusal of `bounded-solver` outside the hybrid entry.

Mutations run at 47d37b6c3, each with `cargo test -p qsl-replay`:

- Disable the coverage check in `from_hybrid_plugin_proof`
  (`if false && !coverage.covers(..)`): the new test fails at tests.rs:1215.
  Killed.
- Let `from_plugin_proof` admit `BoundedSolver`: the new test fails at
  tests.rs:1186. Killed.
- Reviewer's own: reverse the order of the reductions `read_reductions`
  returns. The new test passes. Only the now-untagged
  `tc_774_basis_reader_accepts_the_plugin_spellings` fails (plugin_tests.rs:175).
  Survived (FND-001).

Removed tags. Each is a legitimate removal of a partial claim:

- `every_plugin_basis_is_trusted` (FR-290-AC-5): no reader and no hybrid
  clause. Partial.
- `tc_774_basis_reader_accepts_the_plugin_spellings` (FR-290-AC-5): reader
  and a Reduced settle only, with `exhaustive` `[]` rather than the AC's
  `["time"]`, and no hybrid clause. Partial.
- `plugin_proof_is_trusted_and_names_the_plugin` (FR-289-AC-9): only the
  non-hybrid Exhaustive clause. Partial. It keeps FR-289-AC-10, which it
  asserts.
- `coverage_is_decided_from_the_checked_claims_own_horizon`: FR-290-AC-5
  (no reader vectors, no non-hybrid set), FR-289-AC-9 (matches with `..`, so no
  basis or `BackendId` assertion) and FR-290-AC-1 (no FR-281-AC-1 EN-5,
  FR-281-AC-4 or Kani C-09 record) were each partial. It keeps FR-250-AC-7.
  PR #1435 had added this tag line. The spec lane should know the link
  was dropped.

Doc-comment ids match the tags everywhere in the diff. The new test's doc
names FR-290-AC-5 and FR-289-AC-9. The untagged tests no longer name AC ids
(`every_plugin_basis_is_trusted` keeps an FR-level "(FR-250)" reference,
which is not a claim).

Gates run in a detached worktree with its own target dir: `cargo test -p
qsl-replay` (37 suites, 1054 passed, 0 failed), `cargo fmt --all --check`
clean, `cargo clippy -p qsl-replay --all-targets -- -D warnings` clean.
cargo-deny was not run. The local 0.20.2 rejects `--config` (known env
noise). The root crate was not built because origin/main does not compile
there until #1438.

## Verdict

Changes requested, small. FR-289-AC-9 is complete and correctly tagged. The
FR-290-AC-5 tag is complete except for one reader clause. The Reduced vector
is checked only by length, and a reductions-order mutation survives the
tagged test (FND-001). The tag removals are all sound. FND-002 is a naming nit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-290-AC-5 requires the `reduced` vector to read as `Reduced` holding `PartialOrder{BreadthFirstRevisit}` then `CopySwap{[["a","b"]]}`. `plugin_bases_read_and_settle_by_item_kind`, now the only FR-290-AC-5 test, asserts only `Reduced { reductions } if reductions.len() == 2`. A mutation that reverses `read_reductions`' output passes it. Only the untagged `tc_774_basis_reader_accepts_the_plugin_spellings` catches it. The tag claims a clause the test does not assert. Fix: `assert_eq!` the read value against the full `ProofBasis::Reduced { reductions: vec![PartialOrder{proviso: BreadthFirstRevisit}, CopySwap{classes: vec![vec!["a","b"]]}] }`. | qsl-replay/src/hybrid_replay/tests.rs:1161-1165 |
| FND-002 | low | `tc_774_basis_reader_accepts_the_plugin_spellings` keeps its `tc_774_` name prefix but no longer carries a TC-774 trace tag. Its name now implies a TC-774 binding that the trace model does not record. Once FND-001 is fixed it duplicates the new test. Fix: delete it, or rename it without the `tc_774_` prefix. | qsl-replay/src/proof_result/plugin_tests.rs:135-137 |

## Dispositions

Round 1, reviewed at 63162b5790db5051db878f70d50ef757bd8f7373 (fix commit
63162b579 on 47d37b6c3; `git diff 47d37b6c3 63162b579`). `cargo test -p
qsl-replay` passed 1053 and failed 0 (one fewer test, because one was deleted).
`cargo fmt --all --check` and `cargo clippy -p qsl-replay --all-targets -- -D
warnings` were clean. I re-ran the reductions-order mutation (reverse
`read_reductions`' output). `plugin_bases_read_and_settle_by_item_kind` now
fails at hybrid_replay/tests.rs:1165, so the mutation is killed.

- FND-001: the `reduced` vector is now `assert_eq!`-checked against
  `ProofBasis::Reduced { reductions: vec![PartialOrder { proviso:
  BreadthFirstRevisit }, CopySwap { classes: [["a","b"]] }] }`, in order.
- FND-002: `tc_774_basis_reader_accepts_the_plugin_spellings` was deleted, along
  with its now-unused helper `settle_basis`. The new test covers every
  assertion it made, the full Reduced value included.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 63162b579 |
| FND-002 | fixed | 63162b579 |
