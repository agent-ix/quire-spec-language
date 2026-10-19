---
id: SR-12179
title: "Code review of quire-spec-language PR #1018: one deep-format invocation at three threads"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b571740e523d9dd51978a34ef3792adb394f2b8d; PR #1018: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1018

## Summary

Ticket: QSL-888. Code review with the rust-review lane folded in (the xtask
TC-915 oracle), over `git diff origin/main...HEAD` at b571740e5 only. No builds
or tests were run by the reviewer; the leader gates.

What was checked:

- Exact-listing guard: unchanged. It still names all five cases, runs
  `cargo test ... -- --ignored --exact "$test" --list` for each and fails unless
  exactly one `<name>: test` line matches. The 0 and 2 count controls in
  `deep_formatter_target_runs_each_full_size_case_and_is_in_ci` still run
  against the real recipe through the cargo double.
- All five cases run in the single invocation. The run line passes the same
  five names as libtest filters with `--ignored --exact`. All five live in the
  `it` test target (`tests/it/formatter_identity.rs`, each
  `#[ignore = "runs in make test-deep-format (part of make ci)"]`), and the
  listing guard used the same `--workspace --tests -- --ignored --exact`
  selection, so a name that the guard accepts is one the run selects. libtest
  ORs multiple filters, so none is dropped. The removed second invocation was
  the only one naming the chain case, and the chain case is now the fifth
  filter of the remaining one.
- `deep_calls`: the oracle runs the real Makefile copy and compares the complete
  logged argv sequence with `assert_eq!`. A renamed or removed case in the
  Makefile, a restored second invocation, or any thread count other than
  `--test-threads=3` changes the argv and fails it. A test function renamed in
  source with the Makefile unchanged is caught at run time by the listing
  guard (count 0), not by the oracle; that is the same split as before.
- Rust lane: the change removes the `split_last().unwrap()` and the closure;
  the new code is a plain `Vec` plus `extend(DEEP_CASES)`. No new panic path,
  no new dependency, no trait seam. `DEEP_THREADS` is used once; it names the
  value in the doc comment, which is reasonable.
- Memory and the lane cap (`/home/peter/dev/worktrees/locked-build.sh`
  lines 126-132): `cap = min(20, MemAvailable - 16)` GiB, halved when another
  lane is busy, measured once at launch. There is no 12 GiB floor. See FND-001.
- No `.github` file is touched. No compatibility layer, fallback or second
  code path. Only QSL-888's asks are claimed in the PR body.
- Main has no commit touching either file since the branch's merge base.

## Verdict

Mergeable: no high finding. One medium finding (the Makefile comment states a
12 GiB cap floor that does not exist, so the thread-count justification is
wrong on a contended lane) and two low findings.

On the OOM question: 3 threads is a real but narrow risk, not a blocker. With
another lane busy the cap is `(MemAvailable - 16) / 2` GiB, rounded down. The
3-thread peak is 9.6 GiB (PR body) or 9.85 GiB (Makefile comment); either way
it needs a cap of 10 GiB, so it fails when MemAvailable at launch is 35 GiB or
less while another lane runs. On terra (94 GiB) that takes three other heavy
lanes. At exactly 10 GiB the margin is 0.15-0.4 GiB, and the cgroup also
charges cargo itself and page cache, so treat 10 GiB as marginal too (36-37 GiB
available). The 2-thread run (8.1-8.3 GiB) survives a 9 GiB cap, so the PR
widens the failing window by about one GiB step. The failure is a loud exit
137, never a wrong result. What I would do: keep 3 threads and merge; correct
the comment to state the real rule and the 10 GiB need (FND-001); if this
target ever exits 137 under a contended lane, drop to `--test-threads=2`
(about 150 s slower, peak 8.1 GiB). 4 threads is unmeasured and not claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new comment says three threads peaked "under the 12 GiB floor", and the kept line says the cap is "12 to 20 GiB depending on the other lanes". `locked-build.sh` has no floor: `cap = min(20, MemAvailable - 16)` GiB, halved when another lane is busy. A contended launch with 35 GiB or less available gives a cap of 9 GiB or less, below the 9.6-9.85 GiB three-thread peak, and the run dies with exit 137. Suggested wording: "the cap is min(20, available - 16) GiB, halved when another lane is busy; three threads peak at 9.6 GiB and need a 10 GiB cap (36 GiB available when contended)". | Makefile:231-235 |
| FND-002 | low | The Makefile comment gives the three-thread peak as "10.3 GB (9.9 GiB)" and the PR body as "10.31 GB (9.6 GiB)". 10.3 GB is 9.6 GiB; 9.85 GiB is what 10.33 million KiB (the unit `/usr/bin/time -v` reports) comes to. One of the two conversions is wrong, and the headroom against a 10 GiB cap is 0.15 GiB or 0.4 GiB depending on which. State the raw `Maximum resident set size (kbytes)` figure once and convert from it. | Makefile:233 |
| FND-003 | low | Doc comment reads "then the all five cases in one run"; drop "the". | xtask/src/ci_feature_lane_tests.rs:237-238 |
