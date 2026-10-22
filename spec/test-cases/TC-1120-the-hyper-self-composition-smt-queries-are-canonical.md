---
id: TC-1120
title: "The HP-2 self-composition SMT queries are canonical and refuse what they cannot encode"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
---
# TC-1120: The HP-2 self-composition SMT queries are canonical and refuse what they cannot encode

## Description

Verify that `encode_hyper_query` declares one copy per trace variable,
steps the copies together on the joint steps `μ_U` matches, reads an
undefined match as a refutation, ignores fairness, and refuses a clause,
body, kind, subject binding or match conjunct it cannot encode.

Scope: FR-315-AC-7 to FR-315-AC-11.

## Test Procedure

1. Over ADR-023 §8's secure vault, encode `InductionBase{depth: 1}` for
   §1's `NonInterference` at instance `v`, twice; then for the clause with
   its `match` block removed.
2. Encode `InductionStep{depth: 1}` and `InductionStep{depth: 2}` for
   `NonInterference`, then both kinds for the clause with `fair { weak
   V::Vault::step }` on `a`.
3. Encode `InductionBase{depth: 1}` for `NonInterference` with
   FR-176-AC-4's added `match` conjunct.
4. Call `encode_hyper_query` with the HP-3 clause, FR-176-AC-3's liveness
   clause, `Unrolling{depth: 1}`, `SavesPower` with one subject, and a
   `match` conjunct whose function exhausts the request's `meter`, the last
   twice with equal budgets.
5. Submit step 1's base query and step 2's depth-1 step query to a `QF_LIA`
   solver; then the base query at depth 1 over the vault whose post clause
   is `self.l = self.h`. When no solver is available, the test SHALL be
   marked skipped with the reason "no QF_LIA solver" in the test report,
   never passed.

Tag the tests `#[trace("TC-1120", "FR-315-AC-n")]`.

## Expected Results

- Step 1: the eight declarations of FR-315-AC-7 in order, no `m<i>`, `a`'s
  then `b`'s initial predicate, three joint disjuncts for step 0 to 1
  (`(step(0), step(0))`, `(step(1), step(1))`, `(stutter, stutter)`), and
  without the `match` block all nine tuples in lexicographic order with
  `stutter` last in each component; byte-equal scripts; every term
  linear.
- Step 2: no initial predicate, the property at 0, distinctness of
  positions 0 and 1 (0, 1 and 2 at depth 2), the violation at the last
  position; the fair clause's scripts byte-equal to the unfair ones.
- Step 3: `(step(0), step(0))` kept as a disjunct, and the negated property
  holding when it is taken from copy 0 to copy 1.
- Step 4: `SmtEncodingRefusal` naming `hyper-form`, `hyper-body`,
  `query-kind`, `subject-binding` at alias `N`, and `match-conjunct` at the
  conjunct naming `(step(0), step(0))`, each with no script; equal
  refusals for equal budgets.
- Step 5: both secure queries unsatisfiable, the `Inductive{depth: 1}`
  outcome FR-182-AC-7 settles; the `self.l = self.h` base satisfiable from
  `((0, 0), (1, 0))` with `(step(0), step(0))`. A skipped step 5 is reported
  as skipped, and FR-315-AC-11 then stands on its analysis alone.
