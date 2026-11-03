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

Verify FR-383-AC-18, AC-28, AC-29, AC-30 and the S4 portion of AC-31: each protocol of a unit has its own
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
| 6 | **X** alone | the same refusal identity as row 2: code, typed cause and declaration/reference names equal; `source_index`, declaration span and any cause span are relative to this unit |
| 7 | **W**, **X**, **Y** | **Y** `Refused` `missing_declaration`/`missing-name` at the reference to **X**, with `refused_dependency` naming **X** and `consequence` true; **W** unchanged; aggregate `Refused` with **X**'s code, cause and locus as the primary |
| 8 | **W** and a protocol over `Boolean` (`NoOwnerModel`) | results [Emitted, Refused `unsupported_construct`/`not-yet-implemented`]; aggregate `Refused`; **W** bytes equal row 1's |
| 9 | **W**, **Z**, **V** (renamed copies, equal measured work `n`) with an `s3.work_units` budget of exactly `n` | results [Emitted, `Incomplete` naming `s3.work_units`, `Incomplete` naming `s3.work_units`]; aggregate `Incomplete`; exit 22 |
| 10 | row 9 in the source order **Z**, **W**, **V**; then row 9 with a budget of `3n` in both orders | the first: **Z** `Emitted`, **W** and **V** `Incomplete`; the second: three `Emitted` in either order |
| 11 | row 2 with **X** removed, then with **W** removed | **W**'s result changes in no byte; **X**'s refusal identity (code, typed cause and declaration/reference names) is unchanged. `source_index`, declaration span and any cause span reflect the new unit order and layout |
| 12 | **X** with a syntax error confined to its declaration; **X** with an S2 refusal; **X** with an S3 refusal; **X** with an S4 `NoOwnerModel` refusal; each beside **W** | **X** `Refused` with that stage's code and cause; **W** equal to row 1's |
| 13 | a syntax error in the unit header | `Refused` unit, no `ProtocolResult` |

The checked-package emitter procedure exercises the S3 and S4 portions of rows
12–13 only where a checked protocol result exists. S1, S2 and unit-header
refusal boundaries belong to FR-503 and are verified by TC-1298, not by the
emitter API. TC-1297 row 12's S3 refusal is the missing-join case from rows 2–7.

## Expected Results

- A refused protocol has a `Refused` result with its locus and no node; every
  other protocol has the result it has alone.
- The aggregate is `Refused` or `Incomplete` whenever any protocol is, carries
  no package, and never exits 0.
- Diagnostics hold one record per refused protocol in declaration order.
- Rows 9 and 10 are the only rows whose results depend on declaration order,
  and only through the budget.
- Test code carries FR-383-AC-18, AC-28 to AC-30 and TC-1297 for the checked
  result and budget behavior; FR-383-AC-31's S1/S2/header boundaries are tagged
  to FR-503/TC-1298. The emitter's NoOwnerModel assertion covers only AC-31's
  S4 portion.
