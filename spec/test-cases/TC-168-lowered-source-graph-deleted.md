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
crate's symbols and from every tracked file in the repository — not `src/`
alone, so a move of `lower_source_graph` and its test into `tests/` or
`xtask` cannot pass — including the pre-migration test that called
`lower_source_graph` directly, and that no compatibility shim, disabled
test, relocated copy, or feature-gated dormant copy of any of the three
remains. The symbol scan (step 1) is a weak oracle on its own — rustc emits
no symbol for an unreferenced non-generic struct, so it can pass while
`LoweredDeclaration` still exists in source — which is why the repository-wide
reference scan (step 3) is the load-bearing check. Scope: FR-067-AC-5,
FR-067-AC-6.

## Test Procedure

1. Search the compiled crate's public and crate-internal symbols for
   `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph`.
2. Search `complete::mod`'s re-export list for the same three names.
3. Search every tracked file in the repository — `src/`, `tests/`, `xtask`,
   and any benchmark harness, not `src/` alone — for a source reference to
   any of the three names.
4. Inspect the pre-migration test that exercised `lower_source_graph`
   directly (`complete::package_tests`) for its presence anywhere in the
   repository, and, if present, for a `#[ignore]` attribute or a feature
   gate that would disable it without removing it.

## Expected Results

- Step 1: none of the three symbols are present.
- Step 2: none of the three names appear in the re-export list.
- Step 3: no source reference to any of the three names exists anywhere in
  the repository's tracked files.
- Step 4: the pre-migration test is absent entirely, everywhere in the
  repository — not present but disabled, and not present relocated to
  `tests/` or `xtask`.
