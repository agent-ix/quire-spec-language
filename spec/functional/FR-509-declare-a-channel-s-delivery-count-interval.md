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
  - target: ix://agent-ix/quire-specification/FR-465
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-426
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
---
# FR-509: Declare a channel's delivery count interval

## Description

The composed protocol surface wrote a channel as `... ordering <o> delivery
[lo,hi];`, with `lo` and `hi` natural numbers, `lo <= hi`. The interval is a
per-send cardinality: the number of deliveries one send produces, not elapsed
time (the retired `quire.compiled-protocol/1` wire contract: channel delivery bounds are per-send
cardinalities, not elapsed time). The composed emitter wrote it into the
channel's delivery bounds (FR-048-AC-2). The spine writes a channel's delivery
as one of four policy words (`unknown`, `at-most-once`, `at-least-once`,
`exactly-once-premise`) plus `capacity` and `overflow`, so the interval can be
neither written nor checked. This requirement adds the interval to the spine
channel and states what it means; the four policy words keep their meaning.
QSpec FR-465 "Delivery count interval" owns the channel node's delivery text,
and QSpec FR-426 "Interval sends" and FR-170 own the queue semantics; this
requirement states how QSL's grammar, checker, emitter, package reader and
explorer realize them and invents none.

## Inputs

- `channel <name> from <a> to <b> carries <T> ordering <o> delivery [<lo>,
  <hi>] capacity <c> overflow <p>;` where `lo` and `hi` are non-negative
  integer literals. Source may put spaces inside the brackets.
- A checked package's channel node, whose `delivery` text the package reader
  reads back.

## Outputs

- A checked `ChannelDelivery::Between{lo, hi}` premise, recorded as written
  beside the channel's other premises.
- A channel node whose delivery text is the canonical interval text of QSpec
  FR-465: `[`, the decimal numeral of `lo`, `,`, the decimal numeral of `hi`
  and `]`, with no space and no other character, each numeral a natural
  number no greater than 18446744073709551615 written with no sign and no
  leading zero (`0` itself is a numeral), and `lo <= hi`. One interval has
  exactly one text.

## Behavior

- The grammar SHALL admit the interval as a fifth `delivery` form beside the
  four policy words. A channel with an interval is otherwise a channel
  (FR-308). The source form `delivery [lo, hi]` accepts any spacing inside
  the brackets; spacing is not part of the premise.
- S3 SHALL refuse a source interval with `lo > hi` with
  `ill_typed`/`type-mismatch` at the interval, naming both bounds, and SHALL
  refuse a bound that is negative, fractional or larger than `u64::MAX` at
  parse (`invalid_syntax`). `[0,0]` and `[n,n]` are admitted.
- S3 SHALL record the interval as an authored premise and derive no other
  premise from it, and none from a policy word (FR-218-AC-13): a channel with
  `[1,1]` and one with `exactly-once-premise` are two different checked
  channels.
- S4 SHALL emit the canonical delivery text `[lo,hi]` for an interval
  channel, whatever spacing the source used, and the four policy words
  unchanged for the others. Two channels differing only in `lo` or `hi` emit
  different delivery texts; two channels whose source intervals differ only
  in spacing emit the same text.
- FR-519's package reader SHALL admit, as a channel node's delivery text,
  exactly the four policy words and the canonical interval text.
- If a channel node's delivery text is neither a policy word nor the
  canonical interval text (a space anywhere in the text, a leading zero, a
  sign, a fraction, a numeral above 18446744073709551615, a missing bound or
  bracket, or `lo > hi`), then FR-519's package reader SHALL refuse it with
  `ViewRefusal::MalformedMember` naming the `delivery` member, return no partial value, and
  not trim, re-parse or repair the text. This matches QSpec FR-465's strict wire rule, under which such a text
  refuses `malformed_wire` at the term before QSL reads it.
