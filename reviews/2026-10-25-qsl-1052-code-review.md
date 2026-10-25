---
id: SR-60014
title: "Code review of QSL-1052 stale-object attribute faults"
type: SpecReview
analysis: code-review
scope: "qsl-eval/tests/it/inherited_attributes.rs; FR-090-AC-17/22 and TC-917; existing evaluator attribute producer in qsl-eval/src/value/expression/evaluate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: references
---

## Summary

The public `CheckedPackage::evaluate` test confirms a valid current object can hold a tolerated dangling reference to an object removed from the current state, and an attribute read through that reference surfaces the existing typed internal fault. No production behavior change was needed; the previously private producer probe now has end-to-end evidence.

## Verdict

**PASS** — the traced regression exercises the real public evaluation path, exact fault identity, and fault category; all scoped gates pass.

## Assurance Context

No applicable `AssuranceProfile` exists under `spec/` for this test and evaluator path.

## Coverage

- The test constructs a prior state containing `root` and `removed`, then a current state containing `root` while retaining its allowed dangling `peer` reference to `removed`.
- A nested `deref(r).peer` attribute read reaches `Machine::apply`'s production attribute lookup with a concrete reference absent from the current object closure.
- The test requires `CallFailure::Fault` with stage `S6a`, invariant `attribute-slot-unresolved`, and `Category::InternalFailure`; any `Ok`, input refusal, panic, or other fault fails.
- The evaluator's existing `references_are_unresolved` replay behavior remains distinct; this test uses a concrete runtime state.

## Checks

- `cargo fmt --all -- --check`: pass.
- `cargo clippy -p qsl-eval --all-targets --all-features -- -D warnings`: pass.
- `cargo test -p qsl-eval --test it inherited_attributes::`: 5 passed.
- `cargo test -p qsl-eval --all-features`: 737 passed; 13 doctests passed.
- `make cargo-deny-bans`: pass.
- Temporary scoped `quire matrix`: FR-090-AC-17 and FR-090-AC-22 are `tagged` by the new test.
- No workflow or CI files changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No code-review finding in the scoped QSL-1052 change (placeholder). | qsl-eval/tests/it/inherited_attributes.rs |
