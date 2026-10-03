---
id: SR-1258
title: "Code review of quire-spec-language PR #614: delete arch-lint duplicate-revisions and direction's clone-freshness check (QSL-477, A5)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b9a376cdf5173a9fcc17ca6a0ebf8b5feed5303c; PR #614 diff against origin/main (merge base 8d1deba4): Makefile, spec/functional/FR-059-check-backend-dependency-direction.md, tools/arch-lint/Cargo.toml, tools/arch-lint/duplicate_revisions.rs (deleted), tools/arch-lint/error.rs, tools/arch-lint/graph.rs, tools/arch-lint/main.rs, tools/arch-lint/metadata.rs; context: tools/arch-lint/qualified_core.rs, xtask/src/canonical_types.rs, deny.toml, .github/workflows/ci.yml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
---
# Code review of quire-spec-language PR #614

## Summary

Ticket: QSL-477 (implementation slice A5). PR: quire-spec-language#614, draft,
head b9a376cdf. This file covers code-review and its rust-review lane.

What the PR claims, and what I measured:

- **FR-061 and TC-158 were already deleted on main.** True. #586 (e03e6b1a0)
  deleted both files. #589 (4559561c9) deleted the FR-059-AC-7 row.
- **duplicate-revisions code and wiring deleted.** True. `duplicate_revisions.rs`
  and its eight tests are gone, along with the `main.rs` module, match arm,
  usage line and `run_duplicate_revisions`, plus `Code::InvalidLockfile` and
  `Error::at`. `grep -rn` for `duplicate-revisions`, `duplicate_revisions` and
  `duplicate revision`, outside the review records under `reviews/` and
  `spec/reviews/` and ADR-010's historical OBS-041 row, finds nothing.
- **direction's freshness check deleted.** True. `FRESHNESS_TARGETS`, the
  "Revisions used" block, `--offline`, `Code::Stale` (exit 4), `Error::message`,
  `git_head`, `git_ls_remote_head`, `check_freshness`, `require_current_head`
  and tests `tc_arch_lint_metadata_004` and `_005` are all gone. No spec, TC or
  test names exit 4, `stale-clone`, `Stale`, `FR-059-AC-7` or `--offline` for
  arch-lint. The remaining `--offline` hits are unrelated cargo flags in xtask,
  tests and docs.
- **tc_arch_lint_api_surface_021 now uses `to_string()`.** True. The oracle
  keeps its strength. `to_string()` adds `usage: ` in front of the first line
  only. Every assertion either finds a `T12-x [` line or checks `ends_with`.
- **Makefile.** True. `arch-lint-duplicate-revisions` is out of `ci:`, `.PHONY`
  and the `arch-lint` aggregate, and its recipe and comment are deleted. The
  cargo-deny install hint no longer pins `--version 0.19.8`.
- **graph::classify kept.** True. It has three callers: `metadata::edge_repo`
  (FR-059), `qualified_core.rs:330` (FR-284) and `xtask/src/canonical_types.rs:108`
  (ADR-032 DT-4). The FR-059 Behavior sentence saying `classify` treats a
  QSL-sourced shared leaf as QSL is still backed. xtask `tc_747` classifies a
  QSL-sourced `quire-exact` as an ecosystem package.
- **FR-059 Status.** True. Both "Remaining work" paragraphs are deleted.
- **make ci exit 0 on the head.** True. The log
  `~/dev/worktrees/logs/qsl-a5-fr061-make-ci.log` ends `exit=0`. The worktree
  that produced it is at b9a376cdf with a clean `git status`. The log was
  written after the commit, and it lists the arch-lint lib (9) and bin (83)
  tests passing, with no `metadata_004`/`_005`.

Rust lane (rust-review):
- No new `unwrap` or `expect` on a production path.
- No `unsafe`. `#![forbid(unsafe_code)]` stays.
- No `allow(dead_code)`. `take_bool` is still used by `--qsl-only`, and every
  remaining `Code` variant has a producer. `make ci` runs clippy with
  `-D warnings` and passed.
- `run_direction` now resolves all four manifests and then builds the report.
  Its behaviour on success and on a `cargo metadata` failure is unchanged.
  Dropping `git_head` means a `--ir`/`--rt`/`--cg` path no longer has to be a
  git checkout. Nothing in FR-059 asks for one.
- `exit_code` stays exhaustive over the remaining codes (2 and 3).

CI wiring outside the Makefile: `.github/workflows/ci.yml` names neither
arch-lint nor the deleted target. It still pins `cargo install cargo-deny
--version 0.19.8` (ci.yml:11, :26-27). The Makefile no longer claims to match
that pin. Workflow files are outside this PR, so this goes to the lead, not
into the findings.

## Verdict

