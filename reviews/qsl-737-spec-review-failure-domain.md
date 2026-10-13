---
id: SR-7880
title: "QSL-737 PR 800 failure-domain analysis: FR-359"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@67de856995d7f102f050fad3275c167e4f66b26d; spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md"
review_set: subset
---

## Summary

Ticket: QSL-737. This analysis looks for failure modes, identity confusion
and topological edge cases that FR-359 leaves unstated: equality depth,
redefinition and multi-level ancestry.

## Verdict

CONDITIONAL. These cases are well stated:

- identity confusion on field reads (item 2);
- reverse and unrelated refusals (items 6 and 7, AC-6, AC-8, AC-9);
- the checked-invariant causes that a conforming value must never reach (AC-10).

Three gaps remain. The equality rule's scope is narrower than the places
where a child can sit. Scalar redefinition is ambiguous. No AC exercises
depth-2 ancestry.

## Examined Scope

```yaml
scope:
  - {id: FR-359, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-359-AC-2, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-359-AC-4, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-359-AC-5, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: QSpec-FR-151, path: quire-specification spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md, role: context_only}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Item 4 states equality only for "two operands whose static type is the record type `P`" and for their record fields ("each field under this same rule"). Item 5 lets a child sit as an `Option<P>` payload, a collection element, and inside tuples and unions. Nothing states that `==` on two `Option<Money>`, `Sequence<Money>`, `Set<Money>`, tuple or union values compares nested records by the same identity-first rule, though kernel structural equality recurses into all of them. No AC covers `==` over an `Option<Money>` or `Sequence<Money>` pair that holds a `TaxedMoney` and a `Money`. | FR-359 item 4; FR-359 item 5; FR-359-AC-4; FR-359-AC-5 |
| FND-002 | low | AC-2 covers a redefinition "with a narrower type" and requires that "the result has the type check gave it". Item 1 rules out conversion only for record identity. Under QSpec FR-151 a value-type field may also be redefined by a narrower scalar, for example `Int[0, 9]` for `Integer`. The AC does not say whether that case is in scope, or whether a slot holding an `Int[0, 9]` value is returned unchanged or widened to the parent-typed result. Two implementers would read this differently. | FR-359-AC-2; FR-359 item 2 |
| FND-003 | low | Every AC uses one level of specialization (`TaxedMoney` under `Money`, `ExpressOrderPlaced` under `OrderPlaced`). The key-order misread that item 2 guards against, the identity-first equality and replay's ancestor refusal all have depth-2 variants: a grandchild at a grandparent slot, and an intermediate type at a grandchild slot. No AC exercises them. | FR-359-AC-1; FR-359-AC-4; FR-359-AC-9 |
