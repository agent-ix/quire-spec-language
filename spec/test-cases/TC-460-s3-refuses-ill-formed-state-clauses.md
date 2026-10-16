---
id: TC-460
title: "S3 refuses ill-formed state clauses with their catalog codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104-AC-9
    type: verifies
---
# TC-460: S3 refuses ill-formed state clauses with their catalog codes

## Description

Verify each state clause refusal FR-104 names, at its locus.

Scope: FR-104-AC-3, FR-104-AC-4, FR-104-AC-9, FR-104-AC-10.

## Test Procedure

For rows 1-10, use TC-458's fixture package; row 13 adds the process shown. For rows 11-12, use the
specified model variants. Check one unit per row; each unit holds only the
clause shown (plus `ParentOrder` for the duplicate case).

| # | Clause |
| --- | --- |
| 1 | `invariant A using v on Config::Missing at current { true }` |
| 2 | `pre B using v on Config::ConfigVersion::missing { true }` |
| 3 | `invariant C using v on Config::ConfigVersion at current { self.versionNumber }` |
| 4 | a second `invariant ParentOrder ...` |
| 5 | `invariant D using v on Config::ConfigVersion at current { pre(self.versionNumber) = 1 }` |
| 6 | `post E using v on Config::ConfigVersion::attemptUpdate { pre(result) }` |
| 7 | `invariant F using v on Config::ConfigVersion at current { reaches(self, self, versionNumber) }` |
| 8 | `invariant G using v on Config::ConfigVersion at current { deref(value(self.parent)).versionNumber < 5 }` |
| 9 | `function ParentOrder using v(): Boolean pure { true }` beside `ParentOrder` |
| 10 | `function r using v(x: Config::ConfigVersion, y: Config::ConfigVersion): Boolean pure { reaches(x, y, parent) }` |
| 11 | Under `quire.model.complete/v1`, the precondition on `ArchivedConfigVersion::probe` from TC-459 step 6, but declare `target: UnrelatedVersion` where `UnrelatedVersion` does not conform to the `ConfigVersion` edge owner. |
| 12 | At the S3 checker seam with the same admitted subtype/edge type environment and precondition as TC-459 step 6, select an otherwise admitted model definition other than `quire.model.complete/v1`; the operands have different exact reference types. |
| 13 | With a process `Flow` (identity field `x`, field `n: Integer`) declaring operation `go` added to the fixture package: `pre M using v on Config::Flow::missing { true }` |

Tag the tests `#[trace("TC-460", "FR-104-AC-n")]`.

## Expected Results

1. `missing_declaration`/`missing-name` at `Config::Missing`.
2. `missing_declaration`/`missing-name` at `missing`.
3. `ill_typed`/`non-boolean-root` at the body.
4. `ambiguous_declaration`/`ambiguous-name` at both `ParentOrder`
   declarations.
5. `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
6. `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
7. `ill_typed`/`operator-ineligible` at the `reaches`.
8. `undefined_expression`/`unproved-presence` at `value(self.parent)`.
9. `ambiguous_declaration`/`ambiguous-name` at the clause and the function.
10. `ill_typed`/`operator-ineligible` at the `reaches`.
11. `ill_typed`/`operator-ineligible` at the `reaches`: the target does not
    conform to the edge owner under QSpec FR-043.
12. `ill_typed`/`operator-ineligible` at the `reaches`: the complete/v1
    conformance rule is not selected and the exact-type rule applies.
13. `missing_declaration`/`missing-name` at `missing`: `Flow` is resolved as a context and `go` is its only operation.

No row yields a checked clause or function.
