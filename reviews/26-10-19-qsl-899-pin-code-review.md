---
id: SR-13118
title: "Code review of quire-spec-language PR #1033: quire-semantic-value pin bump"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@56b31ef2573d76fdec054be7671f5d07f547521e; PR #1033 diff origin/main...HEAD (Cargo.lock only); rust-review lane folded in"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: references
---
# Code review of quire-spec-language PR #1033

## Summary

Ticket: QSL-899. PR: quire-spec-language#1033 at 56b31ef. The diff changes two Cargo.lock
source lines. `Cargo.toml` tracks `branch = "main"` for both crates, so the lock rev is the pin.

- `quire-semantic-value` 02e5cf5 → e992478, the head of quire-semantic-value#50. Besides the
  new `step_simulation` module this also takes in SV main's IR-878 and IR-886 commits
  (0b26020, ffdeffd, 653170f), which were already on SV main.
- `quire-exact` 21349174 → 90e9a4f2 (current quire-exact main). SV does not require this: SV's
  own lock pins quire-exact d18927e, an ancestor of 21349174, so the move is a refresh of the
  shared branch pin, not a consequence of the SV bump. QSL depends on quire-exact directly
  (workspace dependency of qsl-eval, qsl-cst, qsl-forms, qsl-analyze, qsl-bench). The six
  commits change production code in two places only, both additive: a new
  `ChargePoint::TemporalVisit` variant (`ALL` 62 → 63, name `temporal.visit`) and a
  `Meter::cancel()` accessor. `src/equality.rs` changes are tests only. QSL does not reference
  `ChargePoint::ALL` or the new variant, and nothing in QSL charges it, so QSL behaviour is
  unchanged; the build shows no exhaustive match broke.

Gates run at the PR head with a private target dir: `cargo build --locked --workspace` pass;
`xtask string-edge` pass ("no unmarked, unlisted string comparison or string match found");
`cargo test --locked --workspace --lib` pass (2043 passed, 0 failed, 1 ignored). The worktree
stayed clean.

Merge order. The lock records `branch=main#e992478`, a commit that is on the SV PR branch and
not on SV main. SV merges by squash by convention, which puts a different sha on main. If QSL
merges as is, its main lock names a commit main never contains; `cargo update` would silently
move it, and a fresh fetch depends on the PR ref staying fetchable. Safe procedure:

1. Merge quire-semantic-value#50 first. Either (a) squash, then re-pin; or (b) use a merge
   commit (`gh pr merge 50 --merge`; the repo allows it), which keeps e992478 reachable from SV
   main so the QSL lock is already valid and needs no re-pin.
2. After (a): in the qsl-899-pin worktree run
   `cargo update -p quire-semantic-value --precise <SV squash sha>`. Confirm
   `git -C <sv clone> merge-base --is-ancestor <sha> origin/main` and
   `git diff e992478 <sha> --stat` is empty (the PR is based on current SV main 653170f, so
   the trees should match). Confirm `git diff origin/main -- Cargo.lock` shows only the
   quire-semantic-value and quire-exact source lines.
3. Re-run `cargo build --locked --workspace`, `xtask string-edge` and
   `cargo test --locked --workspace --lib`, push, then merge #1033. Do not merge #1033 before
   #50.

## Verdict

**CONDITIONAL**: the bump compiles and changes no QSL behaviour. At its current head the lock
points at a pre-merge SV commit, so it must be re-pinned after a squash merge of #50 (or #50
merged with a merge commit) before #1033 merges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Cargo.lock pins `quire-semantic-value` to `branch=main#e992478ef9`, the head of SV PR #50, which is not on SV main. Merged as is after a squash of #50, QSL main's lock names a commit outside the tracked branch. Re-pin with `cargo update -p quire-semantic-value --precise <SV main sha>` after #50 merges, or merge #50 with a merge commit | Cargo.lock:1680 |
| FND-002 | low | `quire-exact` 21349174 → 90e9a4f2 is not required by the SV bump (SV's own lock is at d18927e, an ancestor of 21349174), though the PR body says it "follows". The move is additive (`ChargePoint::TemporalVisit`, `Meter::cancel()`) and the reviewer found no QSL behaviour change; state it as an intentional refresh, or keep 21349174 with `cargo update -p quire-exact --precise 21349174dedae862cb7fdbb76952a37c837c55c1` when re-pinning | Cargo.lock:1635 |
