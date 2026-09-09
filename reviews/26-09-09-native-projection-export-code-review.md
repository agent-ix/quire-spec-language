---
id: SR-203
title: "Code and Rust review of standalone projection export"
type: SpecReview
analysis: code-review
scope: "src/command.rs; src/command/output.rs; src/main.rs; tests/lower_command.rs; shared fixture generator"
review_set: subset
---
## Summary

Author PR-readiness review of 4f2c20f using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining code/Rust findings in standalone projection export.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Compile and lower share one source-only intake/package path. Lower invokes the
existing complete projection and strict binder; output uses the existing bounded
bytes and stdout writer. Original typed lowering errors retain native authority,
located spans and upstream diagnostics. Enum matching is exhaustive. No new
parser, unsafe code, recoverable-input panic, unchecked conversion or dependency
was introduced. The small fixture dead-code allowance explains why individual
test targets use different generator cases.

Three new traced binary tests pass. They exercise both real pinned IR readers
and actual complete codegen output, and refuse a later unsupported clause without
partial stdout. Source-only export, request/refusal identity and fresh retries
are checked. Final local gates pass: 296 tests plus three compile-fail doctests
(four existing assurance tests ignored), strict all-targets/all-features Clippy,
formatting, warnings-denied rustdoc, cached minimal build and Rust fixture audits.
The generated Boolean example runs and exports successfully. No hosted CI ran.
This review does not claim new generated execution or activation qualification.
