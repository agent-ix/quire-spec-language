---
id: TC-1318
title: "Count temporal evidence work on thirteen named counters and keep a stopped instance apart from a stopped declaration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: verifies
---
# TC-1318: Count temporal evidence work on thirteen named counters and keep a stopped instance apart from a stopped declaration

## Description

Verify FR-523-AC-1 to FR-523-AC-24. Each counter is driven to its exact usage and to one below. The vectors of FR-501-AC-12 and FR-516-AC-7 are referenced, not copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | each `temporal_evidence.<counter>` setting | accepted; sets only that counter |
| 2 | 2 | `temporal_evidence.formula_depth`; `temporal_evidence.depth`; the bare name; a non-numeric value | each refused as an unknown or non-numeric setting |
| 3 | 3 | **R** timestamped-event `on origin` `eventually[0,5] holds(p)`, one origin, three eligible positions | `formula_nodes` 2, `instances` 1, `positions` 3, `valuations` 3, `captures` 0, `evaluation_steps` 8, `visited` 8 |
| 4 | 4a | `formula_nodes`, `instances`, `positions`, `valuations` at exact usage | complete |
| 5 | 4b | `captures`, `support`, `history_span`, `evaluation_steps` at exact usage | complete |
| 6 | 4c | `lineage`, `visited`, `request_bytes`, `result_bytes`, `member_bytes` at exact usage | complete |
| 7 | 5a | `formula_nodes`, `instances`, `positions`, `valuations` one below | `Incomplete` naming the counter and setting, no truth |
| 8 | 5b | `captures`, `support`, `history_span`, `evaluation_steps` one below | `Incomplete` naming the counter and setting, no truth |
| 9 | 5c | `lineage`, `visited`, `request_bytes`, `result_bytes`, `member_bytes` one below | `Incomplete` naming the counter and setting, no truth |
| 10 | 6 | a formula sharing one subformula between two parents | shared node counted once |
| 11 | 7 | `positions` 0 | `Incomplete` naming `positions` at the first scan |
| 12 | 8 | a ceiling raised above its default | used as given; not clamped |
| 13 | 9 | a refused charge | usage at its last successful value |
| 14 | 10 | a call one below the exact usage; row 24's call with a meter ceiling of 9 | charge log equals the successful prefix; the log holds nine charges and the stop is the lookup at position 0 |
| 15 | 11 | row 3's call retried with sufficient ceilings after a row 5 stop | byte-equal to a call that never stopped |
| 16 | 12 | `history_span` above its ceiling | `Incomplete` result naming `history_span`, `Unavailable`, no truth; no instance assessed |
| 17 | 13 | `formula_nodes` above its ceiling | `Incomplete` result naming `formula_nodes`, `Unavailable`, no truth; no instance assessed |
| 18 | 14 | two calls for `t1` and `t2` of **R**'s `on each` clause sharing one `Meter` whose ceiling is the first call's usage plus one | first call keeps truth, basis and support; second `Incomplete` at `temporal.visit` naming `t2` |
| 19 | 15 | `evaluation_steps` 1000 with the meter one below usage; the meter unbounded with `evaluation_steps` one below | `Incomplete` at `temporal.visit`; `Incomplete` naming `evaluation_steps` |
| 20 | 16 | `t0 + Hf + m` above `u64::MAX`; equal to `u64::MAX` | `Incomplete` naming `history_span`, empty charge log; evaluates |
| 21 | 17 | a formula nested 100,000 deep, ceilings raised, on a 512 KiB stack | evaluates |
| 22 | 18 | the `usage` of a call; two requests differing in one ceiling | all thirteen counters reported; distinct `result_identity` |
| 23 | 20 | ledger holding `t1`; redelivery of `t1`; new healthy delivery; new faulted delivery | `instances` usage 1 |
| 24 | 21 | fixed-sample `on origin` `eventually[0,3] holds(p)`, `p` false at the one observed position 0, watermark 3, complete through 3, closed | `formula_nodes` 2, `positions` 1, `valuations` 4, `evaluation_steps` 4, `visited` 7, meter usage at `temporal.visit` 13 in the Charge order (3 materialization, origin, activation step, scan at 0, 3 silent scans, lookup at 0, 3 silent lookups); a meter ceiling of 9 stops at the tenth charge, the non-silent lookup at 0, with the log of nine |
| 25 | 22 | the stops of rows 7 to 9, 16, 17 and 20 | each `Ok`, `Unavailable`, `incomplete` naming the counter, no truth; no `Err` |
| 26 | 19 | the first call replayed with a fresh `Meter` after row 18's second call stopped | a result equal to the first call's result |
| 27 | 23 | each stop of rows 7 to 9, 16, 17 after an instance was created | `ledger` and `progress_ledger` equal the request's |
| 28 | 24 | fixed-sample `on origin` `eventually[0,1] holds(p)`, `p` false at the one observed position 0, watermark 3, complete through 3, closed | `formula_nodes` 2, `positions` 1, `valuations` 2, `evaluation_steps` 4, `visited` 5, meter usage 7 (1 materialization); each-silent-sample reading gives 9 |

Tag the test `#[trace("FR-523-AC-1", ..., "FR-523-AC-24", "TC-1318")]`; rows 4a to 4c carry FR-523-AC-4 and rows 5a to 5c carry FR-523-AC-5. Row 5a to 5c also carry FR-516-AC-7 where it is the same vector.

## Expected Results

Each row's Expected column.
