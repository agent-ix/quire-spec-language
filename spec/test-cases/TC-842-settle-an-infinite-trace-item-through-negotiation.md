---
id: TC-842
title: "An infinite-trace item settles only through negotiation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-332
    type: verifies
---
# TC-842: An infinite-trace item settles only through negotiation

## Description

Verify the request writer's classification of an infinite-trace item, its
dispositions against empty, bounded-only and unbounded registries, the
FR-360 to O-16 map, and that S6a evidence settles no item.

Scope: FR-332-AC-1 to FR-332-AC-4.

## Test Procedure

In a harness downstream of CG `negotiate_*`, with test descriptors:

1. Write `Reaches` as an item; request a bounded version with a
   `FiniteBound`.
2. Negotiate it against an empty registry, a (`temporal-satisfaction`,
   `bounded`) descriptor, and a (`temporal-satisfaction`, `unbounded`)
   descriptor whose arm takes the infinite-trace form.
3. Map each FR-360 label, with `failed` under
   `resource-incomplete` and under another execution.
4. Run `Reaches` at S6a on a lasso where it holds, then read the
   empty-registry accounting record.

Tag the tests `#[trace("TC-842", "FR-332-AC-n")]`.

## Expected Results

- Step 1: `unbounded`, `finite_bound_available` false; the bounded request
  refuses `invalid_runtime_input`/`invalid-value` and writes no item.
- Step 2: `unsupported`, warned, naming `temporal-satisfaction`;
  `unsupported`, warned, `unbounded-extent`, naming kind, descriptor and
  mode; `supported`, routed to the descriptor.
- Step 3: success, violation, inconclusive, unsupported, incomplete,
  internal failure.
- Step 4: `tested`; the item is still `unsupported`.
