---
id: SR-243
title: "Code and Rust review of bounded integer IR lowering"
type: SpecReview
analysis: code-review
scope: "src/lowering/; src/lowering.rs; src/cli.rs; src/command/; src/main.rs; integer tests and examples"
review_set: subset
---
## Summary

Author PR-readiness review of b0c02c7 using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining scoped code/Rust findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

One exhaustive BinaryOp conversion owns operator semantics. Its typed result
supplies both target admission and construction; no fallback maps a new operator
to an existing one. One catalog defines the non-exhaustive public target enum,
published names/list and parsing/display. The shared CLI parser owns the optional
lower suffix and reports the named command's usage before dependent I/O.
Typed command failures retain the selected target and metadata-only source context.

The wire boundary constructs fallible typed borrowed views of the already bounded
IR tree before writing bytes. Unadmitted declarations or nested forms retain
clause and native source; foreign coordinates become InvalidCorrespondence.
These views preserve the existing flattened integer contract, without changing
native semantics or adding a parser. Both pinned IR readers accept actual output.
Codegen continues to return its explicit unsupported-expression result for numeric
programs; that is a tested boundary, not numeric backend qualification.

Ten new tests cover exact operators/bounds, source/observation identity, both
readers, atomic limits, actual command output, target spellings/encoding, and
generator I/O failure. The private wire-conversion control is not claimed as a
publicly constructible invalid checked package. Sixty focused tests pass.
Shared generators retain named Boolean/operation payloads from the parent and
propagate selected-path filesystem failures. No dependency, unsafe code,
request panic, unchecked numeric cast or workflow change was introduced.

At b0c02c7, full local suites pass: 342 ordinary tests plus three compile-fail
doctests with all features; 326 plus three with minimal features. Four existing
assurance tests remain ignored in each. Strict all-targets Clippy passes in both
configurations; formatting, cached minimal binary/example build and
warnings-denied all-feature rustdoc pass. Scoped Quire validation passes.
The actual generator's success and error exits were checked locally; integer
healthy/violating runtime cases retain identical source-only projection bytes.
Hosted CI remains manual-dispatch only and was not run.
