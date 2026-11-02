---
id: TC-1299
title: "Carry progress on event-position and fixed-sample traces"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: verifies
---
# TC-1299: Carry progress on event-position and fixed-sample traces

## Description

Verify FR-504-AC-1 to AC-8 and FR-330-AC-10: a fixed-sample watermark settles a
silent deadline, an event-position watermark is a premise that advances nothing,
an incomplete input applies no false extension, an open scope settles only on a
decisive witness or counterexample, and the ledger covers these shapes.

Catches a fixed-sample obligation left pending past its watermark, an
event-position clock advanced by silence, false extension applied to an
incomplete input, an open scope settled without a decisive fact, and a `Finite`
trace without progress that changes meaning.

## Test Procedure

Each row compiles a unit with the stated profile and runs `run_clause` over
`TraceInput::Progressed`. `Fx` is fixed-sample, `Ev` event-position. `C(t)` is
`Complete { through: t }`, `Inc` is `Incomplete`. Indices are coordinates.

| # | Clause (`on origin`) | Trace and progress | Expected |
| --- | --- | --- | --- |
| 1 | Fx `eventually[0,3] holds(p)` | 1 position, p F; Open; W 0; C(0) | `Pending {deadline 3}` |
| 2 | same | W 3; C(3); no new observation | `Completed(false)`, basis `decisive-counterexample`; with scope Closed basis `closed-scope` |
| 3 | Ev same | Open; W 3; C(3) | `Pending {deadline NextEvent}`, premises list W 3 |
| 4 | Ev same | Closed; C(0) | `Completed(false)` (FR-327 false extension) |
| 5 | Fx `eventually[0,3] holds(p)` | 4 positions, p T at index 3; Open; W 3; C(3) | `Completed(true)` |
| 6 | same | 5 positions, p T only at index 4; Open; W 4; C(4) | `Completed(false)` |
| 7 | Fx `always[0,3] holds(p)` | 1 position p T; Closed; W 5; Inc | `Indeterminate {IncompleteInput}` |
| 8 | same | Closed; W 5; C(5) | `Completed(false)` |
| 9 | Ev `eventually[0,3] holds(p)` | 3 positions; p T at 1; Open | `Completed(true)`, `decisive-witness` |
| 10 | Ev `always[0,3] holds(p)` | 3 positions; p F at 1; Open | `Completed(false)`, `decisive-counterexample` |
| 11 | Ev same clause | p T at 0,1,2; Open | `Pending {deadline NextEvent}` |
| 12 | Ev `once[0,2] holds(p)` `on each` | 2 positions, p F | `HistoryCutoff`: `Indeterminate {MissingHistory}` with `where` 0; `AuthoritativeOrigin`: `Completed(false)` |
| 13 | row 5's clause over `Finite` with no progress | 4 positions | the FR-327-AC-1 outcomes unchanged |
| 14 | Fx over 2 positions: W 0; then `C(0)` | any | each refused at stage `admit`, `invalid_runtime_input`/`invalid-value` |
| 15 | `Progressed` over a `Lasso`, a `Model`, an infinite-trace clause over `Finite` | any | stage `admit`, `invalid_runtime_input`/`invalid-value` each |
| 16 | with a ledger: Fx at W 3 then W 1; Ev at W 3 then W 9 | row 1 trace | `Watermark` contradiction; accepted with `Pending` unchanged |
| 17 | `run_clause` for rows 2 and 3 | as rows | the updated ledger returned; outcomes as rows |

## Expected Results

- Outcomes, deadlines and bases are asserted as fields, never message text.
- Expected values come from the stated coordinates and the FR-043 reading of
  silence, never from the evaluator's output.
