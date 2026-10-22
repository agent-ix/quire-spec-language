---
id: SR-16120
title: "Spec review — FR-287 allows the xtask and tools/* developer binaries"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-287-reach-qsl-through-the-driver-cli.md, spec/test-cases/TC-771-qsl-builds-no-binary-and-the-driver-cli-refuses-unknown-arguments.md at 30a4fa088"
review_set: subset
evaluated_revision: "30a4fa088c35c397f76d3cfc7c31af3207cebcb5"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-771
    type: reviews
---

## Summary

Ticket: STD-268 (unblocks QSL-595 / LC7, QSL-603). PR quire-spec-language#1274
restates FR-287's Description and Behavior to exempt the `xtask` package and the packages
under `tools/`, forbids a root `[[bin]]`/`default-run`, forbids normal or build dependencies
on the tools, and moves FR-287-AC-1 and TC-771 step 1 to `cargo metadata --no-deps`.
Reviewed `git diff origin/main...HEAD` at 30a4fa088 against FR-064, MP-006, ADR-029 CB-1,
#1273's FR-002, and `cargo metadata --no-deps --format-version 1 --offline` run on the PR
head.

The PR's fixture-audit claim is verified: `tools/` on main holds only `arch-lint` and
`check-no-committed-binaries.sh`; commit 9e035e7fa (#614, QSL-477) deleted
`tools/fixture-audit/`, its `[[bin]]`, `tests/it/fixture_audit.rs` and the ci-clean-build
step; no shipped file on main names `fixture-audit`, `fixture_audit` or `JSON_DEPTH`
(only historical review/plan prose). So no depth-cap text is needed and QSL-595's
fixture-audit item is stale, as stated. `xtask` and `tools/arch-lint` are bins, and the
root crate's only edge to `xtask` is a dev dependency, as the PR body says. No new AC id.
`make check-spec-validation` exits 0. Wording is behaviour-only. No textual conflict
with any open PR.

On the #1273 interaction: the contradiction between FR-287's no-user-binary rule and
#1273's `quire-spec parse` ACs is real; it is recorded and assigned to #1273 (SR-16110
FND-001), because this PR's rule matches ADR-029 CB-1 and #1273 is the one adding new
durable requirements on the binary. #1274's AC-1 fails on main today on the root
`quire-spec` bin, as main's AC-1 already does; that is the target state LC7 delivers, not a
defect of this PR.

## Verdict

NOT MERGEABLE as written: one high finding. `cargo metadata` on the PR head lists
bin targets in two further workspace members, `qsl-string-edge` (FR-064 requires a binary of
that name) and `qsl-bench` (`qsl-bench-probe`, MP-006 and the Makefile `BENCH_PROBE`), so
the new Behavior bullet and FR-287-AC-1 contradict FR-064 and fail on main for reasons LC7
does not remove. Extend the exemption to those packages (or state it by a property they
share), and TC-771's expected result with it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The exemption covers only `xtask` and `tools/`, but `cargo metadata --no-deps` on the PR head lists two more workspace members with bin targets: `qsl-string-edge` (`qsl-string-edge/src/main.rs`; FR-064: "The standalone package and binary SHALL be named `qsl-string-edge`") and `qsl-bench` (`qsl-bench/src/bin/qsl-bench-probe.rs`; MP-006 and the Makefile's `BENCH_PROBE` run it). FR-287's Behavior bullet and AC-1 therefore contradict FR-064 and fail on main for a reason LC7 does not remove. Add both packages to the exemption (and the dependency rule), or state the rule by a property all developer tools share, and update TC-771's expected result. | spec/functional/FR-287-reach-qsl-through-the-driver-cli.md:61-62,77; Cargo.toml workspace members; qsl-string-edge/src/main.rs; qsl-bench/src/bin/qsl-bench-probe.rs |
