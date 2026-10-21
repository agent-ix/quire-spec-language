---
id: SR-12282
title: 'Spec review of PR #1165: FR-396 and TC-1090, spec-review'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@efa450952; spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md, spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md, spec/spec.md; ticket QSL-937'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1090
  type: reviews
---

## Summary

Ticket: QSL-937. Reviewed the new FR-396 and TC-1090 and the spec.md index rows. Checked against FR-394 (same facade shape), FR-092-AC-12 and the code at efa450952.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396 Description | spec/functional/FR-396-….md | examined |
| FR-396 Behavior (refusal order) | spec/functional/FR-396-….md | examined |
| FR-396-AC-1..AC-4 | spec/functional/FR-396-….md | examined |
| TC-1090 procedure and expected results | spec/test-cases/TC-1090-….md | examined |
| spec.md contains edge and index row | spec/spec.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-396 Description says the kernel `CompositeDeclaration` and `ValueType::Composite` "both take the key of the declaration's `composite_type` node (FR-092-AC-12)". QSL's own checker contradicts this: `assemble.rs` builds `CompositeDeclaration::new(handle, …)` and every `ValueType::Composite(handle)` from `declared_type_handle`, and replay consumes those handles. The requirement fixes the wrong identity. It should name the key QSL's checked `TypeEnvironment` and replay results carry for the declaration, looked up by the `composite_type` node's wire id. | spec/functional/FR-396-….md:22-30 |
| FND-002 | high | FR-396-AC-1 and AC-2 cannot fail for a wrong key. AC-1 only requires that a self-built environment closes. AC-2's oracle is "the record and tuple checked type nodes of the spine's own checked graph", which is the same data the facade copies. Add an AC that the admitted key equals the key of the same declaration in the producer's checked `TypeEnvironment`, and the `Composite.declaration` of a replay result or witness of that type. | spec/functional/FR-396-….md:68-69 |
| FND-003 | low | AC-1 says "names only the `qsl_replay` root, the kernel and the QSV declaration API", but `StageLimits` needs `qsl_foundation::Setting`. The same wording gap exists in FR-394. | spec/functional/FR-396-….md:68 |

## Verdict

The structure, relationships, index entry, EARS-style Behavior and the refusal order (own, Foreign, WrongKind, Unknown, with the enum lookups never returning WrongKind) are clear and testable. FND-001 and FND-002 must be fixed with the code: the spec currently encodes the wrong identity and a circular oracle.
