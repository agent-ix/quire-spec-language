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

Scope: FR-315-AC-1 to FR-315-AC-6, FR-315-AC-15 to FR-315-AC-32 and FR-315-AC-53 to FR-315-AC-56.

Steps 8, 10, 18 and 20 read scripts with a `QF_LIA` solver, steps 18 and
20 with replay as well; every other step reads
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
9. Encode `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)`,
   `InductionBase{depth: 3}` for `always[0,3] holds(c.value <= 3)` and
   `Unrolling{depth: 2}` for `eventually[0,3] holds(c.value = 3)` over
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
16. Encode a query over FR-315-AC-24's `Counter` with `isZero`, then the
    false-extension query of FR-315-AC-17 with the written `id: Integer`
    field.
17. Encode `InductionStep{depth: 1}` over FR-315-AC-25's `Holder`.
18. Read the two `Counter` base models FR-315-AC-26 describes by the rules
    of Reading a satisfiable model, and replay what they yield.
19. Encode `Unrolling{depth: 1}` for `always holds(t.tally < 3)` over
    FR-315-AC-27's `Tally` with `peek`.
20. Read the false-extension `Counter` models and the altered base model
    FR-315-AC-28 describes, and replay what they yield.
21. Inside `qsl-eval`, encode FR-315-AC-29's TP-1 and TP-3 items with their
    profile set to event-position false-extension after S3.
22. Settle FR-315-AC-30's `Inductive` result whose rebuild meets `Fault`,
    the vault's `EncodingRefused` outcome carrying `Fault` with an `Ok`
    product, the `Err`-product outcome built when `HyperProductIdentity::of`
    faults, and that outcome against an item whose `of` gives an identity.
23. Encode FR-315-AC-1's query and read the negated property's disjuncts.
24. Call the encoding inside `qsl-eval` with a TP-1 item whose checked
    clause has its atom removed, and repeat FR-315-AC-21's requests.
25. Over `CasRefinesCounter2`, encode the `Initial` query and the `commitA`
    `Step` query, read the command heads in order, rename and reorder the
    invariant clauses, apply each of the six FR-315-AC-53 mutations (two
    applications swapped; `AaInv` and `CasInv` swapped; a hypothesis on `s1`;
    the initial-state predicate in a `Step` query; one assertion per
    application; the global negation beside a separate disjunction) and check,
    for each, that FR-314 returns `QueryMismatch` at that row's part for the
    unmutated certificate.
26. Over the lost-update variant, add the FR-315-AC-54 equalities to the
    `commitB` query and solve it for each of the two pre-states; encode a row
    with one application and a row with none.
27. Request the refinement queries of FR-315-AC-55's refusal cases and read
    each refusal's code, locus and FR-314 `Unencodable` part, and for the `Initial` refusal of case (6) FR-314's `QueryMismatch` at `Base`.
28. Encode `Initial` and every `Step { row }` query of `CasRefinesCounter`
    and `CasRefinesCounter2`, read each assertion's subterms for the shapes
    FR-315-AC-56 excludes, and normalise one term per rule of FR-315
    "Normal form".

Tag the tests `#[trace("TC-900", "FR-315-AC-n")]`.

## Fixtures

`CasRefinesCounter2` is ADR-020 §8's `CasRefinesCounter` (fixture
`tests/support/refinement_cas.rs`) with the population `counters` holding two
`Impl::Counter` objects with keys `c0` < `c1` (and the `Spec::counters` objects
mapped one to one), and three invariants: `AaInv` on `Impl::Counter`,
`self.tmpA <= 2`; `CasInv` on `Impl::Counter`, `self.busyA or self.tmpA <=
self.value` (FR-146-AC-3); `AbsInv` on `Spec::Counter`, `self.value <= 2`,
which is no hypothesis. The lost-update variant has `commitB`'s precondition
`self.busyB and self.tmpB < 2` and postcondition `pre(self.tmpB) < 2 and
self.value = pre(self.tmpB) + 1 and not self.busyB`, frame `modifies [value,
busyB]`, and the abstract `inc` of ADR-020 §8 (precondition `self.value < 2`,
postcondition `self.value = pre(self.value) + 1`).

A counter's tuple is `(value, tmpA, busyA, tmpB, busyB)`, the declaration
order of the fixture; the script's constants are ordered by field name.
FR-315-AC-54's `s0` tuples, added as equalities, are:

| Case | `c0` | `c1` | Result |
| --- | --- | --- | --- |
| `c0` not enabled, `c1` loses an update | `(0, 0, f, 0, f)` | `(1, 0, f, 0, t)` | sat |
| `c0` enabled and obeying the rule, `c1` not enabled | `(0, 0, f, 0, t)` | `(0, 0, f, 0, f)` | unsat |

