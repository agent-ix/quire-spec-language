---
id: SR-253
title: "Code and Rust review of validated state-scalar projection"
type: SpecReview
analysis: code-review
scope: "src/lowering.rs; src/lowering/inputs.rs; src/main.rs; tests/state_scalar_lowering.rs"
review_set: subset
---
## Summary

PASS for revision `401edc4c378701e3e108303df334be4629cb8cb0`, against PR25
base `2481878`. Applied the actual agent-skills/code-review, rust-review and
rust-style skills with repository conventions. No applicable AssuranceProfile
or deny.toml exists. This is the author's PR gate, not independent review.

Inspected native/IR observation preservation, field alias allocation, exact
checked-context matching, typed state/parameter origins, bounded traversal,
borrowed provenance and atomic failures. Alias candidates are bounded by the
admitted model values plus projected fields; no receiver/graph evaluation is
duplicated. The new module has no I/O, unsafe, mutable shared state or unchecked
wire conversion. Public inputs retain the native context and actual bound
projection. Native validation remains authoritative for closure and frames.

Test assertions distinguish pre/post values and source coordinates, false
Boolean fields, another checked package, state versus invocation captures,
unsupported nonprimitive receivers, and stopped/retried requests. The canonical
`#[trace]` attributes resolve. No mocks, new ignored tests, dependency changes
or workflow changes were introduced. Reverse discovery found no unowned behavior
or implementation/test stub in this change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No blocking code/Rust finding remains in the scoped change. Full backend/activation qualification remains separate. | FR-034; TC-112; Task-020 |

## Validation

All Cargo phases ran serially with nice 10, locked/offline inputs, one job and
one test thread. The all-feature suite passed 320 ordinary tests and three
compile-fail doctests; four existing assurance tests remained ignored. After
the final additional opaque-reference refusal case, all 20 focused minimal
tests passed (`integer_lowering`, `lower_command`, `native_lowering`,
`state_scalar_lowering`). The latter reported:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```

`cargo fmt --all -- --check`, strict all-feature/all-target Clippy, minimal build,
all-feature rustdoc and `git diff --check` exited 0. A fresh Rust-generated
ConfigVersion directory also ran unchanged/changed/forbidden-frame requests
with exits 0/1/1; the actual command exported the bounded pre/post IR. No hosted
workflow was dispatched. Commands and examples are in README and TC-112.
