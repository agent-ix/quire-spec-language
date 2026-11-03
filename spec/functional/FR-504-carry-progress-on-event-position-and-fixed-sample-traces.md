---
id: FR-504
title: "Carry a watermark, completeness and scope closure on event-position and fixed-sample traces, and settle silent deadlines from them"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-094
    type: depends_on
---
# FR-504: Carry a watermark, completeness and scope closure on event-position and fixed-sample traces, and settle silent deadlines from them

## Description

`run_clause` (FR-330) SHALL accept, for a clause under
`quire.temporal.event-position.false-extension/v1` or
`quire.temporal.fixed-sample.false-extension/v1`, a finite trace together with a
progress assertion, and SHALL evaluate it with the progress rules of FR-043
"Progress, closure and settlement" (FR-043-AC-7, AC-8, AC-11): a fixed-sample
watermark advances the clock through a deadline with no business event, and an
event-position watermark is a retained premise that advances nothing. This is
the spine carrier for the legacy behavior FR-043-AC-11 and replaces the GAP that
the absence of a carrier left.

## Use case

A sampler records one sample at tick 0 and then goes quiet; its watermark later
reaches tick 3. The clause "p within 3 samples" is answered false at the
watermark with no further event. An event-ordered log with the same silence stays
pending, because event position does not advance in silence.

## Inputs

`TraceInput::Progressed { trace: Vec<DocumentRef>, progress }` (FR-330), where
`progress` holds:

- `watermark`: a non-negative integer `W`;
- `completeness`: `Complete { through }` (a non-negative integer) or `Incomplete`;
- `scope`: `Closed` or `Open`, the decision scope's closure;
- `execution`: `Closed` or `Open`, the surrounding execution's closure; and
- `lower_boundary`: `AuthoritativeOrigin` or `HistoryCutoff`.

`TraceInput::Progressed` also carries a required `binding`, a `TraceBinding` that
the evaluator compares with the clause's profile and clock rows before it visits
any position (FR-522).

Position `i` of `trace` has coordinate `i`: under event-position the sequence
index, under fixed-sample the sample index. `Finite` without `progress` is
unchanged: FR-327 reads it as a closed complete trace from an authoritative
origin.

## Outputs

FR-501's outcomes and result report (`Completed`, `Pending`, `Indeterminate`,
basis, decision support, premises, result identity), with `Pending.deadline`
defined below.

## Behavior

- **Seen horizon.** The **seen horizon** `S` SHALL be, under fixed-sample,
  `min(W, through)` when `completeness` is `Complete { through }` and `S = n - 1`
  (the last position) when it is `Incomplete`; under event-position, `S = n - 1`
  always, because the watermark does not advance the clock in silence (QSpec
  FR-094) and is only retained as a premise.
- **Silent samples (fixed-sample only).** The evaluator SHALL read samples `n` to
  `S` with no observation with every `holds` atom false and every constant unchanged, because a
  progress assertion advances sample ticks even when no business predicate is
  true.
- **Closed scope, complete input.** When `scope` is `Closed` and the input is
  complete (`Complete { through }` with `through >= S`), positions after `S` read
  by false extension and the clause is evaluated as FR-327 states, with the
  silent samples included; the basis is `closed-scope`.
- **Incomplete input.** When `completeness` is `Incomplete`, the evaluator SHALL
  NOT apply false extension even where `scope` is `Closed` (FR-043-AC-8) and SHALL
  return `Indeterminate { IncompleteInput }` where the verdict depends on a
  position after `S`.
- **Open scope.** When `scope` is `Open`, positions after `S` are an unknown
  continuation: the evaluator SHALL return a definite verdict only when every
  continuation preserves it (a `decisive-witness` or `decisive-counterexample`,
  which includes a window wholly inside `S`) and otherwise `Pending`. A fixed-sample
  `Pending` carries `deadline` = the anchor coordinate plus the clause's reach
  (FR-501 "Established window", margin 0), the least `W` that settles it; an
  event-position `Pending` carries `deadline` `NextEvent`, because only a further
  observation can resolve it.
- **Deadline participation.** A position at coordinate `D` participates when
  `W >= D` (fixed-sample); a position at `D + 1` does not.
- **Missing valuations and history.** The evaluator SHALL give a missing
  valuation a possibility set, as FR-501 "Verdicts" does.
- Position `i` of these traces has coordinate `i`, so no two positions share a
  coordinate and no atom has two valuations at one coordinate; the evaluator has
  no conflicting-valuation case for them.
- The `Indeterminate` of these traces SHALL carry the `instance` and
  `also_held_by` members of FR-501's `MissingValuation` reason for an
  instance-bound atom of an evidence call (FR-524).
