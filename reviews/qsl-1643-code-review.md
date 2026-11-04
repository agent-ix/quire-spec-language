---
id: SR-20261104130232
title: 'Code review of PR #2295: window aggregate emission as quire.op.temporal.aggregate, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@1681cae5f; git diff origin/main...1681cae5f (PR #2295, branch peter/qsl-1643-window-aggregate-emission): qsl-semantics/src/check/{temporal_clause.rs,temporal_clause/condition.rs,lowering/temporal.rs,assemble.rs,check/typing.rs,mod.rs}, qsl-package/src/emit.rs, qsl-package/src/emit/tests/temporal_aggregate.rs; ticket QSL-1643'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1643. PR: quire-spec-language#2295, head 1681cae5f. Method: code-review with the rust-review lane. The PR lowers each checked window aggregate atom to a `quire.op.temporal.aggregate` formula node whose one `aggregate` argument binds kind, q, predicate, value, filter, window_kind, lower, upper, min_count, comparison and threshold in QSpec FR-370 order. It also replaces the S3 threshold check with FR-370 result-family rules and refuses at emission a `duration` window whose unit node is not the clock's `["scalar_type","unit",T]` node.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| aggregate_argument / threshold_literal | qsl-semantics/src/check/lowering/temporal.rs:602-736 | examined |
| aggregate threshold family check | qsl-semantics/src/check/temporal_clause/condition.rs:219-305 | examined |
| CheckedAggregate construction | qsl-semantics/src/check/temporal_clause.rs:1290-1445 | examined |
| infer_in_unit_of | qsl-semantics/src/check/check/typing.rs:788-813 | examined |
| duration-window clock unit refusal | qsl-package/src/emit.rs:6222-6367 | examined |
| aggregate_parts / site spans | qsl-semantics/src/check/assemble.rs:4587-4660,5791-5813 | examined |
| removal of not_emitted path | qsl-semantics/src/check/mod.rs:4051-4135 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A quantity threshold of another dimension (or a non-convertible unit) is refused `ill_typed`/`type-mismatch` by the rewritten family check, but FR-194 (its own preceding bullet), FR-194-AC-7 (`5 m` against a time value) and QSpec FR-370 step 5 require `ill_typed`/`unit-mismatch`. The test only asserts `Code::IllTyped`, so it cannot catch this. Fix: map `(Some(Family::Quantity(_)), Some(Family::Quantity(_)))` mismatch to `IllTypedCause::UnitMismatch`, and assert the tag in the test. | qsl-semantics/src/check/temporal_clause/condition.rs:283-295; qsl-semantics/src/check/temporal_clause/tests.rs:551-555 |
| FND-002 | medium | A non-integer `fraction` threshold cannot be written. `>= 1/2` is refused `ProfileProhibitedOperator(Divide)` at the threshold, and `>= 0.5` fails to parse (S1 "expected identifier"), as reproduced with a scratch probe. That is why the AG-05 vector case cannot be met byte for byte. The fault is in QSL, not in the vector: the vector's threshold is `1/2`, as written by FR-194-AC-5's `>= 1/2` and FR-516-AC-48's `2/3`. Fix: fold an exact rational literal threshold `n/d` the way the quantile `q` is folded (`probabilistic::exact_value`) into a rational literal, then test AG-05 with `>= 1/2` and drop the replace() deviation. Otherwise defer with a ticket. | qsl-semantics/src/check/temporal_clause/condition.rs:270-300; qsl-package/src/emit/tests/temporal_aggregate.rs:170-175 |
| FND-003 | low | Wildcard arms over `qsl_forms::AggregateKindForm` (the predicate extraction, the required result family, and `aggregate_parts`) would silently give a new kind the value-family threshold rule. Fix: list the kinds explicitly. | qsl-semantics/src/check/temporal_clause/condition.rs:264,281; qsl-semantics/src/check/assemble.rs:5799 |

## Verdict

Conditional. The emission is correct for the eight accepted positive vectors. Order, literal kinds, type paths and reference targets are compared in `each_aggregate_kind_emits_the_qspec_vector_argument` against `$QSPEC_DIR` (quire-specification@860e0890, main). The only deviation is AG-05, which comes from a missing source form (FND-002), not from a wrong emission: integer `1` written as rational `1` is admissible under FR-370 (threshold `rational`), and FR-370-AC-18 refuses only an `integer`-kind threshold on `fraction`. The new S3 family refusals are sound and do not over-refuse inputs that FR-370 admits: an integer literal on `fraction` is admitted, a non-literal integer on `fraction` is refused, as QSpec refuses, and a quantity threshold in a convertible unit is converted exactly through `infer_in_unit_of` and bound as a reference to the converted node, which is tested. The duration refusal is honest and consistent: FR-337 states that no `duration` window emits today (QSL-1478), the semantics test shows the exact `0`/`300000` ms lowering, and the emission test shows the refusal. Every match on the new enums (`TemporalSite::Aggregate`, `AggregatePart`, `AggregateKind`, `AggregateWindow`, `Goal::Threshold`, `Family`) is exhaustive, apart from the FND-003 wildcards over the qsl_forms enum. Evidence: `cargo test -p qsl-package --lib temporal_aggregate` 4/4, `emit::tests::temporal` 37/37, `cargo test -p qsl-semantics --lib temporal_clause` 17/17, and `cargo fmt --check` is clean (clippy not run).
