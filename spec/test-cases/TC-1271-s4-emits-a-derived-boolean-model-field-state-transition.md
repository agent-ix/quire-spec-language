---
id: TC-1271
title: "S4 emits a derived Boolean model-field state transition from the authored clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: references
  - target: ix://agent-ix/quire-specification/FR-341
    type: references
---
# TC-1271: S4 emits a derived Boolean model-field state transition from the authored clauses

## Description

Verify which (operation, Boolean field) pairs get a `state`/`transition` node,
how several preconditions and postconditions are bound by conjunction without
losing any authored clause node, and that ineligible pairs are skipped without
a refusal.

Scope: FR-105-AC-13 to FR-105-AC-17.

## Test Procedure

1. Spine-compile a unit over object type `Switch` with required `Boolean`
   fields `on` and `lit`, `armed: Option<Boolean>`, `count: Integer`, and
   operation `set(flag: Boolean)` whose frame modifies all four, with
   `pre P1 { flag or self.on }` and `post Q1 { self.on }`. Decode the graph
   and read it through QSL's I2 reader (strict-reader readback waits on
   IR-1052).
2. Add `post Q2 { self.on = pre(self.on) or flag }` and
   `pre P2 { true }`. Decode; then swap the source order of `Q1` and `Q2`
   and recompile.
3. For each of the four verdict combinations of `P1`, `P2` and of `Q1`, `Q2`,
   evaluate the derived `pre` and `post` and the authored clauses over one
   snapshot pair and call; compare.
4. Compile each AC-16 variant: a non-Boolean parameter; a frame not modifying
   `on`; `armed`; a `Sequence<Boolean>` field; `count`; a postcondition
   reading `on` only through a reference parameter or only inside `pre(...)`;
   a clause with an execution point; a lock with no protocol profile; and
   FR-105-AC-1's ConfigVersion package.
5. Add `lit` to `Q3 { self.lit }` with the same preconditions. Compile twice;
   compare bytes and the derived precondition's node id across subjects.
6. Inject an emitter fault at the transition node, then at a derived node, and
   compile.

Tag the tests `#[trace("TC-1271", "FR-105-AC-n")]`.

## Expected Results

- Step 1: one `state`/`transition` for (`set`, `on`) with `pre` the node of
  `P1` and `post` the node of `Q1`, `inputs` `[flag]`; no derived node; no
  transition for `lit`, `armed` or `count`; `P1` and `Q1` keep their nodes.
- Step 2: `post` references one derived `postcondition` node over
  `and(Q1, Q2)` and `pre` one derived `precondition` node over `and(P1, P2)`,
  both in ascending clause-name order; four authored clause nodes remain; one
  transition. Swapping source order changes no node id and not the
  `package_id`.
- Step 3: derived `pre` equals the conjunction of the authored precondition
  verdicts and derived `post` the conjunction of the `Q` verdicts in every
  combination; authored evaluations, names and `requirements` records are the
  same with and without the transition.
- Step 4: no `state`/`transition` node, no derived node and no refusal for
  each variant; the ConfigVersion package holds no transition.
- Step 5: identical bytes twice; `on` and `lit` share one derived
  precondition node and have two transitions with different `subject` and
  `post`; each transition occurrence carries a contributing postcondition's
  region.
- Step 6: the compile refuses; no package bytes and no `state` node.
