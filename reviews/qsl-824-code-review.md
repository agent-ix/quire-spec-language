---
id: SR-12065
title: "Code review of quire-spec-language PR #931: test-deep-format in the default lane's target directory"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1fe47e139; PR #931: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md"
review_set: subset
---
# Code review of quire-spec-language PR #931

## Summary

Ticket: QSL-824. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...1fe47e139` (3 files, +75/-117).

`test-deep-format` now runs `cargo test --locked --workspace --test it` in the
default lane's directory (`CI_DEFAULT_TARGET_DIR`, else
`${CARGO_TARGET_DIR:-target}/ci-default-features`). It keeps the per-name
`--ignored --exact --list` guard and then runs the three ignored
`formatter_identity` tests in one process.

## Verdict

Pass. No major defect found.

- Feature and profile identity. The default lane's `default-workspace` suite
  runs `cargo test --locked --workspace` (xtask `test_command`; the enumerate
  step only adds `--no-run --message-format=json-render-diagnostics`). The deep
  stage runs the same `--locked --workspace` selection in the test profile.
  `--test it` filters targets only. Under resolver 2, dev-dependency feature
  activation is global for any test target, so feature unification is the same.
  The Makefile sets no RUSTFLAGS for either stage, and both inherit the caller's
  environment. `cargo test` always runs the binary whose fingerprint matches its
  own selection, so no artifact built under other features or flags runs. The
  aggregate `ci-internal` runs Make serially (xtask passes no `-j`).
- All three tests still run. Every name follows `--`, with `--ignored --exact`.
  The exact-listing guard (count == 1 per name) is intact. `--workspace` now also
  runs the eight other members' `it` binaries with these filters. Those binaries
  match zero tests and exit 0, and none of them defines a `formatter_identity::`
  test, so the guard count stays exact.
- Dropped `--test-threads=1` (at 1fe47e139). The three tests share no state:
  no statics, `OnceLock`, env mutation, files or subprocesses in
  `tests/it/formatter_identity.rs` or `tests/support/front_end.rs`. Each test
  builds its own source, and the small-stack formatter runs on its own spawned
  thread. libtest spawns a thread for every test at any thread count
  (`library/test/src/lib.rs` run_test_inner), so the stack size does not change.
  Running two at once cannot race. It only raises peak memory. Not a defect.
  Commit 3d0170812, pushed after this head, restores `--test-threads=1` anyway.
- Removed lines are only the old `target/test-deep-format` plumbing: the
  per-name run lines and the duplicated expectation vectors in two tests.
- Mutation reasoning. The three `deep_calls` assertions (no root, a caller
  root, a `CI_DEFAULT_TARGET_DIR` override) and the aggregate assertion compare
  full argv and environment. Reverting to a separate directory, dropping
  `--workspace`, or dropping a name from the run or from the listing loop each
  changes the recorded calls and fails the test. The count-0 and count-2
  `run_raw` cases still pin the guard.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
