---
id: SR-16359
title: "Gap analysis of QSL-1018 named evaluation meter stops"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1410 (branch qsl-1018-impl); FR-126-AC-19, FR-127-AC-16, FR-192-AC-1, FR-255-AC-9; qsl-analyze/tests/it/{model_check,model_settle}.rs, qsl-replay/src/statistical/settle.rs"
review_set: subset
---

## Summary

Ticket: QSL-1018. Review pass (round 0) by run f859fcbd-414c-44f6-9b9f-a7bb3332a988
at frozen head 6777c521f; this file was reconstructed from that run's posted
Linear comment, which carried the authoritative findings. Disposition round 1
checks fix commits e9a552614 and e3fbb05aa (head e3fbb05aa).

Plan completion: not assessed.

## Verdict

**FIXED / DEFERRED (round 1).** FND-001 and FND-002 are fixed by new or extended
tests. FND-003 is fixed for `accounting.work_units` and deferred to QSL-1026
for `expansion.max_candidates`: the settings operation (qsl-foundation
setting table, CallerLimits::from_operands) has no `expansion.*` row on main,
so that half is a pre-existing gap in FR-255's settings operation, not in this
PR's scope. Residual note, not a defect raised here: the FR-127-AC-16 clause
"each record equals the one FR-203 writes" has no FR-203 writer in the code to
compare against.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-127-AC-16 has no test of the model-check settlement map and its four specified records | qsl-analyze/tests/it/model_settle.rs:609 |
| FND-002 | medium | FR-192-AC-1 statistical test omits value_occurrences 7 and candidate bound 2 | qsl-replay/src/statistical/settle.rs:453 |
| FND-003 | medium | FR-126-AC-19 does not exercise raising the reached bound through the settings operation | qsl-analyze/tests/it/model_check.rs:649 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New a_meter_or_candidate_stop_settles_naming_its_setting (TC-522, FR-127-AC-16) settles all four stops (work_units 0, value_occurrences 7, integer_bits u64::MAX, MaxCandidates 1) and asserts failed/unavailable/resource-incomplete/Incomplete(LimitReached)/category incomplete and each FR-255 setting name. |
| FND-002 | fixed | settle.rs test meter now value_occurrences 7 and max_candidates 2, with value_occurrences and max_candidates rows asserting limit, value and setting on the wire. |
| FND-003 | deferred | QSL-1026. accounting.work_units=1000000 half fixed by raising_the_reached_bound_lets_the_run_continue; the expansion.max_candidates=1000000 half cannot go through the settings operation because the setting table has no expansion.* row on main (pre-existing FR-255 gap needing a spec ownership check and new CallerLimits plumbing); the test raises it via ExpansionLimits meanwhile. |
