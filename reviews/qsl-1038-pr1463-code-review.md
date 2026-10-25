---
id: SR-17013
title: "Code review of PR #1463: fix main ci-docs intra-doc links and seam-probe list"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e6582598bfc56cf16ecfc759e4639ea57f599e87; qsl-semantics/src/check/mod.rs, qsl-replay/src/hybrid.rs, xtask/src/seam_probe.rs; ticket QSL-1038"
review_set: subset
date: '2026-10-24'
---
# SR-17013: Code review of PR #1463, fix main ci-docs intra-doc links and seam-probe list

## Summary

Ticket: QSL-1038 (follow-up also to QSL-992). Reviewed `git diff origin/main...HEAD` at e6582598b (merge base 1f5903829; `origin/main` is a971cc496, two commits ahead, neither touching these files). The review includes the rust-review lane. No spec or test changes, so gap-analysis and spec-review do not apply.

Checks run in the reviewer's own detached worktree and target directory, one cargo job at a time:

- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features` (the Makefile `ci-docs` recipe): exit 0 on the PR head; exit 101 on `origin/main` with `unresolved link to TemporalClaim::population` at qsl-semantics/src/check/mod.rs:4473. On `origin/main`, `cargo doc -p qsl-replay` under the same flags fails with `public documentation for settle links to private item Self::terminal_value` (hybrid.rs:320), which the workspace run never reaches because qsl-semantics fails first.
- `cargo run --package xtask -- seam-probe`: exit 0 on the PR head (18 checked-in locations confirmed, including `qsl-eval/src/temporal/observed_window.rs::ObservedWindowSource::read`); exit 1 on `origin/main` with `unexpected-but-present: [SeamLocation { file: "qsl-eval/src/temporal/observed_window.rs", item: "ObservedWindowSource::read" }]; expected-but-missing: []`.
- `cargo fmt --all --check`: clean. `cargo run -p xtask -- string-edge`: clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `CheckedGraph::population_key` doc link to `TemporalClaim::populations` | qsl-semantics/src/check/mod.rs | examined |
| `TemporalClaim::populations` (link target exists, public) | qsl-semantics/src/check/temporal_clause.rs | context_only |
| `SolverResult::settle` doc, `Self::terminal_value` link to plain code | qsl-replay/src/hybrid.rs | examined |
| `checked_in_locations` new `ObservedWindowSource::read` entry | xtask/src/seam_probe.rs | examined |
| `compare_probe_builds` (set equality in both directions) | xtask/src/seam_probe.rs | context_only |
| `ObservedWindowSource::read` probe arm | qsl-eval/src/temporal/observed_window.rs | examined |
| `ObservedLetters::atom` probe arm (sibling) | qsl-eval/src/temporal/observed.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Approve; mergeable. All three edits are correct and minimal.

- The two doc edits are comment-only. `TemporalClaim::populations` exists as a public method; `terminal_value` is private, so plain code formatting is the right fix (a public link target does not exist).
- The seam-probe entry is faithful. It uses the same `SeamLocation { file, item }` shape and `Type::method` naming as `ObservedLetters::atom`, and the same pattern as precedent #1057. `checked_in_locations()` returns a `BTreeSet`, so source order does not matter. `ObservedWindowSource::read` is an inherent method with `#[deny(clippy::wildcard_enum_match_arm)]` and a `#[cfg(seam_probe_eval_downstream)] FamilyOutcome::__SeamProbe => unreachable!(...)` arm, the same as `ObservedLetters::atom`. That arm comes from the QSL-992 review's FND-003 fix, so qsl-eval's own probe build (no eval-downstream cfg) reports E0004 there by design. The probe is not weakened: the comparison is set equality in both directions, the new entry matches a location rustc really reports, and no probe build or cfg changed.
- Observation, not a defect of this PR: #1434 (QSL-992) landed on main with seam-probe red, so that check did not gate that merge.
