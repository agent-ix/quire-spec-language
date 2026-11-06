---
id: SR-20261105101067
title: "Gap analysis of QSL-1692 numeric family sums"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1692; PR #2447 head 6377e1058; FR-517-AC-16, FR-517-AC-19, FR-517-AC-20, FR-093 Query{visit: Sum} row, TC-1312 rows 16, 19, 20"
review_set: subset
---

## Summary

Ticket: QSL-1692. Planless, diff-scoped acceptance-criteria-to-test check. Plan completion: not assessed. The Quoin matrix was not run; the criteria were traced by hand against the PR's tagged tests.

## Verdict

**GAPS.** FR-517-AC-16 and FR-517-AC-19 are covered by the rewritten handoff tests. FR-517-AC-20 is covered end to end (check, lower, reader admission, evaluation) only for the quantity family. Rational, decimal and float sums are tested only through `qsl-eval`'s `package_with`, which never runs the Contract IR reader. That is why the float lowering defect (SR-20261105101066 FND-001) went unnoticed. The new typing refusals and out-of-domain branches have no tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-517-AC-20 claims that every numeric family lowers to its identity, but only the quantity family is tested through the production writer and reader. No test writes a rational, decimal, float32 or float64 sum through `write` and the reader, and the four new `reader_protocol_projection` and `introduced_binders` arms for those families have no test. | tests/it/checked_handoff.rs:2266-2293 |
| FND-002 | medium | The new branches have no negative tests: typer refusals for a cross-family, cross-width or cross-unit summand (`sum_result_matches`); a non-member seed or prefix for Rational and Decimal (`SumOutOfDomain`); a non-member empty identity; and the recording of Decimal loss and IEEE flags. | qsl-semantics/src/check/check/typing.rs:3597-3616 |

## New findings (disposition pass 2)

Round 2 re-measures on PR #2455 (head 384f6c362), which supersedes PR #2447. #2455 does not carry #2447's added tests (head 1812f938b is not an ancestor). Scoped runs: `qsl-eval --test it family_sums collection_queries` gave 34 passed, and `--features checked-handoff --test it` over the three sum tests gave 3 passed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-093-AC-13 now applies to `quire.op.collection.sum.decimal`, because the FR-093 Sum row lowers it, and QSpec's `positive-operation-identities.json` holds a `sum.decimal` node (`mode: rounding nearest-even`, `laws: []`). But `golden.rs` keeps the identity in `NOT_LOWERED`, so the conformance comparison skips it. Its doc line still says no FR-093 row lowers it. | qsl-package/src/emit/tests/golden.rs:316-331 |

## Dispositions

Round 2, PR #2455 head 384f6c362.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open: rational and decimal sums now go through `write` and the reader (`decimal_and_rational_sums_lower_to_their_family_identity`). No test writes a `sum.float32`/`sum.float64` package through the reader or asserts its `ieee_profile` law. The PR body claims that float sums are reader-admitted, and only a reviewer probe shows it. | - |
| FND-002 | still-open: covered now are a quantity summand mismatch (integer into `Len`, location not asserted), a decimal prefix outside the domain, and a rational empty sum outside the domain. Not covered: cross-width float, cross-scale decimal and cross-unit refusals; `IeeeProfileNotAdmitted` for a float sum without a profile (typing.rs:2289-2294); a seed outside the domain located at the summand, for any non-integer family; Decimal `ValueLoss` and `IeeeFlags` recording in `sum_step`. TC-1312 row 20 asks for an out-of-domain total for each family and a mismatch "at the summand". | - |
