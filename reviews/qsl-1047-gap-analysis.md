---
id: SR-17049
title: "Gap analysis of QSL-1047 FR-177-AC-8..12 clause-complete tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1485 (branch qsl-1047-fr177, head 9ab89aeed); FR-177-AC-8..12, TC-602; qsl-analyze/tests/it/hyper_witness.rs, qsl-analyze/tests/it/hyper_world.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: references
---

## Summary

Ticket: QSL-1047. This is a planless gap analysis scoped to the PR's five
criteria. `quoin matrix --json` (quoin 0.29.1) at head 9ab89aeed reports each
of FR-177-AC-8, AC-9, AC-10, AC-11 and AC-12 as `tagged`, each with one binder,
all in qsl-analyze/tests/it/hyper_witness.rs:
`a_late_undefined_universal_match_and_a_refused_existential_one_rank_against_the_empty_set`
(AC-8, line 1423),
`the_lasso_rotates_to_entry_zero_and_goes_on_by_a_connecting_step_when_the_loop_is_unfair`
(AC-9, line 1489), `an_undefined_universal_match_ends_one_step_short_for_the_partner`
(AC-10, line 1540),
`instances_rank_by_strictly_earlier_findings_and_a_position_zero_finding_bounds_the_rest`
(AC-11, line 1612) and
`an_undefined_universal_match_over_an_empty_set_is_refused_and_ranks_first`
(AC-12, line 1672). Evidence status is `no run evidence`, which is expected
for a matrix computed without a run record. Each trace carries both `TC-602`
and its own AC, and each doc comment names only the AC it is tagged with.

Reverse gap: the diff is test-only. No production file changes. The one
harness change is a `1 mod value(..)` arm in the hand-modelled match reader
(hyper_world.rs:970-990). It reads `1 mod x` as 0 for x = 1, undefined
(`division-by-zero`) for x = 0, and the `else 0` branch (holds) where the step
has no input. For dividend 1, truncated `checked_rem` equals Euclidean `mod`
for every divisor, so it matches QSL's Euclidean `mod`. An unmodelled shape
still panics, so the arm cannot hide a failure. No stub, no `#[ignore]`, no
`#[allow]`.

The 38 hyper_witness tests pass at the head. Three engine mutations re-run by
the reviewer, and the coder's own five, are recorded in SR-17048. The
clause-by-clause assertion quality is covered there.

## Verdict

**PASS.** All five in-scope criteria are traced to passing tests that can
fail. This method found no gap. The test-quality findings are in SR-17048.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-177-AC-8: tagged (1 binder), passes. In scope.
- FR-177-AC-9: tagged (1 binder), passes. In scope.
- FR-177-AC-10: tagged (1 binder), passes. In scope.
- FR-177-AC-11: tagged (1 binder), passes. In scope.
- FR-177-AC-12: tagged (1 binder), passes. In scope.
- FR-177-AC-1, AC-5, AC-6: context only (AC-8, AC-9 and AC-11 refer to them).
- FR-176-AC-1: out of scope. QSL-1047 leaves it for a spec ruling.
- Semantic review: covered by the code-review method (SR-17048), not run
  separately.
- Plan completion: not assessed
