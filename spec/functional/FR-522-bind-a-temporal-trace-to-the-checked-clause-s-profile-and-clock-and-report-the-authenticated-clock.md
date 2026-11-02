---
id: FR-522
title: "Bind a temporal trace to the checked clause's profile and clock before any position, report the authenticated clock, and give each result one identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-522: Bind a temporal trace to the checked clause's profile and clock before any position, report the authenticated clock, and give each result one identity

## Description

The spine temporal evaluator SHALL compare the profile, the clock binding name
and the clock parameters that a clock-bearing request asserts with the checked
clause's own selection, and SHALL refuse a difference before it visits any
position. For a request that agrees, the evaluator evaluates it and the report
retains the authenticated binding the request agreed with. Each result carries one
identity that equal inputs reproduce and that a change of any bound member
alters.

This is the spine successor of FR-043-AC-3 (trace and declaration disagree on
profile or clock), the `/2` authentication of FR-043-AC-14 and FR-050-AC-6
(evaluation half), and FR-043-AC-13 (result identity) for the event-position and
fixed-sample profiles and for the evidence call. FR-501-AC-16 states the same
identity rule for the finite-window profile, and this requirement extends it to
every clock-bearing request.

## Use case

A recorded fixed-sample trace is stamped with a one-millisecond period, and the
clause is authored for a ten-millisecond period. Evaluated anyway, the clause
would read every sample as ten times closer together than it is. The evaluator
refuses the trace, names the period as the differing parameter and visits no
sample. When the period agrees, the report names the package, the clause and
the clock it ran under, so a later reader can tell which clock the verdict
depends on.

## Semantic authority and boundary

QSpec FR-090 owns profile and clock selection and FR-043 states the legacy
evaluation of it. This requirement states the spine mechanism. The checked
package is the authentication: the profile comes from the lock's
`profile_selections` row of the clause (FR-515 `profile()`), and the clock from
the `temporal_bindings` row of the declaration (FR-515 `clock()`; FR-050). The
evaluator does not authenticate the observation owner's evidence (FR-053).

The legacy `/1` entry points retained a trace's clock parameters as
unauthenticated premises. The checked package has one route and it is
authenticated, so no spine counterpart of the `/1` behavior exists (see FR-525).

## Inputs

A `TraceBinding`, carried by every request whose quantities are ticks of a clock:
`TraceInput::Timestamped` (FR-501), `TraceInput::Progressed` (FR-504) and
`EvidenceRequest` (FR-516). It holds:

- `profile`: the profile identity the trace asserts;
- `clock_name`: the clock binding name the trace asserts;
- `parameters`: one `ClockConfiguration` alternative as FR-050 defines it:
  `EventPosition { sequence_authority }`, `FixedSample { epoch, period, unit }`
  or `TimestampedEvent { timestamp_unit }`.

`TraceInput::Finite`, `Lasso` and `Model` carry no `TraceBinding`: their
positions are indices and they hold no tick-valued quantity. A request vector of
FR-501, FR-504 and FR-516 that states no binding carries the binding of its
clause.

## Outputs

On agreement, the report of FR-501, FR-504 or FR-516 adds `authenticated`, an
`AuthenticatedBinding`: the checked package's identity, the declaration's
`CheckedNodeId`, the definition identity and namespaced revision of the profile
selection, the clock binding name and the `ClockConfiguration` of the row.

On difference, a refusal at stage `admit`, `invalid_runtime_input`/`invalid-value`
carrying `mismatched_dimension` and the asserted and expected values.
`mismatched_dimension` is one of `Profile`, `Clock`, `ClockParameters`, `Epoch`,
`SamplePeriod`, `ClockUnit`, `SequenceAuthority`, `TimestampUnit`. The refusal
holds no outcome, no truth and no `authenticated`. `evaluate_temporal_evidence`
reports it as `Err(CallFailure::Admission)` with an empty charge log (FR-516).

The `result_identity` of every outcome, the identity rule below.

## Behavior

### Authentication

- The evaluator SHALL compare the asserted `profile` with the profile identity of
  the clause's `profile_selections` row, byte for byte.
- The evaluator SHALL compare the asserted `clock_name` with the name of the
  clause's `temporal_bindings` row, byte for byte.
- When the asserted `parameters` are of another `ClockConfiguration` alternative
  than the row's, the evaluator SHALL refuse with `ClockParameters`.
- When the alternatives agree, the evaluator SHALL compare each member with the
  row's, and SHALL refuse with the dimension of the first member that differs in
  the order `epoch`, `period`, `unit` (`Epoch`, `SamplePeriod`, `ClockUnit`) for
  `FixedSample`; `SequenceAuthority` for `EventPosition`; `TimestampUnit` for
  `TimestampedEvent`.
- The evaluator SHALL test the dimensions in the order `Profile`, `Clock`,
  `ClockParameters` and the member dimensions, and SHALL report the first that
  differs.
- When the clause's declaration has no `temporal_bindings` row (the lock
  selects no clock configuration, FR-515), the evaluator SHALL refuse a
  `TraceBinding` with `Clock`.
- The evaluator SHALL perform the comparison before it visits any position,
  before it charges the meter, and before it checks any stamp order or
  completeness (FR-501 "Order").
- The evaluator SHALL NOT substitute, widen, default or nearest-match a profile,
  clock name, period, epoch, unit, sequence authority or timestamp unit.
- A typed `ClockConfiguration` cannot hold a missing or an extra member, so the
  legacy key-set check (`ClockParameters` for a missing or extra key) has the
  alternative comparison above as its successor.

