---
id: SR-9009
title: "Gap analysis — QSL-726 AC-2 operation-body intake (PR #812)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0d399b03502e622104231c200cc2b2b7eb0a4172; QSL-726 QSL-INPUT-AC-2 against FR-102, FR-103 (Outputs, Behavior, AC-9), FR-302 (Behavior, AC-7), TC-986, TC-987; tests qsl-semantics/tests/it/operation_bodies.rs, qsl-forms/tests/it/protocol_clause_forms.rs, qsl-cst/tests/it/complete_grammar.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---

## Summary

Ticket: QSL-726, slice QSL-INPUT-AC-2. The review traced requirements to tests and code for the ticket AC, FR-302-AC-7 and FR-103-AC-9.

- The computed `quire matrix` (quire 0.36.2, engine 0.50.2) binds FR-302-AC-7 to 7 tests in operation_bodies.rs, FR-103-AC-9 to 3, and FR-102-AC-4 to the existing dispatch tests plus the two new forms tests.
- The red log shows all 8 new S3 tests failing on assertions before the change. The green log shows 8/8 passing.

## Verdict

**CONDITIONAL.** AC-2 is met. The tests prove the following:

- `has_body` is true exactly for keys a unit body resolves to, and false otherwise. That covers no body at all, a bodyless operation with a unit clause, and an inherited target keyed to the declaring operation.
- Unit pre/post clauses are retained under the resolved key, with spans into the unit source. The association is invariant under relabelling and reordering.
- The domain-package `requires`/`ensures` text `false` stays the operation's own contract (FR-103 as amended by QSL-726 AC-5) and supplies neither a clause nor a body.

Oracle strength is good. Every test asserts concrete keys, literals and spans. The duplicate test pins the code, the cause, both label spans in source order, and exactly one error.

TC-986 and TC-987 stay 🚧. That is correct: their dispatch-table and evaluation steps belong to AC-3 and AC-4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-103 Behavior says the assembler SHALL bind a body's receiver-less parameter names positionally to the operation's parameters and type the body against the declared result. Nothing implements or tests either, and no AC names them. A body with the wrong arity, such as `size(a, b)` on a parameterless `size`, or with a wrong-typed body assembles with `has_body = true`, so it becomes a dispatch candidate. Deferring this is acceptable for AC-2 only if AC-3 explicitly owns it, before any consumer reads the bodies. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:95-105; qsl-semantics/src/check/assemble.rs:2619-2745 |
| FND-002 | low | The new refusal branches in `operation_bodies` are untested: a target with other than three segments, an undeclared `using` alias, a target naming a variant type, and an ambiguous inherited operation. Only the missing-operation branch has a test. | qsl-semantics/src/check/assemble.rs:2633-2700; qsl-semantics/tests/it/operation_bodies.rs:383-403 |

## Coverage

- QSL-INPUT-AC-2: covered (8 S3 tests, 2 S2 tests, 1 S1 corpus line).
- FR-302-AC-7: assembler half covered. The dispatch-candidate and FR-151 refusal half is for AC-3/AC-4 (TC-986 steps 2-4).
- FR-103-AC-9: covered.
- FR-102-AC-4: covered by the existing `dispatch_entry_is_a_single_thin_call`, which iterates every arm including `Body`. The new forms tests that carry this tag are mis-traced (SR-9008 FND-004).
- Reverse gap: `PackageDeclarations::selected_views` and the rebuilt `package.models` have no production reader yet. They are scaffolding for AC-3 (SR-9008 FND-003).
- Stubs: none.
- Semantic review: performed inline by the reviewer.
- Plan completion: not assessed
