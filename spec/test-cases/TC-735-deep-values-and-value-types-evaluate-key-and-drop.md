---
id: TC-735
title: "Deep values and value types evaluate, key, compare, clone and drop"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: verifies
---
# TC-735: Deep values and value types evaluate, key, compare, clone and drop

## Description

Verify iterative handling of 100,000-deep values and value types,
including state keys through the event API.

Scope: FR-262-AC-2.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Build a 100,000-long `List` (`record List { head: Int[0, 9]; tail?: List;
   }`) with a recursive function, under a budget sized for it.
2. Key it as simulation state, and compare the key bytes with the RFC 8785
   text of its canonical JSON form.
3. Clone it, compare it with its clone, and drop both.
4. Build a `ValueType` of 100,000 nested `Option`s around `Boolean`; clone,
   compare, hash, format for debug and drop it and its clone.

Tag the tests `#[trace("TC-735", "FR-262-AC-2")]`.

## Expected Results

- Step 1: the value evaluates.
- Step 2: the key bytes equal the RFC 8785 text.
- Step 3: the clone compares equal, and the drops complete.
- Step 4: the clone compares equal and hashes equal, and formatting and
  drops complete.

## Status

Planned.
