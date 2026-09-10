---
id: SR-253
title: "Code and Rust review of validated state-scalar projection"
type: SpecReview
analysis: code-review
scope: "src/lowering.rs; src/lowering/inputs.rs; src/lowering/target.rs; tests/state_scalar_lowering.rs"
review_set: subset
---
## Summary

PASS for revision b789eed against corrected PR25 base 53dc9ef. Applied the actual
agent-skills/code-review, rust-review and rust-style skills with repository
conventions. No applicable AssuranceProfile or deny.toml exists. This is the
author's PR gate; independent review remains pending.

The state-scalar target uses the shared target catalog and typed CLI parser.
Public target, read-origin and input-stop enums are non-exhaustive. State versus
captured Input selection is exhaustive; the parent supplies exhaustive binary
operator conversion and pre-serialization wire admission. No alternate parser,
model authority or evaluator is introduced.

Inspected native/IR observations, exact checked-context matching, field alias
allocation, typed origins, borrowed provenance and atomic failures. Alias search
is bounded by admitted model values and projected fields. The materializer has
no I/O, unsafe code, mutable shared state or unchecked wire conversion. Native
validation remains authoritative for closure and operation frames.

Tests require actual pre/post values and coordinates, false Boolean fields,
a different checked package, state/invocation captures, unsupported receivers
and stopped/fresh calls. Alias/direct-input checks now require expected counts
and names before accepting filtered observations, so empty subsets cannot pass.
The fixture catalog remains one shared list without unused-import allowances.
Actual trace attributes resolve. No new ignored test, dependency or workflow
was introduced; no scoped stub or unowned behavior was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No blocking scoped code/Rust finding remains. Full backend/activation qualification is separate. | FR-034; TC-112; Task-020 |

## Validation

At b789eed, the full all-feature suite passed 347 ordinary tests plus three
compile-fail doctests; minimal features passed 331 plus three. Four existing
assurance tests remain ignored in each. Both strict all-targets Clippy lanes,
fmt, cached minimal binary/example build, warnings-denied all-feature rustdoc
and scoped Quire validation pass. All Cargo phases ran serially with nice 10,
locked/offline inputs, one job and one test thread.

Fresh Rust-generated ConfigVersion files also return true/false/frame refusal
and export the actual pre/post IR through the shared CLI. These observations
do not establish numeric codegen or activation qualification. No hosted
workflow ran; workflow_dispatch remains the only hosted trigger.
