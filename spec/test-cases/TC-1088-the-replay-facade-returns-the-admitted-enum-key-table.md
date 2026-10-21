---
id: TC-1088
title: "The replay facade returns the admitted enum key table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: verifies
---
# TC-1088: The replay facade returns the admitted enum key table

## Description

Verify `qsl_replay::compile_admitted` returns the emitted package with the
admitted enum key table, finds keys by wire node id, and refuses as typed.
Scope: FR-394-AC-1 to FR-394-AC-5.

## Test Procedure

From an integration test that reaches only the `qsl_replay` root:

1. Compile a unit with an enum that has an unused case through
   `compile_package` and `compile_admitted`; compare the keys with the
   spine's checked graph.
2. Look up the enum and each case by wire node id; look up an id no package
   admitted and a case the enum lacks.
3. Compile a unit importing a library that declares an enum; look up the
   library enum's id from the library's own context.
4. Compile a malformed source, a unit against a library with the unit's
   owner, a source above a 4-byte S1 limit, and a limit above the reader.

## Expected Results

1. Equal bytes and `package_id`; every case present with the spine's keys.
2. The declaration and cases are found; the unknown id and the missing case
   refuse `Unknown`.
3. `Foreign` naming the library's `package_id`; the unit's own enum is found.
4. `Package`, `Package`, `Resource`, `Resource`.
