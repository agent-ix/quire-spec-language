---
id: TC-483
title: "The assembler admits source dimensions and units into a UnitGraph and refuses each source error"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/TC-187
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-483: The assembler admits source dimensions and units into a UnitGraph and refuses each source error

## Description

Verify that the assembler normalizes dimension terms, reduces exact
numbers, mints dimension and unit keys from the required package name and declared paths by
QSpec FR-322, admits them through `UnitGraph::admit`,
and refuses each source error with its own cause.

This catches: unnormalized or unsorted terms; an unreduced scale or offset
(which `UnitGraph::admit` would refuse); a unit keyed before its target; a
unit name read as a type; `UnitGraph::admit` reached only from tests; and a
topology error reported as a reader's `invalid_semantic_graph`.

Scope: FR-091-AC-74, FR-091-AC-33 to FR-091-AC-35, FR-091-AC-44 to FR-091-AC-47, FR-091-AC-50, FR-091-AC-55, FR-091-AC-60, FR-091-AC-62 to FR-091-AC-64, FR-091-AC-67, FR-091-AC-68, FR-091-AC-71, FR-091-AC-72, FR-091-AC-73, FR-092-AC-25, FR-092-AC-29, FR-093-AC-33, FR-093-AC-34, FR-093-AC-36, FR-093-AC-37, FR-093-AC-43, FR-093-AC-44, FR-093-AC-46, FR-093-AC-60.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`. The owner is authority `a`, identity `u`.

1. Assemble one unit holding the sources of FR-091 vectors Q1 to Q10. Read
   `units` and `nominal_spans`. Assemble a unit with no dimension or unit
form and read `units`; repeat with source-owner identity `w` and the same required `package_name`.
   With the decimal-scale bound set to `4`, assemble
   `dimension Length; unit m : Length = rational(1, 1);` with
   `unit c : Length = decimal(1, 5) * m;`, and again with `decimal(1, 4)`.
2. Assemble `dimension Length;`, `unit m : Length = rational(1, 1);` and
   `function f using v(x: m): Boolean pure { true }`.
3. Assemble FR-091-AC-34's unit, then the unit with the second
   `dimension Mass;`, `unit a` and `unit d` removed.
4. Assemble each of FR-091-AC-35's units, the five refused and the two
   that declare `N = L / L`.
5. Assemble FR-091-AC-44's unit with `function f using v(d: Quantity<ms>): Boolean pure { true }`, then with `d: Quantity<Time>`, then with `d: Quantity<nothing>`. Then check the first unit with a further `function g using v(): Boolean pure { f(3 s) }`. Check the first unit with a further `function h using v(): Boolean pure { f(3 ms) }`, and lower it. Assemble a unit declaring a unit `ms` and a record `ms`, a unit declaring two units named `ms` (refused `ambiguous_declaration`/`ambiguous-name`, retaining both declaration-name spans and locating the first complete `Quantity<ms>` type form when present, otherwise the later declaration-name span), and a unit declaring `record Quantity`.
6. Check and lower the first unit of step 5 with each further body of
   FR-093-AC-33 (`h2`, `h3`, `h4` beside `h`, `n`, `k` and `c`, whose
   body is `convert<Quantity<s>>(d) = 3 s`), and pass each checked package
   to IR's v2 reader. Check it with each body of FR-093-AC-34 (`f(3 ghost)`,
   `f(3 Time)`, `f(rational(1, 0) ms)`, `f(rational(1, 0) ghost)`, `e`
   and `g2`), and with `f(rational(1, 100000) ms)` under a rational-bits
   ceiling of `16`.
7. Assemble each unit of FR-091-AC-45, AC-46 and AC-47.
8. With admitted domain packages `P` and `Q` selected in that order, assemble TC-187's U11r, U11s, U11t, U11u, U11y, U11z1 and U11z2 and FR-142-AC-19's `unit tick : M::Rate` and `unit tick : Rate` units; then FR-091-AC-55's shared dimension names (FR-091-AC-55).
9. Declare FR-093-AC-36's additional dimensions and units beside
   FR-091-AC-44's. Check and lower each of its bodies `p1` to `p6`, pass each
   checked package to IR's v2 reader, and evaluate each lowered body with the
   operand values AC-36 lists. Check each of FR-093-AC-37's bodies `z1` to
   `z14` in the same units, including the local `divisor` function used by
   `z14`; evaluate `z9` and `z10` at `d = 6 ms`. Quantity negation (formerly FR-093-AC-45's `n1` to `n7`) is TC-1285's.
10. Assemble FR-091-AC-50's unit, then `unit u : Time2 = rational(1, 1);` beside it, then a second `dimension Time;`.
11. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) each declaring base dimension `Time` and a root unit, assemble each unit of FR-091-AC-60: the admitted equivalences, each refusal, the speeds with and without the length and time equivalences, the two phase-order units, and the equivalences in each source order within a phase; then each unit of FR-091-AC-63: the unit of another dimension, the swapped units, the affine unit, the zero and negative numbers, the speeds with ratios 1 and 2, the base cycle with and without a consistent closing ratio, and the ratio written through a larger unit.
12. Read the structural keys and normalized equivalence bodies of FR-091-AC-68's Q11, Q12 and Q13 sources: `dimension Length ~ Distance by 1 km = 1 dist;`, `dimension Distance ~ Length by 1 dist = 1 km;` and `by 1 dist = 1000 m`, `Speed` beside `dimension Length ~ Distance by 1 km = 1 dist;`, and the three package pairs `R::Time ~ F::Time by 1 R::s = 1000 F::ms`, `R::Time ~ G::Time by 1 R::s = 1000 G::ms` and `F::Time ~ G::Time by 1 F::s = 1000 G::ms` under owner (`agent-ix`, `example`) over QSpec's three package `Time` dimension sources; read `nominal_spans`, then repeat with a changed source-owner unit label and fixed `package_name`.
13. Assemble FR-091-AC-62's `Force` units with and without `dimension R::Mass ~ F::Mass by 1 R::kg = 1 F::kg;`, and read the classes of the admitted graph over `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;` and over Q12; assemble FR-091-AC-64's three spellings of the unequal-root pair, read the class root and the identification edge, and the cycle with `1000` and `1001`.
14. Declare FR-093-AC-43's `Period`, `sec`, `msec` and `dimension Period ~ Time by 1 sec = 1 s;` beside FR-091-AC-44's units. Check and lower each of `c1` to `c4`, pass each checked package to IR's v2 reader, and evaluate `c1` to `c3` with the operand values AC-43 lists; check `c1` again without the equivalence. Declare FR-093-AC-46's `Beat`, `beat`, `bar` and `dimension Beat ~ Time by 1 beat = rational(1, 4) s;`, check and lower `u1` to `u5`, evaluate `u1` to `u4` with the values AC-46 lists, and check `u1`, `u2` and `u5` again under `by 1 beat = 1 s`.
15. Under the same units check each of FR-093-AC-44's `a1` to `a4`, then check and lower `a5` to `a8`, pass each package to IR's v2 reader and evaluate each with the operand values AC-44 lists.

16. Read the retained fields of each topology refusal of steps 4 and 7 as
    values (FR-091-AC-71); assemble the U11p, U11w, U11x, U11n, U11o and
    U11z3 units of FR-091-AC-71, U11x also with `unit e : E = rational(0, 1);`
    added, and read each refusal's code, cause, locus and
    fields.
17. With `acme/rpc` declaring base dimension `Rate` and root unit `us`,
    assemble FR-091-AC-72's `tick` and `tock` units, check and evaluate `k1`
    for `d` = `2 tick`, and assemble each of its refused units in its own unit
    beside the base declarations alone.
18. Check, lower and replay the source `dimension L; dimension N = L / L; unit m : L = rational(1, 1); unit one : N = rational(1, 1);`. Read the checked dimension payloads and replay-admitted dimensions, their `base` flags and terms, and the dimension ids referenced by both units.
19. Emit `dimension L; dimension N = L / L; dimension M = L * L;` with a unit over each, read the emitted checked-package wire bodies of the three dimension nodes and read the package with IR's V2 reader (FR-092-AC-29).

Tag the tests with the AC ids they back and `TC-483`.

## Expected Results

- Step 1: five dimensions and five units keyed by their QSpec FR-322 declared structural paths, with unchanged keys when only the source-owner unit label changes; `Speed`'s exact ordered terms `[(Length, 1), (Time, -1)]` and
  `Accel`'s `[(Length, 1), (Time, -2)]` in ascending structural-key order; `km`'s scale `1000`, `cm`'s `1/100`,
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
  `e`'s and `g2`'s comparisons, and
  `stage_limit_exceeded`/`rational-bits-exceeded` naming `s3.rational_bits`
  and `16`.
- Step 7: each refused unit refuses with exactly the one error its AC
  names, the first defect in FR-142's order; each admitted unit assembles.
- Step 8: each vector's TC-187 result, with `P`'s row-4 and row-6 node
  refusals located at `P`'s `model_root` node. Imported base-dimension
  refusals retain the selecting model's original source provenance separately
  from the backed owner-field locus of FR-091-AC-74 and the procedure below;
  `unit tick : M::Rate` admitted and `unit tick : Rate`
  refused `missing_declaration`/`missing-name` at `Rate`; the source `Time` beside the package `Time`, and two
  packages' `Rate`, assemble with no error; the two `ms` units check.
- Step 9: each of `p1` to `p6` checks and lowers, its divisions to
  `quire.op.quantity.div` nodes over the dividend and the literal node (`p1`'s
  two divisions share one `3 ms` node, `p3`'s literal has value `"-3"` and
  `p4`'s value `"1/2"` at the `s` node) with the compound-unit result type
  AC-36 states; IR's v2 reader admits each package; the evaluations return the
  verdicts AC-36 lists. `z9` and `z10` check and lower to dimensionless quotients; at `d = 6 ms`, both are true with the values and units AC-37 states. `z1` to `z3`, `z5` to `z8`, and `z11` to `z14` refuse `undefined_expression`/`unproved-nonzero` at the first division; `z4` refuses at its zero-denominator literal; every refused function emits no node. In particular, negating a parameter (`z12`), a `let` value (`z13`) or a call result (`z14`) does not prove nonzero, despite each having value `3` in unit `ms` in the fixture. The refused divisions emit no quotient node, result unit or result value.
- Step 10: the first unit assembles with no error and keeps `R::Time` and `R::s` as written; `Time2` refuses with an unresolved-name error naming `Time2`; the second `dimension Time;` refuses `ambiguous_declaration`/`ambiguous-name` at the two declarations, each named (FR-091-AC-50).
- Step 11: each admitted equivalence assembles; each refusal is the one error FR-091-AC-60 and AC-63 name, at the equivalence form, with the retained nodes, units, offsets or numbers they list; the phase-order units report the equivalence first for `L ~ L` and the row 1 defect first for `Dur ~ L2`; the speeds are admitted with ratio 1 and refused with ratio 2; the base cycle is admitted with the consistent closing ratio and refused `conflicting-definition` at the third form otherwise; the decisions are the same in every source order within a phase.
- Step 12: each equivalence key has the required package and QSpec FR-322 derived path `["scalar_type", "dimension_equivalence", first id, second id]`; equivalent ratio spellings and a source-owner unit-label change preserve it. The former Q12 and Q13 sources normalize to `root_scale` `1000/1` and `1/200` for the first ordered node; adding an equivalence leaves the `Length`, `Distance` and `Speed` declaration keys and `Speed`'s terms unchanged. Each package pair holds its two dimension node ids in ascending order with normalized scales `1/1` (rpc/flow), `1000/1` (rpc/clock) and `1000/1` (flow/clock) for the first, and each equivalence has a span under its key.
- Step 13: the equivalence naming `F::kg` refuses `missing-name`, and `Force` refuses the base-dimension rule at `F::Mass` naming `ix://acme/flow/dimension/Mass` and `n` with and without any equivalence; the graph holds one class of `R::Time` and `F::Time` and one of `Distance` and `Length`, and no class for a graph with no equivalence; the unequal-root spellings give one structural key for the clock pair, class root the `acme/rpc` `s` and an identification edge of scale `1/1000` from the `acme/clock` root; the `1001` cycle refuses at itself.
- Step 14: `c1` to `c3` check and lower to `quire.op.quantity.convert` (mode `rounding` = `exact`) with the verdicts AC-43 lists; IR's v2 reader admits each package; `c4` and the no-equivalence `c1` refuse `ill_typed` at the conversion; `u1` to `u4` check and lower with the verdicts AC-46 lists, `u1` false for `1 beat` and true for `rational(1, 4) beat` under ratio 1, and `u5` refuses `ill_typed`/`type-mismatch` under either declaration.
- Step 15: `a1` to `a4` refuse `ill_typed`/`type-mismatch`; `a5` to `a8` check and lower with the node shapes and verdicts AC-44 lists, `a7`'s two divisions sharing one dimensionless compound-unit node and `a8`'s two products one compound-unit node.
- Step 16: each refusal carries the fields the "Topology error fields" table
  lists, as values: the scale and offset rationals, the target name as
  written, both dimension nodes, the roots in source order and the cycle
  edges in order; the step (-1) refusals carry the owning dimension and the
  term (`D` and `K`, `A` and `K`) or the cycle edges, at the dimension
  FR-091-AC-71 names.
