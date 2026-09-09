---
id: SR-183
title: "Code and Rust review of standalone compiler export"
type: SpecReview
analysis: code-review
scope: "src/command.rs; src/command/wire.rs; src/main.rs; tests/compile_command.rs; shared fixture setup"
review_set: subset
---
## Summary

Author PR-readiness review of d1de2a4 using actual code-review, rust-review and
portable rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining code/Rust findings in compiler export.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Reviewed closed source-only decoding, profile-before-body selection, shared
file/count/byte bounds and exact source checks. Both commands use the same model
and static compiler helpers. The returned byte copy intentionally outlives local
models; stdout uses write_all without reserialization or newline. Original
typed errors retain request identity, and static failure writes no package.
No new panic, unsafe block, unchecked numeric conversion, dependency or runtime
input construction was introduced. Actual trace tags bind the binary tests.

The full suite passed 289 tests and three compile-fail doctests, with four
existing assurance tests ignored. After replacing duplicate fixture imports
with a shared module, strict all-targets/all-features Clippy and all nine
compile/run/parse-format tests passed. Formatting, warnings-denied rustdoc,
cached minimal build, both fixture audits and generated fixture execution passed.
The standalone exported artifact has digest
sha256:bf2ce1db5d398f18169742bac213e89d201c2f91918ff5b610be6fd262599071.
No hosted CI ran; dispatch-only policy remains. This is author review.
