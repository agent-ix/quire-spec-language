---
id: TC-1329
title: "Implementation temporal guarantee settles from checked premises"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-529
    type: verifies
---
# TC-1329: Implementation temporal guarantee settles from checked premises

## Description

Verify QSL's compile, read-back, admission and settlement of QSpec FR-490's
`guarantee-decl` and FR-492's request over a real native package, each defect
seeded independently.

## Test Procedure

Compile QSpec TC-984 case 1's unit (the counter model, `Eventually2`, the
relation `A` and `guarantee G`) through S1 to S4 as a real native package,
emit it and read it back. Build QSpec TC-986's settlement request from that
package: each item-backed slot holds the FR-331 request item `qsl-route`
writes for its derived requirement record and a result entry for it, read
through QSL's result readers; the environment slot holds its record. Then:

1. Run QSpec TC-984's cases through compile and read-back (FR-529-AC-5,
   FR-529-AC-9).
2. Settle the positive chain (FR-529-AC-1).
3. Seed one per run, over the positive chain: a refuted transition-simulation
   result; the declaration without `progress`; a not-proved no-divergence
   result; `F_P = fair strong each inc` with a weak scheduling premise
   (FR-529-AC-2, FR-529-AC-3).
4. Remove `inc`'s frame binding; settle the frame slot `unsupported` (no
   Verus backend); stop the planner at `max_work` (FR-529-AC-4).
5. Run QSpec TC-986's admission cases 2 to 5, and fill a slot with a bare
   label and with a reference to another result (FR-529-AC-6,
   FR-529-AC-7).
6. Fill the model-claim slot with a `tested` result of a finite
   10000-step trace (FR-529-AC-8).
7. Read `premise-unproved` and `transfer-not-preserving` with valid payloads
   through `read_plugin_inconclusive_cause`, and encode the planner's causes
   (FR-529-AC-10).

Compile a sibling clause in the same package and settle it in every run.
Tag the tests `#[trace("TC-1329", "<AC id>")]` and, for each QSpec case they
run, `#[trace("QSpec-TC-982")]`, `#[trace("QSpec-TC-984")]` or
`#[trace("QSpec-TC-986")]`.

## Expected Results

1. QSpec TC-984's results; the relation-only unit's records and `route`
   results equal those without the relation.
2. `proved`, `decisive-witness`, with the conclusion record bound to the
   package's source map.
3. `premise-unproved` naming the transition-simulation slot, the first
   no-divergence slot, the no-divergence slot, and the fairness-correspondence
   slot, whose own outcome is `transfer-not-preserving{reason: fairness}`;
   each mutation defeats only its own slot.
4. `missing_declaration`/`missing-name` for the guarantee item only;
   `unsupported` naming the capability; `incomplete` naming `max_work`.
5. QSpec TC-986's refusals; both slots refused at admission.
6. Not `proved`; no result is `refuted`.
7. Both refused `unknown-cause` at `/cause/type`; the planner's spellings are
   QSpec FR-331's.

The sibling clause settles unchanged in every run.
