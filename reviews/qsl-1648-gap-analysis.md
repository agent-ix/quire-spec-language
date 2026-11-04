---
id: SR-2305002
title: "Gap analysis of PR 2305: duration window bounds convert to the selected clock unit (QSL-1648)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6ac5a341f; git diff origin/main...6ac5a341f (PR 2305); FR-337-AC-15, FR-194-AC-7; tests qsl-package/src/emit/tests/temporal_aggregate.rs, qsl-semantics/src/check/temporal_clause/tests.rs, qsl-replay/src/spine/clause/tests/temporal.rs; ticket QSL-1648"
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1648. Planless, diff-scoped gap analysis (Plan completion: not assessed). Manual AC-to-test check over the changed criteria; the full `quoin matrix` was not run (diff scope only).

Focused runs at 6ac5a341f: qsl-semantics `--lib temporal_clause` 17/17; qsl-package `--lib emit::tests::t` 74/74 with QSPEC_DIR set (without QSPEC_DIR two pre-existing vector tests panic on the missing checkout, environment only); qsl-replay `the_spine_converts_a_duration_window_to_the_selected_clock_unit` 1/1 passed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337-AC-15 (clock-unit conversion clause) | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:519 | examined |
| FR-194 window unit-mismatch sentence | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:75-78 | examined |
| a_duration_window_converts_both_bounds_to_the_selected_clock_unit | qsl-package/src/emit/tests/temporal_aggregate.rs:368-410 | examined |
| a_duration_bound_of_another_time_dimension_than_the_clock_refuses | qsl-package/src/emit/tests/temporal_aggregate.rs:412-429 | examined |
| aggregate_atoms_are_checked_and_admitted (unit-mismatch loop) | qsl-semantics/src/check/temporal_clause/tests.rs:542-556 | examined |
| the_spine_converts_a_duration_window_to_the_selected_clock_unit | qsl-replay/src/spine/clause/tests/temporal.rs:3987-4096 | examined |
| clause_clock_selections (CLI join) | src/emit_checked_v2.rs:563-581 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `emit_checked_v2` CLI path (`clause_clock_selections`, the name-to-span join that gives S3 the clock) has no test. A wrong join silently falls back to the lower bound's unit, so the CLI would refuse InvalidTemporalClockValue for `past[0 M::s, 300000 M::ms]` under an ms clock while every library test still passes. Fix: one tests/it/emit_checked_v2.rs case with a timestamped-event selection by clause name and a mixed-unit duration window, asserting `0`/`300000` of the ms node. | src/emit_checked_v2.rs:447-455,563-581 |
| FND-002 | low | The new conversion test asserts only the clock unit node; it does not assert that the unit's model-owned closure (the `Time` dimension and, under an ms clock, the `s` target node) is written, which is the other half of the UnheldNominal fix and is unchecked by any read-back while QSL-1644 is open. Fix: assert the dimension (and target) node is present with a Model owner and `generated` occurrence. | qsl-package/src/emit/tests/temporal_aggregate.rs:390-409 |

## Verdict

Conditional on FND-001. FR-337-AC-15's new clause is backed by the qsl-package test (both ms and s clocks, mixed written units, exact `1/1000`) and by the spine test (selection threaded through parse, select, check, package); FR-194's unit-mismatch sentence is backed by the qsl-semantics loop (`past[0 m, 1 m]`, `past[0 s, 1 m]`) and the cross-owner test. No stub or coverage inflation found. The FR-194 trace on the cross-owner test points at FR-194-AC-7, which does not state that case (spec-review SR-2305003 FND-001).
