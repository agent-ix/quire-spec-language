---
id: TC-1078
title: "S1 and S2 parse and build time declarations, quantity literals and timed intervals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: verifies
---
# TC-1078: S1 and S2 parse and build time declarations, quantity literals and timed intervals

## Description

Verify that S1 parses and S2 builds the `time-decl`, `time-invariant`, `urgent-decl`, `Clock` type, `quantity-literal` and `timed-interval` forms, dispatches each leading token to its one production, keeps the contextual words usable as identifiers, rejects mixed interval bounds, builds the same forms under every facet selection, and defers every meaning check to S3.

Scope: FR-393-AC-1 to FR-393-AC-7.

## Test Procedure

Fixtures: the declarations of FR-393-AC-1; a unit with a field named `time` and a parameter named `period`; a unit with two `time` declarations; the claims of FR-393-AC-3 to FR-393-AC-5; the `Rpc` unit with no facet selected and with the complete facets selected; units with `x: Clock`, `x: M::Clock` and an unqualified type declaration named `Clock`.

1. Build the AC-1 unit and read back each form, its order and its whole-form span.
2. Dispatch units beginning `time invariant`, `time` and `urgent`; build the unit with the `time` field and `period` parameter.
3. Parse the three claims of FR-393-AC-3 and `eventually[0,3] holds(p)`; read each operator's interval.
4. Build `eventually[0 s, 0.003 s] holds(p)`, `eventually(2 ms, 2 ms] holds(p)`, `eventually[3 ms, 2 ms] holds(p)` and the two-`time` unit; parse `eventually[0 ms, x] holds(p)`.
5. Parse `eventually[0, 3 ms] holds(p)`, `eventually[0 ms, 3] holds(p)` and `eventually[0 ms, *] holds(p)`.
6. Build the `Rpc` unit under both facet selections and compare the forms.
7. Build the three `Clock` units.

Tag the tests `#[trace("TC-1078", "FR-393-AC-n")]`.

## Expected Results

- Step 1: the forms of FR-393-AC-1, `TypeForm::Clock` for `x`, each form with its source span; `Tick{Call::send, 1/2 ms}` for the tick declaration.
- Step 2: exactly one call each, to `time_invariant`, `time_decl` and `urgent_decl`; the `time` and `period` names build as identifiers.
- Step 3: `Some(Timed)` with flags `(true, false)` and `(false, true)`, `None`, and `Some(Steps{0, Finite(3)})`.
- Step 4: upper magnitude `3/1000`; the next three build; `eventually[0 ms, x]` fails at S1 at `x`'s span with no form.
- Step 5: a parse diagnostic at `3 ms`, `3` and `*` respectively, no form.
- Step 6: equal forms and no facet diagnostic.
- Step 7: `TypeForm::Clock`; a named type form with no `TypeForm::Clock`; a parse diagnostic at the declared name.
