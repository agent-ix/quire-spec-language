---
id: TC-105
title: "CLI compile intake, labels, arity and output failures for a 1-draft request"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
## Description

Integration, P1: run the real compile binary on `1-draft` native-compile/1
requests and check what it compiles from a source-only directory, what it
refuses at the command boundary, its intake limits, its arity errors and its
output failures.

Scope: FR-027-AC-2 and FR-027-AC-3.

## Test Procedure

1. Copy only the request-selected `1-draft` sources and the compile request
   into a fresh directory and compile from a different working directory.
2. Give `compile` a native-run/1 request; then a native-compile/1 request
   carrying each runtime member in turn: `snapshots`, `invocations`,
   `selection` and `package`.
3. Compile a request whose program source lacks `identity`, then one whose
   program source lacks `authority`.
4. Compile a request whose program source digest is stale, then one whose
   program source has malformed syntax.
5. Compile a request whose library sources exceed `dependent_bytes`; compile
   it again with the limit raised to its size.
6. Run compile with a wrong argument count, with no request file present.
7. On Linux, compile a valid request with the binary's stdout redirected to
   /dev/full.

Tag the tests `#[trace("TC-105", "FR-027-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: exit 0; stdout is the `quire.checked-package/v2` bytes with no
  wrapper or newline.
- Step 2: each refuses `invalid-request` at the request stage, exit 20,
  empty stdout, and no model or runtime file is read.
- Step 3: each refuses `invalid-request`, exit 20, empty stdout.
- Step 4: each refuses with its original stage and code, exit 20, empty
  stdout.
- Step 5: the first refuses naming `dependent_bytes`, its configured value
  and its CLI option; the second compiles, exit 0.
- Step 6: exit 20 before any file is read.
- Step 7: exit 30, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code
  for tool failure.
