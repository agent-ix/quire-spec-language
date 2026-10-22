---
id: TC-551
title: "Per-step simulation records are written for functional refinements and never settle refuted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: verifies
---
# TC-551: Per-step simulation records are written for functional refinements and never settle refuted

## Description

Verify which `operation-contract` records the refinement's `requirements`
hook writes, their hypotheses, and how their backend outcomes settle.

Scope: FR-146-AC-1 to FR-146-AC-10.

## Test Procedure

Fixtures: `CasRefinesCounter` and its lost-update model (ADR-020 §8);
`RegisterHistory` (FR-138); `Coin` with `side` hidden (FR-141-AC-4).

1. Write the requests for `CasRefinesCounter`, and for it with `commitB ->
   any`.
2. Write the requests for `RegisterHistory` and `Coin`.
3. Add the invariant `CasInv` to `Impl::Counter` and write the request
   again.
4. Settle a backend "holds" for the `commitA` record, a backend
   counterexample for the lost-update `commitB` record from `(1, 0, f, 0,
   t)`, and each record with no SMT candidate registered.
5. Write the requests for `CasRefinesCounter` with its seven `step` rows in
   reverse source order, and for FR-136-AC-7's protocol (with its channel `ch` and `tso` outermost `parallel`) with its rows
   in source order and in reverse.
6. Add `AbsInv` on `Spec::Counter` beside `CasInv`, write the request, then
   write the clauses in the opposite source order.
7. Feed the settlement map the checker's `Rejected` and `Unverifiable`
   results for the `commitA` record's "holds" outcome.

8. Settle the safety half of `CasRefinesCounter` from its eight records and hypothesis items: all proved and certified; one hypothesis item `Uncertified`; the invariant `self.value < 0` with every step record proved; a backend counterexample for one `Step` record; a counterexample for the `Initial` record.

9. Settle the eight records of `CasRefinesCounter` when `check_refinement_smt_proof` returns `Verified`; then for `CasRefinesCounterAdd` (FR-315-AC-6), whose `bump` row has a `_` argument (the checker returns `Unencodable` for its step part), and for that fixture with the base query also mismatched.

10. Read the occurrence key of each of `CasRefinesCounter`'s eight records.
11. Read `CheckedGraph::requirements()` for `CasRefinesCounter`, with and
    without `bound CountersBound on Impl::counters [2]`, for the
    `Audit`/`audits` variant, for the variants whose `peek` takes
    `n: Integer` or returns `Integer`, negotiate the records against a
    backend advertising `operation-contract` only in bounded mode, and
    check the variant with a second population `spare` of `Impl::Counter`
    (FR-146-AC-10).

Tag the tests `#[trace("TC-551", "FR-146-AC-n")]`.

## Expected Results

- Step 1: eight records (one `Initial`, seven `Step`) with empty hypotheses;
  none with `commitB -> any`.
- Step 2: no `StepSimulationRecord`; one refinement record each.
- Step 3: every record lists `CasInv`; `CasInv`'s own record is present.
- Step 4: `proved`, `decisive-witness`, `Proved{Inductive{depth: 1}, Uncertified}` with no certificate;
  `inconclusive`, `Inconclusive(InductionNotClosed{depth: 1})`;
  `unsupported`.
- Step 5: the same eight records with the same `StepRowRef`s as step 1; the protocol subject's records name the positions of FR-136's protocol sort key in both orders.
- Step 6: `CasInv` listed and `AbsInv` not; unchanged by clause order.
- Step 7: `inconclusive`, `CertificateRejected{rule, at}` for `Rejected`;
  `proved`, `Uncertified` for `Unverifiable`.
- Step 8: `proved`, `Certified`; `proved`, `Uncertified`; `inconclusive`, `hypotheses-not-established`; `Inconclusive(InductionNotClosed{depth: 1})`; `refuted` through replay (FR-146-AC-8).
- Step 9: every record `Proved{Inductive{depth: 1}, Certified}`; with `CasRefinesCounterAdd` every record `Uncertified`; with the base mismatch the `Initial` record `Inconclusive(CertificateRejected{rule, at})` at `base` and the others `Uncertified` (FR-146-AC-4).
- Step 10: each key is (the refinement node, `generated`, ordinal): 0 for `Initial` and `i + 1` for `Step { row: i }` (FR-146-AC-9).
- Step 11: eight `operation-contract` records keyed by their occurrences, each `Unbounded` with the one `Impl::counters` population domain, boundable by `Cardinality`; each `Bounded` under `CountersBound`; the `audits` variant gives the same single domain; in the `n: Integer` and `Integer`-result variants `peek`'s record also names that `Integer` domain and the other seven are unchanged; against the bounded-only backend each `Unbounded` record settles `requires-bound`; the `spare` variant refuses `ambiguous_declaration`/`ambiguous-name` at each step row naming `counters` and `spare`, with no step record (FR-146-AC-10).
