---
id: TC-888
title: "S2 builds the memory clause, access ordering and fence forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-309
    type: verifies
---
# TC-888: S2 builds the memory clause, access ordering and fence forms

## Description

Verify that S2 records a `parallel`'s memory clause, an attempt's ordering
and each `fence` with their spans, reads the memory keywords only in their
positions and refuses a recovering CST.

Scope: FR-309-AC-1 to FR-309-AC-3.

## Test Procedure

1. Build `SB` with `memory tso` and with no `memory` clause.
2. Build an attempt with `ordering release`, one with no ordering, and a
   `seq_cst` fence between two attempts.
3. Build a guard reading a field named `acquire`; build `fence F ordering
   relaxed;`; build one declaration twice.

Tag the tests `#[trace("TC-888", "FR-309-AC-n")]`.

## Expected Results

- Step 1: `Some(Tso)` with the span of `tso`; `None`.
- Step 2: `Some(Release)` with its span; `None`; a `FenceForm` with
  `SeqCst` as the second of three controls.
- Step 3: an identifier; `FormsCause::RecoveringCst` with no form; equal
  forms.
