---
id: SR-12298
title: "Spec review of FR-398 and TC-1092 (QSL-947, PR #1174)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7f746bf97; spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md; spec/test-cases/TC-1092-the-replay-facade-returns-the-admitted-unit-table.md; spec/spec.md (index rows)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-398
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1092
    type: reviews
---
# SR-12298: Spec review of FR-398 and TC-1092 (QSL-947, PR #1174)

## Summary

Ticket: QSL-947. I checked FR-398's statement, Outputs, Behavior and AC-1 to AC-4, and TC-1092's procedure and expected results, against the implementation at 7f746bf97 and the probes in SR-12296.

Clean:

- The ACs can fail, each AC names its fixture, and AC-1 states the root-only boundary.
- AC-2 names an oracle that does not depend on the facade: the checked signature and the semantic nodes.
- The kernel-id versus node-id distinction (`UnitId::declared` over the same bytes, a different type) is stated plainly.
- The trap that units are listed even though unused ones are absent from the emitted package is stated plainly in Behavior.
- TC-1092 steps 1 to 3 match AC-1 to AC-3 one for one, and the spec.md index rows were added.

## Verdict

No major findings. One medium: FR-398's refusal bullet contradicts its own Outputs and the code for unused declarations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Behavior refusal-order bullet lists "a declared unit or dimension the package does not use" among the ids that are `WrongKind`. That contradicts the Outputs bullet and the prior Behavior bullet ("the context lists every declared unit, used or not ... a consumer ... finds it"). It also contradicts the code: `unit_declaration` finds an unused unit (measured). A declared dimension that no unit names is not in `dimensions` and, by trace, not in `nodes`, so it refuses `Unknown`, not `WrongKind`. Two implementers would read this bullet differently. Restate it: an unused unit is found by `unit_declaration` and is `WrongKind` for every other kind's lookup, and a declared dimension that no admitted unit names is `Unknown`. Or list such dimensions too. | spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md:86-93 |
| FND-002 | low | "The lookups of FR-394 and FR-396 keep their refusals, except that a unit or dimension id looked up as a record or tuple is `WrongKind`, as every other node of the package is" reads as if every lookup now refuses foreign-kind nodes as `WrongKind`. FR-394's enum lookup still refuses a unit id `Unknown` (measured). Say so explicitly, so a consumer does not expect `WrongKind` from `enum_declaration`. | spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md:94-98 |
| FND-003 | low | FR-398 does not say that a package cannot name a dependency's unit in source. `Quantity<d.kg>` is a parse error, and `Quantity<d::kg>` is `UnresolvedTypeName`. So `Foreign` is reachable only for an id read from a dependency's own emitted package. One sentence would stop CG from assuming that cross-package quantity operands exist. | spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md:86-93 |