- The evaluator SHALL return `Indeterminate` where no progress can resolve the
  verdict (fixed-sample: `W` has reached the anchor coordinate plus the reach;
  event-position: `scope` is `Closed`), and `Pending` otherwise. A past operator reaching before position 0 under
  `HistoryCutoff` SHALL return `Indeterminate { MissingHistory }` unless the
  verdict is already singleton; under `AuthoritativeOrigin` it reads as FR-327.
- **Consistency.** The evaluator SHALL refuse at stage `admit`,
  `invalid_runtime_input`/`invalid-value` a fixed-sample `watermark` below the
  last position's coordinate `n - 1`, and a `Complete { through }` below the
  coordinate of the last position it covers, and SHALL NOT shorten either
  silently.
- **Ledger.** FR-502 SHALL govern contradiction for these traces: watermark,
  completeness and closure are monotonic under one binding, and an event-position
  watermark that advances is retained and settles nothing.
- A `Lasso` or `Model` trace, and an infinite-trace clause over `Finite`, carry
  no progress, because a lasso is already the whole infinite behavior and a
  finite prefix under infinite-trace is `Pending` by construction (FR-328).
- When a request supplies `Progressed` for any of them, the evaluator SHALL
  refuse it at stage `admit`, `invalid_runtime_input`/`invalid-value`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-504-AC-1 | Fixed-sample clause `eventually[0,3] holds(p)` `on origin` over one position with `p` false: scope `Open`, `W` 0, `Complete { through: 0 }` is `Pending { where: 0, deadline: 3 }`; `W` 3, `Complete { through: 3 }` is `Completed(false)` with no new observation, basis `decisive-counterexample`; scope `Closed` gives basis `closed-scope`. | Test (TC-1299) |
| FR-504-AC-2 | The same clause and trace under event-position, scope `Open`, `W` 3, `Complete { through: 3 }` is `Pending { deadline: NextEvent }`, and the premises list `W` 3; with scope `Closed` and `Complete { through: 0 }` it is `Completed(false)` by FR-327's false extension. | Test (TC-1299) |
| FR-504-AC-3 | Fixed-sample, four positions with `p` true only at index 3, `eventually[0,3] holds(p)`, scope `Open`, `W` 3, `Complete { through: 3 }` is `Completed(true)`; with `p` true only at a fifth position (index 4) and `W` 4 it is `Completed(false)`. | Test (TC-1299) |
| FR-504-AC-4 | Fixed-sample, one position `p` true, scope `Closed`, `W` 5, `Incomplete`: `always[0,3] holds(p)` is `Indeterminate { IncompleteInput }` and not `Completed(true)`; the same with `Complete { through: 5 }` is `Completed(false)` (the silent samples read `p` false). | Test (TC-1299) |
| FR-504-AC-5 | Event-position, three positions, scope `Open`: `eventually[0,3] holds(p)` with `p` true at index 1 is `Completed(true)` with basis `decisive-witness`; `always[0,3] holds(p)` with `p` false at index 1 is `Completed(false)` with basis `decisive-counterexample`; `always[0,3] holds(p)` with `p` true at 0, 1, 2 is `Pending { deadline: NextEvent }`. | Test (TC-1299) |
| FR-504-AC-6 | Event-position, two positions, `once[0,2] holds(p)` `on each`, `p` false at both: `HistoryCutoff` gives `Indeterminate { MissingHistory }` with `where` 0 (the first activation in position order reaches before the trace), `AuthoritativeOrigin` gives `Completed(false)`. | Test (TC-1299) |
| FR-504-AC-7 | A `Finite` trace with no `progress` gives the outcomes of FR-327-AC-1 unchanged. A fixed-sample `Progressed` request over two positions with `watermark` 0, one over two positions with `Complete { through: 0 }`, and a `Progressed` request over a `Lasso`, a `Model` or an infinite-trace clause are each refused at stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1299) |
| FR-504-AC-8 | With a ledger, a fixed-sample request at `W` 3 followed by one at `W` 1 is refused `Watermark` (FR-502); an event-position request at `W` 3 followed by `W` 9 is accepted and its `Pending` outcome is unchanged. | Test (TC-1296, TC-1299) |

## Dependencies

- FR-043-AC-7, AC-8, AC-11 (the legacy behavior), FR-327 (the closed-trace
  evaluator), FR-330 (the entry), FR-501 (verdict classes and report), FR-502
  (contradiction).
- QSpec FR-094 (a fixed-sample progress assertion advances sample ticks;
  event-position time does not advance in silence).
