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

1. Against a lock that selects the protocol profile, spine-compile a unit
   over object type `Switch` with required `Boolean` fields `on` and `lit`,
   `armed: Option<Boolean>`, `count: Integer`, `seq: Sequence<Boolean>`, and
   operation `set(flag: Boolean)` whose frame modifies all five, with
   `pre P1 { flag or self.on }`, `post Q1 { self.on }`, and one
   postcondition reading each of `armed`, `count` and `seq`. Decode the graph
   and read it through QSL's I2 reader (IR-1052 landed in IR main f862d15;
   strict-reader readback waits only on a QSL `quire-contract-model` lock bump
   from 8b49bd4).
2. Add `post Q2 { self.on = pre(self.on) or flag }` and
   `pre P2 { not flag or self.lit }`. Decode; then swap the source order of
   `Q1` and `Q2` and recompile; then add `pre P3 { (flag or self.on) and true }`
   and compile; then declare `P1` twice with equal bodies under two names.
3. Walk the derived `pre` and `post` nodes' conditions and compare their
   operands with the authored clauses' condition nodes; compare the authored
   clause nodes, occurrences and `requirements` records with and without the
   transition.
4. Compile each AC-16 variant, each beside a postcondition that would
   constrain the field: a non-Boolean parameter; a frame not modifying `on`;
   `armed`; `seq`; `count`; postconditions `self.peer.on`,
   `let s = self in s.on`, `result.on` and one reading `on` only inside
   `pre(...)`; a pre/post clause with an execution point; `post QS on
   Switch::SubSwitch::set { self.on }`; a lock with no protocol profile; and
   FR-105-AC-1's ConfigVersion package.
5. Add `lit` to `Q3 { self.lit }` with the same preconditions. Compile twice;
   compare bytes and the derived precondition's node id across subjects.
6. Inject an emitter fault at the transition node, then at a derived node, and
   compile.

Tag the tests `#[trace("TC-1271", "FR-105-AC-n")]`.

## Expected Results

- Step 1: one `state`/`transition` for (`set`, `on`) with `pre` the node of
  `P1` and `post` the node of `Q1`, `inputs` `[flag]`; no derived node; no
  transition for `lit`, `armed`, `count` or `seq`; `P1` and `Q1` keep their nodes.
- Step 2: `post` references one derived `postcondition` node over
  `and(Q1, Q2)` and `pre` one derived `precondition` node over `and(P1, P2)`,
  both in ascending clause-name order; four authored clause nodes remain; one
  transition. Swapping source order changes no node id and not the
  `package_id`.
- Step 3: derived `pre` is `and(P1, P2)` and derived `post` `and(Q1, Q2)`,
  operands the authored condition nodes in ascending clause-name order; the
  authored nodes, occurrences and records are identical with and without the
  transition. `P3` binds its authored node and gains no borrowed occurrence;
  `P1` declared twice counts as one clause in `R`.
- Step 4: no `state`/`transition` node (for `on`, or for the field the
  variant names), no derived node and no refusal for each variant; the ConfigVersion package holds no transition.
- Step 5: identical bytes twice; `on` and `lit` share one derived
  precondition node and have two transitions with different `subject` and
  `post`; each transition occurrence carries a contributing postcondition's
  region.
- Step 6: the compile refuses; no package bytes and no `state` node.
