---
id: SR-17237
title: "QSL-987 PR #1579 gap analysis: FR-410 and FR-165-AC-1 against TC-1140 and TC-590"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@aabd887c597199216c54f6e77a0c0828922bcf55; FR-410 (Behavior, AC-1 to AC-4), FR-165-AC-1, TC-1140, TC-590 (by id); the diff's traced tests in qsl-package, qsl-semantics and qsl-analyze"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-410
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: references
---

## Summary

Ticket: QSL-987. PR quire-spec-language#1579 at head aabd887c5. Planless gap analysis scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading, and the new tests were run in the full suite. QSpec FR-475 is cited by id.

Bindings examined:

| Criterion | Test | Assertion | Verdict |
| --- | --- | --- | --- |
| FR-410-AC-1 | the_three_forms_emit_claim_nodes_the_i2_read_admits | binding names and order, no declaration or owner, occurrence role claim, over/from/target nodes, I2 roots read back | bound, real (fixture is ConfigVersion, not ADR-022 game units) |
| FR-410-AC-2 | the_form_enters_the_identity_and_equal_claims_share_one_node | possible vs always possible differ in node id and package_id; omitted vs `from (true)` equal id and the true literal; no `over` is the empty aggregate; two names share one node, ordinals 0 and 1 | bound, real |
| FR-410-AC-3 | the_i2_lookup_resolves_the_triple_and_each_lookup_takes_only_its_own_form; a_claim_the_request_package_does_not_hold_settles_failed | triple resolves in the recompiled package; unknown node, temporal node and other ordinal unresolved; FR-337 lookup unresolved; settlement Failed with zero replays | bound, real (mutation: Proved without the early return) |
| FR-410-AC-4 | the_i2_read_refuses_each_fr475_defect_at_its_locus | `eventually-possible` refuses invalid_package/invalid-value at members/0; parameter target refuses ill_typed/operator-ineligible at the target; keys and package_id recomputed | bound, real |
| FR-165-AC-1 | four TC-590 tests, plus possible_and_always_possible_over_one_predicate_have_different_node_identities | form checked; different node identities | bound, real |
| admission corpus | row for ClaimForm::StateGraph; terminal in NOT_EMITTED_BY_QSL | fixture backs a row | bound |

No stub, no tautology and no assertion-free test found. The deleted `a_checked_claim_is_refused_as_not_emitted_by_default` has its replacement; the removed `with_unwritten_state_graph_claims` calls leave no test asserting nothing.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The identity properties the brief and FR-475-AC-3 state are not asserted by any test, though they hold (probed with a temporary test, reverted): `unique path` differs from the other forms over equal predicates; `from` and `to` order matters; whitespace and parentheses in a predicate do not change the id; reordering or adding other claims in the unit does not change a claim's id. AC-2 checks only the possible/always-possible pair. Add one test with these four comparisons. | qsl-package/src/emit/tests/state_graph_claim.rs | correct-requirement-no-evidence |
| FND-002 | low | The settlement cancellation arm (`Incomplete(Cancelled)`) is unbound to any test (also in SR-17236 FND-002). | qsl-replay/src/model_check/graph_settle.rs:128-147 | correct-requirement-no-evidence |
| FND-003 | info | AC-1 and AC-2 use equivalent predicates on the ConfigVersion fixture rather than ADR-022 section 7 text; accepted, the asserted structure does not depend on the predicate (see SR-17236 FND-006). | qsl-package/src/emit/tests/state_graph_claim.rs | correct-requirement-no-evidence |

## Verdict

No code lacks an owning requirement and no criterion lacks a tagged test. FND-001 and FND-002 are low evidence gaps; not blocking.
