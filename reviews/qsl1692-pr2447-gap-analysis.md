---
id: SR-20261105101067
title: "QSL-1692 PR2447 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1692 PR2447 numeric-family sum implementation, tests and specifications"
review_set: subset
---

## Summary

Original review findings recovered verbatim from Linear comment ea69d8a0-f4f2-46c3-bd01-45955dc273ba. Reviewed initial head 6377e1058. Disposition against the recovered frozen head follows separately.

## Verdict

PASS after disposition round 1 at frozen head 1812f938b; every original finding is fixed and no new defect was found in the reviewed diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Only the quantity family is exercised through the production writer and the Contract IR reader (a_quantity_sum_checks_lowers_and_is_admitted). The rational, decimal, float32 and float64 sums are tested only in qsl-eval via package_with, which does not run the reader. The four new arms for those families in reader_protocol_projection.rs and introduced_binders.rs therefore have no test. A handoff test per family would have caught the float lowering refusal (SR-20261105101066 FND-001). | tests/it/checked_handoff.rs:2266-2293 |
| FND-002 | medium | No test covers these new branches: the typer refusal of a summand whose family, float width or unit differs from N (sum_result_matches); Undefined::SumOutOfDomain for a non-member Rational or Decimal seed, running total or empty identity; and the ValueLoss::Decimal and ValueLoss::IeeeFlags records from a rounding sum. | qsl-semantics/src/check/check/typing.rs:3597-3616 |

## Examined Scope

- FR-517-AC-16 (examined): Over R's model with Amount and Total as aliases of ix://quire/unit/U, a source whose pre compares the Amount parameter with 1 U is written and admitted.
- FR-517-AC-19 (examined): A model alias is a source type name: over R's model, sum<M::Total> and capture targetFull: M::Total resolve to the alias's unit-bearing type, and neither refuses UnresolvedTypeName; both check. The alias's original min/max bounds remain selected-model member constraints and enforce the same endpoint values as before, without widening them or imposing them on other aliases of U.
- FR-517-AC-20 (examined): sum<N> checks a summand of any numeric family QSpec FR-145 and FR-322 give a quire.op.collection.sum identity for, the carried-quantity family included, and lowers to that family's identity; over R, the Healthy invariant's receipt sum over Quantity<U> checks and evaluates to the sum of the receipts' magnitudes in U.
- FR-093 (examined): Query{visit: Sum} (sum<N>) | collection | quire.op.collection.sum.<family> selected from the summand's numeric family (integer, rational, decimal, float32, float64 or quantity) | member type_argument of T(node); operation mode and result type follow that family's FR-322 definition
- TC-1312 (examined): 20 | target state: sum<Cash> with type Cash = Quantity<U>;, and Healthy's sum over Quantity<U>, evaluated over receipts of 2 U and 3 U | the clause checks, lowers to the quantity family's quire.op.collection.sum identity and evaluates to 5 U

## Coverage

Plan completion: not assessed

Original run used manual trace reconciliation, not a computed matrix. No current-head outcome is claimed yet.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: numeric_sum_families_emit_their_identities_and_modes writes integer, Rational, Decimal, float32 and float64 sums through the real writer and strict Contract IR reader; the quantity writer test remains. Both float laws and rounding modes are asserted. |
| FND-002 | fixed | Round 1: 4b84fc19b adds family/width/unit refusals; Rational and Decimal seed, prefix and empty-domain stops with locations; Decimal loss and exact IEEE Inexact flags; happy and empty identities remain behavioral tests. |

## Current Review Scope

Reviewed `origin/main...1812f938b` in the existing ticket worktree. Fix commits c7125a91c, 4b84fc19b and 1812f938b were examined; original findings above remain unchanged. Reviewed all changed Rust files, FR-093 lowering row, FR-517-AC-16/19/20 and TC-1312 rows. No changes to CI, locks, manifests or unsafe code. No additional public API or wire format introduced. No applicable AssuranceProfile exists. No new review worktree was created.

## Verification

Focused review command exited 0 at frozen head 1812f938b: qsl-eval quantity_literals (13 passed), production numeric-family writer/reader regression (1 passed), fractional-prefix/insufficient-total regression (1 passed), zero failures or ignored tests. Log: /tmp/qsl1692-review/focused.log. Changed-spec and review-artifact Quire validation exit 0; diff whitespace check exit 0. Latest coder Clippy log /tmp/qsl1692-recovery-clippy.log inspected: warnings-denied qsl-eval test target passed. No aggregate gate was run by the reviewer.

Reconciliation: manual source-tag index for changed criteria. The repository-wide `quire matrix --scope . --format json` did not finish or produce output after more than four minutes and was terminated (exit 143); no computed matrix, whole-repository coverage percentage, or whole-repository PASS is claimed. Scoped AC16, AC19 and AC20 trace tags were checked directly against real tests; every new/changed sum branch has behavioral coverage. Optional whole-repository semantic audit was not run; the code-review lane checked oracle strength for this diff. Plan completion: not assessed.
