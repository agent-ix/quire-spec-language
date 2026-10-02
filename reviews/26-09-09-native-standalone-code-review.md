---
id: SR-173
title: "Code and Rust review of standalone native execution"
type: SpecReview
analysis: code-review
scope: "src/command.rs; src/command; src/main.rs; tests/standalone.rs; Rust example setup"
review_set: subset
---
## Summary

Author PR-readiness re-review of d7437e2 with actual code-review, rust-review and
portable rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**CONDITIONAL** — the four high findings are corrected. The defensive
evaluation-refusal qualification below remains later assurance work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved 1: typed status/stage Serialize views, an immutable result wrapper and an independently authored envelope schema replace JSON key mutation. | src/command/output.rs; src/command/output/types.rs; schemas/native-run-result-1.schema.json |
| FND-002 | low | Resolved 2/3/7: all command codes are catalogued, LimitKind separates intake ceilings, and cause/code classification is shared with PackageError. Public RunCause and Code are non-exhaustive. | src/command.rs; src/diagnostic.rs; src/package.rs |
| FND-003 | low | Resolved 4/5: one typed argv parser precedes I/O; absolute and parent-relative operands are specified and tested through the real command. | src/cli.rs; src/main.rs; tests/cli.rs; tests/standalone.rs |
| FND-004 | medium | EvaluationOutcome::Refused still lacks an end-to-end evaluator-to-command case. Validation refusal is covered and is not a substitute; retain this in the later assurance campaign. | src/command/output.rs; FR-026-AC-3; independent finding 8 |

Finding 6 disposition: command remains the existing public I/O module tree, with
argv/streams in the binary and typed output construction isolated from native
semantics. No new crate is justified by an independent consumer. RunResult.value
is now an immutable NativeResult; as_value() exposes a borrowed view. RunError.value()
is fallible so serialization errors propagate instead of panicking.

Full tests passed at 1971d36: 294 tests and three compile-fail doctests, four
existing ignored. After the package accessor, stream assertion and immutable
wrapper, all 14 focused command/CLI/catalog tests, strict all-targets/all-features
Clippy, formatting and warnings-denied rustdoc passed. The cached minimal build
passed before those final additions. No dependency or hosted workflow changed.
Earlier baseline evidence below is historical; it does not qualify new behavior.

## Checks

Reviewed closed Serde records/variants, explicit profile selection, single-open
bounded reads, exact source/runtime digest verification, authored formal/clause
constructors and reused native compilation/execution. Checked count arithmetic
and inclusive byte reads do not cross a wire with unchecked casts. Output uses
actual native diagnostics, references, counters and event order; only completed
execution emits truth. No production panic, unsafe block, hidden test bypass,
shared request state, new dependency or producer process was added. Example
setup uses existing public APIs and explicitly selected local output files.

Strict all-targets/all-features Clippy, formatting, 287 tests plus three compile-
fail doctests, warnings-denied rustdoc, cached minimal build and both fixture
audits passed. Four existing assurance tests remain ignored. All four generated
examples also ran from /tmp with exits 0/1/0/1 and the expected true/false/true/
frame-refused outcomes. Quire validation and scoped trace coverage passed.
CI remains workflow_dispatch-only; no hosted workflow ran. This is author
review, not independent assurance.
