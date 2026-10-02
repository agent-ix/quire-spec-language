---
id: TC-666
title: "The tso memory model explores store buffers, flushes and locked accesses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: verifies
---
# TC-666: The tso memory model explores store buffers, flushes and locked accesses

## Description

Verify `Tso`'s stores, loads, flushes, locked accesses, fences, gates and the litmus outcomes of its column.

Scope: FR-221-AC-1 to FR-221-AC-4.

## Test Procedure

1. Check `SB` under `tso`; count states; read the counterexample's
   positions.
2. Check the `seq_cst` fence and `seq_cst` access variants.
3. Run FR-221-AC-3's two-branch protocol, with `x` and `y` shared, and
   the loads and `fetch_add` it names.
4. Check `join` and `send` enabledness against buffers; run each litmus
   test of ADR-025 §3.3 under `tso`.

Tag the tests `#[trace("TC-666", "FR-221-AC-n")]`.

## Expected Results

- Step 1: 34 + 4 = 38 states; `refuted` with §10's four-step
  counterexample.
- Step 2: `proved` twice.
- Step 3: the buffer `[x := 1, y := 1]`, `x` flushed first, own load 1,
  other load 0, `fetch_add` disabled until empty.
- Step 4: the gates of FR-221-AC-4; every `tso` litmus outcome as the
  table states.
