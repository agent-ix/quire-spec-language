---
id: SR-20261105101066
title: "QSL-1692 PR2447 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1692 PR2447 numeric-family sum implementation, tests and specifications"
review_set: subset
---

## Summary

Original review findings recovered verbatim from Linear comment ca096a2e-2048-4c08-935a-63b40220607d. Reviewed initial head 6377e1058. Disposition against the recovered frozen head follows separately.

## Verdict

PASS after disposition round 1 at frozen head 1812f938b; every original finding is fixed and no new defect was found in the reviewed diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | sum.float32/sum.float64 are lowered with a rounding mode but no IeeeProfile law. The Contract IR reader refuses every such package with InvalidPackage/OperationLawMissing at /semantic_graph/nodes/N/body/operation/laws. This was reproduced through write() for both Float32 and Float64 sums, while a float a + b in the same unit is admitted. Other IEEE operations carry laws: vec![self.law(LawRole::IeeeProfile, ..)]. The qsl-eval float sum test passes only because package_with never runs the reader. | qsl-semantics/src/check/lowering.rs:4982-5013 |
| FND-002 | high | The FR-146 definedness walk discharges sum prefix obligations only for Int results. Every sum this PR newly admits with a bounded Rational or Decimal result passes with no proof. Reproduced: sum<R>(x in r_items(): x) with R = Rational[-100,100;1,100] over Sequence<R>[0,2] (prefixes reach +/-200) is written and admitted, while the same sum over Int[-100,100] is refused Unproved(Range{required [-100,100], proved [-200,200]}). QSpec FR-145 states that a linked sum whose prefixes are not all proved members is refused undefined_expression/unproved-range. The PR's own numeric_sums_keep_their_family_and_empty_identity fixture depends on this gap for R and D. | qsl-semantics/src/check/facts.rs:1195-1197 |
| FND-003 | medium | A non-member Rational or Decimal seed returns an unlocated Stop, so the halt is located at the sum node. The Integer arm (evaluate.rs:4138-4144) returns Halt::Located at node.at(*body), which matches FR-145's located undefined outcome at that summand. The new arms should do the same. | qsl-eval/src/value/expression/evaluate.rs:4172-4174, 4199-4201 |

## Examined Scope

- qsl-semantics/src/check/lowering.rs (examined): Visit::Sum => { let (identity, mode) = match node.value_type() { ... } }
- qsl-semantics/src/check/check/typing.rs (examined): is_sum_family / sum_result_matches; tally checks integer only for Count
- qsl-eval/src/value/expression/evaluate.rs (examined): (Visit::Sum, Value::Rational|Decimal|Float|Quantity) running-total arms and the empty-sum identity per family
- qsl-package/src/reader_protocol_projection.rs (examined): sum.rational|decimal|float32|float64|quantity => Visit::Sum
- qsl-package/src/introduced_binders.rs (examined): the five new sum identities open a Query binder
- tests/it/checked_handoff.rs (examined): a_quantity_sum_checks_lowers_and_is_admitted; model_alias_names_resolve_for_captures_and_quantity_sums
- qsl-eval/tests/it/quantity_literals.rs (examined): a_quantity_sum_evaluates_to_its_magnitude_in_the_same_unit; numeric_sums_keep_their_family_and_empty_identity
- qsl-semantics/src/check/facts.rs (context_only): NodeKind::Query { visit: Visit::Sum, .. } => { let ValueType::Int(domain) = node.value_type() else { return Ok(()); };

## Coverage

Plan completion: not assessed

Original run used manual trace reconciliation, not a computed matrix. No current-head outcome is claimed yet.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: c7125a91c adds the IEEE-profile law for both float sum widths; the production writer/strict-reader regression asserts identities, modes and laws. |
| FND-002 | fixed | Round 1: c7125a91c dispatches Rational and Decimal sums through conservative prefix-domain proof helpers; 4b84fc19b and 1812f938b cover unsafe linked prefixes and the zero-excluding empty case. |
| FND-003 | fixed | Round 1: 4b84fc19b returns Halt::Located at the summand for both Rational and Decimal non-member seeds, with exact seed-versus-prefix/empty locations asserted. |

## Current Review Scope

Reviewed `origin/main...1812f938b` in the existing ticket worktree. Fix commits c7125a91c, 4b84fc19b and 1812f938b were examined; original findings above remain unchanged. Reviewed all changed Rust files, FR-093 lowering row, FR-517-AC-16/19/20 and TC-1312 rows. No changes to CI, locks, manifests or unsafe code. No additional public API or wire format introduced. No applicable AssuranceProfile exists. No new review worktree was created.

## Verification

Focused review command exited 0 at frozen head 1812f938b: qsl-eval quantity_literals (13 passed), production numeric-family writer/reader regression (1 passed), fractional-prefix/insufficient-total regression (1 passed), zero failures or ignored tests. Log: /tmp/qsl1692-review/focused.log. Changed-spec and review-artifact Quire validation exit 0; diff whitespace check exit 0. Latest coder Clippy log /tmp/qsl1692-recovery-clippy.log inspected: warnings-denied qsl-eval test target passed. No aggregate gate was run by the reviewer.
