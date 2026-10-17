---
id: SR-12050
title: "QSL-819 PR #923 spec review: the authored population bound, its node and the reaches link"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ee77bb9ee57278c2cc281764246067d4aa0663fc; spec/functional/FR-380-bound-a-domain-package-population-and-link-a-reachability-to-its-node.md, spec/test-cases/TC-1054-a-bounded-domain-population-is-authored-linked-and-emitted-as-one-node.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/test-cases/TC-1047-a-checked-reaches-over-a-relationship-end-lowers-to-quire-op-model-reaches.md, spec/spec.md"
review_set: subset
---

## Summary

Ticket: QSL-819. PR quire-spec-language#923, reviewed at ee77bb9ee57278c2cc281764246067d4aa0663fc. The review covers integrity and consistency of FR-380 (AC-1 to AC-7), TC-1054 (rows 1 to 13), FR-104 (the amended "Requirements" text and AC-11), FR-365 ("The population link", AC-4, AC-6 and AC-13), TC-1047 row 7, and the spec.md index row. It also checks cross-repo consistency against QSpec PR #268 (FR-322 "Population link of a `reaches`", FR-322-AC-73). The planner accepted the author's choices, and none of them is flagged here: the `bound` syntax, T being the near-end type E, the key being absent rather than null, the above-maximum refusal, the change of extent to Bounded, and the ids. FR-380 and TC-1054 do not collide with anything on origin/main or on any remote branch. The other FR-380 mentions on main belong to QSpec and quire-exact, which are different repos.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The last clause of FR-380-AC-5 cannot be satisfied. "Two bounds of the same `N` on two populations covering `Order` give one node with two occurrences" cannot happen. A node exists only for a bound that a reachability walks, and its T is the near-end type E = Order. When two populations cover Order, FR-104 and FR-365 (AC-4) refuse that reachability `ambiguous_declaration`/`ambiguous-name`. TC-1054 row 12 asserts exactly that refusal for a second `[3]` bound on `staff`. The same impossible case appears in FR-365 "The population link" ("the populations of two bounds that write the same `N` for the same `E` ... are one node"). | spec/functional/FR-380-bound-a-domain-package-population-and-link-a-reachability-to-its-node.md:147, spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md:180, spec/test-cases/TC-1054-a-bounded-domain-population-is-authored-linked-and-emitted-as-one-node.md:49 |

## Verdict

FIX, because of FND-001. Delete the third clause of FR-380-AC-5 and the matching "populations of two bounds ..." phrase in FR-365 "The population link". The parameter-type sharing case (a `Population<Order>[3]` parameter plus a walked bound) is the one real shared-node case, and TC-1054 row 11 covers it.

Everything else checked is consistent. FR-104's change from "no maximum" to "covers" is safe, because every domain-package population lacks a maximum (QSpec FR-153). The FR-104-AC-11 record agrees with FR-097's extent rule (`Population(Some(N))` is not an unbounded domain). The node shape (body `max`, `Integer` node, `Set<Reference<E>>` over `Reference<E>`, dependencies `Integer` and `Set`) matches QSpec FR-322's `model_population` row and the PR #268 vectors. The least-ordinal occurrence convention matches FR-322. FR-365-AC-6 and AC-13 agree with TC-1047 row 7 and TC-1054 rows 5, 7 and 8. QSL cites QSpec by id only. The spec.md index carries FR-380. TCs are not indexed there, which matches TC-1053. `quire validate` on the six files exits 0, with only the existing FR-104 line 39 EARS warnings.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 00f7403ec: FR-380-AC-5 now ends after the parameter-type clause, and FR-365 "The population link" reads "a `Population<E>[N]` parameter type of the same `E` and `N` and a walked bound are one node". TC-1054 row 11 (one node, two `Type` occurrences) and row 12 (two covering populations refuse `ambiguous-name`) still agree, and the round-1 delta adds no new defect. |
