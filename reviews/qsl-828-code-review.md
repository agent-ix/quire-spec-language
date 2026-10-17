---
id: SR-12085
title: "Code review of quire-spec-language PR #939: the deep formatter cases run concurrently"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@484d683bf38a918b50e5b6b360e9c8a5e32f1403; PR #939: Makefile (test-deep-format), tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs"
review_set: subset
---
# Code review of quire-spec-language PR #939

## Summary

Ticket: QSL-828 (ticket text read as data). Code review with the rust-review
lane folded in, scoped to `git diff origin/main...484d683bf` (3 files,
+87/-61). Read-only: no build or test was run. The leader's measured run is
still in progress and is not assessed here.

## Verdict

Pass. No major bug found.

- Nothing lowered. The helper `deep_source_generator_identity` is the old loop
  body, line for line. All three generators are kept (`sum`, `else if`,
  `let`), each at 100,000 levels. The header, the end text, all four limit
  overrides and the checking limits are unchanged. The `sum` case still runs
  the small-stack control: it parses with the "small-stack" identity, asserts
  admission, formats on its own thread named `deep-formatter-small-stack` with
  a 256 KiB stack, and asserts non-empty output. Every case still goes through
  `emitted_with_authority`, which runs the full format-identity check (format,
  second-pass byte identity, re-check, package-id equality). No assertion was
  dropped or weakened. The old `start.clone()` becomes a fresh `fixture_text`
  call, which gives the same bytes.
- Phase timers. The branch added phase timers in 58ae33b3d and removed them in
  484d683bf. None are left in the net diff.
- Stale name. Nothing in the tree still names
  `all_three_deep_source_generators_keep_format_identity`: not the Makefile,
  the xtask tests, NFR-013, TC-885, FR-003 or any `#[ignore]` reason. All five
  `#[ignore]` reasons say "runs in make test-deep-format (part of make ci)",
  which is true.
- Makefile guard. The guard loop and the run list both name the same five
  cases. Each name must list exactly one ignored test. A renamed or deleted
  case lists 0 and fails the stage. `--test-threads=5` gives one thread per
  case, so the run takes about as long as the slowest case.
- Shared state. The three helpers share no mutable state. There is no static,
  `OnceLock`, `LazyLock` or `thread_local` in `tests/support/` or in the test
  file. Each case builds its own source and limits. The small-stack control
  still spawns its own 256 KiB thread inside its test thread, so concurrency
  does not change its stack bound. The leader's 3-way parallel profile showed
  that the outer check path (S1 to S4 plus packaging) fits on libtest's
  spawned test-thread stack.
- Memory (estimate, not measured). Each generator source is about 1.8 MB. The
  node limit is raised to bytes x 16 (about 29 M), but that is a ceiling, not
  the expected use. At roughly 3 to 8 nodes per level, each case holds a few
  hundred thousand to about 1 M CST nodes per parse. The check path parses
  twice (original and formatted) and checks and packages twice. Peak use is
  likely a few hundred MB per case, so about 1 to 3 GB with all five
  running. That is well within the 64 GB build box and typical CI runners.
  It is not a finding. The leader's final run can confirm it with peak RSS.
- xtask (TC-915). `deep_calls` builds the expected argv from `DEEP_CASES`
  (now five names, `--test-threads=5`). The recorder test compares it for
  exact equality with what the recipe ran. A case removed from or reverted in
  the Makefile, or a thread count changed back, fails all three routing
  assertions. The 0/2 listing-count rejections are unchanged.
- Rust idiom (low, not recorded as a finding). The helper takes a stringly
  typed `label: &str`, and `_` sends any other label to the `let` generator. A
  mistyped label would quietly run a second `let` case. All three call sites
  pass the literal names, so nothing goes wrong today.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
