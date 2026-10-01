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

Verify `xtask refinement layering` end to end over layering corpora with
entries on all five QSpec AD-003 `requires` edges E1 to E5 (QSpec FR-453).
Each `case` and `distinguishing` entry is two units, byte-equal except for
the identity string of one header `profile` declaration, and both are
compiled by the running build.

Scope: FR-345-AC-4 to FR-345-AC-9.

## Test Procedure

Faults are injected only through FR-345's compile seam: the test passes the
gate's comparison core a wrapper around `qsl_replay::spine::compile`,
defined in the test module, that returns a chosen result for a named unit.

1. Build a test-only corpus of `entry.json` entries. For each of E1 to E5
   it holds:
   - a `case` both layers refuse with a typed refusal (a bounded-integer
     range proof that fails);
   - a `case` using the edge's distinguishing form, which the parent
     prohibits and the child admits;
   - a `distinguishing` entry for the edge;
   and for each of the five layers a `witness` entry: a bounded-scalar
   monitor invariant (state core), a named-predicate query over a bounded
   log (state queries), a reachability invariant over a finite universe
   (state graph), a record validation function (complete value) and a
   dispatched operation contract (complete model).
2. Run `xtask refinement layering` over it twice.
3. Run the gate core over step 1's corpus with a wrapper that, for each
   edge's both-refuse case, returns for the child unit the `Ok` compile of
   the same unit with the failing range proof removed.
4. Run the gate over a corpus holding one `case` whose units differ in one
   byte of a declaration body, then over one naming state core as parent
   and complete model as child.
5. Run the gate core with a wrapper that returns a refusal for state core's
   witness, then with one that returns `Ok` for E1's distinguishing unit on
   the parent side.
6. Run the gate over an empty corpus; over step 1's corpus without the state
   graph `witness` and the E5 `distinguishing` entry; over step 1's corpus
   with one `entry.json` holding an extra member `expected`.
7. Run the local test target over `tests/fixtures/refinement/layering/`,
   then over a copy with one `witness` entry removed.

Each expected result is a literal in the test. Tag the test
`#[trace("TC-868", "FR-345-AC-n")]` over the criteria each step backs.

## Expected Results

- Step 2: verdict success, exit 0. On each edge the both-refuse case
  `holds` reporting both codes and the distinguishing-form case is
  `not applicable`; each layer result and each edge result `holds`. The
  two reports are byte-equal.
- Step 3: verdict violation, exit 10; exactly five `regression`s, one per
  edge, each naming its case's two `RawSourceRef`s, its edge's two layer
  identities and the parent's codes; every other result as in step 2.
- Step 4: each run gives `tool failure` naming the entry; verdict tool
  failure, exit 30.
- Step 5: the first run gives a `regression` naming `quire.state.core/v1`;
  the second gives a `regression` naming E1; each verdict violation, exit
  10.
- Step 6: one tool-failure result naming the corpus; exactly two naming
  state graph / `witness` and E5 / `distinguishing`; one naming the
  `entry.json` path and the member `expected`, with every other entry still
  compiled. Each verdict tool failure, exit 30.
- Step 7: the target passes over the real corpus and fails over the copy.
