---
id: SR-7813
title: "Manual acceptance-to-tests review of QSL-254 PR #736"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@155a187ab05b78fa1aab9475443453a203ffd92b; PR #736 diff origin/main...HEAD; FR-016-AC-10, FR-082-AC-8; src/checking/types.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/tests/it/model_conformance.rs, tests/it/composed_domain_models.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-016", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/FR-082", type: reviews }
---

## Summary

Quoin's `gap-analysis` command is unavailable in the installed CLI, so this is the required manual acceptance-criteria-to-tests check for the PR scope. TC-895/TC-148 bind to FR-016-AC-10 and TC-896 binds to FR-082-AC-8; their stated scenarios have executable assertions, but the suite misses the cross-case where multiplicity narrowing is incorrectly discharged by a presence fact.

## Verdict

**FAIL** — the focused test set does not distinguish field presence from non-empty collection multiplicity during redefinition.

## Scope

- FR-016-AC-10 (examined): A domain-package field declared `required` with multiplicity `[0, 1]` types as a sequence of at most one element, and `present(x.f)` refuses `ill_typed`; the field declared `optional` with multiplicity `[1, 1]` types as an option of its leaf, and `x.f` unwrapped under a `present` guard checks; the field declared `optional` with multiplicity `[0, 1]` types as an option of a sequence of at most one element.
- FR-082-AC-8 (examined): Given a field `f` declared `optional` with multiplicity `[1, 1]`, redefined by a field declared `required` with multiplicity `[1, 1]` and written by an exposed operation whose postcondition establishes no presence fact, the checker refuses `unproved-refinement` with obligation `field-presence`; with `present(self.f)` established it admits the redefinition.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | TC-896 verifies presence narrowing at equal multiplicity and unchanged-presence controls, but omits required `[0,1]` → `[1,1]` with a presence fact; that case currently admits without evidence of a non-empty collection. | FR-082-AC-8; qsl-semantics/tests/it/model_conformance.rs:868-915 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: manual acceptance-criteria-to-tests check; `quoin gap-analysis` is not an installed command. `quoin matrix --repo . --json` completed with exit 0, but its full-repository JSON exceeded the tool output limit and the aggregate counts could not be extracted; the filtered retry was interrupted (exit 130). `quire matrix` was also interrupted after it did not return. No matrix counts or repository-wide coverage claim are made.
- Plan completion: not assessed
- Criteria examined: FR-016-AC-10 and FR-082-AC-8.
- Bindings examined: TC-895 → FR-016-AC-10 (correct); TC-148 → FR-016-AC-10 (correct); TC-896 → FR-082-AC-8 (correct).
- Oracle strength: TC-895 checks exact `NativeType` shapes and the composed checker path; TC-896 checks typed refusal cause and detail, proof discharge, and unchanged-presence acceptance. The missing cross-case permits a false discharge of collection non-emptiness.
- Semantic review: focused manual review of the two criteria and their implementations/tests; repository-wide semantic gap analysis was unavailable.

## Disposition summary

Round 1 reviewed at `48f2cb3ae969ccbe7223dbe1403ad7d2e13ee284` against base `f8a641ed53e8d15ca40d04701133024369486216`. FND-001 is fixed by `6005c96b96d4a84ca12b23f322e726a4d2161df2`; the added TC-896 regression directly asserts refusal when a `Presence` postcondition accompanies the required `[0,1]` to `[1,1]` multiplicity narrowing. No new gap-analysis finding was found in the current diff.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6005c96b96d4a84ca12b23f322e726a4d2161df2 |

The regression's setup creates a real `PostconditionClause::Presence` on the redefining operation; its oracle checks the refusal cause and `no-proof-form` detail. The old behavior would have admitted this case because that presence clause set `established.presence`.

Fixed implementation excerpt (`after_excerpt`, `qsl-semantics/src/check/field_refinement.rs`):

```rust
    if raises_lower {
        return Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: ModelRefusalCause::UnprovedRefinement,
            detail: format!(
                "{} raises the lower multiplicity bound of {}, which no FR-146 fact form proves (obligation no-proof-form)",
                redefining_key.node, redefined_key.node
            ),
        }]));
    }
```

