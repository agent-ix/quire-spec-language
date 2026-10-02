---
id: TC-882
title: "An extension's declaration form parses, checks against its typed-node schema and packages as a typed node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: verifies
---
# TC-882: An extension's declaration form parses, checks against its typed-node schema and packages as a typed node

## Description

Verify admission of a declaration form from an extension's grammar and typed-node schemas, and the clause type checks.

Scope: FR-354-AC-1, FR-354-AC-2.

## Test Procedure

Build a definition catalog holding an extension whose grammar schema adds the production `retention <name> { subject: <expr>, days: <expr> }` and whose typed-node schema gives node kind `retention` with clauses `subject: Text` and `days: Int[1, 3650]`.

1. Compile a unit that selects the extension and declares `retention Audit { subject: "audit", days: 30 }`.
2. Compile it with `days: "thirty"`; then with the `days` clause removed.

Tag the tests `#[trace("TC-882", "FR-354-AC-n")]`.

## Expected Results

- Step 1: the checked package holds one node of kind `retention` naming the extension, with subnodes `"audit"` and `30` and the form's span.
- Step 2: `ill_typed`/`type-mismatch` naming `days`, `Int[1, 3650]` and `Text`; then `invalid_package`/`missing-member` naming the missing `days` clause.
