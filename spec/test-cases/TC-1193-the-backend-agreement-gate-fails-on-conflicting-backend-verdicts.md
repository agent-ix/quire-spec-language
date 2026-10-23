---
id: TC-1193
title: "The backend-agreement gate fails on conflicting backend verdicts and catches a deliberately wrong backend"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-448
    type: verifies
---
# TC-1193: The backend-agreement gate fails on conflicting backend verdicts and catches a deliberately wrong backend

## Description

Run the `backend-agreement` gate over a fixture corpus with two registered
backends per item, show that agreeing verdicts pass, that a deliberately
wrong test backend is caught, that bounded and non-definite records are
reported without failing, and that preflight refusals exit 2.

Scope: FR-448-AC-1 to FR-448-AC-6.

## Test Procedure

1. Build a fixture corpus with two `language` vectors: item P, whose
   clause holds, and item R, whose clause is violated by a known input.
   Register two test backends advertising the items' capability, each
   answering from the reference evaluator, so both prove P and both
   return a counterexample for R. Run the gate.
2. Register a test backend in place of one of them that answers `proved`
   for R. Run the gate.
3. Register backends answering P with `BoundedComplete{depth: 3}` and R
   with a counterexample produced at bound 5, then at bound 3; then an
   unbounded proof against the bound-5 counterexample. Separately pair a
   `proved` record with each of `inconclusive`, `incomplete`,
   `unsupported` and `tested`.
4. Register a test backend that returns, for P, a counterexample whose
   input the reference evaluator accepts. Pair it with a backend that
   proves P.
5. Run with a candidate whose tool is not on the search path; with a
   corpus in which each item has one candidate; with `--limit s3.nodes=ten`;
   with `--capability CAP-Z` not in the corpus.
6. Run step 1 twice and compare the JSON reports.

Tag the tests `#[trace("TC-1193", "FR-448-AC-n")]`.

## Expected Results

- Step 1: P and R are `agree`, each entry naming both terminal records;
  exit 0.
- Step 2: R is `conflict`, naming R, both `BackendId`s and both terminal
  records; P stays `agree`; exit 1.
- Step 3: depth 3 against bound 5 is `not-compared` naming the bound, exit
  0; depth 3 against bound 3 is `conflict`, exit 1; unbounded against bound
  5 is `conflict`, exit 1; each non-definite pairing is `not-compared`
  naming the record's category, exit 0.
- Step 4: the counterexample settles `inconclusive` with `replay_parity`,
  never `refuted`; P is `not-compared` and the entry holds the replay
  result; exit 0.
- Step 5: each run refuses naming, in turn, the `BackendId` and its tool,
  `no-comparable-item`, the `--limit` operand and `CAP-Z`; each exits 2 and
  no backend runs.
- Step 6: the two reports are byte-equal, entries in ascending vector id
  and request index order.
