---
id: TC-1304
title: "Declare a channel's delivery count interval"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-509
    type: verifies
---
# TC-1304: Declare a channel's delivery count interval

## Description

Verify the delivery interval of FR-509: parsing, refusals, premises, canonical
emission, the strict package reader and the QSpec FR-426 interval-send queue
semantics. Scope: FR-509-AC-1 to FR-509-AC-10. The QSpec vectors are TC-947
`CH13a`, `CH13b`, `CH14a` and TC-378, referenced by row id and never copied.

## Test Procedure

Fixture protocol `D0` has roles `a` and `b` and one channel `ch` whose clause
is substituted per row; the smallest valid body sends and receives on `ch`.

| Row | Channel clause | Expected |
| --- | --- | --- |
| 1 | `delivery [0,3] capacity 4 overflow block`, then `delivery [ 0 , 3 ]` | both record `Between{0, 3}`, `Literal(4)`, `Block`, unordered |
| 2 | `delivery [4,3]` | `ill_typed`/`type-mismatch` at the interval naming 4 and 3 |
| 3 | `delivery [-1,3]`; `delivery [0.5,3]`; `delivery [0,18446744073709551616]` | each `invalid_syntax` at the offending token |
| 4 | `delivery [2,2]`; `delivery [0,0]`; `delivery [0,18446744073709551615]` | all admitted |
| 5 | `delivery [1,1]` versus `delivery exactly-once-premise` | two different checked channels; capacity, overflow, ordering equal in both |
| 6 | emit row 1's two spellings, a variant with `[0,4]`, `[0,18446744073709551615]`, and each policy word | delivery texts `[0,3]` (both spellings), `[0,4]`, `[0,18446744073709551615]` and the four words; each reads back through FR-519's `delivery` view as the checked premise |
| 7 | `fifo`, capacity 2, `overflow block`, `delivery [2,2]`: `send P; receive R1 of P; receive R2 of P; receive R3 of P` | one `count` 2 send step; R1, R2 complete in every execution; R3 waits `empty-queue` |
| 8 | as row 7 with `delivery [0,1]`: `send P; receive R1 of P` | a `d = 0` step (send record `entries` `[]`, `pending` `"0"`) after which R1 waits `empty-queue`; a `d = 1` step after which R1 completes |
| 9 | as row 7 with `delivery [1,3]`: `send P; receive R1 of P` | `d = 1` and `d = 2` steps queue and settle; the `d = 3` step appends two entries, send record `pending` `"1"`, the thread waits `full-queue` with `send-pending` (1, `P`); after R1 the continuation step, `count` 1, appends the last entry, settles and binds the binder to `P` |
| 10 | `delivery [1,1]`; then each interval of rows 7 to 9 | no execution loses or duplicates the message; no `duplicate` or `lose` step on any interval channel |
| 11 | `delivery [1,2]` in a unit with no temporal profile and no clock | checks and emits |
| 12 | parse a channel clause with `delivery [0,3]` (CST only, no check); the AC is FR-509-AC-7 | parses with no syntax error |
| 13 | a channel node with delivery text `[0,3]`, `[0,0]`, `[2,2]`, `[0,18446744073709551615]`; then each of `[4,3]`, `[0, 3]`, `[ 2 , 2 ]`, `[00,3]`, `[+0,3]`, `[-1,3]`, `[0.5,3]`, `[0,18446744073709551616]`, `[3]`, `[0,3`, `0,3`, read by the `delivery` view | the first four admitted; each of the rest a `ViewRefusal` naming `delivery`, no partial value |
| 14 | `fifo`, capacity 1, `block`, `delivery [0,3]`, the place full | the `d = 0` step enabled, queues nothing, settles; `d >= 1` steps wait |
| 15 | `delivery [0,0]`, capacity 0 | the send completes by the `d = 0` step |
| 16 | capacity 1, `overflow reject`, `delivery [2,2]`, queue empty | one step; send record `entries` `accepted` then `rejected`, `pending` `"0"` |
| 17 | capacity 1, `block`, `delivery [2,2]`; one run's send of `m1` and another run's send of `m2`, otherwise equal, each leave count 1 pending | the two pending states have different keys; after a receive each continuation binds its own payload |
| 18 | `delivery [0,18446744073709551615]` under a finite work limit; `delivery [0,2]` unlimited | steps generated in ascending `d` from `d = 0`; `incomplete` naming the work limit, no count dropped, no `hi` capped, no entries truncated; exactly `d = 0`, `1`, `2` for `[0,2]` |

Row 13 replaces the legacy reader test that admits `[ 2 , 2 ]`.

Tag the test `#[trace("FR-509-AC-1", "FR-509-AC-2", "FR-509-AC-3", "FR-509-AC-4", "FR-509-AC-5", "FR-509-AC-6", "FR-509-AC-7", "FR-509-AC-8", "FR-509-AC-9", "FR-509-AC-10", "TC-1304")]`.

## Expected Results

Each row's Expected column.
