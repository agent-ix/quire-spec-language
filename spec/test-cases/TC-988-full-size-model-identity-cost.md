---
id: TC-988
title: "The full 10,000-edge model case meets deterministic cost and memory bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-988: The full 10,000-edge model case meets deterministic cost and memory bounds

## Description

Verify FR-081-AC-13, FR-081-AC-14 and FR-081-AC-15 at the full supported size.
Deterministic accounting, observed peak memory and measured elapsed time are
separate pass conditions; none is inferred from extrapolation.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_full_size.rs::ten_thousand_edge_chain_cost_and_memory`.

Run the test executable in the release profile on reference host Terra: Intel
Core i9-10900K, 14 allocated threads, WSL2, 94 GiB. Record the actual
`rustc --version` and `cargo --version` used for the run. These are run evidence,
not version pins.

1. Generate a deterministic single-inheritance chain of 10,001 object types
   named by ordinal `T0` through `T10000`; each type after `T0` declares only
   its immediately preceding type as its supertype. Assert the canonical
   package encoding is at most 524,288 bytes; fail fixture setup rather than
   silently shrinking the chain if it is larger.
2. Normalize the package in an isolated subprocess. Set only
   `derivation_facts` to 50,015,001; leave the other limits at NFR-012 defaults.
   The input has 10,001 qualify facts and 50,005,000 inheritance facts.
3. Read the actual counters after complete normalization. Count exact bytes
   admitted to every identity, universe and view hash call, and the complete
   operation's work units. Separately sum the specified phase-3 shared-prefix
   event classes.
4. Obtain peak resident-set bytes for the child process from the Rust test
   harness's operating-system process accounting, including input admission,
   normalization and result retention. Measure monotonic elapsed wall time in
   the child from the start of package input admission through completion and
   retention of the complete normalized result. Fixture generation, process
   launch and compilation are outside the timed interval. Do not infer duration
   from a smaller case or a throughput extrapolation.

## Expected Results

All 10,001 effective type identities and the complete view are present, with
all 50,015,001 logical facts represented. Total `hashed_bytes` is at most
268,435,456; total `work_units` is at most 16,777,216; and phase-3
shared-prefix-specific work is at most 120,012. Peak resident set is at most
21,474,836,480 bytes (20 GiB). The existing published identity-vector files
are unchanged.

On the specified Terra reference host and release profile, the measured
wall-clock interval is at most 300 seconds. Record the measured duration and
the actual Rust and Cargo versions with the run evidence. These thresholds are
acceptance requirements, not a claim that a qualifying run has occurred; no
runtime qualification passes until the full-size measurement exists.
