---
id: TC-741
title: "Clause run reports carry a settlement basis on every disposition and a witness only when decisive"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-266
    type: verifies
---
# TC-741: Clause run reports carry a settlement basis on every disposition and a witness only when decisive

## Description

Verify FR-266: `run_clause` reports carry `basis` on every disposition and
`witness` exactly when the basis is decisive, for `Clause`, `Function` and
`Frame` selections and for every non-evaluating stage.

Scope: FR-266-AC-1 to FR-266-AC-3.

## Test Procedure

Build `ClauseRunRequest`s through `qsl_replay::spine::run_clause`.

1. FR-265's `witness` unit: `Clause` `AllBelow` over `high`, `SomeAtLeast`
   over `high`, `AllBelow` over `low`; the `AllBelow` over `high` request a
   second time.
2. FR-108's corpus requests healthy-parent, violating-parent,
   changed-version, missing-model, dangling-parent, incomplete-population
   and exhausted-work; FR-109-AC-4's `Function` selection of `sameIdentity`
   over distinct identities, and with `b` naming `ghost`.
3. FR-115's `Frame` selection over an invocation that changes a member
   outside the frame; a report whose S6a outcome is FR-109-AC-5's
   `CallFailure::Fault`.

Tag the tests `#[trace("TC-741", "FR-266-AC-n")]`.

## Expected Results

- Step 1: `violation`, exit 10, `decisive-counterexample`, FR-265-AC-1's
  record; `success`, exit 0, `decisive-witness`, FR-265-AC-2's record;
  `success`, `closed-scope`, no `witness`; the repeated request's report
  equals the first.
- Step 2: the FR-109 dispositions and exit codes unchanged; `closed-scope`
  with no `witness` for the three evaluated requests; `unavailable` with no
  `witness` for the four others; `closed-scope` with no `witness` for the
  `sameIdentity` violation; `unavailable` for the `ghost` admission
  refusal.
- Step 3: `violation`, `closed-scope`, no `witness`, the FR-115 frame
  witness unchanged; `internal-failure`, `unavailable`, no `witness`.
