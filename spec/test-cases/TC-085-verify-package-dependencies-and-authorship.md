---
id: TC-085
title: "Verify package dependencies and authorship"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-085: Verify package dependencies and authorship

## Description

Property, priority P1. Verifies FR-020-AC-5, FR-020-AC-6. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Change package, source and model byte selectors and each native/formal/authored identity axis independently. Include missing/duplicate/extra/foreign clause bindings and conflicting unselected model inventory entries. Keep unrelated positive setup valid and recompute the outer digest where necessary.

Permute the complete valid external ClauseBinding inventory and confirm that
its input order does not become source order or change the package identity.

## Expected Results

The exact dependency or authored-binding stage refuses; source/model/authored identities are never adopted from untrusted package fields. Existing missing/stale/ambiguous native diagnostics remain distinguishable and no accepted package is returned.
