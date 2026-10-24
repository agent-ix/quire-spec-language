---
id: SR-11700
title: "Spec review of quire-spec-language PR #896: reachability over a relationship end in a state clause"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@22e9d6a2a52d92fb0a6676a8acd2a4a5caa448c5; spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-105-emit-state-nodes.md, spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md, spec/spec.md, spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md, spec/test-cases/TC-1047-a-checked-reaches-over-a-relationship-end-lowers-to-quire-op-model-reaches.md, spec/test-cases/TC-1048-reaches-over-a-relationship-extent-decides-closure-charges-and-incomplete-outcomes.md"
review_set: subset
---
# Spec review of quire-spec-language PR #896

## Summary

Ticket: QSL-804. One pass of spec-review with the contradiction,
completeness, vector-soundness (reachability, loci, check order),
EARS/AC-consistency and determinism sub-analyses over the PR diff
(`origin/main...HEAD`). Spec-only PR, so no code review. Only major issues
block; mediums and lows are recorded in the Verdict.

Oracles: QSL on main: FR-008 (AC-3, AC-11, AC-16), FR-104 (reaches field
form, observations, requirements record, no parameter in an invariant's
scope), FR-105 (state_clause node table and claim occurrences), FR-107
(field-edge reaches schedule), FR-363 (near-end role, direction,
multiplicity), FR-364 (admission, resolve, read contract, navigation),
`qsl-eval/tests/it/state_model_family.rs` (field-edge missing name gives
missing-name). QSpec on main, by id only: FR-322 "Reaches over a field",
FR-322-AC-57, RN-15 to RN-19, TC-198 L10.

Examined:
- FR-365 statement, Behavior sections, FR-365-AC-1 to AC-12 (examined)
- TC-1046 fixture R and C01-C15 (examined)
- TC-1047 rows 1-7 (examined)
- TC-1048 rows 1-23 (examined; rows 1, 2, 5, 6, 9, 11, 12, 17, 20 recomputed by hand)
- FR-104, FR-105, FR-364 pointer sentences; spec.md entries (examined)
- FR-363, FR-364 resolve/admission, FR-105 table, FR-107 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1046 C15 is unreachable as written. `reaches(self, self, next)` on `M::Person` resolves `next` on `Person`, which has no field or navigable end named `next`, so it refuses `missing_declaration`/`missing-name`, not `ambiguous-name`. "with a parameter `o: Reference<M::Order>`" cannot hold for an invariant (FR-104 brings no parameter into an invariant's scope) and fixture R declares no `Person` operation. So neither expression yields the expected population ambiguity, and FR-365-AC-4 has no reachable row. Fix: declare a `Person` operation with parameter `o: Reference<M::Order>` and use only `reaches(o, o, next)` in its `pre` clause (context population `people`, so `orders` is a second domain). | spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md:69 |
| FND-002 | high | TC-1047 row 4 expects two `state_clause` nodes, "each with its own identity", for two invariants `A` and `B` on `M::Order` with the same body `not reaches(self, self, next)`. By FR-105's table a `state_clause` node's body is the clause kind, the parameter aggregate, the anchor and the condition, with "one `claim` occurrence per declaration that checks to this node". The clause name is not in the body, so A and B are one `state_clause` node with two `claim` occurrences, and the expected result contradicts FR-105. FR-365-AC-6's "one node under two `state_clause` identities" is right only when the clauses differ. Fix: make the two clauses differ (for example an invariant on `M::Order` and one on `M::Rush`, or an extra conjunct), or expect one `state_clause` node with two `claim` occurrences. | spec/test-cases/TC-1047-a-checked-reaches-over-a-relationship-end-lowers-to-quire-op-model-reaches.md:30; spec/functional/FR-105-emit-state-nodes.md:65 |
| FND-003 | medium | FR-365-AC-3 says "multiplicity ahead of destination type", and no TC-1046 vector covers it. C12 covers direction before multiplicity. No fixture relationship has a destination that fails both the multiplicity check and the type check, for example an unbounded `Person` destination that must refuse `unsupported_construct`/`expression-form`, not `operator-ineligible`. Fix: add such a relationship to fixture R and a vector. | spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md:233; spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md:53-69 |

## Verdict

FAIL, three blocking findings (FND-001 to FND-003).

Traced and correct:
- Lowered node: operator `reaches`, semantic_form `reachability` (FR-093),
  identity `quire.op.model.reaches`, member `relationship_end`
  {declaration, name}, no laws/mode, two `reference` arguments, Boolean
  result. This matches QSpec FR-322's definition of `quire.op.model.reaches`
  as the field form's predicate over a relationship end.
- Check order: role lookup, then operands, direction, destination
  multiplicity (unbounded/ordered refuse `unsupported_construct`/`expression-form`),
  then destination type (`Reference`/`Option` of the receiver endpoint's
  type, else `operator-ineligible`). This matches QSpec FR-322-AC-57 and
  RN-15 to RN-19: RN-18's finite multi-valued destination is
  `operator-ineligible`, as C10 is. The absent-multiplicity refusal follows
  merged FR-363.
- Field-edge missing name gives `missing_declaration`/`missing-name` in
  merged tests, so FR-365's no-candidate rule is consistent with the field
  form.
- The evaluation schedule matches FR-107 and QSpec TC-198 L10 (the start is
  discovered but not reached; `next_charge` 2 for a denied second
  `graph.expand`). Rows 1, 2, 5, 6, 9, 11, 12, 17 and 20 were recomputed by
  hand and match. Row 17 is reachable because FR-364 admission compares no
  count. Row 21's undecidable `Rush` follows FR-364 resolve rule 4.
- Closure: rows 3-7 and 23 state one-or-more-edge closure. The oracle row
  covers FR-008-AC-11. The self-loop covers FR-008-AC-3, and exact and
  one-below budgets cover FR-008-AC-16.
- Single-valued, self-typed destination is stated as the design ("A
  set-valued or bag-valued destination end has no reachability in this
  form"), with no roadmap wording. No ticket ids, no ceremony, and QSpec is
  cited by id only.
- Ids: FR-365 and TC-1046 to TC-1048 do not collide with #895 (TC-1045,
  FR-106) or with merged #894.

Recorded (medium/low, not blocking):
- The order between "Resolution of the third argument" and check 1 (context)
  is unstated: `reaches(o, o, nothing)` in a function body could refuse
  `missing-name` or `operator-ineligible`.
- Resolution runs on "the static type `S` of `a`" before the operand check,
  with no rule for a non-`Reference` `a`. C08 tests only `b`.
- FR-365-AC-4's "none when it is the context population" is asserted only
  weakly (TC-1047 row 7 does not say "no second domain").
- TC-1048 row 23's inverse-role oracle should say it is the closure of the
  converse relation.
- C09's second vector and C12 are the same expression. Change-relative
  wording ("unchanged", "as before") appears in TC-1046 C05/C14 and the
  FR-365 description.
