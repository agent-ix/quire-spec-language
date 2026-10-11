---
id: TC-986
title: "Model identity writer prices exact bytes and denies before work"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-986: Model identity writer prices exact bytes and denies before work

## Description

Verify FR-081-AC-11 at the real Rust model-normalization boundary, including
escaped keys, canonical shared-prefix hits and zero/one-less denial behavior.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_accounting.rs::exact_prices_and_denials`.

1. Exercise each admitted v2 declaration, key, block, range, prefix, universe
   and complete-view template with keys containing quote, backslash and
   non-ASCII characters.
2. For each actual admitted node, compare the analytical pre-work price with
   the byte length returned by the sole canonical writer. Count all bytes
   admitted to all hash calls, including reused dependencies and the view.
3. Repeat with the relevant `hashed_bytes` or `work_units` limit set to zero
   and one below the exact next charge. Instrument the writer and hasher call
   boundary. Also exercise a shared-reference hit after its first admission.

## Expected Results

Every admitted price equals the sole writer's exact byte length. A denied
charge performs none of its named work: no denied-node encoding or hashing and
no dependent hash; the result is incomplete with the exact charge point. A
shared hit pays the specified lookup work but does not re-encode or rehash the
already admitted commitment. Byte counts are totals only; the test creates no
digest inventory or revision pin.