- Step 17: `tick` and `tock` assemble with scales `1/2` and `3/2` to `us`;
  `k1` is `true`; `bad` refuses row 3 carrying `bad`, `s`, `M::Rate` and
  `Time`; `r2` row 4 carrying `us` and `r2`; `z` row 1 with scale `0`; the
  `M::ghost` target row 2 with the name as written.
- Step 18: `L` and `N` remain distinct nodes through assembly, checking, lowering and replay; `L` is base with `base=true`, while `N` is derived with `base=false` and no normalized terms. Both units reference their declared dimension, and only `one` is dimensionless.
- Step 19: the wire body of `L` has `base=true` with no terms, `N` has `base=false` with no terms and `M` has `base=false` with one term; IR admits the package and its `dimension_base()` equals the written value for each node.

### Duplicate unit use locus

Assemble two original linked source units that each declare `ms`, with multiple
unqualified `Quantity<ms>` references. Require one typed duplicate-unit refusal
with both original declaration-name spans and its primary span at the first
complete quantity type form in canonical source-unit and source order. Require
the original input source reference and public compiler region to identify that
same input when a different linked input has an identical relative span. Put an
ordinary record type named `ms` and a qualified `Quantity<M::ms>` before the
matching use and require neither to become its locus. Repeat with no matching
quantity reference and with duplicate dimension declarations to retain the
existing declaration-locus controls.

### Imported base-dimension owner-field evidence

Use QSpec TC-187 U34 by reference. Admit the original `acme/rpc` model with
unitless base dimension `Length`, then compile the genuine source selection,
`dimension Area = M::Length ^ 2` and `unit m2 : Area`. Through both the public
assembly and compiler refusal APIs require the typed missing-name cause,
original `ix://acme/rpc/dimension/Length` identity and `m2`, preserving topology
turn/refusal precedence. Separately inspect the selecting model's original
source reference and source span.

Require the owner-field pointer to carry its actual checked-artifact reference.
Resolve `/semantic_graph/nodes/0/owner` against that retained fixture artifact
and assert the owner is the original admitted model declaration. Use a genuine
producer input that places the owner at another node position and resolve its
actual pointer instead of expecting index zero universally. Never manufacture
an owner-only wrapper graph or infer an artifact pointer from the source span
or the model input's `/dimensions/...` path. No partially admitted program is
returned by a refusal. The producer boundary described in FR-091 remains an
explicit prerequisite: a source-span-only refusal characterizes existing
behavior but does not pass FR-091-AC-74.

Tests for these new obligations carry separate two-id tags such as
`#[trace("TC-483", "FR-091-AC-74")]` and the corresponding QSpec authority tag.
