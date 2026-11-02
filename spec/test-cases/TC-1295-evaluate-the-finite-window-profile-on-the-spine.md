---
id: TC-1295
title: "Evaluate a temporal clause under the finite-window profile on the spine"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
---
# TC-1295: Evaluate a temporal clause under the finite-window profile on the spine

## Description

Verify FR-501-AC-1 to AC-17 and FR-330-AC-9 (the finite-window part): the
spine evaluator returns a definite verdict on a closed complete window, pending
or indeterminate only for a genuinely unresolved obligation, and never refuses
the profile.

Catches a `ProfileNotEvaluated` refusal, false extension applied to a sparse
window, an empty window left unknown, a missing valuation read as false, an
incomplete input read as closed, an event at the inclusive deadline dropped, a
watermark that fails to settle a silent window, and a result that depends on
ingestion order.

## Test Procedure

Each row compiles a unit selecting
`quire.temporal.timestamped-event.finite-window/v1` (clock `ms`, `Exact`
cover unless stated), builds the `TimestampedTrace` from the stated stamps,
valuations and progress, and runs it through the spine evaluator and through
`run_clause`. Stamps are in ms. `W` is the watermark, `C(t)` is
`Complete { through: t }`, `Inc` is `Incomplete`, scope is `Closed` unless
stated, and `p`, `q` are `holds` atoms of the fixture.

