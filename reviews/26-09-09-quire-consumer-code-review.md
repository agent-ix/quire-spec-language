---
id: SR-213
title: "Code and Rust review of the actual Quire consumer"
type: SpecReview
analysis: code-review
scope: "src/quire_source.rs; src/source.rs; tests/quire_source.rs; optional dependency and workflow"
review_set: subset
---
## Summary

Author PR-readiness review of bf8c170 using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining code/Rust findings in this consumer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

The consumer calls actual Quire extraction and reuses Source's line index,
SourceMap verification and mapped compilation. Errors preserve typed causes,
source and completed extraction; success does not rewrite upstream availability.
Checked arithmetic and bounded original input precede offset use. There is no
new parser, unsafe block, request panic, asynchronous work or filesystem effect.
The optional pinned producer has no default features; all 180 resolved package
grants are inventoried. The fixture-only dead-code allowance is explained.

Review corrected a byte/scalar column mismatch using producer FR-071; the final
LF/CRLF tests accept a Unicode-space closing fence and preserve body EOF mapping.
Quire recognizes the final trace attributes: FR-030 5/5, FR-011 4/4, TM-007 14/14.
Actual runtime truth/refusal, unsupported syntax and stale selection are tested.

The full suite passed 300 tests plus three compile-fail doctests (four existing
assurance tests ignored). After the coordinate correction, all four affected
integration tests, strict all-targets/all-features Clippy and formatting passed
again. Warnings-denied rustdoc, cached minimal build and both Rust fixture audits
passed before that local correction. No hosted CI ran; its trigger remains manual.

