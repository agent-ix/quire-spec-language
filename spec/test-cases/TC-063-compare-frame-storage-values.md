---
id: TC-063
title: "Compare frame storage values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-063: Compare frame storage values

## Description

Integration, priority P1. Verifies FR-007-AC-14. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Compare pre/post fields and State roots for identical nested records, absent/present options, sequences with reordered or removed duplicates and same-identity references. Change a State root's primitive or object identity without a root-write permission; omit one counterpart. Also change a population field allowed by the frame while preserving a reference State root.

## Expected Results

Storage equality handles all admitted value kinds while source Option/Seq equality remains ineligible. Unauthorized root replacement is frame_violation; missing counterpart is incomplete. An unchanged identity root can observe a permitted population-field mutation. Field-name ordering does not alter equality.
