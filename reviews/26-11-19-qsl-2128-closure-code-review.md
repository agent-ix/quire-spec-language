---
id: SR-2026111921281
title: "Hyper product closure rejection order review"
type: SpecReview
analysis: code-review
scope: "FR-163; fsl-replay/src/certificate/product.rs; fsl-analyze/tests/it/closure_certificate.rs; changes after e115a63860 only"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
---

## Summary

The checker now completes successor membership before reporting undefined letters
or a rejecting projected monitor. Component checks precede the form-specific
violation verdict. The change follows FR-163's existing rule order and adds no
public API, dependency, evaluator, or certificate format.

The Rust review checked the real product/evaluator and settlement calls,
cancellation and state/transition bounds, checked limit conversions, and rejection
loci. Tests construct evidence through the actual engine or projected product;
they do not replace the checker with a mock. No workflow or gate changed.

## Verdict

**PASS for the scoped correction.** This review does not assess the whole partial PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved: a rejecting initial projected member previously hid a missing successor. Closure now precedes violation and a regression exercises both failures together. | FR-163 Behavior: Order; fsl-replay/src/certificate/product.rs |

## Coverage

Only the two Rust files above were reviewed. Formatting and whitespace checks
are scoped to those files and passed. The six `closure_certificate::hyper_product`
tests passed through `qsl-slot` with `CARGO_BUILD_JOBS=5` and this worktree's
target. The `hyper_wire::` filter passed seven of eight tests: the existing
package-refusal regression fails while compiling its handoff fixture, before
executing a hyper item, with `ClauseRequest { member: "handoff" }` because
`package.nodes` differs from the checked unit. This correction changes no
compiler, emitter, handoff admission, or package fixture code. No aggregate
gate was run, as directed by the owner.
