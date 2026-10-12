---
id: TC-991
title: "The full 10,000-edge model case meets deterministic cost and memory bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-991: The full 10,000-edge model case meets deterministic cost and memory bounds

## Description

Verify FR-081-AC-13 and FR-081-AC-14 at the full supported size. Deterministic
accounting and observed peak memory are separate acceptance conditions; neither
is inferred from extrapolation. Elapsed-time qualification remains pending the
owner decision recorded under e11 and is not a current pass/fail condition.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_full_size.rs::ten_thousand_edge_chain_cost_and_memory`.

The proposed elapsed-time environment is the release profile on reference host
Terra: Intel Core i9-10900K, 14 allocated threads, WSL2, 94 GiB. This host and
profile remain a proposal pending owner approval under e11, not an accepted
runtime qualification requirement. If toolchain details are needed to debug an
individual run, capture them only in that run's ephemeral test log; do not
persist Rust or Cargo version tracking in the specification or result inventory.

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

Elapsed time may be compared diagnostically with the proposed 300-second target
on Terra/release. Until e11 is resolved by the owner, elapsed time is not a
pass/fail condition, the proposed host/threshold does not close the owner
decision, and no runtime qualification is claimed. Any toolchain-version
details needed for diagnosis remain in an ephemeral run log and are not
persistently tracked.
