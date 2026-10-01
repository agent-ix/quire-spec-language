---
id: TC-868
title: "A seeded profile-layering regression fails the gate naming the case and its edge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: verifies
---
# TC-868: A seeded profile-layering regression fails the gate naming the case and its edge

## Description

Verify `xtask refinement layering` end to end over a layering corpus with
cases on all five QSpec AD-003 `requires` edges E1 to E5 (QSpec FR-453).
Each case is two units, byte-equal except for the identity string of one
header `profile` declaration, and both are compiled by the running build.

Scope: FR-345-AC-4 to FR-345-AC-7.

## Test Procedure

1. Build a test-only layering corpus. For each of E1 to E5 it holds:
   - a control case both layers refuse with a typed refusal (a
     bounded-integer range proof that fails);
   - a control case using the edge's distinguishing form, which the parent
     prohibits and the child admits;
   - the edge's distinguishing unit;
   and for each of the five layers, its witness unit: a bounded-scalar
   monitor invariant (state core), a named-predicate query over a bounded
   log (state queries), a reachability invariant over a finite universe
   (state graph), a record validation function (complete value) and a
   dispatched operation contract (complete model).
2. Run `xtask refinement layering` over it twice.
3. For each edge, add a seeded case: the edge's both-refuse control run
   with a test-only fault in the child side's compile that skips the failing
   range proof, so the child admits. Run the gate.
4. Run the gate over a corpus holding one case whose units differ in one
   byte of a declaration body, then over one naming state core as parent and
   complete model as child.
5. Run the gate with a test-only fault that makes state core refuse its
   witness, then with one that makes state core admit named predicates.

Each expected result is a literal in the test. Tag the test
`#[trace("TC-868", "FR-345-AC-n")]` over the criteria each step backs.

## Expected Results

- Step 2: verdict success, exit 0. On each edge the both-refuse control
  `holds` reporting both codes and the distinguishing-form control is
  `not applicable`; each layer result and each edge result `holds`. The
  two reports are byte-equal.
- Step 3: verdict violation, exit 10; exactly five `regression`s, one per
  edge, each naming its case's two `RawSourceRef`s, its edge's two layer
  identities and the parent's codes; every other result as in step 2.
- Step 4: each run gives `tool failure` naming the case; verdict tool
  failure, exit 30.
- Step 5: the first run gives a `regression` naming `quire.state.core/v1`;
  the second gives a `regression` naming E1; each verdict violation, exit
  10.
