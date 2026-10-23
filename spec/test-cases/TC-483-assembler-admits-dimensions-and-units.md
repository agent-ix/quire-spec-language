---
id: TC-483
title: "The assembler admits source dimensions and units into a UnitGraph and refuses each source error"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-483: The assembler admits source dimensions and units into a UnitGraph and refuses each source error

## Description

Verify that the assembler normalizes dimension terms, reduces exact
numbers, mints dimension and unit keys over the unit's source owner by
QSpec FR-142's nominal preimages, admits them through `UnitGraph::admit`,
and refuses each source error with its own cause.

This catches: unnormalized or unsorted terms; an unreduced scale or offset
(which `UnitGraph::admit` would refuse); a unit keyed before its target; a
unit name read as a type; `UnitGraph::admit` reached only from tests; and a
topology error reported as a reader's `invalid_semantic_graph`.

Scope: FR-091-AC-32 to FR-091-AC-35, FR-091-AC-44 to FR-091-AC-47, FR-091-AC-50, FR-091-AC-55, FR-091-AC-60 to FR-091-AC-64, FR-093-AC-33, FR-093-AC-34, FR-093-AC-36, FR-093-AC-37, FR-093-AC-43, FR-093-AC-44, FR-093-AC-45, FR-093-AC-46.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`. The owner is authority `a`, identity `u`.

1. Assemble one unit holding the sources of FR-091 vectors Q1 to Q10. Read
   `units` and `nominal_spans`. Assemble a unit with no dimension or unit
   form and read `units`.
   With the decimal-scale bound set to `4`, assemble
   `dimension Length; unit m : Length = rational(1, 1);` with
   `unit c : Length = decimal(1, 5) * m;`, and again with `decimal(1, 4)`.
2. Assemble `dimension Length;`, `unit m : Length = rational(1, 1);` and
   `function f using v(x: m): Boolean pure { true }`.
3. Assemble FR-091-AC-34's unit, then the unit with the second
   `dimension Mass;`, `unit a` and `unit d` removed.
4. Assemble each of FR-091-AC-35's units, the five refused and the two
   that declare `N = L / L`.
5. Assemble FR-091-AC-44's unit with `function f using v(d: Quantity<ms>): Boolean pure { true }`, then with `d: Quantity<Time>`, then with `d: Quantity<nothing>`. Then check the first unit with a further `function g using v(): Boolean pure { f(3 s) }`. Check the first unit with a further `function h using v(): Boolean pure { f(3 ms) }`, and lower it. Assemble a unit declaring a unit `ms` and a record `ms`, a unit declaring two units named `ms` (refused `ambiguous_declaration`/`ambiguous-name` at the two declarations, each named), and a unit declaring `record Quantity`.
6. Check and lower the first unit of step 5 with each further body of
   FR-093-AC-33 (`h2`, `h3`, `h4` beside `h`, `n`, `k` and `c`, whose
   body is `convert<Quantity<s>>(d) = 3 s`), and pass each checked package
   to IR's v2 reader. Check it with each body of FR-093-AC-34 (`f(3 ghost)`,
   `f(3 Time)`, `f(rational(1, 0) ms)`, `f(rational(1, 0) ghost)`, `e`,
   `g2` and `m`), and with `f(rational(1, 100000) ms)` under a rational-bits
   ceiling of `16`.
7. Assemble each unit of FR-091-AC-45, AC-46 and AC-47.
8. With admitted domain packages `P` and `Q` selected in that lock order, assemble TC-187's U11r, U11s, U11t, U11u, U11y, U11z1 and U11z2 and FR-142-AC-19's `unit tick : M::Rate` and `unit tick : Rate` units; then FR-091-AC-55's shared dimension names (FR-091-AC-55).
9. Declare FR-093-AC-36's additional dimensions and units beside
   FR-091-AC-44's. Check and lower each of its bodies `p1` to `p6`, pass each
   checked package to IR's v2 reader, and evaluate each lowered body with the
   operand values AC-36 lists. Check each of FR-093-AC-37's bodies `z1` to
   `z10` in the same units. Declare FR-093-AC-45's temperature units, check
   and lower its bodies `n1` to `n6`, pass each package to IR's v2 reader and
   evaluate each lowered body with the operand values AC-45 lists, then check
   `n7`.
10. Assemble FR-091-AC-50's unit, then `unit u : Time2 = rational(1, 1);` beside it, then a second `dimension Time;`.
11. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) each declaring base dimension `Time` and a root unit, assemble each unit of FR-091-AC-60: the admitted equivalences, each refusal, the speeds with and without the length and time equivalences, the two phase-order units, and the equivalences in each source order within a phase; then each unit of FR-091-AC-63: the unit of another dimension, the swapped units, the affine unit, the zero and negative numbers, the speeds with ratios 1 and 2, the base cycle with and without a consistent closing ratio, and the ratio written through a larger unit.
12. Mint the keys of FR-091-AC-61: Q11, Q12 and Q13, `dimension Length ~ Distance by 1 km = 1 dist;`, `dimension Distance ~ Length by 1 dist = 1 km;` and `by 1 dist = 1000 m`, Q3's `Speed` beside `dimension Length ~ Distance by 1 km = 1 dist;`, and the three package pairs `R::Time ~ F::Time by 1 R::s = 1000 F::ms`, `R::Time ~ G::Time by 1 R::s = 1000 G::ms` and `F::Time ~ G::Time by 1 F::s = 1000 G::ms` under owner (`agent-ix`, `example`) over QSpec's three package `Time` dimension vectors; read `nominal_spans`.
13. Assemble FR-091-AC-62's `Force` units with and without `dimension R::Mass ~ F::Mass by 1 R::kg = 1 F::kg;`, and read the classes of the admitted graph over `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;` and over Q12; assemble FR-091-AC-64's three spellings of the unequal-root pair, read the class root and the identification edge, and the cycle with `1000` and `1001`.
14. Declare FR-093-AC-43's `Period`, `sec`, `msec` and `dimension Period ~ Time by 1 sec = 1 s;` beside FR-091-AC-44's units. Check and lower each of `c1` to `c4`, pass each checked package to IR's v2 reader, and evaluate `c1` to `c3` with the operand values AC-43 lists; check `c1` again without the equivalence. Declare FR-093-AC-46's `Beat`, `beat`, `bar` and `dimension Beat ~ Time by 1 beat = rational(1, 4) s;`, check and lower `u1` to `u5`, evaluate `u1` to `u4` with the values AC-46 lists, and check `u1`, `u2` and `u5` again under `by 1 beat = 1 s`.
15. Under the same units check each of FR-093-AC-44's `a1` to `a4`, then check and lower `a5` to `a8`, pass each package to IR's v2 reader and evaluate each with the operand values AC-44 lists.

Tag the tests with the AC ids they back and `TC-483`.

## Expected Results

- Step 1: dimensions keyed Q1 to Q5 and units keyed Q6 to Q10; Q3's and
  Q4's terms as the vectors list; `km`'s scale `1000`, `cm`'s `1/100`,
  `C`'s offset `5463/20`; `C` affine and `km` not; a span for each key in
  `nominal_spans`. The unit with no dimension or unit form has the empty
  `units`. `decimal(1, 5)` gives a limit refusal,
  `stage_limit_exceeded`/`work-budget-exceeded`, naming bound `4`, actual
  `5` and its span; `decimal(1, 4)` assembles.
- Step 2: an unresolved-type-name error naming `m`.
- Step 3: one refusal holding exactly the duplicate-name error (`Mass`), the
  unresolved-name error (`Ghost`) and the zero-denominator error, each with
  the code FR-091-AC-34 names, and no `PackageDeclarations`; then exactly
  one topology error, row 6 at `Length` with edges `b -> c`, `c -> b`.
- Step 4: each refused unit refuses with exactly one topology error carrying
  its QSpec FR-142 row's code and cause and naming what the row retains, and
  no `PackageDeclarations`; the two `N = L / L` units assemble.
- Step 5: the first assembles with the type of `d` the node of `ms` and no new declaration; the second and third each refuse the unresolved name (`missing_declaration`/`missing-name`) at assembly; the check of `g` refuses `ill_typed`/`type-mismatch` at the argument `3 s`, raised by check, not by the assembler; the check of `h` admits it and lowers `3 ms` to one `value`/`literal` node of `value_kind` `quantity`, value `"3"`, whose `semantic_type` and `type` are the `ms` unit node, referenced by the call's argument (FR-093-AC-33); the record-and-unit unit assembles, the two-unit unit refuses `ambiguous_declaration`/`ambiguous-name` at the two declarations, naming each, and `record Quantity` refuses `invalid_syntax`.
- Step 6: `h2`'s and `h3`'s arguments share one literal node of value
  `"1/2"`; `h4`'s `3 ms` is `h`'s node; `n` lowers a literal of value `"-3"`; `k` lowers `quire.op.quantity.eq`
  over `d` and a literal of value `"1/1000"` at the `s` node; `c` (`convert<Quantity<s>>(d) = 3 s`) lowers
  `quire.op.quantity.convert` with mode `rounding` = `exact` and then
  `quire.op.quantity.eq` over the convert node and the `3 s` literal; no
  `rational` literal is emitted; IR's v2 reader admits each package. The
  refusals carry FR-093-AC-34's codes and emit no literal node:
  `missing_declaration`/`missing-name` naming `ghost` and `Time`,
  `undefined_expression`/`unproved-nonzero` for both zero denominators
  (`ghost` included), `ill_typed`/`type-mismatch` at
  `e`'s and `g2`'s comparisons, `ill_typed`/`operator-ineligible` at `m`'s
  negation, and
  `stage_limit_exceeded`/`rational-bits-exceeded` naming `s3.rational_bits`
  and `16`.
- Step 7: each refused unit refuses with exactly the one error its AC
  names, the first defect in FR-142's order; each admitted unit assembles.
- Step 8: each vector's TC-187 result, with `P`'s node refusals located at
  `/lock/model_selections/0`; `unit tick : M::Rate` admitted and `unit tick : Rate`
  refused `missing_declaration`/`missing-name` at `Rate`; the source `Time` beside the package `Time`, and two
  packages' `Rate`, assemble with no error; the two `ms` units check.
- Step 9: each of `p1` to `p6` checks and lowers, its divisions to
  `quire.op.quantity.div` nodes over the dividend and the literal node (`p1`'s
  two divisions share one `3 ms` node, `p3`'s literal has value `"-3"` and
  `p4`'s value `"1/2"` at the `s` node) with the compound-unit result type
  AC-36 states; IR's v2 reader admits each package; the evaluations return the
  verdicts AC-36 lists. `z1`, `z2` and `z3` each refuse
  `undefined_expression`/`unproved-nonzero` at the first division, `z4` at the
  literal, `z5` to `z10` at the first division; none emits a node. `n1` to
  `n6` check and lower as AC-45 states (`n1` and `n2` to the same nodes, `n3`
  and `n4` to a `quire.op.quantity.div` over the negate node of the `3 ms`
  literal, `n5` to two nested negate nodes), IR's v2 reader admits each
  package, and the evaluations return the verdicts AC-45 lists; `n7` refuses
  `ill_typed`/`operator-ineligible` at the negation and emits no node.
- Step 10: the first unit assembles with no error and keeps `R::Time` and `R::s` as written; `Time2` refuses with an unresolved-name error naming `Time2`; the second `dimension Time;` refuses `ambiguous_declaration`/`ambiguous-name` at the two declarations, each named (FR-091-AC-50).
- Step 11: each admitted equivalence assembles; each refusal is the one error FR-091-AC-60 and AC-63 name, at the equivalence form, with the retained nodes, units, offsets or numbers they list; the phase-order units report the equivalence first for `L ~ L` and the row 1 defect first for `Dur ~ L2`; the speeds are admitted with ratio 1 and refused with ratio 2; the base cycle is admitted with the consistent closing ratio and refused `conflicting-definition` at the third form otherwise; the decisions are the same in every source order within a phase.
- Step 12: the keys FR-091-AC-61 gives, Q12 `9be8d947…3ff4c` from every spelling of its ratio, Q13 `ad8ecf79…a4e4` (`root_scale` 1/200), Q1's, Q11's and Q3's keys unchanged, `bbf770ea…00d4`, `d707eb94…6c24` and `584c6bbc…714b` for the three package pairs, and a span under each equivalence key.
- Step 13: the equivalence naming `F::kg` refuses `missing-name`, and `Force` refuses the base-dimension rule at `F::Mass` naming `ix://acme/flow/dimension/Mass` and `n` with and without any equivalence; the graph holds one class of `R::Time` and `F::Time` and one of `Distance` and `Length`, and no class for a graph with no equivalence; the unequal-root spellings give one key `d707eb94…6c24`, class root the `acme/rpc` `s` and an identification edge of scale `1/1000` from the `acme/clock` root; the `1001` cycle refuses at itself.
- Step 14: `c1` to `c3` check and lower to `quire.op.quantity.convert` (mode `rounding` = `exact`) with the verdicts AC-43 lists; IR's v2 reader admits each package; `c4` and the no-equivalence `c1` refuse `ill_typed` at the conversion; `u1` to `u4` check and lower with the verdicts AC-46 lists, `u1` false for `1 beat` and true for `rational(1, 4) beat` under ratio 1, and `u5` refuses `ill_typed`/`type-mismatch` under either declaration.
- Step 15: `a1` to `a4` refuse `ill_typed`/`type-mismatch`; `a5` to `a8` check and lower with the node shapes and verdicts AC-44 lists, `a7`'s two divisions sharing one dimensionless compound-unit node and `a8`'s two products one compound-unit node.
