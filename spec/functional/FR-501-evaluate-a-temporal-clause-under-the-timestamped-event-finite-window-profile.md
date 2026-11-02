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
profile text leaves to the evaluator (see "Required QSpec definition").

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
  the decision scope's closure (`Closed` or `Open`); and the lower-boundary kind
  (`AuthoritativeOrigin` or `HistoryCutoff`).
- The clock binding's declared **cover** for a stamp: `Exact` (default; a stamp
  is the instant itself) or `Counter { uncertainty }` (a counter reading `k`
  covers the half-open interval `[k, k+1)` of the counter unit, widened to
  `[k-u, k+1+u)`; ADR-026 MN-2, QSpec FR-160).
- A kernel `Meter` (ADR-014 B-2).

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

`Pending` maps to O-16 inconclusive. `Indeterminate` maps to the O-16 category
`undefined` with its `reason` as the cause, as `InsufficientData` does (QSpec
FR-414): the trace lacks the data to decide the claim, so it neither holds nor
fails on it. Neither is `Completed(false)`, and neither is a truth value.

## Behavior

### Positions, anchor and eligible instants

- The evaluator SHALL address each position by `TemporalPosition` (FR-327). The
  **anchor** of an activation is the position at which FR-327's activation rule
  fires (`on origin`: position 0; `on each`: every position), and its tick is
  `t0` = that position's stamp.
- For a future operator with interval `[a,b]`, a position `j` at or after the
  anchor in admitted order is **eligible** exactly when `stamp(j) - t0` lies in
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
  order authorities, the evaluator SHALL refuse `invalid_runtime_input`/
  `invalid-value` at the later position and SHALL NOT substitute trace index or
  ingestion order (QSpec FR-090-AC-4).

### Established window

- The clause's **reach** `H` is the greatest sum of upper bounds along any
  nesting path of its interval operators, the horizon FR-326 computes. An
  activation at tick `t0` is **established** exactly when the completeness
  assertion is `Complete { through }` with `through >= t0 + H` and
  `W >= t0 + H`.
- The watermark `W` is an inclusive reading: progress covers every tick up to
  and including `W` (QSpec FR-094). A deadline `D` settles while `W >= D`; a
  position stamped `D` participates, one stamped `D + 1` does not. A watermark
  the supplier states as half-open SHALL be consumed only through a QSpec FR-289
  mapping that proves inclusive coverage, and the evaluator SHALL NOT read a
  half-open end equal to `D` as coverage of `D` (QSpec FR-094).
- An activation whose window is closed (`W >= t0 + H`) is **closed-complete**
  exactly when it is established. An activation is **open** otherwise.

### Verdicts

- Every atom at an eligible position has a possibility set over `{true, false}`
  (QSpec FR-160): a valuation present in the trace is a singleton; a valuation
  absent from the admitted observation of an eligible position is `MissingValuation`
  (both possibilities); two admitted valuations of one atom at one position that
  differ are `ConflictingValuation` (both possibilities, distinct reason). An
  empty possibility set refuses `invalid_runtime_input`/`invalid-value`.
- Boolean and temporal operators lift pointwise over every consistent valuation
  of the possibilities and of every admissible continuation (a continuation is
  a set of further positions at stamps greater than `W`, and at stamps at most
  `W` only where completeness is not asserted through them).
- When the lifted result is a singleton, the verdict is definite:
  `Completed(true)` or `Completed(false)`. This includes an early witness or
  counterexample on an open window (a `decisive-witness` or
  `decisive-counterexample` that every admissible continuation preserves), a
  missing valuation outside the verdict's decision support, and the empty
  existential window (`false`) and empty universal window (`true`) of an
  established activation.
- When the lifted result is two-valued: if `W < t0 + H` it is `Pending` with
  `deadline` the least tick whose establishment would settle it; if `W >= t0 +
  H`, the evidence stays unresolved even with all declared progress applied and
  the result is `Indeterminate`, its `reason` `IncompleteInput` when the cause
  is a completeness assertion short of `t0 + H`, `MissingValuation` or
  `ConflictingValuation` when a valuation is the cause.
- A past operator whose computed lower boundary precedes the trace and whose
  lower-boundary kind is `HistoryCutoff` is `Indeterminate` with reason
  `MissingHistory` unless the verdict is already singleton from the history
  present; with `AuthoritativeOrigin` the same trace is definite. An interior
  history interval with no admitted position and no completeness through it is
  `IncompleteInput`, never false padding.
- The four dimensions of progress (decision-scope closure, surrounding-
  execution closure, assessment-execution disposition and input completeness)
  stay independent: a `Closed` scope does not make the input complete, and an
  incomplete input never admits empty-window truth even when the scope is
  `Closed` (QSpec FR-094).
