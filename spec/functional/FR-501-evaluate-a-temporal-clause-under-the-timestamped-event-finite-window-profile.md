---
id: FR-501
title: "Evaluate a temporal clause under the timestamped-event finite-window profile at S6a, with definite verdicts on a closed complete window"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-160
    type: depends_on
---
# FR-501: Evaluate a temporal clause under the timestamped-event finite-window profile at S6a, with definite verdicts on a closed complete window

## Description

The layer-5 TemporalTrace evaluator SHALL evaluate a checked temporal clause
(FR-326) selected under `quire.temporal.timestamped-event.finite-window/v1`
over a timestamped trace, and SHALL return a definite `Completed(true)` or
`Completed(false)` whenever the clause's window is closed and the evidence
through it is complete. It SHALL return `Pending` or `Indeterminate` only for
an obligation that is genuinely unresolved under QSpec FR-160's possibility
rules, the same rules every other temporal profile uses. The evaluator SHALL
NOT refuse this profile with `ProfileNotEvaluated` and SHALL NOT default any
finite-window case to unknown or not-evaluated by design. This requirement is
the spine successor of the legacy `src/temporal` finite-window behavior
(FR-043 "Operators", "Atoms, constants and boundaries", "Progress, closure and
settlement"; FR-043-AC-2, AC-8, AC-11) and states the definition QSpec's
profile text leaves to the evaluator (see "Required QSpec definition"). The
legacy `unavailable` basis is renamed `Indeterminate { MissingValuation }` and
the legacy single global anchor is replaced by FR-327's activation rule; both
renamings are accepted and no other legacy behavior is dropped (see "Result
report").

## Use case

An operator holds events stamped in milliseconds from a payment system and a
clause "every refund request is answered within 5 ms". The watermark has passed
the deadline and the recording is complete: the evaluator answers true or false
and names the first failing activation. While the watermark is short of the
deadline it answers pending, and when the closed recording still lacks a fact
that the answer depends on it says indeterminate and names the fact.

## Inputs

- A `CheckedTemporalClause` (FR-326) whose profile selection is
  `quire.temporal.timestamped-event.finite-window/v1`, the checked package and
  the binding of the clause's `over` parameter.
- A `TimestampedTrace`: a non-empty `Vec<TimestampedObservation>` in admitted
  order. Each holds an FR-106-admitted observation, its **stamp** (a
  non-negative integer count of ticks of the clock binding's declared timestamp
  unit, non-decreasing along the admitted order) and an optional **order key**
  (an order-authority identity and a key within it).
- The trace's **progress**: a watermark `W` in ticks; a **completeness**
  assertion (`Complete { through }` with `through` a tick, or `Incomplete`);
  the decision scope's closure (`Closed` or `Open`); the surrounding
  execution's closure (`Closed` or `Open`); and the lower-boundary kind
  (`AuthoritativeOrigin` or `HistoryCutoff`). `through` MAY exceed `W`: a
  completeness assertion may run ahead of progress, and the activation is then
  established only when both conditions of "Established window" hold.
- The clock binding's declared **cover** for a stamp: `Exact` (default; a stamp
  is the instant itself) or `Counter { uncertainty }` (a counter reading `k`
  covers the half-open interval `[k, k+1)` of the counter unit, widened to
  `[k-u, k+1+u)`; ADR-026 MN-2, QSpec FR-160).
- A kernel `Meter` (ADR-014 B-2).

A trace with no positions SHALL be refused `invalid_runtime_input`/
`invalid-value` at stage `admit`, as FR-327 refuses it.

## Outputs

An evaluation outcome:

- `Completed(true)`; `Completed(false)` with the `TemporalPosition` of the
  first activation that evaluated false;
- `Pending { where, deadline }`: an activation whose verdict declared progress
  could still resolve, with `where` its anchor position and `deadline` the
  first tick whose establishment would resolve it;
- `Indeterminate { where, reason }`: the window is closed and the evidence still
  leaves the verdict open, with `reason` one of `MissingValuation { position,
  atom }`, `ConflictingValuation { position, atom }`, `IncompleteInput`,
  `MissingHistory` or `StraddlesBound` (a counter cover straddling a bound,
  QSpec FR-160);
- `Undefined` and `InsufficientData` as FR-327 states them;
- `Incomplete` with its charge point (work), as FR-327 states it; and
- a refusal (below) in place of any of them.

### Result report

Every outcome other than a refusal is returned in a report that also carries,
as separate fields that no one field determines:

- the **basis**: `closed-scope` when the activation is established and the
  decision scope is `Closed`; `decisive-witness` (true) or
  `decisive-counterexample` (false) when the verdict is definite and the scope is
  `Open`, every admissible continuation preserving it; `unsettled` for `Pending`;
  `unavailable` for `Indeterminate`. A definite verdict on an activation that is
  not established is reported with `decisive-witness` or
  `decisive-counterexample` whatever the scope. Truth is absent for `unavailable`
  and a `Pending` carries no Boolean;
- the **decision support**: the set of `(position, atom)` pairs whose single
  valuation, flipped within its possibility set, or whose position, other than
  the anchor, removed from the trace, would change the lifted result of the first deciding activation
  (for `Pending` and `Indeterminate`, the pairs whose valuation would decide it);
- the **premises**: the decision-scope closure, the surrounding-execution
  closure, the assessment-execution disposition (`Completed` when the evaluator
  produced an outcome, `Failed` when it returned `Incomplete` or a refusal), the
  input completeness, the watermark, the lower-boundary kind, the clock cover and
  unit; these four dimensions are independent, so no input of one changes
  another;
- the **result identity**: a content identity over the checked clause identity,
  the profile, the clock unit and cover, every stamp, valuation and order key,
  the progress inputs and the meter ceilings. Equal inputs SHALL give one
  identity, and a change of the profile, unit, cover, any one of those inputs or a
  ceiling SHALL give a distinct identity, with the earlier result neither reused
  nor rewritten (FR-043-AC-13).

`Pending` maps to O-16 inconclusive and exits 0 (a supplied-trace clause pending
when the trace ends, FR-285). `Indeterminate` maps to the category indeterminate
with its `reason` as the cause: the trace lacks the data to decide the claim, so
it neither holds nor fails on it. Neither is `Completed(false)`, and neither is
a truth value. An `Indeterminate` exits 11, the code FR-285 gives the category,
which is neither the violation's 10 nor success's 0; in a multi-item outcome it
ranks below 10.

## Behavior

### Positions, anchor and eligible instants

- The evaluator SHALL address each position by `TemporalPosition` (FR-327). The
  **anchor** of an activation is the position at which FR-327's activation rule
  fires (`on origin`: position 0; `on each`: every position), and its tick is
  `t0` = that position's stamp.
- For a future operator with interval `[a,b]`, the evaluator SHALL treat a
  position `j` at or after the anchor in admitted order as **eligible** exactly when `stamp(j) - t0` lies in
  `[a,b]`; for a past operator, a position `j` at or before the anchor is
  eligible exactly when `t0 - stamp(j)` lies in `[a,b]`. Both endpoints are
  inclusive (QSpec FR-091, FR-092 timestamped rule). A position stamped exactly
  at `t0 + b` participates before settlement.
- Quantification SHALL range only over eligible positions. The evaluator SHALL
  add no synthetic atom after the window closes and SHALL NOT apply false
  extension (QSpec FR-090 profile table).
- `until` and `since` SHALL use the admitted order from the interval's lower
  bound to their witness and require the left operand at every eligible earlier
  position in that order; `release` and `triggered` are their Boolean duals.
  `once[1,1]` is strong previous. `not`, `and`, `or` and `implies` evaluate
  pointwise at each eligible position (FR-043 "Operators").
- When two positions that both participate in an order-sensitive operator share
  a stamp and either lacks an order key, or their keys come from different
  order authorities, the evaluator SHALL refuse at stage `evaluate`
  `invalid_runtime_input`/`invalid-value` at the later position, and SHALL NOT
  substitute trace index or ingestion order (QSpec FR-090-AC-4). A trace whose
  stamps decrease along its admitted order SHALL be refused at stage `admit`,
  `invalid_runtime_input`/`invalid-value`, before any position is visited.

### Established window

- The clause's **future reach** `Hf` is the greatest sum of the upper bounds of
  its future operators (`eventually`, `always`, `until`, `release`) along any
  nesting path; a past operator (`once`, `historically`, `since`, `triggered`)
  contributes 0, because it reads only positions at or before the position it is
  evaluated at. A clause with no future operator has `Hf` 0. The **margin** `m`
  is `2u` under `Counter { uncertainty: u }` and 0 under `Exact`.
- An activation at tick `t0` is **established** exactly when the completeness
  assertion is `Complete { through }` with `through >= t0 + Hf + m` and
  `W >= t0 + Hf + m`. The margin keeps every unseen reading above `W` outside the
  window: its cover starts at `W + 1 - u`, and the window's latest instant is
  `t0 + 1 + u + Hf`.
- The watermark `W` is an inclusive reading: progress covers every tick up to
  and including `W` (QSpec FR-094). A deadline `D` settles while `W >= D`; a
  position stamped `D` participates, one stamped `D + 1` does not. A watermark
  the supplier states as half-open SHALL be consumed only through a QSpec FR-289
  mapping that proves inclusive coverage, and the evaluator SHALL NOT read a
  half-open end equal to `D` as coverage of `D` (QSpec FR-094).
- An activation is **open** when it is not established. The `deadline` of a
  `Pending` result is `t0 + Hf + m`.

### Verdicts

- The evaluator SHALL give every atom at an eligible position a possibility set over `{true, false}`
  (QSpec FR-160): a valuation present in the trace is a singleton; a valuation
  absent from the admitted observation of an eligible position is `MissingValuation`
  (both possibilities); two admitted valuations of one atom at one position that
  differ are `ConflictingValuation` (both possibilities, distinct reason). An
  empty possibility set refuses `invalid_runtime_input`/`invalid-value`.
- The evaluator SHALL lift Boolean and temporal operators pointwise over every consistent valuation
  of the possibilities and of every admissible continuation (a continuation is
  a set of further positions at stamps greater than `W`, and at stamps at most
  `W` only where completeness is not asserted through them).
- When the lifted result is a singleton, the evaluator SHALL return a definite verdict:
  `Completed(true)` or `Completed(false)`. This includes an early witness or
  counterexample on an open window (a `decisive-witness` or
  `decisive-counterexample` that every admissible continuation preserves), a
  missing valuation outside the verdict's decision support, and the empty
  existential window (`false`) and empty universal window (`true`) of an
  established activation.
- When the lifted result is two-valued: if `W < t0 + Hf + m` it is `Pending` with
  `deadline` `t0 + Hf + m`; if `W >= t0 + Hf + m`, the evidence stays
  unresolved even with all declared progress applied and the result is
  `Indeterminate`, its `reason` `IncompleteInput` when the cause is a
  completeness assertion short of `t0 + Hf + m`, `MissingValuation` or
  `ConflictingValuation` when a valuation is the cause.
- When a past operator's computed lower boundary precedes the trace and the
  lower-boundary kind is `HistoryCutoff`, the evaluator SHALL return
  `Indeterminate` with reason `MissingHistory` unless the verdict is already
  singleton from the history present; with `AuthoritativeOrigin` the same trace
  is definite. An interior history interval with no admitted position and no
  completeness through it SHALL be `IncompleteInput`, never false padding.
- The four dimensions (decision-scope closure, surrounding-execution closure,
  assessment-execution disposition and input completeness) SHALL stay
  independent: a `Closed` scope does not make the input complete, an incomplete
  input never admits empty-window truth even when the scope is `Closed` (QSpec
  FR-094), and the scope and surrounding-execution closures change only the
  reported basis and premises, never the verdict class.
- The evaluator SHALL return `Completed(true)` when every activation is
  definitely true, and `Completed(false)` naming the first activation in
  position order that is definitely false even when an earlier activation is
  `Pending` or `Indeterminate`. Otherwise it SHALL return the first `Pending` or
  `Indeterminate` activation in position order, `Indeterminate` winning over
  `Pending` at one anchor. An atom that evaluates `Undefined` or a condition
  that refuses is returned as FR-327 states, in evaluation order.

### Counter cover

- When the clock binding declares `Counter { uncertainty: u }`, each stamp `k`
  is the half-open interval `[k-u, k+1+u)`; the difference of two stamps is the
  interval QSpec FR-160 gives, with endpoint flags; and an interval bound
  `[a,b]` decides true for a pair relation only when every pair of instants in
  the two covers agrees (QSpec FR-160 "Counter-tick intervals and endpoints").
  A bound the covers straddle gives a two-valued possibility, resolved to
  `Indeterminate` with reason `StraddlesBound`, or to `Pending` while `W` is
  short, by the rules above.
- Under `Exact` cover the evaluator SHALL compare the integer difference and
  admit no straddle. Establishment under `Counter { uncertainty: u }` SHALL use
  the margin `2u` of "Established window".

### Work

- The evaluator SHALL charge `temporal.visit` (FR-327 "Work") once per visit of
  a temporal node at a position, and once for each position it scans to
  determine eligibility. It visits every eligible position and takes no
  short-circuit, so the charge count depends on the trace and not on the
  valuations. A denied charge returns `Incomplete` with that charge
  point and no truth value, and an activation already settled on complete exact
  support keeps its verdict.

### Entry through the spine

- FR-330's `run_clause` SHALL dispatch a clause under this profile to this
  evaluator, with the trace supplied as `TraceInput::Timestamped` (FR-330), and
  SHALL NOT report `unsupported_profile` or `ProfileNotEvaluated` for it. The
  evaluator SHALL refuse a `Finite` or `Lasso` trace, which carries no stamps,
  `invalid_runtime_input`/`invalid-value` for this profile.
- Where a progress ledger is supplied, FR-502 governs contradiction, and the
  report returns the updated ledger (FR-330).

### Required QSpec definition

This requirement relies on QSpec FR-090, FR-091, FR-092, FR-094 and FR-160 for
profile selection, eligible instants, deadlines and possibility sets. QSpec
FR-091 says only that an empty window's truth is emitted after completeness and
that before then the result is "pending or incomplete". The evaluator needs the
established-window rule, the `Pending`/`Indeterminate` partition, the future
reach and the equal-stamp eligibility rule above. Until QSpec states them this
requirement is the definition the evaluator implements. The matching QSpec
FR-091 and FR-094 amendment is written in `quire-specification` and is tracked
as its own ticket, referenced from QSL-1493; it is not part of this repository.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-501-AC-1 | Over a `TimestampedTrace` with stamps 0, 2, 5 ms, `W` 5, `Complete { through: 5 }`, scope `Closed`: with `p` true at stamps 0 and 5 and false at 2, `always[0,5] holds(p)` `on origin` is `Completed(false)` at position 0; with `p` true at all three it is `Completed(true)`; `eventually[0,1] holds(p)` with `p` true only at stamp 2 is `Completed(false)` at position 0 (the position stamped 2 is outside `[0,1]`), and `eventually[0,2] holds(p)` is `Completed(true)`. With a position stamped 6 added, `[0,5]` still excludes it, and a position stamped 5 is included. | Test (TC-1295) |
| FR-501-AC-2 | One position stamped 0 with `p` true, `W` 1, `Complete { through: 1 }`: `always[0,1] holds(p)` is `Completed(true)` (no synthetic false atom after closure) where the same valuations under event-position false-extension give `Completed(false)` (FR-043-AC-2). With the same single position, `W` 3 and `Complete { through: 3 }`: `eventually[2,3] holds(p)` is `Completed(false)` and `always[2,3] holds(p)` is `Completed(true)` (empty windows of an established activation). | Test (TC-1295) |
| FR-501-AC-3 | Stamps 0 and 1 with `p` false at both, `W` 1, `Complete { through: 1 }`, scope `Open`: `eventually[0,5] holds(p)` is `Pending { where: 0, deadline: 5 }`; re-evaluated with the same two positions, `W` 5 and `Complete { through: 5 }`, with no further observation, it is `Completed(false)`. With `p` true at stamp 1 and `W` 1 it is `Completed(true)` while the scope is `Open` (decisive witness), and `always[0,5] holds(p)` with `p` false at stamp 1 and `W` 1 is `Completed(false)` while `Open` (decisive counterexample). | Test (TC-1295) |
| FR-501-AC-4 | Stamps 0, 2, 5, `eventually[0,5] holds(p)`, `W` 5, `Complete { through: 5 }`: with `p` false at 0 and 5 and its valuation absent at stamp 2, the result is `Indeterminate { where: 0, reason: MissingValuation { position: 1, atom: p } }`; with `p` true at 5 and absent at 2 it is `Completed(true)`; with `p` both true and false admitted at stamp 2 and false elsewhere it is `Indeterminate` with `ConflictingValuation { position: 1, atom: p }`; with the absence at stamp 2 and `W` 3 it is `Pending { deadline: 5 }`. None is `Completed(false)`. | Test (TC-1295) |
| FR-501-AC-5 | Stamps 0, `p` true, scope `Closed`, `W` 5, completeness `Incomplete`: `always[0,5] holds(p)` is `Indeterminate { reason: IncompleteInput }`, and the same trace with `Complete { through: 5 }` is `Completed(true)`; with completeness `Incomplete`, `eventually[2,3] holds(p)` is `Indeterminate { reason: IncompleteInput }` and not `Completed(false)`; with `Complete { through: 2 }` and `W` 3 it is also `Indeterminate`. | Test (TC-1295) |
| FR-501-AC-6 | Stamps 0 and 3, `q` true at 3, `W` 3, `Complete { through: 3 }`: `eventually[0,3] holds(q)` is `Completed(true)` (the position exactly at the inclusive deadline participates). Stamps 0 and 4, `q` true at 4, `W` 4, `Complete { through: 4 }`: `eventually[0,3] holds(q)` is `Completed(false)`. One position stamped 0 with `p` false, `W` 3, `Complete { through: 3 }` and no later observation: `eventually[0,3] holds(p)` is `Completed(false)`. A watermark supplied as half-open `[.., 3)` with no QSpec FR-289 mapping proving inclusive coverage is treated as `W` 2. | Test (TC-1295) |
| FR-501-AC-7 | Stamps 0, 1, 1 (positions 1 and 2 share stamp 1) with no order keys: `holds(p) until[0,1] holds(q)` refuses `invalid_runtime_input`/`invalid-value` at position 2, and so do `release` and, anchored at position 2 where positions 1 and 2 participate, `holds(p) since[0,1] holds(q)` and its `triggered` dual, each at stage `evaluate`; `eventually[0,1] holds(p)` over the same positions evaluates. With keys from one order authority on positions 1 and 2 `until` evaluates and reversing the trace's insertion order with keys unchanged changes nothing; with keys from two order authorities it refuses. | Test (TC-1295) |
| FR-501-AC-8 | Stamps 3 and 4 (the trace begins at 3), activation `on each`, `once[0,4] holds(p)` at position 1 (future reach 0, so the activation is established at `W` 4 with `Complete { through: 4 }`) with `p` false at both: lower-boundary kind `HistoryCutoff` gives `Indeterminate { reason: MissingHistory }` and `AuthoritativeOrigin` gives `Completed(false)`; with `p` true at stamp 3 both kinds give `Completed(true)`. | Test (TC-1295) |
| FR-501-AC-9 | With the clock binding `Counter { uncertainty: 0 }`, a request `holds(req)` at tick 0 and `holds(req) implies eventually[0,3000] holds(ack)`, `W` and completeness through 3001: `ack` first at tick 2998 gives `Completed(true)`, at tick 3000 gives `Indeterminate { reason: StraddlesBound }`, and at tick 3001 gives `Completed(false)`; under `Exact` cover the three give true, true and false. The neighbouring ticks with uncertainty 1 give the verdicts of QSpec `tick-cover-vectors.json` `TCV-01` to `TCV-12`, and the same-tick pair gives `TCV-13` to `TCV-15`. | Test (TC-1295) |
| FR-501-AC-10 | `eventually[0,2] holds(p)` `on each` over positions stamped 0 and 10 with `p` false at both, `W` 10, `Complete { through: 10 }`: the activation at position 0 is definitely false and the one at 10 is `Pending`, so the outcome is `Completed(false)` at position 0. With `p` true at stamp 0 the outcome is `Pending { where: 1, deadline: 12 }`, not `Completed(true)`. | Test (TC-1295) |
| FR-501-AC-11 | From source, a unit selecting the finite-window profile with a clause `eventually[0,5] holds(c.level > 0)`, run through `run_clause` over `TraceInput::Timestamped` reports `success` (exit 0) for AC-1's closed-complete true case, `violation` (exit 10, position 0) for a closed-complete false case, `inconclusive` (exit 0) for AC-3's `Pending` case and `indeterminate` (exit 11, cause `MissingValuation`) for AC-4's first case; a multi-item run holding that item and a `violation` exits 10, and one holding it and a `success` exits 11; it reports no `unsupported_profile` and no `ProfileNotEvaluated`, and the same clause over `TraceInput::Finite` reports stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1295) |
| FR-501-AC-12 | Over AC-1's trace, `eventually[0,5] holds(p)` `on origin` makes 7 charges at `temporal.visit`: the `eventually` node at position 0, the `holds` atom at each of the 3 eligible positions, and one scan charge for each of the 3 positions. With a meter of 6 it returns `Incomplete` at the `temporal.visit` charge point and no truth value; with a meter of 7 it returns its verdict and charges 7. A sibling activation settled before the denied charge keeps its verdict. Running any request of AC-1 to AC-11 twice gives equal outcomes. | Test (TC-1295) |
| FR-501-AC-13 | With the clock binding `Counter { uncertainty: 1 }` and AC-9's clause and request at tick 0, `Complete { through: 3002 }`: `W` 3001 and no `ack` gives `Pending { deadline: 3002 }`, and `W` 3002 gives `Completed(false)`. With `Exact` cover `W` 3000 and `Complete { through: 3000 }` already gives `Completed(false)`. | Test (TC-1295) |
| FR-501-AC-14 | Over AC-1's trace with `p` true at 0 and 5, false at 2, and `W` 5, `Complete { through: 5 }`: `always[0,5] holds(p)` reports basis `closed-scope` with scope `Closed` and `decisive-counterexample` with scope `Open`, and the verdict `Completed(false)` in both; its decision support is exactly {(position 1, p)}. `eventually[0,5] holds(p)` over stamps 0, 2, 5 with `p` false, false, true reports basis `closed-scope`, verdict `Completed(true)`, support {(position 2, p)}. A `Pending` result reports basis `unsettled` and an `Indeterminate` result reports basis `unavailable` with no truth. | Test (TC-1295) |
| FR-501-AC-15 | The report keeps scope closure, surrounding-execution closure, assessment-execution disposition and input completeness as four fields: changing the scope from `Closed` to `Open` changes the basis and the scope field and no other field or the verdict; changing the surrounding execution changes only its field; an `Incomplete` (work) outcome reports assessment `Failed` where a verdict reports `Completed`; changing completeness from `Complete { through: 5 }` to `Incomplete` changes the verdict class exactly as AC-5 states and no closure field. | Test (TC-1295) |
| FR-501-AC-16 | Evaluating AC-1's request twice gives one result identity; changing the stamp unit, the cover, any one stamp, valuation, order key, `W`, `through`, the lower-boundary kind or a meter ceiling, each alone, gives a distinct identity, and the earlier result is neither reused nor rewritten. | Test (TC-1295) |
| FR-501-AC-17 | `Complete { through: 9 }` with `W` 5 for `eventually[0,5] holds(p)` over stamps 0, 2, 5 with `p` false, false, false is `Completed(false)` (the activation is established at 5); with `W` 3 it is `Pending { deadline: 5 }`. A trace with no positions, and a trace whose stamps are 0, 5, 2, are each refused at stage `admit`, `invalid_runtime_input`/`invalid-value`, before any position is visited; AC-7's equal-stamp refusal is at stage `evaluate`. | Test (TC-1295) |

## Dependencies

- FR-043 (the legacy behavior this requirement succeeds), FR-326 (profile identity
  and horizon), FR-327 (evaluator, work and outcomes), FR-330 (entry), FR-502
  (progress contradiction), FR-106 (observations).
- QSpec FR-090, FR-091, FR-092, FR-094 and FR-160. The QSpec FR-091/FR-094
  amendment is tracked as a separate `quire-specification` ticket.
- FR-043-AC-14 (`/2` clock-parameter authentication, `Refusal::Binding`) is not
  restated here: its successor is the authenticated temporal binding tracked by
  QSL-1384 and QSL-1446, and it remains a GAP row of FR-499 until they land.
- ADR-026 MN-2 and QSL FR-251 (half-open counter cover, which QSpec FR-160 now
  fixes).

## References

- Linear QSL-858 (deletion of `src/temporal` waits on this requirement).
- Legacy behavior parity list: `src/temporal/formula.rs` `offsets`, `established`
  and `window`; `src/temporal/profile.rs` `extends_false`/`advances_on_progress`.
