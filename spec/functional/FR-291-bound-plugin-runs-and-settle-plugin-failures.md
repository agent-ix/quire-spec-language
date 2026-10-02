---
id: FR-291
title: "Settle plugin-run failures as typed terminal records per item"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: "ix://agent-ix/quire-specification/FR-305"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-462"
    type: depends_on
---
# FR-291: Settle plugin-run failures as typed terminal records per item

## Description

QSpec FR-462 owns a plugin run: its lifecycle (start at the path the caller
names, `hello`, registration, request and result exchanges, `shutdown`,
exit), its budgets, its cancel and grace period, and the settlement of every
failure event. QSpec FR-305 owns the plugin forms and the run budgets. The
driver's plugin host implements them (ADR-029 PL-2, PL-3, PL-6, PL-7).

QSL's part is the terminal record each FR-462 settlement becomes, and its
category:

| FR-462 settlement | QSL terminal record | O-16 category | Exit (FR-285) |
| --- | --- | --- | --- |
| `failed` with a tool-failure cause (crash, non-zero exit, malformed or non-canonical frame, foreign protocol identity, frame or total byte limit, unrouted item, unadvertised kind, missing record) | `failed`, with a typed `ToolFailure` cause naming the event, and the limit and its value for a byte limit | internal failure | 30 |
| `incomplete` with cause `timed-out` | `incomplete`, cause `TimedOut` (ADR-014 B-5) | incomplete | 22 |
| `incomplete` with cause cancelled | `incomplete`, cause cancelled (FR-276) | incomplete | 22 |

## Inputs

The driver's settlement of each item routed to a plugin, as QSpec FR-462
states it.

## Outputs

One QSL terminal record per item routed to the plugin, in the outcome that
also holds the records of items routed to other providers.

## Behavior

- QSL shall represent each FR-462 tool-failure settlement as a `failed`
  terminal record whose typed `ToolFailure` cause names the event, and, for a
  frame or total byte limit, the limit and its value.
- A `failed` record with a `ToolFailure` cause shall have the O-16 category
  internal failure.
- An `incomplete` record with cause `TimedOut` or cancelled shall have the
  O-16 category incomplete.
- When an outcome holds a plugin item's `failed` or `incomplete` record
  beside other providers' records, QSL shall leave each other record
  unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-291-AC-1 | For each of the eight FR-462 tool-failure events, a `failed` record built with the `ToolFailure` cause for that event serializes (FR-286) with category internal failure and the event named, and FR-285 maps it to exit 30; the frame-limit record also names the limit and its value. | Test (TC-775) |
| FR-291-AC-2 | An `incomplete` record with cause `TimedOut` and one with cause cancelled each serialize with category incomplete, and FR-285 maps each to exit 22. | Test (TC-775) |
| FR-291-AC-3 | An outcome of two items, one a plugin item's `failed` record and one a compile-time provider's `proved` record, holds the second record with bytes equal to that record in a one-item outcome, and FR-285 maps the outcome to exit 30. | Test (TC-775) |

## Dependencies

- ADR-029 PL-2, PL-3, PL-6, PL-7: transport, budgets, lifecycle and failures.
- ADR-014 B-5: `TimedOut`.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): cancellation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md): plugin results.
- QSpec FR-462: the plugin run, its failure events and their settlements;
  QSpec FR-305: the plugin forms and run budgets; QSpec FR-331: `failed`
  and `incomplete`.

## Overlap

The driver repository specifies and tests `quire-plugin-host` against QSpec
FR-462 and FR-305: starting the plugin at the named path, stderr capture up
to the caller's byte limit, deadline and kill, `cancel` and the grace
period, and the detection of each failure event.

## References

- QSL-393 (V1-A06a): the plugin wire.
- QSL-390 (ARCH-50).
- QSpec FR-305, FR-462 (STD-141): the QSpec half.
