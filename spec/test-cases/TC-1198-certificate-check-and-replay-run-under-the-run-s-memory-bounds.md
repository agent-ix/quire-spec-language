---
id: TC-1198
title: "The certificate check and replay run under the run's memory bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: verifies
---
# TC-1198: The certificate check and replay run under the run's memory bounds

## Description

Verify that the closure-certificate checker and the weak-memory replay build
their `Tso` and `Ra` components under the memory bounds the request and the
replay subject carry, that a certificate with a state the bound cuts is
rejected, and that a counterexample step the bound cuts refuses.

Scope: FR-338-AC-15, FR-227-AC-5.

## Test Procedure

1. Run FR-225-AC-2's `tso` program with `always holds(x.v <= 1)` and
   `max_store_buffer` 3; check its certificate under a request carrying 3,
   then 2.
2. Run the same program under `ra` with `max_messages` unset; check its
   certificate under a request with `max_messages` unset, then 1.
3. Replay `SB`'s `tso` counterexample with `max_store_buffer` unset, then 1.
4. Replay `SB`'s `ra` counterexample with `max_messages` unset, then 1.

Tag the tests `#[trace("TC-1198", "FR-338-AC-15")]` and
`#[trace("TC-1198", "FR-227-AC-5")]`.

## Expected Results

- Step 1: V-1; accepted; rejected `SuccessorMissing` at a product state
  where the third store is cut, with no refusal.
- Step 2: V-1; accepted; rejected `SuccessorMissing` at a bound-limited
  product state.
- Step 3: `reproduced-with-evaluated-witness` both times.
- Step 4: `reproduced-with-evaluated-witness`; refused
  `invalid_runtime_input`/`invalid-value` at the first store.
