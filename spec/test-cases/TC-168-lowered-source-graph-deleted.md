---
id: TC-168
title: "SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: verifies
---
# TC-168: SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core

## Description

Verify that after the S2 forms stage lands, `LoweredSourceGraph`,
`LoweredDeclaration` and `lower_source_graph` are absent from the compiled
crate's symbols and from every source reference under `src/`, including the
pre-migration test that called `lower_source_graph` directly, and that no
compatibility shim, disabled test, or feature-gated dormant copy of any of
the three remains. Scope: FR-067-AC-5, FR-067-AC-6.

## Test Procedure

1. Search the compiled crate's public and crate-internal symbols for
   `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph`.
2. Search `complete::mod`'s re-export list for the same three names.
3. Search every file under `src/`, including test targets, for a source
   reference to any of the three names.
4. Inspect the pre-migration test that exercised `lower_source_graph`
   directly (`complete::package_tests`) for its presence, and, if present,
   for a `#[ignore]` attribute or a feature gate that would disable it
   without removing it.

## Expected Results

- Step 1: none of the three symbols are present.
- Step 2: none of the three names appear in the re-export list.
- Step 3: no source reference to any of the three names exists anywhere
  under `src/`.
- Step 4: the pre-migration test is absent entirely, not present but
  disabled.
