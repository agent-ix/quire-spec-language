---
id: SR-223
title: "Code and Rust review of standalone Markdown execution"
type: SpecReview
analysis: code-review
scope: "src/command.rs; src/command/; tests/extracted_command.rs; shared fixture generator"
review_set: subset
---
## Summary

Author PR-readiness review of 0d3d294 using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**PASS** — no remaining code/Rust findings in this command mode.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Three typed mode refusals have separate catalogued codes and pre-I/O tests.
One binding guard retains the selected binding. A shared closed wire identity
binds original and body selections to the same native/formal identity type;
malformed types, unknown fields and duplicate keys are exercised for both uses.
Quire context construction uses the consumer's version constants, and rejected
context output carries those versions with actual producer diagnostics.

The private compilation module owns preparation and feature selection; one
runtime-input helper serves snapshots and invocations. Public error enums are
non-exhaustive. Necessary feature-gated type declarations remain; there is no
runtime trait object or test-only bypass. Typed extraction output is constructed
before serialization and immutable result exposure. Schema negative controls
remove required provenance/map fields and require rejection. The Quire-owned
embedded records are deliberately not redefined as a second producer contract.

Actual binary tests cover LF/CRLF truth/refusal, original byte maps, unavailable
extraction, stale bytes, source-line ceilings and incomplete/fresh retries.
The context-error adapter control uses real Quire validator diagnostics; it
does not claim a reachable end-to-end context-construction failure. No new
parser, dependency, unsafe block or request panic was found.

Local full suite at 0d3d294: 327 ordinary tests plus three compile-fail doctests
with all features; 312 plus three with minimal features. Four existing assurance
tests remain ignored in both. Strict all-targets Clippy passes in both feature
configurations; formatting, cached minimal build and warnings-denied all-feature
rustdoc pass. The workflow diff against #22 is empty and retains both lanes,
with workflow_dispatch as its only trigger. No hosted workflow ran.
