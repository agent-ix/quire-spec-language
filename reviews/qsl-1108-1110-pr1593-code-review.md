---
id: SR-60022
title: "Code and gap review of QSL PR #1593: residual tests for QSL-1108 and QSL-1110"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@775f5ad8ec69a9a076995fb2aada4a14b1ab608d (qsl-package/src/emit/tests/state_graph_claim.rs, qsl-analyze/tests/it/state_graph_settle.rs, qsl-semantics/tests/it/timed_declarations.rs; FR-410, FR-253, TC-1140, TC-708)"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-410
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: reviews
---
## Summary

Ticket: QSL-1108, QSL-1110 (QSL-1104 deliberately excluded). Verdict: PASS, no
blocker, no medium finding. The diff is test-only (three files, no production
change); the PR body matches it.

Checks run (own worktree, serialized through the shared cargo lock): the three
touched test targets pass at head (7 + 16 state-graph tests; the 6 selected
`timed_declarations` tests, which need `--features test-support`); clippy
`-D warnings` over qsl-semantics, qsl-package and qsl-analyze with all targets
and test-support is clean; `xtask string-edge` clean; `cargo fmt --check` clean.

Mutations (each reverted; worktree clean afterwards):
- M1, the `always-possible` form lowered with kind `possible`: five state_graph_claim
  tests fail, including all three new ones.
- M2, `delay()` accepting any time source (Tick): `a_delay_on_a_tick_model_refuses_at_the_declaration`
  fails (5 other selected tests pass, as expected).
- M3, the Cancelled arm of `settle_state_graph` returning `Row::failed()`:
  `a_cancelled_request_settles_incomplete_before_any_replay` fails, as does the
  existing `a_certificate_or_evidence_of_another_package_never_settles`.

Tags: TC-only is right. TC-1140 scopes FR-410-AC-1..4 and the new tests each assert
a property of the id (AC-2 asserts form-distinct ids and the true literal; the new
tests do not assert a whole AC). FR-253-AC-3 lists four refusals and a race
disposition; no new test asserts it whole. Expected values check out: `1 s` in a
`ms` model is 1000/1, exponential `rational(1, 1000)` is kept unconverted per
AC-1, `uniform[a, a]` and zero delays satisfy `0 <= a <= b`. Spans are meaningful
(the offending weight, the delay literal with its unit, the whole `uniform[...]`
range, the whole second declaration, the unresolved operation name). No existing
test duplicates the new ones (existing delay tests cover zero rate, repeated
discrete delay, backwards range and untimed model only).

QSL-1104: the reasoning is sound. `MatchReading::Refused` is built only in
`qsl-eval/src/hyper/checked.rs:77` for a non-Incomplete, non-undefined undecided
conjunct, the matcher is the concrete `CheckedStepMatch`, and
`qsl-replay/tests/it/hyper_query.rs:454` already records that the sibling arm has no
source route until float literals exist. I did not independently exhaust every
source route; the claim rests on the PR's enumeration plus that existing note.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | M1 (form ignored in the node's kind) is caught by the new id tests only through a panic in the `claim_of` lookup, not through the `assert_ne!`; the `unique path` from/to swap assertion has no mutation of its own tried here (id ignoring predicate roles). The tests can fail, but the role-sensitivity assertion is not proven by this pass. Optional: assert `claim_of` finds the claim before comparing. | qsl-package/src/emit/tests/state_graph_claim.rs:318 |
| FND-002 | low | `a_true_target_reuses_the_true_literal_and_the_over_name_enters_the_id` bundles two unrelated properties (true-literal reuse, over-name in the id). A failure of one hides the other and the name says both; consider splitting. | qsl-package/src/emit/tests/state_graph_claim.rs:381 |
| FND-003 | low | New helper `delay_cause`/`malformed_delay` overlaps the existing `delay_refusal`; two helpers now compute the refusal and span text two ways. Not worth a rewrite here; fold when the file is next touched. | qsl-semantics/tests/it/timed_declarations.rs:630 |
| FND-004 | low | FR-169/FR-410 do not say a package read-back fault settles `Failed` (the PR body's SR-17236 FND-003 note). Spec-owner follow-up, correctly not a test. | FR-410 Behavior |

## Verdict

PASS. Mergeable as is; FND-001..003 are optional polish.
