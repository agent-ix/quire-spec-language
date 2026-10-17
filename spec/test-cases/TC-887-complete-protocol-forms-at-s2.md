---
id: TC-887
title: "S2 refuses a recovering protocol CST and builds equal forms twice"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: verifies
---
# TC-887: S2 refuses a recovering protocol CST and builds equal forms twice

## Description

Verify that S2 refuses a recovering CST with no partial form and builds one
declaration to equal forms twice.

Scope: FR-308-AC-5.

## Test Procedure

1. Build a protocol whose CST holds a recovery node; build one declaration
   twice.

Tag the tests `#[trace("TC-887", "FR-308-AC-5")]`.

## Expected Results

- Step 1: `FormsCause::RecoveringCst` with no form; equal forms.
