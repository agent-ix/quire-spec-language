---
id: SR-9058
title: "Gap analysis of quire-spec-language PR #820: QSL-756 acceptance criteria 1-3 and FR-082-AC-15"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@65cc4b55e0c01e7ea2a0031c4a22d60284d61d23; PR #820: QSL-756 AC 1-3, FR-082 (cycle paragraph), FR-082-AC-2, FR-082-AC-15 (TC-1020) against qsl-semantics/src/model/{normalize,intake,refusal}.rs and qsl-semantics/tests/it/{model_normalization,tc235_record_event,business_meanings}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
---
# Gap analysis of quire-spec-language PR #820

## Summary

Ticket: QSL-756. Plan completion: not assessed.

AC 1 (tests-first typed vectors for R01, M20, M17 run 3 and a 3-cycle):
present. Each vector asserts typed fields only, through `cycle_refusal`
(model_normalization.rs) or `assert_specialization_cycle_refusals`
(tc235_record_event.rs). The red log shows only a compile failure of the typed
vectors against the old variant. Judged by mutation reasoning instead:

- An engine that keeps one refusal per cycle fails R01 (count 2), both M20
  sites and M17 run 3 (the helper asserts the count), the multi-cycle vector
  (4) and the 3-cycle vector (3).
- An engine that refuses `A` only once fails the multi-cycle vector.
- Reversing detection order fails the multi-cycle vector, which expects
  `[A, B]` before `[A, C]`.
- Starting the listing at the node the walk revisits fails the 3-cycle vector
  (`[C, D, B]` against `[B, C, D]`), and so does reversing the walk direction
  (`[B, D, C]`).
- Listing the keys in sorted order, or ordering a cycle's refusals by path
  order, passes every vector (FND-001).

AC 2 (consumer audit, no consumer breaks): re-measured by grep over every
crate, `xtask`, `tools`, `src/`, `tests/`, `fuzz/` and `examples/`. The PR
body's list is complete and accurate. No serde, Display or detail-text parsing
touches the cause. Clean.

AC 3 (lib and it N>0, no new failure, the old "exactly once" text gone): the
gate logs carry the coder's worktree on their first line, postdate the head
commit and exit 0. They show sem lib 522 passed / 1 ignored and it 656 passed /
1 ignored, xtask 182, the root crate, qsl-eval/replay/package/forms, workspace
clippy `-D warnings`, fmt and spec validation. No "exactly once" cycle wording
remains in qsl-semantics or spec. Clean, for the head as pushed. The base has
since taken #817; the merge is textually clean, but the gate needs its
pre-merge re-run on the rebased head.

Charge accounting: unchanged. Deduplication and sort code are untouched, and
the `work_units=46` and `h2` vectors keep their charge expectations.

## Verdict

Changes requested: FR-082-AC-15's order and listing rules are not fully
pinned (FND-001), the record path has no multi-cycle vector (FND-002), and
event-type and state-machine cycles look unrouted (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | In every cycle the tests use, path order equals sorted key order: all 2-cycles, and `B -> C -> D -> B`. An engine that lists the cycle's keys sorted instead of in rotated path order passes every vector, and so does one that orders a cycle's refusals by path order instead of declaration key. Add a 3-cycle whose path order is not sorted, for example `B -> D -> C -> B`, which lists `[B, D, C]` with refusals at `B`, `C`, `D`. FR-082-AC-15's own vector has the same blind spot (see the spec review). | qsl-semantics/tests/it/model_normalization.rs:5055-5079 |
| FND-002 | medium | The record path (M17, `business_meanings.rs` record cycle) is tested only with one 2-cycle. No record vector has a declaration on two cycles or a 3+ cycle whose DFS choice matters, so the record path's single refusal per declaration (code review SR-9057 FND-001) passes every test. | qsl-semantics/tests/it/tc235_record_event.rs:1179-1187; qsl-semantics/tests/it/business_meanings.rs:946-987 |
| FND-003 | medium | FR-082 requires a specialization-cycle refusal for every declaration on a cycle. Event types and state machines accept same-meaning supertypes, but `business_generalization_refusal` maps only record-value cycles, and the process path is routed to normalization. On a code read, an event-type or state-machine cycle diagnostic falls through to the generic `malformed_declaration` ("refused this document") branch. No test covers either meaning. Not reproduced (reviewer-only, no builds). The fix shares FND-001's mechanism, so it belongs in the same change. | qsl-semantics/src/model/intake.rs:2034-2073; qsl-semantics/src/model/intake.rs:1439-1470 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e6c8822b2441bd96beeb6c2bb87066fd776109c8: `a_cycle_lists_path_order_and_refuses_in_declaration_key_order` (object types) and `a_record_cycle_lists_path_order_and_refuses_in_key_order` (records) cover `B -> D -> C -> B`, listing `[B, D, C]` with refusals at B, C, D. |
| FND-002 | fixed | e6c8822b2441bd96beeb6c2bb87066fd776109c8: `a_record_declaration_on_two_cycles_is_refused_once_per_cycle` expects RecA,RecB `[RecA, RecB]` then RecA,RecC `[RecA, RecC]`. It was red before the fix. |
| FND-003 | fixed | e6c8822b2441bd96beeb6c2bb87066fd776109c8: event-type and state-machine cycle diagnostics are dropped at intake and refused by normalization as typed `specialization-cycle`. `an_event_type_cycle_is_a_specialization_cycle` and `a_state_machine_cycle_is_a_specialization_cycle` fail on any other refusal, and both were red before the fix. |
