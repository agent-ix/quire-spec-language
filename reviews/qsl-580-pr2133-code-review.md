---
id: SR-2026110215401
title: "Code review — QSL-580 quantified invariant bounds"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2133 qsl-cst, qsl-forms, qsl-semantics, qsl-package"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---
# SR-2026110215401: QSL-580 PR #2133 code review

## Summary

Reviewed the Rust parser, typed forms, timed-model checking, lowering, package emission, and tests at `f8cdadaf`. The implementation has focused tests for the typed carrier and positive/adverse forms, but accepts mixed quantified and ordinary bounds under one invariant-wide binder scope, which can drop an ordinary bound when the quantified collection is empty.

## Verdict

**CONDITIONAL** — one medium correctness finding and one low public API documentation mismatch remain.

## Coverage

Code-review with the Rust lane; touched-package focused tests and clippy were reported passing by the coder. No aggregate gate was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Mixed quantified and ordinary invariant bounds share one invariant-level binder scope, so an empty quantified collection can make an ordinary bound vacuous too. | qsl-semantics/src/check/timed_model.rs:3913; qsl-semantics/src/check/lowering/timed_clause.rs:275 |
| FND-002 | low | `TimeInvariantForm` still documents only ordinary `bound` items although its public `bounds` field now also carries `QuantifiedClockBoundForm`. | qsl-forms/src/syntax.rs:4454 |

## Scope

- {id: "qsl-cst/src/cst.rs", path: "qsl-cst/src/cst.rs", role: examined, excerpt: "TimeInvariant; ClockBound; QuantifiedClockBound productions"}
- {id: "qsl-cst/src/grammar.rs", path: "qsl-cst/src/grammar.rs", role: examined, excerpt: "TimeInvariant accepts ordinary and quantified bounds in source order"}
- {id: "qsl-cst/tests/it/complete_grammar.rs", path: "qsl-cst/tests/it/complete_grammar.rs", role: examined, excerpt: "The time-invariant grammar corpus includes a quantified bound"}
- {id: "qsl-forms/src/lib.rs", path: "qsl-forms/src/lib.rs", role: examined, excerpt: "Public re-exports include QuantifiedClockBoundForm and TimeInvariantBoundForm"}
- {id: "qsl-forms/src/syntax.rs", path: "qsl-forms/src/syntax.rs", role: examined, excerpt: "TimeInvariantForm.bounds is Vec<TimeInvariantBoundForm>; its adjacent doc comment remains ordinary-only"}
- {id: "qsl-forms/src/time_model.rs", path: "qsl-forms/src/time_model.rs", role: examined, excerpt: "S2 maps the quantified CST node into quantifier, binder, collection, clock, strictness, quantity and span"}
- {id: "qsl-forms/src/value.rs", path: "qsl-forms/src/value.rs", role: examined, excerpt: "QuantifiedClockBound is rejected by generic expression mapping rather than treated as an expression"}
- {id: "qsl-forms/tests/it/time_forms.rs", path: "qsl-forms/tests/it/time_forms.rs", role: examined, excerpt: "quantified_time_invariant_bounds_build_in_source_order_with_spans traces FR-393-AC-9"}
- {id: "qsl-package/src/emit.rs", path: "qsl-package/src/emit.rs", role: examined, excerpt: "time_invariant wire bodies emit binders and distinguish invariants by binder identity"}
- {id: "qsl-package/src/emit/tests/timed_clauses.rs", path: "qsl-package/src/emit/tests/timed_clauses.rs", role: examined, excerpt: "Positive binder/slot carrier and exists/presence refusal tests trace FR-230-AC-56"}
- {id: "qsl-semantics/src/check/lowering/timed_clause.rs", path: "qsl-semantics/src/check/lowering/timed_clause.rs", role: examined, excerpt: "One binder aggregate is attached to each invariant and passed while lowering every invariant bound"}
- {id: "qsl-semantics/src/check/timed_model.rs", path: "qsl-semantics/src/check/timed_model.rs", role: examined, excerpt: "time_invariant collects ordinary and quantified bounds into one draft with one binders vector"}
- {id: "qsl-semantics/src/check/timed_model/quantified.rs", path: "qsl-semantics/src/check/timed_model/quantified.rs", role: examined, excerpt: "invariant_bound constructs a Family slot constraint and ElementBinding for a universal quantifier"}
- {id: "qsl-semantics/tests/it/clock_places.rs", path: "qsl-semantics/tests/it/clock_places.rs", role: examined, excerpt: "a_quantified_time_invariant_bounds_the_live_slot_family traces FR-230-AC-56 and QSpec-FR-415-AC-24"}
- {id: "FR-230-AC-56", path: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md", role: examined, excerpt: "A quantified invariant emits a binder and upper bound over live slots; exists and element-presence forms refuse without partial output."}
- {id: "FR-393-AC-9", path: "spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md", role: examined, excerpt: "S2 builds ordinary and quantified invariant bounds in source order and retains the quantified fields and source span."}
- {id: "FR-230-AC-50", path: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md", role: context_only, excerpt: "Quantified element-presence timed forms and point-qualified timed anchors remain refused."}
- {id: "FR-322-AC-295", path: "/home/peter/dev/worktrees/qspec-main-current/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: context_only, excerpt: "Invariant binders resolve to ordered slot parameter references used by place-constraint slot terms."}
- {id: "FR-415-AC-24", path: "/home/peter/dev/worktrees/qspec-main-current/spec/functional/temporal/FR-415-declare-clocks-and-timed-models.md", role: context_only, excerpt: "A quantified invariant carries its typed slot parameter and bounds each live place only while live; forall is true for empty collections."}
- {id: "FR-342-AC-11", path: "/home/peter/dev/worktrees/qspec-main-current/spec/objects/interfaces/FR-342-checked-package-operation-anchor-body.md", role: context_only, excerpt: "Timed clauses use no-point operation anchors; point-qualified anchors are refused."}

## Findings Detail

### FND-001

The S3 checker accepts a mixed source such as `time invariant Mixed when { true } { forall(e in self.ts: e.x <= 9 ms) and self.x < 12 ms }`. It collects `e` on the invariant draft and retains both bounds, then lowering attaches the single invariant-level `binders` aggregate to the node containing both constraints. FR-415-AC-22 says those binders scope the predicate and upper bounds. With `ts` empty, the universal scope can make the whole invariant vacuously true, dropping the independent `self.x < 12 ms` condition. Refuse mixed quantified/unquantified bounds until the carrier can preserve separate scopes, or encode them as distinct invariant scopes.

### FND-002

The public type comment at `qsl-forms/src/syntax.rs:4454` still describes only `bound (and bound)*`, while the public field now contains `TimeInvariantBoundForm::Clock` and `::Quantified`. Update the comment to match the supported source form.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The S3 checker now refuses mixed ordinary and quantified bounds before guard or bound lowering; the adverse package test asserts one `time-invariant-shape` refusal and no partial timed model. |
| FND-002 | fixed | The public type documentation now states that each bound is ordinary or quantified. |

| FND-001 | fixed | Rechecked at `e0af4b3d`: mixed quantified and ordinary bounds still refuse before lowering, and the adverse test still asserts no partial timed output. |
| FND-002 | fixed | Rechecked at `e0af4b3d`: public docs still state that bounds are ordinary or quantified. |
