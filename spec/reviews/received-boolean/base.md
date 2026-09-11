---
id: SR-360
title: "Base review of received-Boolean choice requirements"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; tests/native_protocol_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

The received-Boolean increment makes the admission boundary materially clearer:
it names atom identity, causal availability, the guard subset, conservative
valuation semantics, the distinction between FamilyProof and Invalid(Control),
and all-feasible-branch progress. Capture wording preserves provenance and scope
authority; it does not promise a positive same-run capture scenario, which the
language cannot form.

## Verdict

**PASS (scoped)** — no new base-requirement contradiction was found in the
received-Boolean increment. Broader corpus AC-9 evidence debt is recorded but
is not a new Boolean acceptance condition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No newly introduced base-requirement defect found: protocol captures predate the run and compensation captures are not in choice scope, so let-only Boolean aliasing omits no admissible received atom. | FR-042:95-105; compiled-protocol-v1.md:361-385; TC-121:96-98 |
| FND-002 | low | Full AC-9 per-dimension zero/exact/one-short vectors remain wider corpus debt; the focused increment's required twelve-atom References/locus/retry case is explicit. | FR-042-AC-9; TC-121:174-176; native_choice_emission.rs:1035-1110 |

### Requirement coherence

The core Boolean rule is otherwise coherent. Atom identity is tied to receive
binder, selected field/export and original anchor; the receiver and source scope
are separate prerequisites. Visibility does not widen into composite knowledge,
and the guard grammar deliberately rejects unsupported operands even when
short-circuit evaluation would skip them. The abstraction explicitly avoids
turning a failed symbolic proof into a fabricated runtime counterexample.

The related historical native-protocol fixture now treats nonliteral operands in
otherwise constant-looking guards as FamilyProof rather than as closed. This is
an intentional tightening under FR-042's rule that every original operand and
immutable initializer must be closed; its fully literal operation/branch shape
remains the positive control.

### Scope boundary

Protocol captures are evaluated at activation before the protocol run's receive,
and compensation captures are locally scoped. Accordingly, a positive captured
received-Boolean fixture is not an admissible source scenario. The current
requirement prevents a capture from manufacturing availability; no Boolean
capture capability is required.

The general AC-9 matrix should eventually identify source shapes that isolate
each counter and state expected limit, dimension, locus, usage and requested
work at zero, exact and one-short boundaries. That is not a newly imposed
precondition on this Boolean increment, whose bounded valuation scenario is
already specified.
