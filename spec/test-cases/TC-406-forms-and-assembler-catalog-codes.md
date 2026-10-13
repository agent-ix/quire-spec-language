---
id: TC-406
title: "Each S2 and assembler cause maps to its catalog code with an exhaustive match"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-406: Each S2 and assembler cause maps to its catalog code with an exhaustive match

## Description

Verify that each S2 and assembler cause has one exhaustive `catalog_code()`
(ADR-013 O-17) that returns the code in FR-091's Catalog codes table.

Scope: FR-091-AC-21.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Construct one value of each S2 cause and each assembler cause, and call
   `catalog_code()`.
2. Construct the diagnosed-source cause from a diagnostic with code
   `unknown_profile`, and call `catalog_code()`.
3. With `syn`, inspect each `catalog_code()` body's `match`.

Tag the test `#[trace("FR-091-AC-21", "TC-406")]`.

## Expected Results

- `RecoveringCst` returns `invalid_syntax`.
- `NoDispatchEntry` and `UnrepresentedConstruct` return
  `unsupported_construct`.
- The floating-type cause returns `unknown_required_feature`.
- Unresolved type name returns `missing_declaration`, ambiguous type name
  returns `ambiguous_declaration`, and ill-formed bounds return `ill_typed`.
- The undeclared-alias cause returns `missing_declaration`, the
  duplicate-alias cause `ambiguous_declaration`, and the alias-cycle cause
  `invalid_package`.
- The duplicate-enum-member cause returns
  `ambiguous_declaration`/`ambiguous-name`, and the nominal-admission-fault
  cause `runtime_invariant`/`established-invariant-broken`.
- The duplicate dimension or unit name cause returns
  `ambiguous_declaration`/`ambiguous-name`, the unresolved dimension or unit
  name cause `missing_declaration`/`missing-name`, the dimension or unit
  cycle cause `invalid_package`/`definition-cycle`, the zero-denominator
  cause `undefined_expression`/`unproved-nonzero`, and the decimal-scale
  limit `stage_limit_exceeded`/`work-budget-exceeded`. The unit-graph topology
  cause returns the STD-112 cause once QSpec publishes it.
- Step 2 returns `unknown_profile`.
- Step 3 finds no `_` arm.
