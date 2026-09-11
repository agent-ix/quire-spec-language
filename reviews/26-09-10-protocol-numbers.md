---
id: SR-316
title: "Code and Rust review of exact protocol numbers"
type: SpecReview
analysis: code-review
scope: "FR-038; TC-117; src/protocol_artifact/; tests/protocol_number.rs; export and indexing"
review_set: subset
---
## Summary

PASS. The code-review, rust-review, rust-style and implementation-gap discovery
checks found no defect in this numeric slice. Compiler #40 remains open for the
enclosing artifact and source producer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings. | src/protocol_artifact/number.rs; tests/protocol_number.rs |

## Code and test assessment

Canonical ASCII spelling precedes checked i64 parsing. Positive denominators,
unsigned_abs and Euclidean gcd safely enforce coprime components even at i64::MIN;
private fields prevent invalid rational construction. No float, cast, repair,
panic-prone indexing, unsafe code, I/O, shared state or test-only bypass appears.

The existing object-only Serde adapter and deny_unknown_fields enforce closed
shapes. NumberWire deliberately lacks an encoder and exposes typed NumberError
conversion; convenience Deserialize retains Serde errors without requiring callers
to classify message text. Tags preserve integer 1 versus rational 1/1.

| Acceptance | Inspected public-API controls |
| --- | --- |
| FR-038-AC-1 | Fixed objects for zero, safe endpoints, adjacent wider integers and signed extrema |
| FR-038-AC-2 | Independent rational objects/extrema and exact kind/component assertions |
| FR-038-AC-3 | Typed spelling, overflow, denominator and reduction failures; raw tokens in every component |
| FR-038-AC-4 | Missing/extra/duplicate/cross-kind members, escaped duplicate tag and arrays refuse; reordered valid object succeeds |
| FR-038-AC-5 | Valid wire value outside an illustrative model bound; no source normalization or model-admitted artifact claimed |

All nine tests use real public APIs, unconditional assertions and existing trace
tags. Fixed expectations supplement round trips; a compile-fail doc protects
private fields. The facade exports implemented logic. Public docs and license
notices are present; dependencies, workflows and warning policy are unchanged.
No deny.toml or repo-specific Rust-style override was found. Reverse discovery
maps the added guards to FR-038; enclosing budgets/canonicalization remain #40.

## Local gates

The coordinating agent ran formatting, strict all-targets Clippy and the complete
standard test suites in both minimal and all-feature configurations: all passed.
The four existing gated/ignored tests remain ignored. The numeric tests and
private-field compile-fail example pass in both configurations. Quire validation
and the PR diff check pass. Runs were serial, offline and single-job; this
reviewer ran no Cargo command. Complete artifact/consumer integration remains #40.
