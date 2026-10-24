---
id: SR-14012
title: "gap analysis of STD-234 new and changed ACs against TC rows (PR #1190)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b8b3c38bd2e3ca0d6073db552ab5fc315911ddfc; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md, spec/functional/FR-252-hand-timing-obligations-to-code-generation.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/test-cases/TC-706-tick-based-monitor-plans.md, spec/test-cases/TC-707-timing-obligations-handed-to-cg.md, spec/test-cases/TC-720-a-reached-limit-names-its-kind-bound-count-and-setting.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
---
# SR-14012: gap analysis of STD-234 (PR #1190)

## Summary

Ticket: STD-234. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows. It covers spec only: no code for these
ACs exists yet (code lane remaining work, QSL-869), so no test is tagged.

- FR-251-AC-2 (changed) maps to TC-706 step 2, which is updated to
  `LimitReached{limit: event_buffer, value: 21, setting:
  monitor.max_event_rate}`.
- FR-252-AC-1 (renamed setting) maps to TC-707 step 1, updated.
- FR-252-AC-2 (changed) maps to TC-707 step 2, not updated (FND-001).
- FR-252-AC-3 (new) maps to TC-707 step 3, added; Scope is now AC-1 to AC-3.
- FR-255-AC-3 (changed) maps to TC-720 step 3, not updated (FND-002).
- FR-391-AC-6 (changed) maps to TC-1076 step 8, updated (its fixture gap is
  SR-14010 FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-252-AC-2 now requires `timestamp_parameters` `{now, start}` for the `TimestampContract` obligation, but TC-707 step 2 still reads "Write obligations for the timestamp contract ..." with no setting. With the new default (empty), a test written from TC-707 alone gets no `TimestampContract` and contradicts the step's expected result. Fix: state `timestamp_parameters` `{now, start}` in step 2. | spec/test-cases/TC-707-timing-obligations-handed-to-cg.md:22; spec/test-cases/TC-707-timing-obligations-handed-to-cg.md:30 |
| FND-002 | low | FR-255-AC-3 now says the monitor target's `max_event_rate` is not a setting-table row. TC-720 step 3's expected result still says "their union equals the table", with neither this exception nor FR-277's. A test written from TC-720 does not check that `monitor.max_event_rate` stays out of the union. Fix: add both exceptions to step 3's expected result. | spec/test-cases/TC-720-a-reached-limit-names-its-kind-bound-count-and-setting.md:43-44 |

## Verdict

Every new or changed AC has a TC row, and the new AC FR-252-AC-3 has a new
step. Two changed ACs have TC rows whose text was not brought up to date.
Both are low.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@655c3660668f4835ba281a90d223412bb1c0784b` (fix commit `655c3660`, on top of `9f940889`, a merge of origin/main into the branch; the head contains origin/main `b5190738`). I checked each outcome against the spec text at that head, not against the commit message. Each `fixed` row quotes the fixed text.

| FND | Outcome | Reason or after excerpt |
| --- | --- | --- |
| FND-001 | fixed 655c3660 | TC-707 step 2: "Write obligations for the timestamp contract with `timestamp_parameters` keying that contract to `{now, start}`, for the elapsed-time clause (with `handler` declared nowhere) and for the WCET clause." Step 3 adds the one-parameter contract of the new FR-252-AC-3 clause. |
| FND-002 | fixed 655c3660 | TC-720 step 3 expected: "their union equals the table and the accounting table, and neither the supplied-value ceilings of FR-277 nor the monitor target's `monitor.max_event_rate` is in that union;" This matches FR-255-AC-3. |

The ACs changed by the fix commit (FR-252-AC-2 and FR-252-AC-3, FR-391-AC-6) each have a matching TC-707 or TC-1076 step.
