---
id: FR-502
title: "Reject a regressing progress watermark and a conflicting completeness revision under one temporal binding on the spine"
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
  - target: ix://agent-ix/quire-specification/FR-094
    type: depends_on
---
# FR-502: Reject a regressing progress watermark and a conflicting completeness revision under one temporal binding on the spine

## Description

A caller evaluating one temporal clause repeatedly as observation arrives
carries one `ProgressLedger`. The spine temporal evaluator (FR-327, FR-328,
FR-329, FR-501) SHALL keep, for each binding, what that binding has already
asserted about its watermark, completeness and decision-scope closure, and SHALL
refuse an assertion that contradicts it. This is the spine successor of
FR-043-AC-12 and of the legacy progress ledger; it closes the spine gap for
watermark regression and conflicting completeness revision that
`src/temporal`'s deletion would otherwise remove.

## Use case

A monitor receives a progress assertion "complete through tick 100", then later
one "complete through tick 60" from a replica that lagged. The evaluator refuses
the second, names the contradicted dimension and clock, keeps the earlier
progress and leaves every earlier result as it was.

## Inputs

- An optional `ProgressLedger` in FR-330's `Temporal` selection, or in an
  `EvidenceRequest` (FR-516), whose call applies this requirement to its
  progress assertion and returns the updated ledger.
- A **binding**: the checked package identity, the checked clause identity, the
  clock binding name and the selected profile identity.
- A progress assertion in a `TraceInput::Timestamped` trace (FR-330, FR-501): a
  watermark `W`, a completeness assertion, the decision-scope closure and the
  surrounding-execution closure. `TraceInput::Progressed` (FR-504) carries the same assertion for
  event-position and fixed-sample clauses; the `Lasso` and `Model` traces and a
  `Finite` trace without `progress` carry none, and their closure is the trace
  itself (FR-327, FR-328, FR-329).

## Outputs

On acceptance: the outcome of FR-501 and the updated ledger, which FR-330's
`ClauseRunReport` returns. Each entry retains the watermark, the completeness
assertion, the scope closure and the surrounding-execution closure, and the
**covered position set**: the identities of the positions each retained
completeness assertion covered (a position's identity is the identity of its
admitted observation, FR-106), so that "a position the retained assertion did
not cover" is decidable. On contradiction: `invalid_runtime_input`/`invalid-value` carrying a
`contradicted_dimension` (`Watermark`, `Completeness` or `Closure`), the
binding's clock name, the retained value and the asserted value; no outcome,
no truth value, and the ledger unchanged: the report of a refused request
returns the ledger the request supplied, equal to it, and returns none when the
request supplied none. The same holds for a request that returns
`Incomplete(LimitReached{...})` below.

## Behavior

- The ledger SHALL key each entry by the whole binding. An assertion under any
  other package, clause, clock binding name or profile identity is progress for
  a different binding, which settles nothing for this binding and creates or
  updates only its own entry.
- Within one binding the watermark SHALL be monotonic. When an assertion
  carries a `W` below the retained watermark, the evaluator SHALL refuse it as a
  contradiction on `Watermark`; an assertion equal to it is accepted.
- Completeness extent SHALL be monotonic. While `Complete { through: T }` is
  retained, the evaluator SHALL refuse as a contradiction on `Completeness` an
  assertion `Complete { through: T' }` with `T' < T`, an assertion of
  `Incomplete`, and an admitted position stamped at or before `T` that the
  retained assertion did not cover. A later assertion with `T' >= T` is accepted. A later
  `Incomplete` after a retained `Incomplete` is accepted.
- While a decision scope is retained `Closed`, the evaluator SHALL refuse an
  `Open` assertion as a contradiction on `Closure`.
- Closing a scope SHALL NOT change the retained completeness, and completing the
  input SHALL NOT close the scope (FR-043 "Progress, closure and settlement").
- When the evaluator refuses an assertion it SHALL NOT roll back progress,
  restamp closure, change the retained entry or rewrite the bytes of any earlier
  result.
- When the inputs of a result reported before the contradiction are evaluated
  again, the evaluator SHALL reproduce that result byte for byte.
- The ledger SHALL apply to every trace shape that carries progress:
  `TraceInput::Timestamped` (FR-501) and `TraceInput::Progressed` (FR-504). An
  event-position watermark is retained and settles nothing, but its regression
  is refused like any other.
- When a request supplies a `ledger` with a `Finite` trace without progress, a
  `Lasso`, a `Model`, or an infinite-trace clause, the evaluator SHALL refuse it
  at stage `admit`, `invalid_runtime_input`/`invalid-value`. Those traces carry
  no progress assertion: a lasso is the whole infinite behavior, and a finite
  prefix under infinite-trace is `Pending` by construction (FR-328).
- The evaluator SHALL bound the covered position sets by one ledger capacity,
  the FR-255 setting `progress.covered_positions`, counted over every entry of
  the ledger together and not per binding. When recording a position would make
  the covered identities of all entries together exceed it, the evaluator SHALL return `Incomplete(LimitReached{limit, value,
  setting})` with `limit` the kind (node count) and `setting` the name, and the ledger is unchanged.
