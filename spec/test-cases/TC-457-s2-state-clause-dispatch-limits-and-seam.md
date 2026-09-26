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

Verify the S2 entry table's new arms, the depth limit over a clause body,
and the S2 seam probe.

Scope: FR-102-AC-4, FR-102-AC-5, FR-102-AC-6.

## Test Procedure

1. Run `dispatch_entry_is_a_single_thin_call` (`qsl-forms/src/dispatch.rs`)
   over the extended `dispatch`.
2. Build a unit whose only declaration begins `temporal` (a spelling no
   family claims).
3. Let `d` be `FormsLimits`' nesting-depth limit. Build an invariant whose
   body is `d` nested parentheses around `true`, then `d + 1`.
4. Run `xtask seam-probe` with the `seam-probe` feature.

Tag the tests `#[trace("TC-457", "FR-102-AC-n")]`. The existing
`no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token` test
moves from `invariant` to `temporal`.

## Expected Results

- Step 1: every arm, including `Invariant`, `Pre` and `Post`, is one call.
- Step 2: `FormsCause::NoDispatchEntry { spelling: "temporal" }`.
- Step 3: depth `d` builds; depth `d + 1` gives `StageFailure::Limit` with
  `nesting-depth-exceeded` at the body.
- Step 4: the E0004 locations equal the checked-in S2 list, which now names
  the `protocol_clause::state_clause` entry.

## Status

Planned (QSL-273).
