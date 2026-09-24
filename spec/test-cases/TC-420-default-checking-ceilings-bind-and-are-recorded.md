---
id: TC-420
title: "The default checking ceilings bind wide and long leaf lists, admit large enum packages, and are recorded with the result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: verifies
---
# TC-420: The default checking ceilings bind wide and long leaf lists, admit large enum packages, and are recorded with the result

## Description

Verify NFR-011's default checking ceilings:

- At the defaults, FR-093's text-leaf walk refuses on the node ceiling
  over a Text-reachable recursive cluster, and on the work ceiling over
  long leaf paths.
- A large enum-parameter package checks.
- The byte ceiling binds before the work ceiling.
- `CheckingLimits::new` keeps the default byte and work ceilings.
- A checked result records the ceilings it was checked under.

Scope: NFR-011-M-1 to NFR-011-M-4.

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
   against the checked package. Build `CheckingLimits::new(7, 9)`.
4. Declare `enum Country` with 250 cases and 4000 functions
   `fK(x: Country): Boolean { true }`, and check them at
   `CheckingLimits::default()`. Check two such functions with the input-byte
   and work ceilings both set to 10000.
5. Declare a one-record chain of optional fields into a 17-level binary tree
   of optional fields whose last level holds a text field, every field name
   256 bytes long: 65536 text leaves. Check `eq` over its first record at
   `CheckingLimits::default()`. Check the same shape with one-byte names, a
   4-record chain and a 5-level tree.

## Expected Results

- Step 1 is refused with
  `ResourceExhausted{stage: Typing, kind: Nodes, limit: 100000}`, cause
  `insufficient-next-charge`.
- Step 2 is admitted.
- Step 3: the defaults are 100000 nodes, depth 128, 16777216 input bytes and
  16777216 work units. Each checked package and each checked expression
  records exactly the limits it was checked under. `new(7, 9)` has 7 nodes,
  depth 9, 16777216 input bytes and 16777216 work units.
- Step 4: the 4000-function package is admitted. The two-function package
  is refused with `ResourceExhausted{stage: Typing, kind: InputBytes,
  limit: 10000}`, not on the work ceiling.
- Step 5: the long-name package is refused with
  `ResourceExhausted{stage: Typing, kind: WorkBudget, limit: 16777216}`; the
  short-name package is admitted.

## Status

Backed:

- Steps 1 and 2: `a_text_reachable_cluster_refuses_on_the_default_node_ceiling`.
- Step 3: `a_checked_result_records_its_effective_limits` and
  `new_keeps_the_default_byte_and_work_ceilings`.
- Step 5: `long_leaf_paths_refuse_on_the_default_work_budget`.

These are in `qsl-semantics/src/check/lowering/tests/leaves.rs`.

- Step 4: `preimage_bytes_bind_before_the_work_budget`
  (`qsl-semantics/src/check/lowering/tests/rows.rs`).
