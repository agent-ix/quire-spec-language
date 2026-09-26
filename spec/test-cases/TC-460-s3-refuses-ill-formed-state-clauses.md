---
id: TC-460
title: "S3 refuses ill-formed state clauses with their catalog codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-460: S3 refuses ill-formed state clauses with their catalog codes

## Description

Verify each state clause refusal FR-104 names, at its locus.

Scope: FR-104-AC-3, FR-104-AC-4.

## Test Procedure

Against TC-458's fixture package, check one unit per row. Each unit holds only
the clause shown (plus `ParentOrder` for the duplicate case).

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

Tag the tests `#[trace("TC-460", "FR-104-AC-n")]`.

## Expected Results

1. `missing_declaration`/`missing-name` at `Config::Missing`.
2. `missing_declaration`/`missing-name` at `missing`.
3. `ill_typed`/`non-boolean-root` at the body.
4. The code a duplicate function name gets, at the second `ParentOrder`.
5. `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
6. `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
7. `ill_typed`/`operator-ineligible` at the `reaches`.
8. The definedness refusal an unguarded `value(..)` gets today, at
   `value(self.parent)`.

No row yields a checked clause.

## Status

Planned (QSL-273).