### Result identity

- The `result_identity` SHALL be a function of exactly these members: the checked
  clause identity (the package identity and the declaration), the authenticated
  binding, the stamps, valuations and order keys or the positions of the trace,
  the progress inputs (watermark, completeness, scope closure, surrounding
  execution closure, lower boundary), the work ceilings of FR-523 and the
  meter's work-unit ceiling, and, for an evidence result, the request's trigger
  identity and the relation of FR-526.
- When two requests carry equal members, the evaluator SHALL give them one
  `result_identity`.
- When two requests differ in one member, the evaluator SHALL give them
  distinct `result_identity` values.
- The evaluator SHALL NOT reuse an earlier result for a request whose
  `result_identity` differs, and SHALL NOT rewrite the bytes of the earlier
  result.
- `result_identity` is an equality of members, not a digest of a document
  (FR-516), and FR-523's `input_identity` is the same function without the
  relation.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-522-AC-1 | Over **R**'s fixed-sample clause, a `Progressed` request whose `profile` names another profile identity is refused at stage `admit`, `invalid_runtime_input`/`invalid-value` with `mismatched_dimension` `Profile`, the asserted and expected identities, no outcome and no `authenticated`. | Test (TC-1317) |
| FR-522-AC-2 | Over **R**'s event-position clause, a request whose `clock_name` differs from the row's is refused with `Clock`. | Test (TC-1317) |
| FR-522-AC-3 | Over **R**'s fixed-sample clause, a request whose `parameters` are the `EventPosition` alternative is refused with `ClockParameters`. | Test (TC-1317) |
| FR-522-AC-4 | Over **R**'s fixed-sample clause, a request that differs from the row only in `epoch`, only in `period` and only in `unit` is refused with `Epoch`, `SamplePeriod` and `ClockUnit` respectively, and a request that differs in `epoch` and `unit` reports `Epoch`. | Test (TC-1317) |
| FR-522-AC-5 | Over **R**'s event-position clause a request that differs only in `sequence_authority` is refused with `SequenceAuthority`, and over **R**'s timestamped-event clause a request that differs only in `timestamp_unit` is refused with `TimestampUnit`. | Test (TC-1317) |
| FR-522-AC-6 | A request that asserts a profile differing from the row and a clock name differing from the row reports `Profile`, and a request that asserts a clock name and a parameter that differ reports `Clock`. | Test (TC-1317) |
| FR-522-AC-7 | A request that agrees on profile, clock name and parameters is evaluated, and its report's `authenticated` holds the package identity, the declaration, the definition identity and revision, the clock name and the `ClockConfiguration` equal to the values the test reads from the lock and the `temporal_bindings` row. | Test (TC-1317) |
| FR-522-AC-8 | A refused binding leaves the meter's charge log empty, and a `Timestamped` request that both asserts a wrong clock name and holds stamps 0, 5, 2 is refused with `Clock`, not with the stamp-order refusal of FR-501. | Test (TC-1317) |
| FR-522-AC-9 | A clause whose declaration has no `temporal_bindings` row, evaluated with a `TraceBinding`, is refused with `Clock`, and no other clock configuration is substituted. | Test (TC-1317) |
| FR-522-AC-10 | `evaluate_temporal_evidence` reports the refusal of AC-1 to AC-5 as `Err(CallFailure::Admission)` carrying the same `mismatched_dimension`, with an empty charge log and no result. | Test (TC-1317) |
| FR-522-AC-11 | The `Finite`, `Lasso` and `Model` traces carry no `TraceBinding`, and their reports hold no `authenticated`. | Test (TC-1317) |
| FR-522-AC-12 | Evaluating one request of each of **R**'s three clauses twice gives one `result_identity` per request, and the three identities are pairwise distinct. | Test (TC-1317) |
| FR-522-AC-13 | For **R**'s fixed-sample clause, changing alone the `epoch`, the `period`, the `unit` of the clock row, the profile, the clock name, one stamp, one valuation, the watermark, `through`, the lower boundary, one of the thirteen work ceilings of FR-523 or the meter's work-unit ceiling each gives a `result_identity` distinct from AC-12's. | Test (TC-1317) |
| FR-522-AC-14 | After AC-13's changed request is evaluated, the first request's result is neither reused nor rewritten: its bytes equal those it returned before. | Test (TC-1317) |
| FR-522-AC-15 | For an event-position `Progressed` request, changing alone the `sequence_authority` of the row, the watermark or the scope closure gives a distinct `result_identity`, and the identity of the in-process package equals the identity of the package read back with the reader. | Test (TC-1317) |

## Dependencies

- FR-043-AC-3, AC-13 and AC-14 and FR-050-AC-6 (the legacy criteria this
  requirement succeeds on the spine), FR-501 and FR-504 (the carriers of the
  tick-valued requests), FR-516 (the evidence call), FR-515 (the profile and clock
  rows), FR-523 (the work ceilings that join the identity), FR-526 (the relation
  that joins the evidence identity).
- The evaluation of the compensation-window clock row of FR-050-AC-13 and AC-14 is
  not a temporal-clause evaluation. It stays with FR-050 and the timed `/2`
  tickets (QSL-1414, QSL-1384, QSL-1446) and is not restated here.
- Feeds QSL-1545 (FR-516): the `TraceBinding` member of `EvidenceRequest`, the
  `authenticated` member of `EvidenceResult` and the `result_identity` rule.

## References

- Linear QSL-1502 (this requirement), QSL-858 (deletion of `src/temporal` waits on
  it), deletion-table rows B3, B13 and B15.
