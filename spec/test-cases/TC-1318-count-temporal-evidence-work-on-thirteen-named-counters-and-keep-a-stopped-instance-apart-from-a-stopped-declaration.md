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

Verify FR-523-AC-1 to FR-523-AC-12. Each counter is driven to its exact usage and to one below. The vectors of FR-501-AC-12 and FR-516-AC-7 are referenced, not copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | each `temporal_evidence.<counter>` setting; `temporal_evidence.depth`; the bare name; a non-numeric value | accepted and sets only that counter; the three refused as unknown or non-numeric |
| 2 | 2 | **R** timestamped-event `eventually[0,5] holds(p)`, three eligible positions; `evaluation_steps` 7 then 6 | usage 7, 3 and 3 for `evaluation_steps`, `positions`, `valuations`; completes; `Incomplete` naming `evaluation_steps` and `temporal_evidence.evaluation_steps`, no truth |
| 3 | 3 | the twelve other counters at exact usage and one below | completes; `Incomplete` naming the counter, no truth |
| 4 | 4 | a formula sharing one subformula between two parents; `always[0,1] (holds(p) implies eventually[0,1] holds(q))` | shared node counted once; `formula_depth` 4 |
| 5 | 5 | `positions` 0; a ceiling raised above its default | `Incomplete` naming `positions` at the first inspection; the raised ceiling used as given |
| 6 | 6 | the call one below exact usage | usage at its last successful value; charge log equals the successful prefix |
| 7 | 7 | row 2's call retried with sufficient ceilings | byte-equal to a call that never stopped |
| 8 | 8 | `history_span` above its ceiling; `formula_nodes` above its ceiling | declaration stopped before any instance; no instance carries a result |
| 9 | 9 | two `on each` activations with `evaluation_steps` at the first's usage plus one; a third activation | first keeps truth, basis and support; second `Incomplete` naming `evaluation_steps` and its instance; third `Unactivated`, no truth |
| 10 | 10 | `t0 + Hf + m` above the `u64` tick domain; one below | `Incomplete` naming `history_span`, no truth; evaluates |
| 11 | 11 | two requests differing in one ceiling; the `usage` of a call | distinct `result_identity`; all thirteen counters reported |
| 12 | 12 | a formula nested 100,000 deep, ceilings raised, on a 512 KiB stack; `formula_depth` one below | evaluates; `Incomplete` naming `formula_depth` |

Tag the test `#[trace("FR-523-AC-1", ..., "FR-523-AC-12", "TC-1318")]`; it also carries FR-516-AC-7 where it is the same vector.

## Expected Results

Each row's Expected column.
