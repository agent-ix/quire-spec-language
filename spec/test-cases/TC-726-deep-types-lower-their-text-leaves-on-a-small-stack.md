---
id: TC-726
title: "Deep types key and lower their text leaves on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: verifies
---
# TC-726: Deep types key and lower their text leaves on a small stack

## Description

Verify the iterative text-leaf walk and type-keying walk over 100,000-deep
types.

Scope: FR-258-AC-2.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. With S1 and S3 limits raised to fit, check structural equality over a
   parameter typed with 100,000 nested `Option`s around `Text[0, 8; nfc]`.
2. With S1 and S3 limits raised to fit, check structural equality over a
   chain of 100,000 records, each holding the next in a field, the last
   holding a `Text[0, 8; nfc]` field.
3. Key a parameter typed with 100,000 nested `Option`s around `Boolean`.

Tag the tests `#[trace("TC-726", "FR-258-AC-2")]`.

## Expected Results

- Steps 1 and 2: each equality lowers with one text leaf whose path has one
  segment per level, and the thread completes.
- Step 3: the parameter is keyed, and the thread completes.

## Status

Planned.
