---
id: TC-1330
title: "Native composition carries and checks premises"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-530
    type: verifies
---
# TC-1330: Native composition carries and checks premises

## Description

Verify QSL's compile and read-back of QSpec FR-491's `composition-decl` and
its settlement from QSpec FR-492's request over real native `Producer` and
`Consumer` packages and independently produced premise results.

## Test Procedure

Compile `Producer` and `Consumer` (QSpec TC-983) as real native packages, and
the composing unit of QSpec TC-985 case 1 importing them. Produce each
component guarantee's result by running its own item (the in-process model
checker where it routes, a plugin result entry where the case says so), and
build the settlement request from those results. Then:

1. Run QSpec TC-985's cases through compile and read-back, and replace the
   slots with a Boolean flag (FR-530-AC-1).
2. Settle the positive request (FR-530-AC-3).
3. Supply, one per run, the dependency's guarantee over `Consumer`, a weaker
   `Producer` clause, a safety guarantee, and a same-named `Sends` from a
   third package (FR-530-AC-2).
4. Add the mutual dependency; then select the lattice-declaring profile and
   fill the fixed-point slot with a `proved`, then an `inconclusive`, FR-176
   result (FR-530-AC-4, FR-530-AC-5).
5. Run QSpec TC-983 case 5 (FR-530-AC-6).
6. Add a discharge record naming no pool entry, one whose pool entry is over
   another subject, and one naming an `inconclusive` result (FR-530-AC-7).
7. Add a forged slot, then a premise result for another slot's identity
   (FR-530-AC-8).

Compile an unrelated supported clause in the composing package and settle it
in every run. The TL-9 algorithm, FR-348 negotiation and FR-380 transfer are
exercised only through the results they produce; each mismatch and cycle case
must fail against a planner that accepts a referenced result as proof. Tag
the tests `#[trace("TC-1330", "<AC id>")]` and `#[trace("QSpec-TC-983")]` or
`#[trace("QSpec-TC-985")]` for each QSpec case they run.

## Expected Results

1. QSpec TC-985's results; the flag refused `invalid_runtime_input`/
   `wrong-value-kind`.
2. `proved`, `decisive-witness`, with the discharge record.
3. Each `inconclusive`, `premise-unproved` naming the liveness-dependency
   slot.
4. The item refused `invalid_model_binding`/`malformed-declaration` naming
   both guarantees; with the profile, `proved` carrying the FR-176 result,
   then `inconclusive`, `premise-unproved` naming the fixed-point slot.
5. QSpec TC-983 case 5's five distinct outcomes.
6. The first two refused `invalid_runtime_input`/`invalid-value` at the
   record; the third discharges nothing.
7. Each refused `invalid_runtime_input`/`invalid-value`.

The unrelated clause settles unchanged in every run.
