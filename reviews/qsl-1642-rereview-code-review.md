---
id: SR-20261104130239
title: "Code review (PR #2294 re-review) — QSL-1642 timed population refusal"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2294 head dd4c417ee: qsl-semantics/src/check/timed_model.rs, qsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

Ticket: QSL-1642. Independent review of PR #2294 at head dd4c417ee, Rust lane (rust-review) folded in. The change adds a pre-pass `named_population_read` in `typed_alternative_in`, which serves only `time invariant when` and `urgent when` predicates (operation guards go through `check_part`, `max`-bound invariants have predicate `true`). It refuses a declared named population with the existing typed cause `TemporalCause::TimedPredicate { cause: CheckCause::PopulationFormNotChecked { what: "timed predicate population scope" }, span }`, which maps to `unsupported_construct`/`not-yet-implemented` at the population reference. On main the same input was refused late as `missing_declaration`/missing-segment by the generic FR-391 "no population scope" path in typing.rs, so the PR turns a misleading diagnostic into a typed, located one. No enum variant is added or removed, so no match in the workspace needs updating (existing `PopulationFormNotChecked` arms in qsl-replay refinement.rs:322 and hyper_clause.rs:1964 are unaffected).

## Verdict

**PASS with findings** — correct on the tested path; the reported span is the last, not the first, population reference when a predicate names more than one, contrary to FR-230's new statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `named_population_read` walks a LIFO stack (`pending.pop()` after `pending.extend(expression.children())`, children in location order), so with two population references it reports the last one, while FR-230 says the refusal is "at the first population reference". Untested: both test predicates contain one reference. Fix: push children reversed (`pending.extend(expression.children().into_iter().rev())`) and add a two-population case. | qsl-semantics/src/check/timed_model.rs:4648-4673 |
| FND-002 | low | `match expression.node()` ends in `_ => None` over the large `ExprNode` enum; a future population-reading variant would silently skip the refusal and fall back to the old missing-segment diagnostic. Consider listing the population-carrying variants exhaustively or adding a comment that typing.rs remains the backstop. | qsl-semantics/src/check/timed_model.rs:4652-4665 |

## Scope

- Examined: `typed_alternative_in` guard and `named_population_read` traversal (timed_model.rs:4604-4675).
- Examined: callers of `typed_alternative_in` (invariant from-bound, invariant, urgency-when) and `check_part` for guards (not affected).
- Examined: `CheckCause::PopulationFormNotChecked` code/tag mapping (refusal.rs:2724, 3283, 3360) and `TemporalCause::TimedPredicate` span/code delegation.
- Examined: workspace matches on `CheckCause::PopulationFormNotChecked` (qsl-replay/src/spine/refinement.rs:322, qsl-semantics/src/check/hyper_clause.rs:1964); no new variants.
- Examined: test `timed_predicates_refuse_named_populations_without_restricting_state_clauses` (tests/it/timed_model.rs:290-337): asserts code, tag, located text, typed cause, and the positive controls.

## Verification

Receipts on head dd4c417ee recorded in the reviewer hand-off report.