| # | Clause (`on origin` unless stated) | Trace and progress | Expected |
| --- | --- | --- | --- |
| 1 | `always[0,5] holds(p)` | stamps 0,2,5; p = T,F,T; W 5; C(5) | `Completed(false)` at position 0 |
| 2 | same | p = T,T,T | `Completed(true)` |
| 3 | `eventually[0,1] holds(p)` | stamps 0,2,5; p = F,T,F; W 5; C(5) | `Completed(false)` at 0; with `[0,2]` `Completed(true)` |
| 4 | `always[0,1] holds(p)` | stamp 0; p T; W 1; C(1) | `Completed(true)`; the event-position control gives `Completed(false)` |
| 5 | `eventually[2,3] holds(p)`; then `always[2,3] holds(p)` | stamp 0; p T; W 3; C(3) | `Completed(false)`; `Completed(true)` |
| 6 | `eventually[0,5] holds(p)`, scope `Open` | stamps 0,1; p F,F; W 1; C(1) | `Pending {where 0, deadline 5}`; then W 5, C(5), no new event: `Completed(false)` |
| 7 | `eventually[0,5] holds(p)`; `always[0,5] holds(p)`, scope `Open` | stamps 0,1; W 1; C(1); p true at 1; p false at 1 | `Completed(true)`; `Completed(false)` |
| 8 | `eventually[0,5] holds(p)` | stamps 0,2,5; p F,absent,F; W 5; C(5) | `Indeterminate {MissingValuation, position 1, p}` |
| 9 | same, p true at 5 | p F,absent,T | `Completed(true)` |
| 10 | same, p admitted both T and F at stamp 2, F elsewhere | W 5; C(5) | `Indeterminate {ConflictingValuation, position 1, p}` |
| 11 | row 8 with W 3 | C(3) | `Pending {deadline 5}` |
| 12 | `always[0,5] holds(p)` | stamp 0; p T; W 5; Inc | `Indeterminate {IncompleteInput}`; with C(5) `Completed(true)` |
| 13 | `eventually[2,3] holds(p)` | stamp 0; W 3; Inc; then C(2) | `Indeterminate {IncompleteInput}` both |
| 14 | `eventually[0,3] holds(q)` | stamps 0,3; q at 3; W 3; C(3) | `Completed(true)` |
| 15 | same | stamps 0,4; q at 4; W 4; C(4) | `Completed(false)` |
| 16 | `eventually[0,3] holds(p)` | stamp 0; p F; W 3; C(3); no later event | `Completed(false)` |
| 17 | `holds(p) until[0,1] holds(q)` and `release`, `since`, `triggered` forms | stamps 0,1,1; no order keys | `invalid_runtime_input`/`invalid-value` at position 2; `eventually[0,1] holds(p)` evaluates |
| 18 | row 17 `until` | order keys, one authority; then two authorities; then trace reversed in insertion order, keys unchanged | evaluates; refuses; same result as the first |
| 19 | `once[0,4] holds(p)` `on each`, activation at position 1 (future reach 0, established at W 4) | stamps 3,4; p F,F; W 4; C(4) | `HistoryCutoff`: `Indeterminate {MissingHistory}`; `AuthoritativeOrigin`: `Completed(false)`; p T at 3: `Completed(true)` for both |
| 20 | `holds(req) implies eventually[0,3000] holds(ack)`, cover `Counter{u=0}` | req at 0; ack at 2998, 3000, 3001; W, C beyond 3001 | `Completed(true)`; `Indeterminate {StraddlesBound}`; `Completed(false)`; with `Exact` cover true, true, false |
| 21 | same, `Counter{u=1}` and the same-tick pair | per QSpec `tick-cover-vectors.json` `TCV-01` to `TCV-15` | the file's verdict for each vector |
| 22 | `eventually[0,2] holds(p)` `on each` | stamps 0,10; p F,F; W 10; C(10); then p T at 0 | `Completed(false)` at 0; then `Pending {where 1, deadline 12}` |
| 23 | row 3's first clause, meter 6 then 7 | stamps 0,2,5; p F,T,F | `Incomplete` at `temporal.visit`; the verdict with 7 charges |
| 24 | `run_clause` over rows 1, 3 first case, 6, 8 | as rows | `violation`/exit 10 pos 0; `success`/0; `inconclusive`/0; `undefined`/10 cause `MissingValuation`; no `unsupported_profile` |
| 25 | the row 1 clause over `TraceInput::Finite`, and an event-position clause over `Timestamped` | none | stage `admit`, `invalid_runtime_input`/`invalid-value` each |
| 26 | rows 1 to 22 run twice | same inputs | equal outcomes and one result identity |
| 27 | row 20's clause, cover `Counter{u=1}`; then `Exact` | req at 0; no ack; W 3001, C(3002); W 3002, C(3002); then Exact W 3000, C(3000) | `Pending {deadline 3002}`; `Completed(false)`; `Completed(false)` |
| 28 | `always[0,5] holds(p)`, scope `Closed` then `Open`; `eventually[0,5] holds(p)` | stamps 0,2,5; p T,F,T; W 5; C(5); and p F,F,T | basis `closed-scope` / `decisive-counterexample`, verdict false both, support {(1,p)}; second clause basis `closed-scope`, true, support {(2,p)}; `Pending` basis `unsettled`; `Indeterminate` basis `unavailable` |
| 29 | row 28 with scope, surrounding execution, completeness and a work stop varied one at a time | as row 28 | only the named field (and the basis for scope, the verdict class for completeness as rows 12 and 13 state) changes; a work stop reports assessment `Failed` |
| 30 | row 1's request varied one input at a time: unit, cover, one stamp, one valuation, one order key, W, through, lower-boundary kind, a meter ceiling | each alone | a distinct result identity each time; the original identity is unchanged |
| 31 | `eventually[0,5] holds(p)`; then the same clause with an empty trace; then stamps 0,5,2 | stamps 0,2,5; p F,F,F; W 5, C(9); W 3, C(9) | `Completed(false)`; `Pending {deadline 5}`; stage `admit` `invalid_runtime_input`/`invalid-value` for the empty trace and for 0,5,2 |

## Expected Results

- Outcomes are asserted as fields (variant, position, deadline, reason), never
  message text.
- No row returns a truth value for a case the table marks `Pending` or
  `Indeterminate`, and no row marked definite returns `Pending` or
  `Indeterminate`.
- The independent expected values come from the authored stamps and QSpec's
  vectors, never from the evaluator's own output.