Each tuple satisfies both hypotheses on both counters (`tmpA = 0`, and `0 <=
value`), and the definedness `D` is `true` (total integer terms). The
postcondition and frame fix `s1` deterministically for an enabled application:
the written fields are `value = tmpB + 1` and `busyB = false`, and every other
field of every counter equals `s0`. In the first case `c1`'s application gives
`s1.c1.value = 1` from `s0.c1.value = 1`, which the mapped abstract `inc`
(`value` must become 2) does not admit, so the rule fails and the script is
satisfiable; no application of `c0` is enabled. In the second case `c0`'s
application gives `s1.c0.value = 1` from `s0.c0.value = 0`, which `inc` admits,
and no other application is enabled, so the script is unsatisfiable.

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
- Step 5: pairwise distinctness of copies 0 to 2; for the TP-3 item, `m0.0`
  to `m2.0` with the monitor steps.
- Step 6: the scripts FR-315-AC-6 lists, byte-equal across the two encodings; the `any` row `SmtEncodingRefusal`; the abstract-operation row's script with the abstract precondition, frame and postcondition over `map(s0)`, `map(s1)` negated; the `bump` row `SmtEncodingRefusal` naming the row and the argument; the out-of-fragment row `SmtEncodingRefusal` naming it, no script.
- Step 7: each step assertion has the `bump` disjunct and then the stutter,
  the stutter holding the two field equalities and no other conjunct.
- Step 8: the satisfiable and unsatisfiable results FR-315-AC-16 lists.
- Step 9: `h0` to `h5` declared, `(not h0)`, each step assertion printed as
  FR-315-AC-17 gives it, with `inc`'s enabledness expanded over the
  candidates 0 to 3 and no stutter disjunct, `h<p>` in the violation at each
  position 0 to 5; the `InductionBase` and `eventually[0,3]` requests
  refused `query-kind`.
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
- Step 16: the `isZero` disjunct's postcondition part `(or Z0 Z1)` and no
  result constant; the `id` query refused `field-domain` at `c.id`.
- Step 17: the stutter and distinctness printed with the optional field
  equality FR-315-AC-25 gives.
- Step 18: the first model yields the prefix `0, 1, 2`, which replays to
  `refuted`; the second is refused by `replay_model_trace` as a stutter loop
  on a non-terminal state, read as no candidate, and the base query settles
  `InductionNotClosed{depth}`.
- Step 19: `peek`'s postcondition failure `(or U1 (and (not A1) U2))` and
  postcondition part `(or A1 A2)`, results `false` then `true`.
- Step 20: the prefixes `0, 1, 2, 3` (replayed to `refuted`) and `0, 1, 2`;
  the altered model's replay refused naming the altered step, settled as
  FR-127 settles that refusal (V-6 `ReplayRefused`); the
  `always[0,5] holds(c.value <= 1)` model at depth 2 gives no outcome.
- Step 21: `SmtEncodingRefusal::Fault` naming the infinite-trace invariant,
  with no script and no `Refused` code, for both items.
- Step 22: `SmtProofCheck::Fault` and `failed`, `Failed`, category failed;
  the `Ok`-product hyper outcome `failed`, `Failed`, not V-8; the
  `Err`-product outcome `failed`, `Failed`, with no `ItemIdentity` refusal;
  against the other item, `ProductMismatch` naming `clause`.
- Step 23: `h<p>` and then `(not ...)` at positions 0 to 3 only, each
  followed by `inc`'s postcondition failure; no atom-failure or
  precondition-failure disjunct.
- Step 24: `Fault` carrying `InternalFault` (`runtime_invariant`) naming the
  atom invariant, not `Refused` with `item-form`; no AC-21 request returns
  it.
- Step 25: the head sequence of FR-315-AC-53, the hypothesis order
  `AaInv`/`c0`, `AaInv`/`c1`, `CasInv`/`c0`, `CasInv`/`c1`, a `Step` query with
  one row assertion over the applications in `c0`, `c1` order; every mutation
  changes the bytes and gives `QueryMismatch`.
- Step 26: satisfiable for the first pre-state, unsatisfiable for the second;
  `(assert false)` for the row with no application.
- Step 27: the codes and loci of FR-315-AC-55, in its stated order.
- Step 28: no excluded shape in any assertion, the `commitB` row (row 1)
  included; the `Initial` and `commitA` assertions FR-315-AC-56 gives; each
  rule's stated result.