- Exploration of a channel's queue SHALL follow QSpec FR-426 "Interval sends"
  and FR-170 "Delivery count interval":
  - a thread at a send with no pending send has one `send` step per
    (payload, `d`) with `lo <= d <= hi`, the step identity carrying `count`
    `d`; the step processes `d` entries of that payload one after another,
    each taking one place of the channel's capacity and each subject to the
    overflow policy for that entry (under `reject` or `loss` all `d` are
    processed in the step);
  - the `d = 0` step places nothing and settles past the send, so it is
    enabled whatever the queue holds, under every overflow policy and at
    capacity 0;
  - under `block` a step with `d >= 1` is enabled only while the queue is
    below capacity; it appends entries while the queue has room, and if the
    queue fills with `r > 0` entries unprocessed the send is **pending**: the
    thread rests at the send, waits `full-queue`, binds no binder, and the
    state key holds the pair (`r`, payload) as the thread's `send-pending`
    entry (QSpec FR-425), so two pending sends differing in count or payload
    are different states; once the queue is below capacity the pending send
    has one continuation `send` step, with `count` `r` and that payload, that
    appends the next entries the same way, and the step that settles past the
    send binds the binder to the payload;
  - the step's send record is `{type: send-record, channel, payload, count,
    entries, pending}`, with one `entries` member (`outcome` and `displaced`)
    per processed entry in processing order (`[]` for `d = 0`) and `pending`
    the entries left pending (`"0"` when none), in place of the single-entry
    record's `outcome` and `displaced`;
  - an interval channel has no `duplicate` or `lose` step: `d = 0` is its only
    transport loss and `d > 1` its only duplication (FR-211-AC-5 applies to
    the named policies only);
  - the explorer generates a send's interval steps lazily in ascending `d`,
    the step for `d + 1` only after the step for `d`, and charges each
    generated step one work unit and each processed entry one work unit
    against the run's work limit; an exhausted limit settles the run
    `incomplete` naming the work limit, with the expansion's state on the
    unexplored frontier. It SHALL NOT drop a count, cap `hi`, truncate a
    step's entries or merge counts to stay within the limit.
- The interval SHALL NOT be read as time: no clock, unit or timed profile is
  required or consulted by it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-509-AC-1 | `channel ch from a to b carries Boolean ordering unordered delivery [0,3] capacity 4 overflow block;` parses, checks and records `Between{0, 3}`, capacity `Literal(4)`, `Block` and the unordered ordering, each as written; source `delivery [ 0 , 3 ]` records the same. | Test (TC-1304) |
| FR-509-AC-2 | Source `delivery [4,3]` refuses `ill_typed`/`type-mismatch` at the interval naming 4 and 3; `delivery [-1,3]`, `delivery [0.5,3]` and `delivery [0,18446744073709551616]` each refuse `invalid_syntax` at the offending token; `delivery [2,2]`, `delivery [0,0]` and `delivery [0,18446744073709551615]` admit. | Test (TC-1304) |
| FR-509-AC-3 | Channels with `delivery [1,1]` and with `delivery exactly-once-premise`, otherwise equal, are two different checked channels, and neither changes the other's recorded capacity, overflow or ordering. | Test (TC-1304) |
| FR-509-AC-4 | The emitted package holds the delivery text `[0,3]` for AC-1's channel written either as `[0,3]` or as `[ 0 , 3 ]`, a different text `[0,4]` when only `hi` changes, `[0,18446744073709551615]` for that bound, and the unchanged word text for each policy word; each reads back through FR-519's `delivery` view as the checked premise. | Test (TC-1304) |
| FR-509-AC-5 | Under QSpec FR-426 "Interval sends", on a `fifo` channel of capacity 2 under `overflow block`: with `delivery [2,2]`, `send P` has one `count` 2 step queuing two entries, so `receive R1 of P` and `receive R2 of P` complete in every execution and a third receive waits `empty-queue`; with `[0,1]`, `send P` has a `d = 0` step queuing nothing (send record `entries` `[]`, `pending` `"0"`), after which `receive R1 of P` waits `empty-queue`, and a `d = 1` step after which it completes; with `[1,3]`, `send P` has `d = 1` and `d = 2` steps that queue and settle past the send, and a `d = 3` step that appends two entries and leaves the send pending with count 1 and payload `P` (send record `pending` `"1"`), the thread waiting `full-queue` at the send; after one receive the pending send's one continuation step, `count` 1, appends the last entry, settles and binds the binder to `P`. On a `[1,1]` channel no execution loses or duplicates the message, and on any interval channel no `duplicate` or `lose` step exists; on `[0,1]` the only losing execution is `d = 0`. | Test (TC-1304) |
| FR-509-AC-6 | A `delivery [1,2]` channel in a unit whose header selects no temporal profile and no clock checks and emits; the interval reads no clock. | Test (TC-1304) |
| FR-509-AC-7 | The parser accepts `delivery [<lo>,<hi>]` in a channel clause as the fifth delivery form beside the four policy words, so a channel that declares an interval parses with no syntax error. | Test (TC-1304) |
| FR-509-AC-8 | FR-519's `delivery` view admits a channel node whose delivery text is `[0,3]`, `[0,0]`, `[2,2]` or `[0,18446744073709551615]`, and refuses with `ViewRefusal::MalformedMember` naming the `delivery` member (never `MissingMember`), with no partial value, each of `[4,3]`, `[0, 3]`, `[ 2 , 2 ]`, `[00,3]`, `[+0,3]`, `[-1,3]`, `[0.5,3]`, `[0,18446744073709551616]`, `[3]`, `[0,3` and `0,3` (the QSpec FR-465-AC-10 spellings). | Test (TC-1304) |
| FR-509-AC-9 | On a `fifo` channel of capacity 1 under `block` with `delivery [0,3]` and its one place full, the `d = 0` step is enabled, queues nothing and settles past the send while the `d >= 1` steps wait; a `[0,0]` channel of capacity 0 completes its send by the `d = 0` step; under `overflow reject` with capacity 1 and `[2,2]`, the one step records `entries` `accepted` then `rejected` and `pending` `"0"`; on a `fifo` channel of capacity 1 under `block` with `delivery [2,3]` and `carries Boolean`, one exploration of `send P` (payload `true` or `false`) followed by one `receive R of P` whose binder no later step reads reaches, after the receive has emptied the queue, the states pending (1, `true`), (1, `false`) and (2, `true`), whose `send-pending` entries are the only member in which they differ and whose keys are pairwise different; each continuation step binds the send's binder to its own payload. | Test (TC-1304) |
| FR-509-AC-10 | On a `fifo` channel of capacity 2 under `overflow reject` with `delivery [0,1000]`, let `W0` be the work an unlimited run consumes before it expands the state at the send. Under work limit `W0 + 3` the explorer generates the `d = 0` step (1 unit) and the `d = 1` step (1 unit and 1 entry), never generates the `d = 2` step, consumes exactly 3 units at the send, and settles `incomplete` naming the work limit with the state at the send on the unexplored frontier; it is not a refusal and not a completed result with a count dropped, `hi` capped or a step's entries truncated. Under an unlimited work limit the same state has exactly 1001 `send` steps, `count` 0 to 1000, generated in ascending `count` (QSpec FR-426-AC-21, TC-378 CH-21). | Test (TC-1304) |

## Dependencies

- **Contract (QSpec):** QSpec FR-465 "Delivery count interval" and "Delivery
  spellings" (FR-465-AC-3, AC-4 and AC-10) define the channel node's
  delivery text and its strict refusals; QSpec FR-426 "Interval sends" and
  "Interval size" (FR-426-AC-19 and AC-20), FR-170 "Delivery count interval"
  and FR-425's `send-pending` state-key entry define the step relation, the
  send record and the work charge; vectors TC-947 `CH13a`, `CH13b`, `CH14a`
  and TC-378. The Contract IR strict reader parses the same text and refuses
  a non-canonical one before QSL reads the package. This requirement cites
  them by id.
- **Implementation:** QSL-1526 (grammar, checking, emission, package reader,
  exploration). The package reader's legacy `delivery` parser in
  `qsl-package/src/protocol_view.rs` trims spaces and admits a sign, leading
  zeros and `lo > hi`, and its test in
  `qsl-package/src/emit/tests/protocol_view.rs` asserts `[ 2 , 2 ]` is
  admitted; both contradict FR-509-AC-8 and are a code defect QSL-1526 owns.
- **Upstream:** FR-048-AC-2 (the legacy behavior), FR-218 (channel premises),
  FR-519 (the `delivery` view); QSpec TC-378 (queue semantics).
- **Fixture note:** `tests/fixtures/composed-choreography.native` is refused
  by the spine today, and TC-885 asserts that refusal (a refusal that is not
  an incomplete outcome). The fixture also uses composed-only forms (`on
  each`, capture, role and relationship forms), so the refusal stays after
  this requirement; its cause is not promised to change. The fixture is not
  edited.
