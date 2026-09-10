---
id: SR-203
title: "Code and Rust review of standalone projection export"
type: SpecReview
analysis: code-review
scope: "src/command; src/command.rs; src/cli.rs; src/main.rs; src/lowering.rs; src/diagnostic.rs; tests/lower_command.rs; shared fixture generator"
review_set: subset
---
## Summary

Author PR-readiness review of d1fcf16 using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining code/Rust findings in standalone projection export.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Compile and lower share source-only intake/package construction through the
existing export closure. The typed Command parser admits lower and retains one
output/error dispatcher. LoweringCode maps exhaustively to catalogued native
codes; all existing wire spellings are retained.

Lowering errors now hold boxed source metadata and coordinates resolved before
the program is released. The old FormalSource clone shared an Arc rather than
copying text; the correction removes retention of that text as well. Absent and
invalid spans are distinct in the typed API and serialized span_status, with
unmapped_span retained for invalid coordinates. Boxing metadata resolves the
strict result_large_err finding without suppressing it. No new unsafe code,
parser, dependency, unchecked cast or recoverable-input panic was introduced.

Four binary tests exercise exact output through both pinned IR readers and
actual codegen/syn output, later-clause refusal, fresh retry, shared intake and
command-specific arity. Two unit tests cover coordinate resolution and controlled
adapter serialization/classification; these are not claimed as end-to-end
production failures. Shared fixture variants name flag/frame payloads and adverse
request cases use exhaustive enum dispatch.

At b910d64, 311 tests plus three compile-fail doctests passed, with four existing
assurance tests ignored. After boxing metadata at d1fcf16, all 21 library/lower
tests, strict all-targets/all-features Clippy, formatting, cached minimal build
and warnings-denied rustdoc passed. Cargo phases ran serially with one build job.
No hosted CI ran. This author review does not claim independent acceptance,
new generated execution or activation qualification.
