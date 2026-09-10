---
id: SR-153
title: "Code and Rust review of native runtime intake"
type: SpecReview
analysis: code-review
scope: "src/runtime/reading.rs; src/runtime/input.rs; src/serde_object.rs; package adapter reuse; tests/runtime_reading.rs"
review_set: subset
---
## Summary

Author re-review of `0016103` using actual code-review, rust-review and rust-style
skills. No applicable AssuranceProfile was found. Public integration tests use
the repository's existing fixture convention and real trace attributes.

## Verdict

**CONDITIONAL** — the independent review's two required structural corrections
are implemented and checked. The broader follow-ups below remain open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved independent findings 1/2: distinct typed causes now determine stage/code exhaustively; tests assert variants and payloads. | src/runtime/reading.rs; tests/runtime_reading.rs; FR-024-AC-2 |
| FND-002 | high | Type-owned wire decoders remain follow-up work: per-field adapters still require callers adding fields to select the correct decoder. Existing malformed-field controls pass. | src/runtime/input.rs; independent findings 3/8 |
| FND-003 | medium | Bare-hex digest conversion and a wider wire-format catalog/schema remain follow-ups. The input format is now shared by reader and writer, but that is only partial resolution of finding 6. | src/runtime/input.rs; independent findings 5/6 |

Findings 4/7 are also resolved: an internal constructor owns adoption of external
bytes and InputReadStage owns its stable spelling. The default Serde recursion
guard is documented at the decode site. Cross-cutting error remediation is #27.

On this correction: full suite 278 tests plus three compile-fail doctests passed
(four existing ignored); strict all-targets/all-features Clippy, formatting,
minimal build and rustdoc with warnings denied passed. The focused 48-test
runtime/package regression set also passed. No workflow or dependency changed.
The earlier baseline checks below are retained as historical evidence.

## Checks

Reviewed byte-before-hash/decode limits, immutable selected provenance,
envelope-before-body compatibility checks, exact integer/digest/identifier
decoding, required null results, and existing constructor paths/counters.
Serde owns grammar; package and runtime readers share the existing object-only
adapter. No runtime panic, unsafe code, unchecked numeric cast, I/O or new
dependency version. Raw draft deserialization is not artifact admission.
Reverse-gap discovery found no unowned changed behavior or placeholder.

A real negative test initially exposed Serde accepting an extra field on the
unit Absent variant; the closed empty-payload adapter fixes it and the same test
passes. Existing package code already enforced object records; its helper was
moved to shared private support. After that move, all 18 package-reader and five
runtime-reader tests plus strict all-targets/all-features Clippy passed.
Before that move, the full suite passed 278 tests and three compile-fail doctests,
with four existing ignored tests. Formatting, rustdoc with warnings denied,
minimal build and both fixture audits passed. No deny.toml exists.
The existing raw_value feature now applies to production intake, retaining
serde_json's recorded license. No hosted workflow ran; dispatch-only CI remains.
This is author review, not independent review.
