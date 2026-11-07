---
id: TC-1340
title: "The bounded native-run-result/2 consumer boundary reads through the crate root"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-535
    type: verifies
---
# TC-1340: The bounded native-run-result/2 consumer boundary reads through the crate root

## Description

Verify [FR-535](../functional/FR-535-expose-the-bounded-native-run-result-consumer-boundary.md)
from an integration test that names only `qsl_replay` crate-root items.

## Test Procedure

1. Build a `ClauseRunReport`, write it with `write_clause_run_result`, read it
   with `read_native_run_result_bounded` under generous limits, and assert the
   clause variant's stage, category, code and basis. Read a command-error
   document and assert its stage, code and message.
2. Mutate a valid document to `/1`, add an unknown member, contradict the basis
   and witness, and malform the command-error shape. Assert each refusal.
3. Read the same valid document with `max_input_bytes` one below its length,
   and with a small `max_working_bytes`. Assert the input-byte limit and
   `WorkingBytes` refusals and that no result is returned. Read it again with
   limits exactly sufficient and assert the original result.

## Expected Results

Every refusal is the owning typed refusal. The in-budget read is identical to
the unbounded read.
