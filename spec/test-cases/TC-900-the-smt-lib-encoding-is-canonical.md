---
id: TC-900
title: "The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
---
# TC-900: The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs

## Description

Verify that `encode_smt_query` prints the unrolling and induction queries
in canonical order and bytes, and refuses a construct it cannot encode.

Scope: FR-315-AC-1 to FR-315-AC-6 and FR-315-AC-15 to FR-315-AC-23.

Steps 8 and 10 read scripts with a `QF_LIA` solver; every other step reads
the printed script.

## Test Procedure

1. Encode `Unrolling{depth: 3}` for the TP-2 `on origin` claim
   `always[0,3] holds(c.value <= 3)` over FR-315-AC-1's `Counter` under
   event-position false-extension, twice.
2. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for
   `always holds(c.value <= 3)`.
3. Encode an item over a field of an unencodable sort.
4. Encode `Unrolling{depth: 2}` for
   `always holds(t.tally < 3 and t.next = t.tally + 1)` over FR-315-AC-4's
   `Tally` subject.
5. Encode `InductionStep{depth: 2}` for `always holds(c.value <= 3)` and for
   a TP-3 item.

6. Encode `Initial` and `Step { row }` for each row of `CasRefinesCounter`, twice, then for a refinement with one row changed to `any`, then with an abstract-operation row, with `CasRefinesCounterAdd` (FR-315-AC-6), whose `bump` row has a `_` argument, and with a mapping row outside `QF_LIA`.

7. Encode `Unrolling{depth: 3}` for `always holds(t.tally < 2)` over `Tally`.
8. With a solver, check the `Tally` queries FR-315-AC-16 lists: the
   `always holds(t.tally < 2)` and `always holds(t.tally < 3)` queries, and
   the deadlock-freedom item's base queries without and with
   `terminal when` `t.tally = 2`.
9. Encode `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` and
   `InductionBase{depth: 3}` for `always[0,3] holds(c.value <= 3)` over
   `Counter` under event-position false-extension.
10. With a solver, check the `Counter` false-extension queries FR-315-AC-18
    lists.
11. Encode `InductionStep{depth: 1}` for `always holds(t.tally < 3)` over
    `Tally`.
12. Encode `InductionBase{depth: 2}` for `Tally`'s deadlock-freedom item,
    without and with `terminal when` `t.tally = 2`.
13. Encode each refused request FR-315-AC-21 lists, twice.
14. Encode `InductionStep{depth: 2}` for `always holds(x.value <= 2)` over
    FR-315-AC-22's two-object `Counter`.
15. Encode `Unrolling{depth: 2}` and `InductionStep{depth: 2}` for
    FR-315-AC-23's TP-3 item over `Counter`.

Tag the tests `#[trace("TC-900", "FR-315-AC-n")]`.

## Expected Results

- Step 1: the declarations, with `h0` to `h3` interleaved, the assertions,
  `(not h0)` and `(check-sat)` FR-315-AC-1 lists; byte-equal scripts.
- Step 2: the base asserts the initial predicate on copy 0; the step
  asserts none, the property at 0 and its violation at 1.
- Step 3: `SmtEncodingRefusal` naming the field, no script.
- Step 4: the negated property holds, at each position 0 to 2, the
  claim's coercion failure (`s<i>.t.tally < 3` with `s<i>.t.tally + 1`
  outside `[0, 3]`), `G`'s precondition failure over copy `i`, and `C`'s
  postcondition failure over copy `i` and the 16 candidates; `B` contributes
  no failure term; every term is linear.
- Step 5: pairwise distinctness of copies 0 to 2; for the TP-3 item, `m0`
  to `m2` with the monitor steps.
- Step 6: the scripts FR-315-AC-6 lists, byte-equal across the two encodings; the `any` row `SmtEncodingRefusal`; the abstract-operation row's script with the abstract precondition, frame and postcondition over `map(s0)`, `map(s1)` negated; the `bump` row `SmtEncodingRefusal` naming the row and the argument; the out-of-fragment row `SmtEncodingRefusal` naming it, no script.
- Step 7: each step assertion has the `bump` disjunct and then the stutter,
  the stutter holding the two field equalities and no other conjunct.
- Step 8: the satisfiable and unsatisfiable results FR-315-AC-16 lists.
- Step 9: `h0` to `h5` declared, `(not h0)`, each step assertion in the
  halting form with `inc`'s enabledness expanded over the candidates 0 to 3
  and no stutter disjunct, `h<p>` in the violation at each position 0 to 5;
  the `InductionBase` request refused `query-kind`.
- Step 10: the satisfiable and unsatisfiable results FR-315-AC-18 lists.
- Step 11: the precondition and postcondition failures over copy 1 among the
  position-1 disjuncts, reading copy 1 and literals only; their negations
  over copy 0 in the position-0 assertion.
- Step 12: no `m<i>` or `h<i>`; the stutter disjunct kept; the violation at
  each position copy `i` terminal, and with `P` conjoined with `P`'s
  negation over copy `i`.
- Step 13: each code and locus FR-315-AC-21 lists, no script, equal across
  the two calls; the `too-large` refusal names `constants`, 1,048,577 and
  1,048,576.
- Step 14: the declaration, assertion and disjunct order FR-315-AC-22 lists.
- Step 15: the range assertions and monitor implications FR-315-AC-23
  lists, with no position-0 implication in the step case.
