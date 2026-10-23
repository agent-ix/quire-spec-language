---
id: TC-1193
title: "The backend-agreement gate fails on conflicting verdicts, including a wrong backend's proof or refutation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-448
    type: verifies
---
# TC-1193: The backend-agreement gate fails on conflicting verdicts, including a wrong backend's proof or refutation

## Description

Run the `backend-agreement` gate over a fixture corpus with two registered
backends per item. Show that agreeing verdicts pass; that a test backend
proving a refuted item, and one whose refutation the gate's replay does
not reproduce, are both caught; that a provider-settled `replay_parity`
record, bounded-mode proofs outside their domain, non-definite records,
unnegotiated candidates and compile refusals are reported without
failing; and that preflight refusals exit 2.

Scope: FR-448-AC-1 to FR-448-AC-8.

## Test Procedure

1. Build a fixture corpus with two `language` vectors: item P, whose
   clause holds, and item R, whose clause is violated by a known input.
   Register two test backends advertising the items' capability, each
   negotiating `supported` and answering from the reference evaluator, so
   both prove P and both return a `refuted` record for R. Run the gate.
2. Replace one of them with a test backend that answers `proved` for R.
   Run the gate.
3. Pair a `BoundedComplete{depth: 3}` proof of R with the other backend's
   `refuted` record. Then pair a `bounded`-mode proof of an item over the
   declared domain `0..=10` with a reproduced counterexample with witness
   12, then with witness 4. Separately pair a `proved` record with each of
   `inconclusive` (`BoundReached{depth: 3}`), `incomplete`, `unsupported`
   and `tested`.
4. Register a test backend that returns, for P, a `refuted` record whose
   counterexample input the reference evaluator accepts. Pair it with a
   backend that proves P. Then replace it with a test backend that
   returns, for P, `inconclusive` with `replay_parity`.
5. Run with a candidate whose tool is not installed; with a corpus in
   which each item has one candidate; with `--limit s3.nodes=ten`; with
   `--capability CAP-Z` not in the corpus.
6. Run step 1 twice and compare the JSON reports.
7. Register a third test backend for P whose negotiation settles
   `requires-bound`, and remove one of the two `supported` backends.
8. Add to the step 1 corpus a vector whose source `compile` refuses
   `ill_typed`, then a vector naming a source file absent from the corpus.

Tag the tests `#[trace("TC-1193", "FR-448-AC-n")]`.

## Expected Results

- Step 1: P and R are `agree`, each entry naming both dispositions and
  terminal records, and R's entry the gate's replay results; exit 0.
- Step 2: R is `conflict`, naming R, both `BackendId`s and both terminal
  records; P stays `agree`; exit 1.
- Step 3: `BoundedComplete` against the refutation is `conflict`, exit 1;
  the bounded-mode proof against witness 12 is `not-compared` naming the
  domain `0..=10`, exit 0, and against witness 4 is `conflict`, exit 1;
  each non-definite pairing is `not-compared` naming the record's
  category, exit 0.
- Step 4: the unreproduced refutation makes P `conflict`, naming that
  backend, its record and the replay result, exit 1; the `replay_parity`
  record leaves P `not-compared` with that cause, exit 0.
- Step 5: each run refuses naming, in turn, the `BackendId` and its tool,
  `no-comparable-item`, the `--limit` operand and `CAP-Z`; each exits 2 and
  no backend runs.
- Step 6: the two reports are byte-equal, entries in ascending bytewise
  vector id and then request index order.
- Step 7: the `requires-bound` candidate does not run and is reported with
  that disposition; P is `not-compared`; exit 0.
- Step 8: the `ill_typed` vector is `not-compiled` holding the refusal and
  the exit stays 0; the missing-file vector is `tool-failure` holding the
  I/O error, P and R keep their settlements, and the gate exits 1.
