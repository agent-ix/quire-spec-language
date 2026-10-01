---
id: TC-808
title: "The driver sends CG only the items the export bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-808: The driver sends CG only the items the export bound

## Description

Scope: FR-307-AC-6.

## Test Procedure

Run the orchestrating driver's emission path onto implementation code over
two items, one bound and one referencing an unbound element, with CG replaced
by a recording double that captures the QSpec FR-331 request.

## Expected Results

The captured request holds only the bound item. The driver's report names the
refused item and its `missing_declaration`/`missing-name` refusal.
