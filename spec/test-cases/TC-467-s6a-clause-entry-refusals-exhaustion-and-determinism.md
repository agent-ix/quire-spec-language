---
id: TC-467
title: "S6a clause entry refuses bad selections, reports exhaustion and is deterministic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: verifies
---
# TC-467: S6a clause entry refuses bad selections, reports exhaustion and is deterministic

## Description

Verify meter exhaustion, the entry's input refusals, determinism and the S6a
seam probe for the `ProtocolClause` arm.

Scope: FR-107-AC-4, FR-107-AC-5, FR-107-AC-6.

## Test Procedure

1. Evaluate `ParentOrder` over healthy-parent with a meter budget of zero,
   then again with the default budget.
2. Call `evaluate_clause` with the name `sameIdentity` (a function), and with
   the name `NoCycle` over observations admitted for `ParentOrder`; read the
   meter after each.
3. Evaluate every FR-108 case that reaches S6a twice, keeping each charge
   log.
4. Run `xtask seam-probe` with the `ProtocolClause` S6a variant probed.

Tag the tests `#[trace("TC-467", "FR-107-AC-n")]`.

## Expected Results

- Step 1: `Incomplete`, `resource_exhausted`/`insufficient-next-charge`, no
  truth; then `Completed(true)`.
- Step 2: `InputRefusal::UnknownClause`; `InputRefusal::ObservationsMismatch`;
  the meter is uncharged after both.
- Step 3: equal `Evaluation`s and equal charge logs.
- Step 4: E0004 at exactly the checked-in S6a seam list, which names the
  `ProtocolClause` arm.

## Status

Planned (QSL-273).