- The activation outcome is `Completed(true)` when every activation is
  definitely true. It is `Completed(false)` naming the first activation in
  position order that is definitely false even when an earlier activation is
  `Pending` or `Indeterminate`. Otherwise it is the first `Pending` or
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
- Under `Exact` cover the same comparison uses the integer difference and
  admits no straddle.

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
  SHALL NOT report `unsupported_profile` or `ProfileNotEvaluated` for it. A
  `Finite` or `Lasso` trace carries no stamps and SHALL be refused
  `invalid_runtime_input`/`invalid-value` for this profile.
- Where a progress ledger is supplied, FR-502 governs contradiction.

### Required QSpec definition

This requirement relies on QSpec FR-090, FR-091, FR-092, FR-094 and FR-160 for
profile selection, eligible instants, deadlines and possibility sets. QSpec
FR-091 says only that an empty window's truth is emitted after completeness and
that before then the result is "pending or incomplete". The evaluator needs the
established-window rule, the `Pending`/`Indeterminate` partition, the nested
reach and the equal-stamp eligibility rule above. Until QSpec states them this
requirement is the definition the evaluator implements, and the QSpec FR-091
amendment (proposed text in the PR that adds this requirement) mirrors it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-501-AC-1 | Over a `TimestampedTrace` with stamps 0, 2, 5 ms, `W` 5, `Complete { through: 5 }`, scope `Closed`: with `p` true at stamps 0 and 5 and false at 2, `always[0,5] holds(p)` `on origin` is `Completed(false)` at position 0; with `p` true at all three it is `Completed(true)`; `eventually[0,1] holds(p)` with `p` true only at stamp 2 is `Completed(false)` at position 0 (the position stamped 2 is outside `[0,1]`), and `eventually[0,2] holds(p)` is `Completed(true)`. With a position stamped 6 added, `[0,5]` still excludes it, and a position stamped 5 is included. | Test (TC-1295) |
| FR-501-AC-2 | One position stamped 0 with `p` true, `W` 1, `Complete { through: 1 }`: `always[0,1] holds(p)` is `Completed(true)` (no synthetic false atom after closure) where the same valuations under event-position false-extension give `Completed(false)` (FR-043-AC-2). With the same single position, `W` 3 and `Complete { through: 3 }`: `eventually[2,3] holds(p)` is `Completed(false)` and `always[2,3] holds(p)` is `Completed(true)` (empty windows of an established activation). | Test (TC-1295) |
| FR-501-AC-3 | Stamps 0 and 1 with `p` false at both, `W` 1, `Complete { through: 1 }`, scope `Open`: `eventually[0,5] holds(p)` is `Pending { where: 0, deadline: 5 }`; re-evaluated with the same two positions, `W` 5 and `Complete { through: 5 }`, with no further observation, it is `Completed(false)`. With `p` true at stamp 1 and `W` 1 it is `Completed(true)` while the scope is `Open` (decisive witness), and `always[0,5] holds(p)` with `p` false at stamp 1 and `W` 1 is `Completed(false)` while `Open` (decisive counterexample). | Test (TC-1295) |
| FR-501-AC-4 | Stamps 0, 2, 5, `eventually[0,5] holds(p)`, `W` 5, `Complete { through: 5 }`: with `p` false at 0 and 5 and its valuation absent at stamp 2, the result is `Indeterminate { where: 0, reason: MissingValuation { position: 1, atom: p } }`; with `p` true at 5 and absent at 2 it is `Completed(true)`; with `p` both true and false admitted at stamp 2 and false elsewhere it is `Indeterminate` with `ConflictingValuation { position: 1, atom: p }`; with the absence at stamp 2 and `W` 3 it is `Pending { deadline: 5 }`. None is `Completed(false)`. | Test (TC-1295) |
| FR-501-AC-5 | Stamps 0, `p` true, scope `Closed`, `W` 5, completeness `Incomplete`: `always[0,5] holds(p)` is `Indeterminate { reason: IncompleteInput }`, and the same trace with `Complete { through: 5 }` is `Completed(true)`; with completeness `Incomplete`, `eventually[2,3] holds(p)` is `Indeterminate { reason: IncompleteInput }` and not `Completed(false)`; with `Complete { through: 2 }` and `W` 3 it is also `Indeterminate`. | Test (TC-1295) |
| FR-501-AC-6 | Stamps 0 and 3, `q` true at 3, `W` 3, `Complete { through: 3 }`: `eventually[0,3] holds(q)` is `Completed(true)` (the position exactly at the inclusive deadline participates). Stamps 0 and 4, `q` true at 4, `W` 4, `Complete { through: 4 }`: `eventually[0,3] holds(q)` is `Completed(false)`. One position stamped 0 with `p` false, `W` 3, `Complete { through: 3 }` and no later observation: `eventually[0,3] holds(p)` is `Completed(false)`. A watermark supplied as half-open `[.., 3)` with no QSpec FR-289 mapping proving inclusive coverage is treated as `W` 2. | Test (TC-1295) |
| FR-501-AC-7 | Stamps 0, 1, 1 (positions 1 and 2 share stamp 1) with no order keys: `holds(p) until[0,1] holds(q)` refuses `invalid_runtime_input`/`invalid-value` at position 2, and so do `release`, `since` and `triggered` forms over them; `eventually[0,1] holds(p)` over the same positions evaluates. With keys from one order authority on positions 1 and 2 `until` evaluates and reversing the trace's insertion order with keys unchanged changes nothing; with keys from two order authorities it refuses. | Test (TC-1295) |
| FR-501-AC-8 | Stamps 3 and 4 (the trace begins at 3), activation `on each`, `once[0,4] holds(p)` at position 1 with `p` false at both: lower-boundary kind `HistoryCutoff` gives `Indeterminate { reason: MissingHistory }` and `AuthoritativeOrigin` gives `Completed(false)`; with `p` true at stamp 3 both kinds give `Completed(true)`. | Test (TC-1295) |
| FR-501-AC-9 | With the clock binding `Counter { uncertainty: 0 }`, a request `holds(req)` at tick 0 and `holds(req) implies eventually[0,3000] holds(ack)`, `W` and completeness through 3001: `ack` first at tick 2998 gives `Completed(true)`, at tick 3000 gives `Indeterminate { reason: StraddlesBound }`, and at tick 3001 gives `Completed(false)`; under `Exact` cover the three give true, true and false. The neighbouring ticks with uncertainty 1 give the verdicts of QSpec `tick-cover-vectors.json` `TCV-01` to `TCV-12`, and the same-tick pair gives `TCV-13` to `TCV-15`. | Test (TC-1295) |
| FR-501-AC-10 | `eventually[0,2] holds(p)` `on each` over positions stamped 0 and 10 with `p` false at both, `W` 10, `Complete { through: 10 }`: the activation at position 0 is definitely false and the one at 10 is `Pending`, so the outcome is `Completed(false)` at position 0. With `p` true at stamp 0 the outcome is `Pending { where: 1, deadline: 12 }`, not `Completed(true)`. | Test (TC-1295) |
| FR-501-AC-11 | From source, a unit selecting the finite-window profile with a clause `eventually[0,5] holds(c.level > 0)`, run through `run_clause` over `TraceInput::Timestamped` reports `success` (exit 0) for AC-1's closed-complete true case, `violation` (exit 10, position 0) for a closed-complete false case, `inconclusive` (exit 0) for AC-3's `Pending` case and `undefined` (exit 10, cause `MissingValuation`) for AC-4's first case; it reports no `unsupported_profile` and no `ProfileNotEvaluated`, and the same clause over `TraceInput::Finite` reports stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1295) |
| FR-501-AC-12 | Over AC-1's trace, `eventually[0,5] holds(p)` `on origin` makes 7 charges at `temporal.visit`: the `eventually` node at position 0, the `holds` atom at each of the 3 eligible positions, and one scan charge for each of the 3 positions. With a meter of 6 it returns `Incomplete` at the `temporal.visit` charge point and no truth value; with a meter of 7 it returns its verdict and charges 7. A sibling activation settled before the denied charge keeps its verdict. Running any request of AC-1 to AC-11 twice gives equal outcomes. | Test (TC-1295) |

## Dependencies

- FR-043 (the legacy behavior this requirement succeeds), FR-326 (profile identity
  and horizon), FR-327 (evaluator, work and outcomes), FR-330 (entry), FR-502
  (progress contradiction), FR-106 (observations).
- QSpec FR-090, FR-091, FR-092, FR-094 and FR-160. QSpec FR-091 amendment:
  state the established window, the `Pending`/`Indeterminate` partition, the
  nested reach and the eligibility of positions at equal stamps as this
  requirement states them (see the PR that adds this requirement).
- ADR-026 MN-2 and QSL FR-251 (half-open counter cover, which QSpec FR-160 now
  fixes).

## References

- Linear QSL-858 (deletion of `src/temporal` waits on this requirement).
- Legacy behavior parity list: `src/temporal/formula.rs` `offsets`, `established`
  and `window`; `src/temporal/profile.rs` `extends_false`/`advances_on_progress`.
