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
4. Compile a request whose program source file is missing, then one whose
   program source has malformed syntax.
5. Compile a request file one byte over the fixed `request_bytes` ceiling, and
   a request whose program source exceeds the fixed `dependent_bytes` ceiling.
6. Compile a request whose program source and models are under the fixed
   `dependent_bytes` ceiling and whose one `libraries` source takes the total
   one byte over it. Then compile a request selecting 64 small library sources
   in addition to its program.

Capture the process streams around every call. Tag the tests
`#[trace("TC-105", "FR-027-AC-n")]` with the AC each backs: steps 1 and 2
AC-2, steps 3 to 5 AC-3, step 6 AC-13.

## Expected Results

- Step 1: exit code 0; the outcome's package bytes are the
  `quire.checked-package/v2` bytes with no wrapper or newline.
- Step 2: the native-run/1 request refuses `unknown_wire`; each runtime-only
  request member refuses `invalid-request` at the request stage, exit code 20,
  with no package bytes, and no model or runtime file is read.
- Step 3: each refuses `invalid-request`, exit code 20, with no package
  bytes.
- Step 4: each refuses with its original stage and code, exit code 20, with
  no package bytes.
- Step 5: each request refuses naming its fixed setting and ceiling, with no
  package bytes or option hint.
- Step 6: the first refuses at intake naming `dependent_bytes` and its fixed
  maximum, with no package bytes or option hint; the second refuses at intake
  with `selected files`, category `programs`, requested 65, maximum 64,
  before reading a selected source.
- No call writes a byte to stdout or stderr.
