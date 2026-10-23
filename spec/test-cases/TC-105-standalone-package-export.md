---
id: TC-105
title: "CLI compile intake, arity and output failures for a 1-draft request"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
## Description

Integration, P1: run the real compile binary on `1-draft` native-compile/1
requests and check its intake refusals, intake limits, arity errors and
output failures.

Scope: FR-027-AC-3.

## Test Procedure

1. Compile a `1-draft` request whose program source digest is stale, then
   one whose program source has malformed syntax.
2. Compile a `1-draft` request whose library sources exceed
   `dependent_bytes`; compile it again with the limit raised to its size.
3. Run compile with a wrong argument count, with no request file present.
4. On Linux, compile a valid `1-draft` request with the binary's stdout
   redirected to /dev/full.

Tag the tests `#[trace("TC-105", "FR-027-AC-3")]`.

## Expected Results

- Step 1: each refuses with its original stage and code, exit 20, empty
  stdout.
- Step 2: the first refuses naming `dependent_bytes`, its configured value
  and its CLI option; the second compiles, exit 0.
- Step 3: exit 20 before any file is read.
- Step 4: exit 30, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code
  for tool failure.
