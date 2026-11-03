---
id: TC-1309
title: "Specify the spine type report, the inventory on exhaustion and the multi-unit cycle refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-514
    type: verifies
---
# TC-1309: Specify the spine type report, the inventory on exhaustion and the multi-unit cycle refusals

## Description

Verify the type report, the inventory outcomes and the cross-unit cycle
refusals of FR-514. Scope: FR-514-AC-1 to FR-514-AC-12.

## Test Procedure

Fixture `T0`: a unit with `function f using v(x: Int[0, 9]): Int[0, 9] pure {
x }`, a state `post` clause on an object operation, and a protocol with one
`check` and one `requirement`.

| Row | Input | Expected |
| --- | --- | --- |
| 1 | `T0`, defaults | `type_report()` lists declarations in ascending `NodeRef` order with value types, one `RequirementRecord` per requirement, one `PendingObligation` per claim and per protocol requirement obligation, each `Pending` with its claim form's kind; no source reference field |
| 2 | FR-480-AC-2's units `A` and `B` | two `CheckedUnit`s, each report listing its own declarations only |
| 3 | `T0`, `s3.work_units` one below measured need | `check` is the limit outcome; inventory `Incomplete`, one requirement row per requirement form, no obligation row, cause (S3, work budget, `s3.work_units`) |
| 4 | `T0`, `s1.nodes` below the node count | `Incomplete`, zero rows, cause (S1, node count, `s1.nodes`) |
| 5 | `T0` with an ill-typed body | an error, no rows |
| 6 | `T0`, sufficient limits | `Complete` |
| 7 | `T0` with formal identities `F1`, `F2`; `Incomplete` and `Complete` outcomes with `F1` | two identities; one identity |
| 8 | FR-480-AC-6 documents, list `[1, 2]`; list `[2, 1]` | `invalid_package`/`invalid-value` at `z` in document 1; `invalid_package`/`definition-cycle` at `u_self` in document 2, edge `u_self` to `u_self` |
| 9 | document 1 `unit z : T = z2 * rational(2, 1); unit z2 : T = z * rational(2, 1);`, document 2 `dimension L;` | `invalid_package`/`definition-cycle`, both edges, naming document 1 |
| 10 | the QSpec TC-187 dimension-cycle vector split across documents 1 and 2 (vector referenced by case ID, not copied) | `invalid_package`/`definition-cycle`, naming the first document in list order |
| 11 | the pieces table of FR-514 against FR-480's acceptance criteria | every FR-480-AC id present; only AC-2 in two rows, with disjoint sentences |
| 12 | `T0` with an `enum` beside the forms, `s3.work_units` one below need | `Incomplete`; one requirement-form row each for `f`, the state clause and the protocol with ordinals 0, 1, 2; no row for the `enum` |
| 13 | `T0`, sufficient limits | `Complete`; one row per requirement record and per pending obligation; each obligation row's IR identity holds the exact capability kind name, the owning requirement form ordinal, unit 0 and the span |
| 14 | the limits of rows 3 and 4 | the IR cause holds stage, setting, dimension `constraints` / `expressions`, `limit` the bound, `observed` the actual value, no prior, no requested, no native site |
| 15 | `parsed` `None` over an S1 limit, an S3 limit and `Ok(unit)` | `Incomplete` with zero rows; `MissingParsedUnit`; `MissingParsedUnit` |
| 16 | a pending obligation whose span lies outside every requirement form | the owner lookup gives none; the inventory is refused with `UnownedObligation`, no row with an invented ordinal |
| 17 | a `TickCount` limit outcome | `NoInventory`, no rows |

Tag the test `#[trace("FR-514-AC-1", "FR-514-AC-2", "FR-514-AC-3", "FR-514-AC-4", "FR-514-AC-5", "FR-514-AC-6", "FR-514-AC-7", "FR-514-AC-8", "FR-514-AC-9", "FR-514-AC-10", "FR-514-AC-11", "FR-514-AC-12", "TC-1309")]`.

## Expected Results

Each row's Expected column.