- The evaluator SHALL decide whether a position is one that "the retained
  assertion did not cover" from the position's identity and its stamp: the stamp
  of a position in the supplied trace says whether it falls at or before the
  retained `through`, and the covered set, which holds identities only, says
  whether the retained assertion covered that identity.
- A later trace that omits a position an earlier assertion covered is accepted:
  the ledger keeps the omitted position as covered and does not treat its absence
  as a contradiction; only a position that was not covered and now appears at or
  before a retained `through` contradicts.
- The evaluator SHALL derive the covered position set from the position
  identities of the trace in each accepted request, and the ledger returned for
  a request SHALL equal the ledger a later request carries.
- Running the same sequence of requests from the same initial ledger SHALL give
  the same sequence of outcomes and the same final ledger; running one request
  twice from different ledgers MAY give different outcomes.

## Dependencies

- FR-043-AC-12 (the legacy criterion this requirement succeeds on the spine),
  FR-327 and FR-501 (the evaluators that read the ledger), FR-330 (the entry
  that carries it).
- QSpec FR-094 (progress is monotonic within its identified binding; a
  regressing or conflicting assertion produces a typed contradiction and does
  not roll back progress).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-502-AC-1 | With a ledger holding binding B at `W` 100, an assertion `W` 60 under B is refused `invalid_runtime_input`/`invalid-value` with `contradicted_dimension` `Watermark`, B's clock name, retained 100 and asserted 60; the ledger afterwards holds `W` 100 and the earlier result evaluates to the same bytes; `W` 100 and `W` 130 are accepted. | Test (TC-1296) |
| FR-502-AC-2 | After `Complete { through: 100 }` is retained under B, `Complete { through: 60 }`, `Incomplete`, and an admitted position stamped 80 that the retained assertion did not cover are each refused with `contradicted_dimension` `Completeness` and leave the ledger unchanged; `Complete { through: 100 }` and `Complete { through: 140 }` are accepted. | Test (TC-1296) |
| FR-502-AC-3 | After scope `Closed` is retained under B, an assertion `Open` is refused with `contradicted_dimension` `Closure`; asserting `Closed` with completeness `Incomplete` leaves completeness `Incomplete`, and asserting `Complete { through: 100 }` with scope `Open` leaves the scope `Open`. | Test (TC-1296) |
| FR-502-AC-4 | A progress assertion under another package identity, another clause, another clock binding name or another profile identity, each varied alone from B, settles nothing for B (B's `Pending` outcome stays `Pending` and its entry is unchanged) and appears as its own ledger entry. | Test (TC-1296) |
| FR-502-AC-5 | A request that supplies a `ledger` with a `Finite` trace without progress, a `Lasso`, a `Model` or an infinite-trace clause is refused at stage `admit`, `invalid_runtime_input`/`invalid-value` and the ledger is unchanged; the same ledger with a `Timestamped` trace (finite-window), a fixed-sample `Progressed` trace and an event-position `Progressed` trace is accepted, a fixed-sample watermark regression is refused `Watermark`, and an event-position watermark that advances leaves its `Pending` outcome `Pending`. | Test (TC-1296, TC-1299) |
| FR-502-AC-6 | Evaluation with no ledger accepts a `W` 100 call followed by a `W` 60 call and reports each from its own trace; evaluation with a ledger refuses the second. The report of an accepted request returns the updated ledger, and passing it to the next request gives the same outcome as one request sequence replayed from the same initial ledger. | Test (TC-1296) |
| FR-502-AC-7 | After a request retaining `Complete { through: 100 }` over positions 0 to 3, a later request whose trace adds a position stamped 80 that was not among positions 0 to 3 is refused `Completeness`, and one adding a position stamped 120 is accepted; the returned ledger lists positions 0 to 3 (and the new one) as covered. | Test (TC-1296) |
| FR-502-AC-8 | With `progress.covered_positions` set to 3, a request covering 4 positions returns `Incomplete(LimitReached{limit: node count, value: 3, setting: progress.covered_positions})` and leaves the ledger unchanged; a later trace that omits two earlier covered positions and adds one new position beyond `through` is accepted and the ledger still lists the omitted positions as covered. | Test (TC-1296) |
| FR-502-AC-9 | With `progress.covered_positions` set to 3, a ledger holding two covered positions under B, and a request under a second binding B2 covering two positions, the request returns `Incomplete(LimitReached{limit: node count, value: 3, setting: progress.covered_positions})` and the ledger is unchanged; a request under B2 covering one position is accepted. | Test (TC-1296) |
| FR-502-AC-10 | After `Complete { through: 100 }` is retained under B over a position of identity `p1` stamped 80, a later trace holding a position of another identity stamped 80 is refused `Completeness`, and one holding identity `p1` stamped 80 is accepted. | Test (TC-1296) |
| FR-502-AC-11 | The report of a request refused by FR-502 returns the ledger the request supplied, equal to it, and the report of a refused request that supplied no ledger returns none. | Test (TC-1296) |
