---
id: SR-7812
title: "Code review of quire-spec-language PR #736 (QSL-254)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@155a187ab05b78fa1aab9475443453a203ffd92b; PR #736 diff origin/main...HEAD; src/checking/types.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/tests/it/model_conformance.rs, tests/it/composed_domain_models.rs; FR-016-AC-10, FR-082-AC-8"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-016", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/FR-082", type: reviews }
---

## Summary

Reviewed the exact Rust diff for PR #736, including the `rust-review` lane, test bindings, and the two acceptance criteria changed by the work. The native type mapping and the new direct assertions are sound, but field-refinement still accepts a presence fact as evidence for a required collection's non-empty multiplicity narrowing.

## Verdict

**FAIL** — a presence postcondition can discharge a different obligation: non-empty collection multiplicity.

## Scope

- FR-016-AC-10 (examined): A domain-package field declared `required` with multiplicity `[0, 1]` types as a sequence of at most one element, and `present(x.f)` refuses `ill_typed`; the field declared `optional` with multiplicity `[1, 1]` types as an option of its leaf, and `x.f` unwrapped under a `present` guard checks; the field declared `optional` with multiplicity `[0, 1]` types as an option of a sequence of at most one element.
- FR-082-AC-8 (examined): Given a field `f` declared `optional` with multiplicity `[1, 1]`, redefined by a field declared `required` with multiplicity `[1, 1]` and written by an exposed operation whose postcondition establishes no presence fact, the checker refuses `unproved-refinement` with obligation `field-presence`; with `present(self.f)` established it admits the redefinition.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | A `Presence` postcondition still discharges a lower-bound increase on a required single-valued collection, so `[0,1]` → `[1,1]` can be admitted without evidence that the collection contains an element. | qsl-semantics/src/check/field_refinement.rs:293-303 | implementation-bug-despite-evidence |

## Coverage

- Rust lane: reviewed ownership, error/refusal behavior, integer conversion, visibility/docs, and test oracles; no build or test command was run.
- Test-oracle check: TC-895 directly asserts required `[0,1]` maps to `Sequence(maximum: 1)`, optional `[1,1]` maps to `Option`, and optional `[0,1]` maps to `Option<Sequence>`. TC-896 exercises optional-to-required presence narrowing and equal-presence controls. Neither exercises required `[0,1]` to required `[1,1]` with a `Presence` postcondition, which exposes the finding.
- `git diff --check`: clean.

## Disposition summary

Round 1 reviewed at `48f2cb3ae969ccbe7223dbe1403ad7d2e13ee284` against base `f8a641ed53e8d15ca40d04701133024369486216`. FND-001 is fixed by `6005c96b96d4a84ca12b23f322e726a4d2161df2`; the regression oracle rejects the required `[0,1]` to `[1,1]` case with a `Presence` postcondition using `no-proof-form`. No new code-review finding was found in the current diff.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6005c96b96d4a84ca12b23f322e726a4d2161df2 |

The new test constructs both fields as `required`, raises the lower bound from 0 to 1, adds a `Presence` postcondition, and asserts `UnprovedRefinement` with `no-proof-form`. The current branch separates the presence check from the lower-multiplicity refusal, so the old false admission is prevented.

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

