---
id: TC-420
title: "The default checking ceilings bind a Text-reachable cluster and are recorded with the result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: verifies
---
# TC-420: The default checking ceilings bind a Text-reachable cluster and are recorded with the result

## Description

Verify NFR-011's default checking ceilings. At the defaults, FR-093's
text-leaf walk over a Text-reachable recursive cluster refuses on the node
ceiling. A checked result records the ceilings it was checked under. Scope:
NFR-011-M-1 to NFR-011-M-4.

## Test Procedure

1. Declare nine records `R0` to `R8`. Each has `label: Text[0, 8; nfc]` and
   an optional field of every other record. Check `eq(a: R0, b: R0): Boolean
   { a = b }` at `CheckingLimits::default()`, with text-profile lock
   evidence.
2. Check the same shape with six records at `CheckingLimits::default()`.
3. Check `eq` over FR-093's `Node` at `CheckingLimits::default()`, at raised
   ceilings (nodes, input bytes and work at `u64::MAX`, depth 64), and at
   lowered ceilings (64 nodes, 4096 input bytes, 5000 work units). Under
   each set of limits, also check a standalone Boolean name expression
   against the checked package.

## Expected Results

- Step 1 is refused with
  `ResourceExhausted{stage: Typing, kind: Nodes, limit: 100000}`, cause
  `insufficient-next-charge`.
- Step 2 is admitted.
- Step 3: the defaults are 100000 nodes, depth 128, 16777216 input bytes and
  1000000 work units. Each checked package and each checked expression
  records exactly the limits it was checked under.

## Status

Backed: `a_text_reachable_cluster_refuses_on_the_default_node_ceiling` and
`a_checked_result_records_its_effective_limits`
(`qsl-semantics/src/check/lowering/tests/leaves.rs`).
