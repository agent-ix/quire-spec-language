---
id: SR-12077
title: "Code review of quire-spec-language PR #931 final head: test-deep-format selects --workspace --tests in the default lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@74651fc5a8aeaa94b7c2c709e31bdf05791f2223; PR #931: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, xtask/src/ci_feature_lane_tests/cargo_double.rs, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md, reviews/qsl-824-*.md"
review_set: subset
---
# Code review of quire-spec-language PR #931 (final head)

## Summary

Ticket: QSL-824. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...74651fc5a` (6 files, +196/-122). This is the second
round. SR-12065 reviewed 1fe47e139. Since then the PR restored
`--test-threads=1`, reworded NFR-013, and changed the deep stage from
`--workspace --test it` to `--workspace --tests`, with the filter after `--`.
The explicit `--test it` errored because qsl-source's `it` target has
required features that are off in the default lane.

## Verdict

Pass. No major defect found. Read-only review: no build was run. The leader
measured the real run (logs/qsl931-ci2.log, EXIT=0): the deep stage compiled
nothing in `target/ci-default-features` and took 1439.59 s.

- Same artifacts as the default lane. Default position 3 runs
  `cargo test --locked --workspace` through xtask `test_command`. Only
  `CARGO_TERM_COLOR` is added, and it does not affect fingerprints. With no
  target flag, `cargo test` selects lib and bin unit tests, integration tests,
  `test = true` examples, and doctests. `--tests` selects the same compiled
  test targets, without the doctests. It also drops the plain example builds,
  which the deep stage does not need. Both runs select dev units for the same
  workspace package set, so resolver 2 unifies the same features. Both
  silently skip targets whose required features are off: qsl-source's `it`,
  and the root `handoff-writer` examples. The Makefile sets no RUSTFLAGS or
  profile on either stage. Make runs `ci-internal` serially (xtask removes
  MAKEFLAGS), and `ci-default-features` comes before `test-deep-format` in
  CI_CHECKS. Even if a future unification difference made Cargo recompile,
  the fingerprint includes the features, so the deep run would execute only
  an artifact built under its own selection. That would cost time, not
  correctness. The leader's zero-`Compiling` evidence shows it does not
  happen today.
- Exact filter across binaries. Each listing runs every selected binary with
  `--ignored --exact <name> --list`. The awk count sums matching lines across
  all binaries' stdout. If a name were present as an ignored test in two
  binaries, the count would be 2 and the guard would fail. The three names
  are defined only in `tests/it/formatter_identity.rs` of the root `it`
  binary. No other unit or integration target has a `formatter_identity`
  module. The listing and the run use the same `--locked --workspace --tests`
  selection and directory, so the guard checks the binary set that runs.
- Serial run. `--test-threads=1` is back on the one combined run. Running in
  one process instead of three does not change stack size, because libtest
  spawns a thread for each test. No test-binary failure is masked: all three
  tests are in one binary, and libtest runs all of them before it reports.
- xtask test double. It now reads the filter after `--exact`. A revert to
  `--test it <name> -- --ignored --exact --list` would make the double print
  `--list: test`, so the guard would count 0 and fail. Separately, the
  argv-equality assertions in `deep_formatter_target_runs_each_full_size_case_and_is_in_ci`
  (no root, a caller root, a `CI_DEFAULT_TARGET_DIR` override) and in the
  aggregate test fail on any change to the directory, `--tests`/`--workspace`,
  the thread flag, or the name set. The count-0 and count-2 `run_raw` cases
  still pin the guard.
- Rust: the `deep_calls` helper and `DEEP_CASES` const are straightforward.
  The trace tags are unchanged. No panics outside test code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
