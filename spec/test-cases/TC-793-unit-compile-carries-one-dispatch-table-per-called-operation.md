---
id: TC-793
title: "Unit compile carries one linked dispatch table per called operation, named by each call"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-793: Unit compile carries one linked dispatch table per called operation, named by each call

## Description

Verify that spine `compile` links each called operation's family at S3 and
that every dispatched call's checked node names its operation's table.
Scope: FR-302-AC-1.

Catches a compile that links per call site (two tables for one operation),
links nothing, or leaves a call's node naming no table or the wrong one.

## Test Procedure

1. Declare `size` with an `operation-body` at `A` and a redefinition at `B`
   (`supertypes: [A]`), and `area` with an `operation-body` at `A` only.
2. Compile a unit with two preconditions, each calling `self.size()` and
   `self.area()`.
3. Read the checked package's linked dispatch tables and each dispatched
   call's checked node.

## Expected Results

1. The compile succeeds.
2. The package carries exactly two tables, one for `size` and one for
   `area`. The `size` table maps `A` to `A`'s candidate and `B` to `B`'s;
   the `area` table maps `A` and `B` to `A`'s candidate. The expected tables
   are literals in the test.
3. Both `size` calls name the `size` table and both `area` calls name the
   `area` table.
