---
id: TC-457
title: "S2 state clause dispatch is thin, bounded and seam-probed"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: verifies
---
# TC-457: S2 state clause dispatch is thin, bounded and seam-probed

## Description

Verify the S2 entry table's new arms, deep clause bodies building with no
S2 limit, and the S2 seam probe.

Scope: FR-102-AC-4, FR-102-AC-5, FR-102-AC-6.

## Test Procedure

1. Run `dispatch_entry_is_a_single_thin_call` (`qsl-forms/src/dispatch.rs`)
   over the extended `dispatch`.
2. Build a unit whose only declaration begins `temporal` (a spelling no
   family claims).
3. Build an invariant whose body is 128 nested `not`s around `true` (depth
   129, since the root counts as depth 1), parsed at the default S1 limits;
   then, on a thread with a 512 KiB stack, one of 100,000 nested `not`s,
   parsed under S1 limits raised to fit it.
4. Run `xtask seam-probe` with the `seam-probe` feature.

Tag the tests `#[trace("TC-457", "FR-102-AC-n")]`. The existing
`no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token` test
moves from `invariant` to `temporal`.

## Expected Results

- Step 1: every arm, including `Invariant`, `Pre` and `Post`, is one call.
- Step 2: `FormsCause::NoDispatchEntry { spelling: "temporal" }`.
- Step 3: both bodies build their forms, and no S2 outcome names a depth.
- Step 4: the E0004 locations equal the checked-in S2 list, unchanged: a
  state clause has no match over a probed enum, so it adds no new location.
