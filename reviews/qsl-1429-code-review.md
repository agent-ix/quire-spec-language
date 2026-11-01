---
id: SR-202610310147
title: "Code review — full-size model identity metric handling"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/tests/it/model_identity_full_size.rs; spec/test-cases/TC-1025-full-size-model-identity-case.md; spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md"
review_set: subset
---

## Summary

Reviewed PR #2039's changed Rust integration test against QSL-1429 and TC-1025 / FR-081-AC-12/13. The metric parser skips only the unavailable peak-memory measurement after a successful child; failed children, absent metric output, malformed metrics, and available over-limit measurements still fail.

## Verdict

**PASS** — no defects found in the reviewed diff. Focused formatting, Clippy, and integration tests pass; the `/usr/bin/time -l` branch was validated through its parser tests, but could not be executed natively on macOS here.

## Review evidence

- Diff scope: `origin/main...HEAD`, one changed file: `qsl-semantics/tests/it/model_identity_full_size.rs`.
- The Mac-specific `run_child` remains under `#[cfg(target_os = "macos")]` and invokes `/usr/bin/time -l`. `macos_peak_bytes` returns `None` only after receiving a successful child and finding both `sysctl kern.clockrate` and `Operation not permitted` in stderr, with no peak line.
- The parent integration test asserts child status success before parsing counters or metrics. The helper also rejects a failed child. Missing output and non-numeric peak output return errors. A numeric peak remains subject to the 21,474,836,480-byte ceiling.
- The 10,001 declaration count, exact facts/work counters (20,001 / 50,003), 524,288-byte stack, completed normalization, wall-time ceiling, and pre-existing full-size assertions remain in the diff. Unavailable memory skips only the memory ceiling assertion.
- The added parser test passes for successful-child denial, failed-child denial, missing output, malformed output, and a valid peak.

### Acceptance criteria to tests (manual; Quoin gap-analysis not applicable)

- FR-081-AC-12 and FR-081-AC-13 are both still attached to `ten_thousand_edge_chain` by the Rust `#[trace]` attribute. TC-1025 remains the owned test case.
- AC-12: the child builds the full chain and normalizes at default limits on the 524,288-byte thread; the test requires completion and exactly 10,001 declarations and 20,001 retained facts. The exact work counter remains 50,003.
- AC-13: the isolated child wall time remains bounded by 30 seconds. Linux still checks VmHWM and macOS still checks `/usr/bin/time -l` when it reports a peak; the ceiling is enforced whenever that metric is available.
- Failure oracle: a failed child fails before unavailable-metric handling; the unit test independently verifies this. Unexpected/missing or malformed metric output also fails.
- The reviewed host is Linux. The macOS target-specific process invocation and sandbox behavior were not run on macOS; the shared parser behavior was exercised by the focused unit test on Linux.

### Checks

Commands below ran through `/home/peter/dev/worktrees/locked-build.sh` with worktree-local targets. Cargo used the existing writable cache at `/home/peter/.cache/cargo-qsl-1438` after the default `~/.cargo` cache refused writes.

| Command | Exit | Log / note |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 0 | `/tmp/qsl-1429-review/fmt.log` |
| `cargo clippy --locked -p qsl-semantics --test it --features test-support -- -D warnings` | 0 | `/tmp/qsl-1429-review/clippy-final.log`; initial default-cache attempt exited 101 because the read-only Cargo Git cache could not create a pack file |
| `cargo test --locked -p qsl-semantics --test it --features test-support model_identity_full_size:: -- --test-threads=1` | 0 | `/tmp/qsl-1429-review/focused-test-last.log`; 2 passed, 0 failed. Earlier default-cache attempt exited 101; initial compile attempts were stopped after lock waits / no output (130) and retried successfully with a warm worktree-local target |
| `cargo deny check` | 4 | `/tmp/qsl-1429-review/deny-final.log`; existing policy rejects AGPL-3.0-or-later dependencies and warns on Git sources; this PR changes neither dependencies nor policy |

`make ci` was not run. Quoin gap-analysis was not applicable because the diff changes only test code; the manual acceptance-criteria-to-tests check above was performed. No SpecReview spec sub-analysis was applicable because no spec or plan file changed.

## Findings

| ID      | Severity | Summary         | Refs |
| ------- | -------- | --------------- | ---- |
| FND-001 | low      | No issues found | -    |
