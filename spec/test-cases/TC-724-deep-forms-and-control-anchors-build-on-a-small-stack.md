---
id: TC-724
title: "Deep forms and control anchors build on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-257
    type: verifies
---
# TC-724: Deep forms and control anchors build on a small stack

## Description

Verify that S2 builds forms and scoped anchors of any depth over arenas and
explicit stacks, with no limit set.

Scope: FR-257-AC-1, FR-257-AC-2, FR-257-AC-3.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Run S2 on each function-body input of TC-722 step 1, parsed under S1
   limits raised to fit it. Clone each built unit, compare it with its
   clone, format it for debug and drop both.
2. Run S2 on a protocol clause holding a 100,000-deep `await … then` chain
   and on one holding a 100,000-deep `repeat … exhausted` chain, and
   resolve each anchor.
3. Call `qsl_forms::build_unit` on an invariant whose body is 128 nested
   `not`s around `true`, parsed at the default S1 limits.

Tag the tests `#[trace("TC-724", "FR-257-AC-1")]`, `#[trace("TC-724", "FR-257-AC-2")]`, `#[trace("TC-724", "FR-257-AC-3")]`.

## Expected Results

- Step 1: each builds; each form tree is as deep as its body; the clone
  compares equal; formatting and drops complete.
- Step 2: both build, and every anchor resolves in the scope FR-112 gives it.
- Step 3: `build_unit` takes no limit set, and the form builds.

