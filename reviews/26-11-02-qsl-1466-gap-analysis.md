---
id: SR-202611021418
title: "Gap analysis of PR #2117: FR-090-AC-3 and FR-049-AC-1 coverage after restating two qsl-eval tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@99840350e; git diff origin/main...99840350e (PR #2117, branch qsl-1466): qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/tests/it/equality_matrix.rs, qsl-eval/tests/it/window_aggregates.rs; ticket QSL-1466"
review_set: subset
date: '2026-11-02'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: reviews
---
# SR-202611021418: gap analysis of PR #2117 (QSL-1466)

## Summary

Ticket: QSL-1466. Plan completion: not assessed. This review is planless and
scoped to the PR diff. It is a manual mapping of the ACs the touched tests
claim, made against the tagged tests and production code.

- FR-090-AC-3 / TC-384 is still tagged in three places:
  qsl-eval/src/value/expression/mod.rs:2322 (the two TC-384 cases),
  family.rs:426, and the new invariants.rs:753. The x04 tag is removed, and
  coverage of the AC's named cases does not change.
- TC-629 (window aggregates) keeps its single test, which still passes and
  still exercises the failed-call arm.
- FR-049-AC-1 (the foreign checked expression) is proved by
  checked_package_call.rs `evaluate_refuses_expression_from_another_checked_graph`,
  added by #2098.
- FR-049-AC-2 and FR-049-AC-3 (object values admitted against the checked
  environment, with their unit contract) have a reproduced code gap. It is
  recorded in the code-review SR-202611021417 as FND-001 and repeated below
  as the coverage finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No test covers FR-049-AC-2/AC-3 for an `ObjectEnvironment` closed under a foreign `TypeEnvironment` given to `CheckedPackage::evaluate`, and the code accepts one. A unit outside the package's table reaches the S6a `quantity-unit-unresolved-past-admission` fault. A unit inside the table returns a value whose unit does not match its checked type. Both were reproduced. | qsl-eval/src/value/expression/mod.rs:1448-1454 |

## Verdict

The PR's own changes leave no coverage gap. The single finding predates the
PR and is outside its diff. It needs its own ticket, with a public-API test
in which a foreign object environment is refused at admission.
