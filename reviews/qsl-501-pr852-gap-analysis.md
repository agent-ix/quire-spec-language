---
id: SR-11085
title: "QSL-501 quire-spec-language PR 852 gap analysis: FR-313 union and case forms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@da5e8f44f9ff7687813e37e545d76603f2ab6f8d; FR-313-AC-1, FR-313-AC-2, FR-313-AC-3, FR-360-AC-5, TC-815, TC-816, TC-1007; qsl-forms/tests/it/sum_case_forms.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/tests/it/variant_fields.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-501. This is a manual check of acceptance criteria against
tests. `quire trace` timed out and was not used. The trace tags were read
directly.

- FR-313-AC-1 is backed by
  `authored_union_keeps_nullary_payload_arity_order_and_spans`. It checks
  the names, payload arity, `Integer` heads, the member, name and
  declaration spans, and the trailing comma. TC-815.
- FR-313-AC-2 is backed by
  `authored_case_keeps_member_binder_body_order_and_source_spans`. It checks
  the scrutinee `s`, the member spellings, the binders `[r]`, `[w, h]` and
  `[]`, the bodies and the spans. The same AC is backed by
  `construction_stays_name_or_call_and_duplicate_arms_stay_authored`, which
  checks that `Shape::Circle(3)` is a Call and `Shape::Empty` is a Name.
  TC-815.
- FR-313-AC-3 is backed by
  `unparenthesized_record_opens_arm_list_and_fails_there`, which checks the
  diagnostic at `}`, `;` expected and a recovering CST. It is also backed by
  `expression_and_real_protocol_choice_cases_coexist_in_one_admitted_unit`,
  which checks a parenthesized record scrutinee, one `CaseExpression` and
  two protocol `Case` nodes with their forms. TC-816.
- The FR-313 behaviour bullet on the dispatch rewrite is backed by
  `dispatch_substitution_walks_case_children_and_preserves_each_arm_binding`.
- Each test can fail, and none is vacuous.

## Verdict

FAIL on one finding. FR-360-AC-5 states behaviour that this PR changes, and
the test traced to it was edited to assert the new behaviour. The AC was
left as it was.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-360-AC-5, item 5 and TC-1007 step 3 say S1 refuses an expression-position `case` at the `case` token, so it never reaches the checker. After FR-313, S1 refuses at `when` instead: `variant_fields.rs:1410` now asserts `when`, but it still traces FR-360-AC-5. A `case t { Gold: ...; }` over a variant now reaches the checker and refuses `unknown_required_feature/unsupported-feature`. Item 5 requires `ill_typed/type-mismatch` at the scrutinee. Fix: amend FR-360 item 5, AC-5 and TC-1007 step 3 in this PR to state the post-FR-313 refusal, or implement the scrutinee refusal. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:126-130,275; qsl-semantics/tests/it/variant_fields.rs:1395-1413; qsl-semantics/src/check/check/typing.rs:662-670 |
