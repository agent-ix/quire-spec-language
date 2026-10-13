---
id: TC-920
title: "Native specification validation precedes aggregate Cargo execution"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-002
    type: verifies
---
# TC-920: Native specification validation precedes aggregate Cargo execution

## Description

Verify NFR-002-AC-1 through NFR-002-AC-4 with owned Rust fixtures,
real Make and native Quire. Cargo process recording proves aggregate routing
and refusal before invocation; it does not qualify the compiler.

## Test Procedure

1. Execute the default structural gate over the complete repository spec/ tree
   using installed scoped module discovery. Also validate a nested valid owned
   fixture with spaces in its path using unset, relative and absolute Cargo
   target configurations resolved from the workspace root.
2. Remove the fixture's required id, Statement section and Method table header
   in turn. Compare Make's failure diagnostics and reported native exit to a
   direct native validator execution; restore the fixture and require success.
3. Select an unmatched glob, remove Quire from PATH, and isolate module
   discovery in an empty home. Require failure and no module installation.
4. Execute the actual ci dependency graph with a malformed owned document,
   serial Make, parallel Make and parallel keep-going Make. Observe the Cargo
   process boundary with a native Rust recorder and require zero calls. Restore
   the document and require the same recorder to observe aggregate Cargo calls.

## Expected Results

The full valid corpus and restored owned fixtures pass native validation.
Malformed documents fail with the intended structural diagnostic and the
actual native exit retained in Make's error report. Empty selection and missing
configuration fail distinctly, with no module installation. Invalid aggregate
runs return nonzero and invoke no Cargo process for all scheduling controls;
restored positive runs reach Cargo. All fixture directories are created under
the workspace-resolved target and removed when their Rust owners are dropped.