Approve after one fix round. Three low findings, all in lines this PR touches
or in a file comment that this PR makes false. No medium or high code finding.
The gap analysis (SR-1259) has a medium finding that blocks merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `Code`'s doc comment says "Spellings never change once released." This PR deletes two spellings, `invalid-lockfile` and `stale-clone`, and arch-lint is `publish = false`. The promise protects nothing and is now false, so under the value test it is ceremony. Fix: delete the sentence and keep "Stable error classification." or drop that too. | tools/arch-lint/error.rs:5, tools/arch-lint/Cargo.toml:4 |
| FND-002 | low | The rewritten comment says the `arch-lint` aggregate "Runs the two checks that need only this repository". One of them, `arch-lint-api-surface`, exits 2 without `CG_CLONE`, as Makefile:337-338 and FR-060 Status say. So `make arch-lint` always fails on this repository alone. Fix: point the aggregate at the repo-only targets (`arch-lint-api-surface-qsl arch-lint-canonical-encoder arch-lint-qualified-core`), or delete the aggregate, since `ci:` already runs those three. | Makefile:372-375 |
| FND-003 | low | The rewritten `classify` doc says it is "Shared by the FR-059 direction check ... and `xtask canonical-types`". It leaves out the third caller, `qualified_core::` (FR-284), which uses it to classify CG and RT crates. Fix: name `qualified-core` (FR-284) as well, or drop the list of callers. | tools/arch-lint/graph.rs:48-55, tools/arch-lint/qualified_core.rs:330 |

## New findings (disposition pass 1)

Round 1 also reviews the commits added after b9a376cdf that answer no
finding: 977891a6c, 4950778c8, ae3373a40, 2af8ca465, ea51436ef and a7f8047f5.

- **4950778c8 (deny.toml).** `{ crate = "quire-canonical",
  deny-multiple-versions = true }` parses, and `cargo deny --workspace check
  bans --config deny.toml` at this head prints `bans ok`. I read the
  cargo-deny 0.19.8 source (`src/bans.rs`). It groups duplicates by crate name
  only, so two git revisions of `quire-canonical` at one version still count
  as two copies, and a `deny-multiple-versions` entry raises them to Deny even
  with `multiple-versions = "allow"`. A second copy reached only through a
  dev-dependency is not counted, because `multiple-versions-include-dev`
  defaults to false.
- **ae3373a40 (Makefile).** `cargo deny --workspace check bans` takes in xtask,
  arch-lint and qsl-bench, which the root-package graph did not reach. The
  TC-205 trigger test runs cargo-deny on a throwaway manifest and does not
  depend on that flag.
- **2af8ca465 (code).**
  - The `[[bin]] fixture-audit` target is gone, along with its `mod` line, the
    `tools/fixture-audit` scan root in `seam5_retired.rs` and the
    `member_src_roots` doc.
  - The deleted tool used only `qsl_foundation`, the root crate, `serde` and
    `serde_json`. All four are still used elsewhere, so no dependency is left
    orphaned.
  - Every test deleted with `tests/it/fixture_audit.rs` and `tools/fixture-audit`
    traced only deleted ids (FR-012, FR-017-AC-4, TC-001 to TC-009), plus
    NFR-005-M-1, whose method is now inspection.
  - The coder's `cargo test --workspace --no-run` log
    (`qsl-a5-fr061-workspace-no-run.log`, written after 2af8ca465) ends
    `exit=0`.
- **ea51436ef (AGENTS.md).** The workflow rule now cites NFR-002 alone, and
  NFR-002's Verification carries the `workflow_dispatch`-only text that IT-004
  used to hold.
- **a7f8047f5 (ci.yml).** The `fixture-audit -- self-test` step is removed, and
  no other `fixture-audit` reference is left anywhere outside review records.
  The workflow's bans step does not match the Makefile's after ae3373a40
  (FND-004).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The hosted workflow's bans step still runs `cargo deny check bans --config deny.toml` without `--workspace`. ae3373a40 added the flag to the Makefile because, without it, the `quire-canonical` deny misses a second copy pulled in by xtask, arch-lint or qsl-bench. `make ci` and the manual workflow now run different bans checks. Fix: add `--workspace` to ci.yml:45, as `cargo deny --workspace check bans --config deny.toml`. This is a workflow edit, so it needs the owner's clearance. | .github/workflows/ci.yml:45; Makefile:62 |

## Dispositions

Round 1, reviewed at a7f8047f5de649bff7e245210f21f3520f5d2505
(`git log b9a376cdf..a7f8047f5`). The coder's `cargo test -p arch-lint` log
(`qsl-a5-fr061-arch-lint-test-2.log`) ends `exit=0` with 84 bin tests.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d554b5bea |
| FND-002 | fixed | 696a16234 |
| FND-003 | fixed | 2f1d6aa41 |

Round 2, reviewed at 87dde2e41105a8c8c83a74010d91cdc66db7bc43
(`git log a7f8047f5..87dde2e41`). The round adds no new finding.
- 87dde2e41 changes only ci.yml:45, the one-line edit the owner approved.
- At 87dde2e41, `cargo deny --workspace check bans --config deny.toml` prints
  `bans ok`.
- ci.yml:45 and Makefile:65 now run the same command.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 87dde2e41 |
