---
id: TC-1297
title: "Protocol results stay independent when one protocol is refused"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: verifies
---
# TC-1297: Protocol results stay independent when one protocol is refused

## Description

Verify FR-383-AC-18, AC-28 and AC-29: each protocol of a unit has its own
result, one protocol's S3 refusal gives the unit a `Refused` aggregate with no
package, and the other protocols' results are retained byte for byte. This
keeps FR-042-AC-8's independent per-protocol results on the spine.

Catches a spine that refuses the whole unit and discards the good protocol's
result, a good protocol whose bytes change because a neighbour was refused, a
refusal record that depends on declaration order, a unit that exits 0 with a
refused protocol, and a dependent protocol that silently emits.

Fixture **W** is TC-1061's. **X** is **W**'s shape with a `parallel` join
naming a branch label that does not exist. **Y** is a protocol whose predicate-join
function names **X**. **Z** is a second good protocol (**W** renamed).

## Test Procedure

Compile through S1 to S4 and assert fields, never message text.

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **W** alone | aggregate `Emitted`; package and `package_id` present; record the bytes of **W**'s nodes and clause key |
| 2 | **W**, then **X** | results [W `Emitted`, X `Refused`]; **W**'s nodes and clause key equal row 1's bytes; **X** `missing_declaration`/`missing-name` at the label, locus the source index and span of **X**'s declaration; aggregate `Refused`; no package, no `package_id` |
| 3 | **X**, then **W** | the same two results and records as row 2, reported in this declaration order |
| 4 | row 2's diagnostics and exit | exactly one record (**X**'s); exit 20 (FR-285-AC-2 over {0, 20}), never 0 |
| 5 | **W**, **X**, **Z** | results [Emitted, Refused, Emitted]; both good protocols' nodes equal their standalone emissions |
| 6 | **X** alone | the same refusal record as row 2 in every field |
| 7 | **W**, **X**, **Y** | **Y** `Refused` `missing_declaration`/`missing-name` at the reference to **X**, naming **X**; **W** unchanged; aggregate `Refused` |
| 8 | **W** and a protocol over `Boolean` (`NoOwnerModel`) | results [Emitted, Refused `unsupported_construct`/`not-yet-implemented`]; aggregate `Refused`; **W** bytes equal row 1's |
| 9 | **W**, **Z** under an `s3.work_units` budget that stops **Z** | results [Emitted, `Incomplete` naming `s3.work_units`]; aggregate `Incomplete`; exit 22 |
| 10 | row 2 with **X** removed, then with **W** removed | **W**'s result changes in no byte; **X**'s refusal record changes in no field |

## Expected Results

- A refused protocol has a `Refused` result with its locus and no node; every
  other protocol has the result it has alone.
- The aggregate is `Refused` or `Incomplete` whenever any protocol is, carries
  no package, and never exits 0.
- Diagnostics hold one record per refused protocol in declaration order.
- Test code carries the tags FR-383-AC-18, FR-383-AC-28, FR-383-AC-29 and
  TC-1297.
