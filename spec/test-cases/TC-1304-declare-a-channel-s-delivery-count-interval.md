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

Verify the delivery interval of FR-509: parsing, refusals, premises, emission
and queue semantics. Scope: FR-509-AC-1 to FR-509-AC-6.

## Test Procedure

Fixture protocol `D0` has roles `a` and `b` and one channel `ch` whose clause
is substituted per row; the smallest valid body sends and receives on `ch`.

| Row | Channel clause | Expected |
| --- | --- | --- |
| 1 | `delivery [0,3] capacity 4 overflow block`, then `delivery [ 0 , 3 ]` | both record `Between{0, 3}`, `Literal(4)`, `Block`, unordered |
| 2 | `delivery [4,3]` | `ill_typed`/`type-mismatch` at the interval naming 4 and 3 |
| 3 | `delivery [-1,3]`; `delivery [0.5,3]`; `delivery [0,18446744073709551616]` | each `invalid_syntax` at the offending token |
| 4 | `delivery [2,2]`; `delivery [0,0]` | both admitted |
| 5 | `delivery [1,1]` versus `delivery exactly-once-premise` | two different checked channels; capacity, overflow, ordering equal in both |
| 6 | emit rows 1 and a variant with `[0,4]`, and each policy word | delivery texts `[0,3]`, `[0,4]`, and the four words; all read back through I2 |
| 7 | `fifo`, capacity 2, `delivery [2,2]`: `send P; receive R1 of P; receive R2 of P; receive R3 of P` | R1, R2 complete in every execution; R3 stops `empty-queue` |
| 8 | capacity 2, `delivery [0,1]`: `send P; receive R1 of P` | one execution stops `empty-queue` at R1, one completes |
| 9 | capacity 2, `overflow reject`, `delivery [1,3]`: `send P` | the executions with d = 1 and 2 enqueue; d = 3 follows `reject` for the third entry |
| 10 | `delivery [1,2]` in a unit with no temporal profile and no clock | checks and emits |

Tag the test `#[trace("FR-509-AC-1", "FR-509-AC-2", "FR-509-AC-3", "FR-509-AC-4", "FR-509-AC-5", "FR-509-AC-6", "TC-1304")]`.

## Expected Results

Each row's Expected column.
