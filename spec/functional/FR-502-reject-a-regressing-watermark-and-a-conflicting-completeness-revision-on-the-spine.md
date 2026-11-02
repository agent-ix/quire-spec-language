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

- An optional `ProgressLedger` in FR-330's `Temporal` selection.
- A **binding**: the checked package identity, the checked clause identity, the
  clock binding name and the selected profile identity.
- A progress assertion in a `TimestampedTrace` (FR-501) or a trace of another
  profile: a watermark `W`, a completeness assertion and a closure of the
  decision scope.

## Outputs

On acceptance: the outcome of FR-327/FR-501 and an updated ledger entry for the
binding. On contradiction: `invalid_runtime_input`/`invalid-value` carrying a
`contradicted_dimension` (`Watermark`, `Completeness` or `Closure`), the
binding's clock name, the retained value and the asserted value; no outcome,
no truth value, and the ledger unchanged.

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
- Under the event-position profile the watermark is a retained premise and does
  not advance the clock (FR-501 applies to fixed-sample and timestamped-event
  only), but regression and conflict are refused as above.
- When a call supplies no ledger, the evaluator SHALL stand on the trace's own
  assertions and detect no contradiction across calls.

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
| FR-502-AC-5 | Under the fixed-sample, timestamped-event and event-position profiles alike, a watermark regression is refused; the event-position binding's accepted watermark leaves its `Pending` obligation `Pending` (the clock does not advance during silence) while the same watermark settles the fixed-sample and finite-window obligations. | Test (TC-1296) |
| FR-502-AC-6 | Evaluation with no ledger accepts a `W` 100 call followed by a `W` 60 call and reports each from its own trace; evaluation with a ledger refuses the second. | Test (TC-1296) |
