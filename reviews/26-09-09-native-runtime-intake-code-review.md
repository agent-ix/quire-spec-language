---
id: SR-153
title: "Code and Rust review of native runtime intake"
type: SpecReview
analysis: code-review
scope: "src/runtime/reading.rs; src/runtime/input.rs; src/serde_object.rs; package adapter reuse; tests/runtime_reading.rs"
review_set: subset
---
## Summary

Author review of `67c68da` using actual code-review, rust-review and rust-style
skills. No applicable AssuranceProfile was found. Public integration tests use
the repository's existing fixture convention and real trace attributes.

## Verdict

**PASS** — no remaining code/Rust findings in the delivered reader scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

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
