---
id: TC-105
title: "Compile intake, labels and limits for a 1-draft request"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
## Description

Integration, P1: call `command` in process with `1-draft` native-compile/1
requests and check what it compiles from a source-only directory, what it
refuses at the request boundary, its label checks and its intake limits.

Scope: FR-027-AC-2, FR-027-AC-3 and FR-027-AC-13.

## Test Procedure

1. Copy only the request-selected `1-draft` sources and the compile request
   into a fresh directory and call `command` with its path from a different
   working directory.
2. Give `compile` a native-run/1 request; then a native-compile/1 request
   carrying each runtime member in turn: `snapshots`, `invocations`,
   `selection` and `package`.
3. Compile a request whose program source lacks `identity`, then one whose
   program source lacks `authority`.
4. Compile a request whose program source digest is stale, then one whose
   program source has malformed syntax.
5. Compile a request whose sources exceed `dependent_bytes`; compile it
   again with the limit raised to its size.
6. Compile a request whose program source and models are under
   `dependent_bytes` and whose one `libraries` source takes the total one
   byte over it; compile it again with the limit raised to that total. Then
   compile a program that imports 65 libraries, each its own small
   `libraries` source file, at the default limits.

Capture the process streams around every call. Tag the tests
`#[trace("TC-105", "FR-027-AC-n")]` with the AC each backs: steps 1 and 2
AC-2, steps 3 to 5 AC-3, step 6 AC-13.

## Expected Results

- Step 1: exit code 0; the outcome's package bytes are the
  `quire.checked-package/v2` bytes with no wrapper or newline.
- Step 2: each refuses `invalid-request` at the request stage, exit code 20,
  with no package bytes, and no model or runtime file is read.
- Step 3: each refuses `invalid-request`, exit code 20, with no package
  bytes.
- Step 4: each refuses with its original stage and code, exit code 20, with
  no package bytes.
- Step 5: the first refuses naming `dependent_bytes` and its configured
  value; the second compiles, exit code 0.
- Step 6: the first refuses at intake naming `dependent_bytes` and its
  configured value, with no package bytes; the second returns FR-027-AC-10's
  package bytes, exit code 0; the 65-library request compiles, exit code 0.
- No call writes a byte to stdout or stderr.
