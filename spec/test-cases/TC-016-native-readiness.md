---
id: TC-016
title: "Inclusive formatter byte ceilings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: verifies
---

## Description

Inclusive formatter byte ceilings. Type: Property; priority P1. Traces: FR-003-AC-4, FR-003-AC-5, FR-003-AC-6, NFR-001-M-1.

## Test Procedure

Generate lower ceilings from zero through the exact formatted length including UTF-8/comment/final-newline boundaries. Exercise a near-1-MiB source whose formatting expands beyond the hard ceiling and request usize::MAX.

## Expected Results

Every ceiling below the required length returns format-phase resource_exhausted with no String; exact or larger admitted ceilings return identical bytes. Default API preserves compatibility and the hard ceiling cannot be bypassed.

