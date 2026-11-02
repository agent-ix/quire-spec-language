---
id: FR-509
title: "Declare a channel's delivery count interval"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-464
    type: depends_on
---
# FR-509: Declare a channel's delivery count interval

## Description

The composed protocol surface wrote a channel as `... ordering <o> delivery
[lo,hi];`, with `lo` and `hi` natural numbers, `lo <= hi`. The interval is a
per-send cardinality: the number of deliveries one send produces, not elapsed
time (`docs/compiled-protocol-v1.md`, "Channel delivery bounds are per-send
cardinalities, not elapsed time"). The composed emitter wrote it into the
channel's delivery bounds (FR-048-AC-2). The spine writes a channel's delivery
as one of four policy words (`unknown`, `at-most-once`, `at-least-once`,
`exactly-once-premise`) plus `capacity` and `overflow`, so the interval can be
neither written nor checked. This requirement adds the interval to the spine
channel and states what it means; the four policy words keep their meaning.

## Inputs

- `channel <name> from <a> to <b> carries <T> ordering <o> delivery [<lo>,
  <hi>] capacity <c> overflow <p>;` where `lo` and `hi` are non-negative
  integer literals.

## Outputs

- A checked `ChannelDelivery::Between{lo, hi}` premise, recorded as written
  beside the channel's other premises, and a channel node whose delivery text
  is `[lo,hi]` with no space.

## Behavior

- The grammar SHALL admit the interval as a fifth `delivery` form beside the
  four policy words. A channel with an interval is otherwise a channel
  (FR-308), and the form `delivery [lo, hi]` accepts any spacing inside the
  brackets.
- S3 SHALL refuse `lo > hi` with `ill_typed`/`type-mismatch` at the interval,
  naming both bounds, and SHALL refuse a bound that is negative, fractional or
  larger than `u64::MAX` at parse (`invalid_syntax`). `[0,0]` and `[n,n]` are
  admitted.
- S3 SHALL record the interval as an authored premise and derive no other
  premise from it, and none from a policy word (FR-218-AC-13): a channel with
  `[1,1]` and one with `exactly-once-premise` are two different checked
  channels.
- S4 SHALL emit the delivery text `[lo,hi]` for an interval channel, and the
  four policy words unchanged for the others. Two channels differing only in
  `lo` or `hi` emit different delivery texts.
- Exploration of a channel's queue (the semantics TC-383 states for the named
  policies) SHALL let one send enqueue any count `d` with `lo <= d <= hi` of
  deliverable entries, each occupying one place of the channel's capacity and
  each subject to its overflow policy; a receive removes one entry. An interval channel enables exactly the executions its counts give and adds no separate `lose` or `duplicate` nondeterminism (FR-211-AC-5 applies to the named policies only).
  `[0,1]` therefore offers the executions of a send with no delivery and with
  one delivery, and `[2,2]` offers only the execution with two deliveries.
- The interval SHALL NOT be read as time: no clock, unit or timed profile is
  required or consulted by it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-509-AC-1 | `channel ch from a to b carries Boolean ordering unordered delivery [0,3] capacity 4 overflow block;` parses, checks and records `Between{0, 3}`, capacity `Literal(4)`, `Block` and the unordered ordering, each as written; `delivery [ 0 , 3 ]` records the same. | Test (TC-1304) |
| FR-509-AC-2 | `delivery [4,3]` refuses `ill_typed`/`type-mismatch` at the interval naming 4 and 3; `delivery [-1,3]`, `delivery [0.5,3]` and `delivery [0,18446744073709551616]` each refuse `invalid_syntax` at the offending token; `delivery [2,2]` and `delivery [0,0]` admit. | Test (TC-1304) |
| FR-509-AC-3 | Channels with `delivery [1,1]` and with `delivery exactly-once-premise`, otherwise equal, are two different checked channels, and neither changes the other's recorded capacity, overflow or ordering. | Test (TC-1304) |
| FR-509-AC-4 | The emitted package holds the delivery text `[0,3]` for AC-1's channel, a different text `[0,4]` when only `hi` changes, the unchanged word text for each policy word, and reads back through the I2 reader. | Test (TC-1304) |
| FR-509-AC-5 | On a `fifo` channel of capacity 2 with `delivery [2,2]`, `send P; receive R1 of P; receive R2 of P` completes in every execution and a third receive stops `empty-queue`; with `[0,1]` the executions include one where `receive R1 of P` stops `empty-queue` and one where it completes; with `[1,3]`, capacity 2 and `overflow block`, `send P` has the executions d = 1 and d = 2 that enqueue, and one (d = 3) that stops `full-queue` at the send; on a `[1,1]` channel no execution loses or duplicates the message (FR-211-AC-5's `lose` and `duplicate` stay disabled), and on a `[0,1]` channel the only losing execution is d = 0. | Test (TC-1304) |
| FR-509-AC-6 | A `delivery [1,2]` channel in a unit whose header selects no temporal profile and no clock checks and emits; the interval reads no clock. | Test (TC-1304) |

## Dependencies

- **Contract change required (QSpec FR-464/FR-466):** the protocol `channel`
  node's delivery text vocabulary currently names the four policy words; it
  must admit the text `[lo,hi]` (two non-negative integers, `lo <= hi`).
  Without it FR-509-AC-4 cannot pass. No Contract IR change is needed if
  QSpec carries the delivery text as an opaque string; if the IR validates
  the four words, that validator must admit the interval (a question to the
  Contract IR owner filed with this batch).
- **Upstream:** FR-048-AC-2 (the legacy behavior), FR-218 (channel premises),
  TC-383 (queue semantics).
- **Fixture note:** `tests/fixtures/composed-choreography.native` is refused
  by the spine today, and TC-885 asserts that refusal (a refusal that is not
  an incomplete outcome). The fixture also uses composed-only forms (`on
  each`, capture, role and relationship forms), so the refusal stays after
  this requirement; its cause is not promised to change. The fixture is not
  edited.
