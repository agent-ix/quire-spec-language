---
id: TC-1024
title: "The full 10,000-edge model identity case"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-1024: The full 10,000-edge model identity case

## Description

Verify FR-081-AC-12 and FR-081-AC-13 at the full size. Deterministic counters and
observed peak memory and elapsed time are separate acceptance conditions;
none is inferred from a smaller case.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_full_size.rs::ten_thousand_edge_chain`.

1. Generate a single-inheritance chain of 10,001 object types `T0` through
   `T10000` in package `test/orders`; each type after `T0` names only the
   preceding type as its supertype. Assert the canonical package encoding is at
   most 524,288 bytes; fail setup rather than shrink the chain.
2. Normalize it in an isolated subprocess at the NFR-012 defaults, with no
   limit raised.
3. Read the counters after complete normalization: retained facts, bytes
   admitted to every hash call, work units. Read the child's peak resident set
   from `VmHWM` in `/proc/self/status` after the result is retained.
4. Measure monotonic wall time in the child from package input admission through
   retention of the complete result, in the release profile. Fixture generation,
   process launch and compilation are outside the interval.

## Expected Results

All 10,001 effective type identities and the complete view are present. Retained
facts are 20,001; hashed bytes are about 14.1 million for this fixture and at most
16,777,216; work units are at most 100,000. Peak resident set is at most
21,474,836,480 bytes. Wall time is at most 30 seconds on the reference host
Terra (Intel Core i9-10900K, 14 allocated threads, WSL2, 94 GiB). Memory and
time are measured results; the expected figures follow from the byte formula
(14.1 MB at the 7.74 MB/s debug-profile encoder rate is 1.8 s) and are not
extrapolated into the pass condition.
