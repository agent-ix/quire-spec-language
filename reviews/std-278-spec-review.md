---
id: SR-15980
title: "Spec review of quire-spec-language PR #1250: STD-278, the FR-366 object-read check follows calls"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3c41e213035f44412c8ddbccb300ed2e028f76be; PR #1250 (git diff origin/main...3c41e2130): spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md, spec/test-cases/TC-1050-a-type-level-clause-is-classified-parsed-and-checked-as-an-invariant.md; read for context: QSL FR-366 main text, TC-1050 step 9, FR-104-AC-1, FR-093-AC-15 (`value(o)` over `Option`); QSpec FR-149 (object reference row), FR-143; agent-ix/quire-specification main 5ee0071f"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1050
    type: reviews
---
# SR-15980: Spec review of quire-spec-language PR #1250: STD-278, the FR-366 object-read check follows calls

## Summary

Ticket: STD-278. Sub-analyses folded in: integrity, EARS, criterion strength (by hand: no Jev client), failure-domain. The head 3c41e2130 is behind main 423ac6aa0 and merges cleanly. `make check-spec-validation` on the merge exits 0. The only FR-366 warning is the existing `ac:non-canonical-shape` on FR-366-AC-4. FR-366-AC-10 is new, and no other open PR touches FR-366 or TC-1050. The rule is the ticket's lead default: transitive over the checked call graph, the same code and cause at `TypeDefinition.clauses[k]`, and a reference that is only passed, tested or compared is admitted. Identity comparison is correctly not a read: QSpec FR-149's object-reference row compares the FR-143 identity triple and inspects no referenced state, so no object universe is needed. The two-level vector catches a checker that follows one call level only, and the `linked` vector catches one that refuses any reference parameter. AC-10's first function body is ill-typed, though (FND-001).

Units examined:

- `FR-366-AC-10` (examined): spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md
- `FR-366` (examined): spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md
- `TC-1050` (examined): spec/test-cases/TC-1050-a-type-level-clause-is-classified-parsed-and-checked-as-an-invariant.md
- `FR-093-AC-15` (context_only): spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md
- `FR-149` (context_only): agent-ix/quire-specification:spec/functional/type-model/FR-149-apply-complete-equality-matrix.md

## Verdict

**NOT MERGEABLE until FND-001 is fixed; two minor findings to fix or accept.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] `total_of(b: Reference<Order>)` has body `value(b).total.amount_minor`, but `value(e)` is the guarded unwrap of an `Option` (FR-093-AC-15 types it over `o: Option<Int[0, 9]>`; step 9 applies it to the optional `buddy`). `b` is not optional, so the body refuses at typing (`ill_typed`) before the object-read check runs, and the one-level and two-level vectors cannot produce the stated `unsupported_construct`/`expression-form` at `TypeDefinition.clauses[0]`. A correct implementation fails AC-10. Write the body as `b.total.amount_minor`, in step 9's read-through-reference form after the unwrap, or type the parameter `Option<Reference<Order>>` and pass `self.buddy`. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:235 |
| FND-002 | medium | [minor] AC-10 does not say where `total_of`, `checked` and `linked` are declared, and FR-366 does not say how a type-level invariant (clause text in the domain package's `TypeDefinition.clauses[k]`) resolves a call. The package has no function declarations, so the callees must come from the compiling QSL unit (or a library). Two implementers could put them in different places, and the invariant's meaning would then depend on the unit. State the fixture (for example, the functions declared `pure` in the K2 unit beside the model declaration), and say in FR-366 that 'the checked call graph' is the compiling unit's. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:108-116,235 |
| FND-003 | low | [minor] Two cases have no vector. The admitted list names `present`, identity comparison and pass-through to a non-reading callee, but AC-10 tests only `present`. And the rule says 'function' while named predicates are also callable Boolean functions (QSpec FR-033). Add `same(a: Reference<Order>, b: Reference<Order>): Boolean` with body `a = b`, and a pass-through `wrap(b) = linked(b)`, both compiling. Say 'function or predicate', or say that predicates are excluded. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:108-116 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | [minor] AC-10 does not state the profile of the unit that declares the `pure` functions, and the profile decides `same`. A function declaration is admitted only under `quire.value.complete/v1` or `quire.model.complete/v1` (LAYER_ROWS `FunctionDeclaration`), or in a unit with no profile. Under value.complete, `same`'s body `a = b` is a reference equality, and #1247's FR-110 rule (AC-10) refuses it naming `quire.state.graph/v1`. State the unit: no header profile, or `quire.model.complete/v1`. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:237 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 153ceca82: `total_of`'s body is `b.total.amount_minor`, a field read through a reference as FR-104-AC-10 types `self.n` over `Reference<Config::Flow>`; it type-checks, so the object-read check is reached. |
| FND-002 | fixed | 153ceca82: the call graph is 'the checked call graph of the compiling unit (the functions and named predicates the unit declares or imports; a domain package declares none)', and AC-10 says the unit declares the `pure` functions. |
| FND-003 | fixed | 153ceca82: AC-10 adds `same` (identity comparison) and `wrap` (pass-through), both compiling, and the named predicate `positive` under `quire.state.queries/v1`, refusing. The rule says 'function or named predicate'. |
| FND-004 | fixed | 2cd8b1928 and 32a475f22: the unit declaring the callees selects `quire.model.complete/v1`. Its `requires` closure admits function and predicate declarations (value.complete and state.queries through state.graph) and reference equality (state.graph), so `same`'s `a = b` compiles under #1247's FR-110 rule. The `positive` predicate vector now runs in the same unit. |

Disposition pass 1 verdict (head 153ceca82): **NOT MERGEABLE until FND-004 is fixed or accepted (minor).**

Disposition pass 2 verdict (head 32a475f22): **MERGEABLE.** Every finding's latest outcome is `fixed`.
