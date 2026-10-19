---
id: SR-12155
title: "Code review of quire-spec-language PR #993: faster deep formatter identity tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a52e2ed0fdd6004e00a2956f5c8368f8e93a7119; PR #993: tests/it/formatter_identity.rs, tests/support/front_end.rs, Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, Cargo.toml ([profile.dev.package.*])"
review_set: subset
---
# Code review of quire-spec-language PR #993

## Summary

Ticket: QSL-828. Code review with the rust-review lane folded in, over
`git diff origin/main...HEAD` at a52e2ed0f only. No builds or tests were run
by the reviewer; the leader gates.

What was checked:

- The split of `all_three_deep_source_generators_keep_format_identity` into
  `deep_sum_`, `deep_else_if_` and `deep_let_source_generator_keeps_format_identity`
  is a mechanical lift of the old loop body into
  `deep_source_generator_keeps_format_identity(label)`. Every generator still
  appends 100,000 levels, the limits widening is unchanged, the
  `emitted_with_authority` check and its panic message are unchanged, and the
  256 KiB small-stack formatter control still runs only on `sum`. Each test
  keeps `#[trace("TC-885", "FR-003-AC-9")]` and the same `#[ignore]` text.
- The Makefile listing guard names all five cases and requires exactly one
  `<name>: test` line each, so a renamed or removed case fails before the run.
  The run line names the same five. The TC-915 argv oracle
  (`every_feature_recipe_uses_its_child_and_preserves_complete_argv`) runs the
  real recipe and compares the complete argv against `DEEP_CASES` plus
  `--test-threads=2`, so a reverted recipe, a dropped case or a changed thread
  count fails it. No case is dropped from the run.
- `dependency_fixture_identity`: the original side runs on a scoped thread and
  the formatted side on the calling thread. A panic in the formatted side
  unwinds out of `thread::scope`, which joins the spawned thread first and
  then resumes the panic. A panic in the original side makes `join()` return
  `Err`, and `.expect("the original check thread runs")` fails the test; the
  original panic message is still printed, because spawned threads inherit
  libtest's output capture. The closures capture only shared references
  (`&source`, `&packages`, `&original_input`, `&lock`) and the `Copy` value
  `limits`; `checked_once` creates its own `Cancel`. No mutable state is
  shared. The emission and identity comparison loop after the scope is
  unchanged and serial.
- Stack: libtest runs every test on its own spawned thread regardless of
  `--test-threads`, so dropping `--test-threads=1` does not change any test's
  stack size. The scoped thread gets the same default stack as the test
  thread. The removed comment ("deep-stack tests must not share a thread
  pool") protected nothing that the thread count controls.
- `[profile.dev.package.*] opt-level = 3`: all five names are external
  dependencies present in `Cargo.lock` (no workspace member is affected).
  Only `opt-level` is overridden; `debug-assertions` and `overflow-checks`
  stay at the dev profile's values for those packages, and the `test` profile
  inherits the overrides. No test's assertions or arithmetic semantics change.
  Optimized frames are smaller, so recursion depth limits inside those crates
  move toward release behaviour, which is what ships. The rebuild of those
  five crates is a one-time cost per target directory. No entry is plainly
  pointless: sha2 and serde_json sit on the hashing and canonical-encoding
  path the profile names, and the three quire crates own package encoding.
- No `.github` file is touched.
- PR #939 (closed unmerged) split the same test and ran five threads. This PR
  carries the split forward and supersedes the thread count. #939's other
  files were its own review artifacts. Nothing from it is needed.

## Verdict

Mergeable: no high finding. One medium finding on memory headroom under a
contended build lane, and two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The 18.5 GB stage peak (and the dependency-chain case's 16.5 GB alone, about 15.4 GiB) was measured against the best-case 20 GiB cap. `locked-build.sh` computes `cap = min(20, avail - 16)` GiB and halves it when another lane is busy, so with any other chain running on the 64 GB host the cap is about 12-17 GiB. That is below the 18.5 GB two-thread peak, and below the chain case alone unless 47+ GiB is free. The stage then dies with exit 137: a loud false red, not a hidden failure. Running the chain case on its own (the four lighter cases at `--test-threads=2`, then the chain at 1) bounds the stage at the chain's own peak and makes the passing condition simply "cap >= the chain". Whether the scoped thread raised the chain's own peak was not measured: the serial and parallel peaks of that case are not both reported. | Makefile:242-247; tests/support/front_end.rs:240-267 |
| FND-002 | low | The Makefile comment says "`--test-threads=2`, one thread per case", but two threads run five cases, and it does not say why two: the memory cap. Suggested: "two at a time: five concurrent cases peak at 26.8 GB, over the 20 GiB lane cap". | Makefile:225-226 |
| FND-003 | low | `let mut source = start.clone();` clones a `String` that is not used again; `let mut source = start;` is enough. | tests/it/formatter_identity.rs:1328 |
