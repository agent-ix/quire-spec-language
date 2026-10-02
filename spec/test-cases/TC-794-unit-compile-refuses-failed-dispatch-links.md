---
id: TC-794
title: "Unit compile refuses ambiguous and inapplicable dispatch with no checked package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-794: Unit compile refuses ambiguous and inapplicable dispatch with no checked package

## Description

Verify that a failed dispatch link refuses the whole compile at S3.
Scope: FR-302-AC-2, FR-302-AC-3, FR-302-AC-4.

## Test Procedure

1. Compile a unit whose called operation is redefined at `B` and at `C`,
   both subtypes of `A`, with concrete `D` inheriting from both.
2. Compile a unit whose called operation has bodies only on `B`, with
   concrete `A` a supertype of `B` and the call's receiver typed `A`.
3. Compile a unit with both failing calls.

## Expected Results

1. Refused at S3, `ambiguous_dispatch`/`multiple-undominated`, naming
   `D`, both candidates and the dominance pairs; no checked package.
2. Refused at S3, `ambiguous_dispatch`/`no-applicable`, naming `A`; no
   checked package.
3. One refusal naming both failures.
