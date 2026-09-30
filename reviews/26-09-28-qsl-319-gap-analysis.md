---
id: SR-783
title: "QSL-319 gap analysis of PR 520 (value/parameter occurrence role)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/claims.rs; qsl-package/src/emit/tests.rs; qsl-semantics/src/check/lowering/tests/binder_scope.rs (unchanged); qsl-semantics/src/check/lowering/tests/differential.rs (unchanged); QSpec FR-341-AC-10 in quire-specification (reference, read only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-319. PR: quire-spec-language#520.

The PR has no plan bundle. The ticket's acceptance criterion is QSpec
FR-341-AC-10: "A `value`/`parameter` node's body is an `aggregate` of exactly
two bindings ... and every occurrence has role `expression`". QSpec owns that
requirement, so here it is checked against code and tests, not re-specified.

Traceability:

- The code for FR-341-AC-10 is `parameter()` (lowering.rs:2169). It is the
  only place a parameter node gets a non-read occurrence. Reads were already
  `expression` (lowering.rs:2763).
- The test for FR-341-AC-10 is
  `source_text_compiles_through_the_spine_and_reads_back_verified`, now tagged
  `FR-341-AC-10`. It checks the emitted wire and fails without the fix
  (mutation M3, recorded in SR-782).
- The `claims.rs` exclusion has no AC of its own. It is an internal invariant
  of ADR-012 §13.5 keying: a guard or site must resolve to exactly one
  occurrence. Existing tests cover it:
  `a_sibling_may_reuse_a_binder_name` (FR-093-AC-15, TC-415) and
  `keying_each_content_once_keeps_every_key_and_preimage` (FR-092-AC-6/11,
  TC-414) fail without it. `tc_160_guards_carry_their_outcome_outermost_first`
  and the region tests fail if it drops its location check (SR-782, M1 and
  M2). No new test is needed.
- The body-shape clauses of FR-341-AC-10 (binding order, text `name`,
  integer `level`) were already on main and are out of scope for this PR.

FR-093-AC-9 now contradicts the code. That is recorded as SR-782 FND-001, not
repeated here.

## Verdict

Mostly complete. The one behaviour the ticket asks for is implemented and
has a test that fails without it. The claims fix is covered by existing
tests. One low gap: the AC is about state-clause parameters (FR-341 is the
state-clause body requirement), but the only role assertion is on function
parameters.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-341-AC-10 role assertion only covers function parameters (`both(a, b)`). FR-341 is QSpec's state-clause body requirement, and its parameter nodes are `self`, `result` and the operation parameters of a `pre`/`post`/invariant clause, lowered by `state_clause` in `lowering/state.rs`. No test asserts role `expression` on those, or on let, query or fold binder parameters. All of them go through the same `parameter()` call, so the risk is low. But a future change that records a clause parameter somewhere else (for example the `anchor` recorded beside it at state.rs:188) would not be caught. Fix: in the existing state-clause emission test (TC-462/TC-463 in qsl-replay spine clause tests), add one assertion that every `value`/`parameter` node's occurrences have role `expression`. | qsl-package/src/emit/tests.rs:1722-1740; qsl-semantics/src/check/lowering/state.rs:160-172 |

## New findings (disposition pass 1)

Reviewed at the fix commit.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The new `s4_emits_exactly_the_fr_105_state_nodes` doc comment and inline comment say the assertion covers "the clauses' `self`, `result` and operation parameters". The ConfigVersion fixture's `attemptUpdate` declares `"params": []`, so no operation-parameter node is built. The assertion covers `self` (the invariants and the post) and `result` (the post), plus the unit's function parameters `a`/`b`. Operation parameters go through the same `parameter()` call in the `state.rs:160-162` loop, so no code path is left untested. The comment overstates coverage. Fix: drop "and operation parameters" from both comments, or add an operation parameter to the fixture. | qsl-replay/src/spine/clause/tests.rs:3597-3600; qsl-replay/src/spine/clause/tests.rs:3788-3793; qsl-replay/src/spine/clause/tests.rs:1428 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | `s4_emits_exactly_the_fr_105_state_nodes` (TC-462, now also traced to FR-341-AC-10) iterates every `value`/`parameter` node of the real ConfigVersion compile (`compiled.package.graph()`) and asserts that every one of its `graph.occurrences()` entries has role `expression`, with at least one entry per node. State-clause `self`/`result` go through `parameter()` in state.rs:162, so reverting lowering.rs:2169 to `Anchor` makes the assertion fail on their ordinal-0 entry. `make ci` passes it. |
