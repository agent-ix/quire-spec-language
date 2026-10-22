---
id: SR-12337
title: "Gap analysis of PR #1230 (IR-587 S3: qsl-foundation no_std + alloc) against NFR-015 / TC-1110"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@43bdfabe9adfd46bb52b2ca8a031ba7a176f3ed3; Makefile, qsl-foundation/**; spec/non-functional/NFR-015-*.md, spec/test-cases/TC-1110-*.md, spec/test-cases/TC-1111-*.md"
review_set: subset
---

## Summary

Ticket: IR-587. A planless gap analysis of the S3 row of NFR-015, which is
the `qsl-foundation` closure tier. Plan completion: not assessed.

`quoin matrix --repo . --json` (quoin 0.28.3) ran for about 10 minutes and
wrote nothing to stdout. Its only stderr was module-duplication warnings,
so the matrix could not be read. Trace coverage was checked by searching
the Rust sources for `TC-1110` and `NFR-015` tags instead.

TC-1111 (the `qsl-package` row, S5/IR-589) is outside this PR. It was read
for context only.

Per acceptance criterion of NFR-015's S3 row, measured at the reviewed head:

- AC-1: met. `default = ["std"]`, and the governed-target build succeeds on
  the pinned 1.98.1 toolchain. It is checked by `make build-no-std`, which
  is in `CI_CHECKS`.
- AC-2: met by measurement. The depth-1 tree is exactly `ix-trace-rs`
  (proc-macro), `quire-exact`, `serde`, `sha2` and `thiserror`, and the
  `-e normal,features` tree names no `"std"` feature. No gate or test runs
  this check.
- AC-3: met by measurement. `cargo test -p qsl-foundation` passes with and
  without default features. No gate runs the `--no-default-features` test
  pass (FND-001). The workspace lanes, which all build with `std`, are
  covered by `make ci`, but the code review's FND-001 makes that gate red.
- AC-8: the positive half is met: the recipe, the `-p`, the target, the own
  `no-std` directory and the `CI_CHECKS` entry. The negative half, where a
  shipped `std::` path must fail the target, was met by a reviewer probe
  that failed with E0433, but nothing automated checks it.
- No Rust test carries `#[trace("TC-1110", "NFR-015-AC-...")]`. NFR-015's
  Verification section says the tests do (NFR-005), so the matrix will show
  AC-1, AC-2, AC-3 and AC-8 as untested.
- No stubs, no dead `cfg(feature = "std")` code, and no production code
  outside the row: the feature gates no item, as the row's "Behind `std`:
  no item" requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1110 is backed only by the governed-target build. NFR-015-AC-3's `cargo test --locked -p qsl-foundation --no-default-features` is in no gate, so the test modules' `alloc` imports can stop compiling without anything failing. AC-2's tree check and AC-8's negative case have no automated check either. No test is tagged TC-1110 / NFR-015-AC-1, AC-2, AC-3 or AC-8. Fix: in the xtask tests that FND-001 of SR-12336 already has to touch, add a test tagged `TC-1110`, `NFR-015-AC-8` that asserts the `build-no-std` recipe's recorded commands and its `CI_CHECKS` entry, following the existing `string_edge` / `seam_probe` CI_CHECKS checks. Then either run the `--no-default-features` test pass inside `build-no-std`, or record in TC-1110 that AC-3's second half is a manual check | Makefile:280-282; spec/test-cases/TC-1110-qsl-foundation-builds-no-std-on-the-governed-target.md; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md |

## Verdict

Mostly met. The S3 row is implemented, and every acceptance criterion
holds when measured at the reviewed head. The behavioural core of TC-1110 is
the governed-target build, which cannot pass on the host by accident. That
build is real and is wired into `make ci`. The gap is that half of what
TC-1110 checks (AC-2, AC-3 without `std`, and AC-8's failure case) runs
only by hand, and nothing in the matrix binds to TC-1110 (FND-001, medium).
