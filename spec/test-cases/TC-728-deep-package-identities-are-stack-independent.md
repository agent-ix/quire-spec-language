---
id: TC-728
title: "A deep package's node keys do not depend on the stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: verifies
---
# TC-728: A deep package's node keys do not depend on the stack

## Description

Verify that a 100,000-term package checks without native recursion and
that its checked package and structural node keys equal those checked on a
large stack.

Scope: FR-259-AC-1.

## Test Procedure

1. On a thread with a 512 KiB stack, check a package holding a
   100,000-term sum under limits raised to fit it, and read its checked
   package and every node key.
2. Check the same source on a thread with an 8 MiB stack and read the
   same.
3. **RETIRED (node preimages are deleted, FR-259-AC-2).**
4. **RETIRED (FR-259-AC-2 is retired).**
5. **RETIRED (node and package identity preimages are deleted, FR-259-AC-4).**

Tag the tests `#[trace("TC-728", "FR-259-AC-1")]`.

## Expected Results

- Steps 1 and 2: the checked packages are equal and every node key is
  equal across the two threads.
- Steps 3 to 5: **RETIRED.**
