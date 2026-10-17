---
id: SR-11759
title: "QSL-502 quire-spec-language PR 914 gap analysis: FR-317, FR-319-AC-2/3 trace and AC coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c763259750f0863aa35df16792044fbcd6276ac9; qsl-semantics/src/check/sum_case/tests.rs, qsl-replay/src/spine/dependency_tests.rs, qsl-semantics/src/check/assemble/tests.rs; spec FR-317, FR-319, TC-819, TC-820"
review_set: subset
---

## Summary

Ticket: QSL-502 (remainder). Planless gap analysis of the PR's tagged tests
against the ACs they claim. Done by reading each test, not through
`quoin matrix` (brief: no builds). Plan completion: not assessed.

Bindings checked:

- FR-317-AC-1 / TC-819: correct for `Shape::Empty`, `Shape::Rect(2, 3)` and
  `Shape::Rect(2, true)` at `true`. The QSpec TC-262 clause is not covered
  (FND-003).
- FR-317-AC-2 / TC-819: correct. All four shapes refuse at the construction,
  one refusal each.
- FR-317-AC-3 / TC-820: correct for steps 1, 2 and 4-7 (the import-alias
  vector in qsl-replay; the hand-built union+union, union+enum and
  one-declaring-union scopes; enum member and tuple constructor). Step 3 has
  no test (FND-001).
- FR-317-AC-4 / TC-819: the measure half is correct (`ClauseKind::Body` is
  the measure kind). The state-clause half is weak (FND-002).
- FR-319-AC-2 / TC-826 (checker half): correct. The member key is checked
  against an independent SHA-256 of the FR-441 preimage text, `VariantId`
  equals the key bytes, and A/B/E give three ids that select only their own
  union.
- FR-319-AC-3 / TC-826 (checker half): correct for typing, but only over a
  test-sealed scope whose final key equals the handle. Production `check()`
  does not seal unions yet (QSL-504, known; not reported).

## Verdict

PASS for this method. No high finding. Three medium coverage gaps are
recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-317-AC-3 / TC-820 step 3 has no test. No test checks that a unit with two unions named `q`, or a union and an enum named `q`, is refused by the assembler at the second declaration as `ambiguous_declaration`/`ambiguous-name`. The only duplicate-type-name test covers a record and a type alias. The AC's argument that source cannot reach the resolver with two types named `q` depends on this unchecked refusal. | qsl-semantics/src/check/assemble/tests.rs:1065-1088 |
| FND-002 | medium | FR-317-AC-4's state-clause half is tagged on `a_construction_checks_the_same_under_every_clause_kind`. That test passes `Invariant`, `Precondition` and `Postcondition` to the typer over a function body. It never checks a construction inside a real model invariant or an operation pre/postcondition. This is the same harness pattern SR-11177 FND-001 flagged for FR-318-AC-5. | qsl-semantics/src/check/sum_case/tests.rs:625-668 |
| FND-003 | medium | FR-317-AC-1 requires each of QSpec TC-262's construction cases to give its verdict. TC-819 step 4 says to run them tagged `QSpec-TC-262`. No test carries that tag. Over-arity (`Circle(3, 4)`) and the `Empty2::E` / `Empty2::E(1)` single-member vector are never run. Only under-arity `Rect(1)` is tested. | qsl-semantics/src/check/sum_case/tests.rs:542-597 |
