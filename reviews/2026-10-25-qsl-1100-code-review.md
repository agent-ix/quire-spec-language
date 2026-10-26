---
id: SR-16356
title: "Code review — QSL-1100 CI regressions"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d9735657cfef692c71a43f4c9fdc0505b82f440d..QSL-1100 follow-up; qsl-replay/src/application.rs; qsl-replay/src/spine/lifecycle.rs; xtask/src/string_edge.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: references
---

## Summary

Reviewed the QSL-1100 follow-up for current-main scan regressions from #1483. The fixture is registered as a string-edge scan root, and CheckedUnit no longer exposes a stage-typed returner outside the TC-244 allow-list; no remaining code-review findings were found. The canonical encoding and original string-edge fixes are now on main in #1571.

## Verdict

**PASS** — focused gates, the full xtask suite, qsl-replay tests, formatting, workspace Clippy, and all CI lanes pass. The first `make ci` invocation stopped at no-std because Homebrew Rust lacked the configured target; rerunning the complete no-std lane with the target-installed rustup toolchain passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No actionable findings remain after the fixes | - |

## Coverage

- Code review covered all changed production files and the workspace scan-root registration.
- String-edge behavior was checked against ADR-013 §2 and FR-064-AC-11. The public CheckedUnit package accessor remains unchanged; the internal shared ownership only removes a new stage-returning API.
- No spec changes were needed. The optional repository-wide semantic review was not run.
- Plan completion: not assessed.

## Verification

- Rebased onto current main at #1571 (`d9735657cfef692c71a43f4c9fdc0505b82f440d`); that merge already carries the canonical-encoder and original string-edge fixes.
- After rebasing: `cargo test -p xtask every_workspace_member_is_a_scan_root`, `cargo test -p xtask tc_244_only_the_named_stage_constructors_return_stage_types`, `make string-edge`, `cargo test -p qsl-replay` (877 unit tests, all integration tests, 25 doctests), and `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Before rebasing onto #1571: `cargo fmt --all -- --check`, `cargo test -p xtask` (219 unit + 16 refinement-versioning tests), `make arch-lint-canonical-encoder`, and `make cargo-deny-bans` all passed.
- Before rebasing, full `QSPEC_DIR=/Users/peter/dev/quire-specification make ci` passed validation, tests, lint, and architecture lanes; its first no-std attempt selected `/opt/homebrew/bin/rustc`, which lacks `thumbv7em-none-eabi`. `PATH=/Users/peter/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin:$PATH make build-no-std` passed Clippy, tests, and 41 doctests.
