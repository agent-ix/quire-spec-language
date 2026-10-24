---
id: SR-12090
title: "QSL-506 PR #941 gap analysis: FR-333, FR-334, FR-335 and TC-843 to TC-845"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cbbbe83f3ef4d0f4415a23b01b5978716882aeec; FR-333-AC-1..FR-333-AC-3, FR-334-AC-1..FR-334-AC-4, FR-335-AC-1..FR-335-AC-5, FR-097-AC-8, TC-441, TC-843, TC-844, TC-845; tests/it/unbounded_declarations.rs, qsl-eval/tests/it/unbounded_set_call.rs, qsl-eval/tests/it/state_model_family.rs, qsl-semantics/src/check/type_form.rs, qsl-semantics/src/check/lowering/model.rs"
review_set: subset
---

## Summary

Ticket: QSL-506. PR quire-spec-language#941. This is a planless gap analysis scoped to the diff, so plan completion was not assessed. Each criterion was traced to its tagged tests by reading them, against QSL FR-333, FR-334 and FR-335 and QSpec FR-322 (read in place, cited by id). The author's recorded gap is not reported again: the FR-335-AC-5 charge point is the code review's FND-001 (SR-12089).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-097 still says its AC-8 lowering clause is unimplemented and that an unbounded population refuses `UnrepresentableBound`. TC-441 step 5 still expects that interim refusal. This PR removes the refusal from `population_type`, deletes TC-441's test for it, and lowers `Population<T>` to its own `model_population` node. The spec now states behaviour the code no longer has. The new lowering test is not tagged FR-097-AC-8, so FR-097-AC-8's lowering clause has a test that does not claim it | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:43-46, spec/test-cases/TC-441-an-unbounded-collection-never-refuses-for-cardinality.md:24, spec/test-cases/TC-441-an-unbounded-collection-never-refuses-for-cardinality.md:34-35, qsl-semantics/src/check/lowering/model.rs:875-901, qsl-eval/tests/it/state_model_family.rs:877-884 |
| FND-002 | medium | `an_unbounded_population_has_its_own_node_and_no_maximum` tags `FR-153-AC-9`, and its doc comment names it too. That id is QSpec's criterion. In this repository FR-153 is "Decide each fairness on the annotated quotient", which has no AC-9, so `quire matrix` binds the test to a criterion that does not exist instead of to the QSpec criterion | qsl-eval/tests/it/state_model_family.rs:877, qsl-eval/tests/it/state_model_family.rs:882, spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md:3 |

## Verdict

FIX, for two spec and trace corrections. The code-side gap is SR-12089 FND-001. Coverage of the criteria in scope:

- FR-333-AC-1: `an_absent_collection_bound_compiles_to_the_unbounded_type` covers the spine, all four kinds, no `collection_bounds`, bounded unequal to unbounded, and a `collection_bounds` node for `[0, 8]`. The unit test `an_absent_collection_bound_resolves_to_none` checks `bound: None` and `Some{0, 8}` at the resolver.
- FR-333-AC-2: the unit test `an_absent_population_maximum_resolves_to_none` checks `Population(None)` and `Population(Some(4))`.
- FR-333-AC-3: the spine test covers `[3, 2]` refusing `type-mismatch` and the S1 diagnostic at `x`. The unit test checks the span. Each test checks one half, and together they cover the criterion.
- FR-334-AC-1: different node key and `package_id`, no `min`/`max` on the unbounded `set` node, and `min`/`max` with 18446744073709551615 on the bounded node.
- FR-334-AC-2: three distinct keys, a `model_population` node without `max` and one with `max`, and lowering with no refusal.
- FR-334-AC-3: same keys and `package_id` on recompilation.
- FR-334-AC-4: `conformance_fr334_collection_and_population_keys_match_qspec` keys the unbounded `Set<Reference<Order>>`, `Population<Order>[3]`, `Population<Order>` and `Population<Order>[0]` to the keys QSpec FR-322 publishes, read in place.
- FR-335-AC-1 to AC-4: item kind, extent and `finite_bound_available`, domain list, empty and one-candidate sets, own request indices, bounded classification, and per-bound domains are all checked. The settlement half uses fixture dispositions, as TC-845 states. The amended AC-4 wording (a requirement built directly) matches its test. AC-2's "no harness is emitted" and AC-3's "unbounded item still settles requires-bound" have no assertion of their own.
- The edits to the FR-333, FR-334 and TC-843 to TC-845 wording are consistent with the grammar (`Integer`, not `Int` without a range) and with QSpec FR-322's `collection_bounds` member names `min` and `max`.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-097 and TC-441 step 5 now state that an unbounded population lowers to its own `model_population` node with no `max`. The new population test is tagged `TC-441`, `FR-097-AC-8`. |
| FND-002 | fixed | The `FR-153-AC-9` tag and its doc-comment mention are removed; the test is tagged with this repository's `FR-097-AC-8` instead. |
