---
id: SR-12884
title: "Code review (Rust lane) — QSL-865 make ci on macOS (PR 975)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ea854a05679de3c70fefb8bcc2e9b2282ee9c465; git diff origin/main...HEAD: Makefile, qsl-semantics/tests/it/model_identity_full_size.rs, tests/it/cli.rs, tools/check-no-committed-binaries.sh, xtask/src/conformance/tests.rs, xtask/src/lib.rs, xtask/src/test_suites.rs"
review_set: subset
---

## Summary

Ticket: QSL-865. Code review with the rust-review lane folded in, over PR 975's own two commits (ffdd57fd6, ea854a056). Focused checks run with a scratch CARGO_TARGET_DIR: `cargo test -p xtask --lib` (QSPEC_DIR unset), `cargo test -p xtask --lib` (211 pass), `cargo test -p qsl-semantics --features test-support --test it --release model_identity_full_size` (pass; `peak_resident_bytes=47693824`, about 45 MiB, a plausible byte count), `cargo test -p quire-spec-language --test it cli_preserves` (macOS: the argv half passes and the gated half is absent), clippy `-D warnings` on xtask, qsl-semantics (with and without test-support) and the root crate (clean), and `tools/check-no-committed-binaries.sh`. The Makefile change was probed on GNU Make 3.81 (host) and GNU Make 4.4.1 (Linux container) with a reduced Makefile carrying the old and new forms. Full `make ci` not run (brief); the coder's draco log ends `exit=0`.

## Verdict

**Mergeable. No blocking findings.**

- Makefile: on Make 4.4.1 the old combined form and the new split form give the same result for env, command-line and unset callers: origin `override`, flavor `simple`, and the recipe environment gets the raw `$qspec$(X)` bytes. On 3.81 the old form left the variable recursive and expanded `$q`/`$(X)` for a command-line value; the split form fixes that. Equivalent on Linux, fixed on macOS.
- check-no-committed-binaries.sh: `wc -c <"$file" | tr -d ' '` prints a bare byte count on GNU and BSD. The redirect keeps file names with spaces, newlines or a leading `-` safe, and the NUL-delimited `git ls-files -z` loop is unchanged. One difference: `wc` follows a symlink and `stat -c%s` did not, but the repo tracks no symlinks, and the magic check (`head -c`) already followed them.
- model_identity_full_size: Linux is unchanged (the child reads its own VmHWM, in KiB, times 1024). macOS reads `ru_maxrss` of the child through `/usr/bin/time -l`, which reports bytes, so the units are right. Both measure the peak RSS of the same child process against the same 20 GiB ceiling, so the bound still tests what it did. The counter assertions are unchanged.
- cli.rs: Linux still runs both halves (two tests, both tagged TC-017), so Linux coverage is the same. Only macOS drops the non-UTF-8 file-name half, because APFS returns EILSEQ for such names.
- is_unreaped: probed on macOS. `ps -p` exits 0 for an unwaited zombie (state Z), 1 once it is reaped, and 1 for a pid that does not exist. procps `ps -p` on Linux behaves the same way and lists `<defunct>` entries. Note that the old `/proc` probe was always false on macOS, so the negative assertions passed there without testing anything. The new probe makes them real.
- test_support::tempdir: canonicalising `temp_dir()` before `tempdir_in` is a no-op on Linux (`/tmp` is not a symlink) and removes the `/var` vs `/private/var` mismatch on macOS. The helper is `#[cfg(test)]`, so it does not change non-test code.
- No new `unsafe`, `unwrap` or panic in non-test code. The new panics are all in test code and carry messages.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `run_child` and `child_peak_bytes` exist only for `linux` and `macos`, so on any other target (Windows, FreeBSD) the qsl-semantics `it` test binary no longer compiles. Before this PR it compiled there and failed only at runtime. CI is ubuntu-only and draco is macOS, so nothing breaks today. A `#[cfg(not(any(linux, macos)))]` fallback that skips with a reason, or a cfg on the test, would restore that. | qsl-semantics/tests/it/model_identity_full_size.rs:109-147 |
| FND-002 | low | The parent no longer prints `VMHWM_BASELINE_BYTES` (peak before normalization) on Linux, although the child still prints it. The N19 line loses the before-normalization figure it used to report. This is only diagnostic output; no assertion used it. | qsl-semantics/tests/it/model_identity_full_size.rs:70,103,183 |
| FND-003 | low | `is_unreaped` is only ever asserted false (lines 2987, 3016), and no positive control asserts it is true for a live unwaited child. If the probe ever went vacuous again (as `/proc` was on macOS), these tests would still pass. The PR swaps the probe without adding a positive control. This predates the PR. | xtask/src/test_suites.rs:2957-2966 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The new probe self-test `the_liveness_probe_sees_an_exited_child_until_it_is_waited_for` carries `#[trace("TC-950", "NFR-014-AC-10")]`, but it checks the test helper `is_unreaped`, not the product's reap-on-I/O-failure behavior that NFR-014-AC-10 requires. That adds one more binding to the AC that does not test the AC. Fix: drop the trace attribute, or keep it only if the repo's convention is to tag helper self-tests to the AC they support. | xtask/src/test_suites.rs:2968-2969 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | afdc5a920: `run_child` is now `cfg(not(target_os = "macos"))`, and there is a `child_peak_bytes` stub for other targets that panics with a clear message, so the test fails loudly there instead of passing unchecked. Read as correct for Windows/BSD (all cfg arms covered, unused params prefixed); host clippy is clean. |
| FND-002 | fixed | afdc5a920: on Linux the parent prints `N19 vmhwm_before_normalization_bytes=` from the child's `VMHWM_BASELINE_BYTES`. Verified in a Linux arm64 container (rustc 1.98.1, procps-ng 4.0.4): `N19 vmhwm_before_normalization_bytes=11591680`, `peak_resident_bytes=47845376`, which matches the macOS peak (47693824) in magnitude and confirms both are bytes. |
| FND-003 | fixed | afdc5a920: new test `the_liveness_probe_sees_an_exited_child_until_it_is_waited_for` polls `ps -o stat=` until the child is a zombie, asserts `is_unreaped` is true, then waits and asserts false. It passes on macOS and on Linux (container), together with the four existing reap tests. |
| FND-004 | fixed | 73854d4cf: `#[trace("TC-950", "NFR-014-AC-10")]` removed from `the_liveness_probe_sees_an_exited_child_until_it_is_waited_for`; the test still runs (5 reap/liveness tests pass) and xtask clippy -D warnings is clean. Since afdc5a920 the PR files changed only by this line and the review records; the merge of origin/main touched no PR file. |
