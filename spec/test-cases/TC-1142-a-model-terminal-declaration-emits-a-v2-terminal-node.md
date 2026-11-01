---
id: TC-1142
title: "A model terminal declaration emits a v2 terminal node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-411
    type: verifies
---
# TC-1142: A model terminal declaration emits a v2 terminal node

## Description

Verify that S4 emits a checked model `terminal` declaration as a QSpec FR-476
`temporal`/`terminal` node, that its mode and predicate enter identity while
its name does not, that the deadlock-freedom item and its replay read it from
the package, that the I2 reader refuses a malformed node, and that QSL
settlement refuses a deadlock-freedom verdict decided under another
declaration.

Scope: FR-411-AC-1 to FR-411-AC-6.

## Test Procedure

Use FR-126-AC-9's `Counter` unit.

1. Compile it with `terminal Done using v on Counter when { self.value = 3 };`,
   with `terminal Halt using v on Counter any;`, and with no declaration;
   read each package back through the I2 reader.
2. Compare `package_id`s of the `when` and `any` units, and of the `when`
   unit with `Done` renamed `Stop`.
3. Compile FR-128-AC-6's unit with `terminal Done using v on Counter when {
   6 mod (3 - self.value) = 0 };` and replay its deadlock-freedom
   counterexample.
4. Edit step 1's `when` node to `mode` `some`, and separately to `condition`
   the empty aggregate; after each edit set `dependencies` to the body's
   targets, recompute the node key and the
   `package_id`, and feed the package to the I2 reader.
5. Write the request over step 1's `when` package and over its
   no-declaration package, and build each deadlock-freedom item's
   obligation; compile the unit also with `terminal Done using v on Counter
   when { self.value = 2 };` and compare its terminal node key with the
   `when` package's. Through the FR-281 derived-item adapter, settle the
   `when` item with the run and certificate request over the no-declaration
   package, with only the certificate request over it, with only the run
   over it, with both over the `value = 2` package, and with both over the
   `when` package; settle the no-declaration item over the `when` package
   and over its own. Then settle the no-declaration item over its own run
   with its replay recompiling the `value = 2` package.
6. Hand `model_item` the Counter's deadlock-freedom run, `protocol_item` a
   protocol subject's deadlock-freedom and race-freedom runs, and
   `zone_item` a timed deadlock-freedom run. Hand `derived_model_item` the
   dense `Rpc`'s derived deadlock-freedom item with the Counter's
   obligation and run. Over FR-430-AC-3's latching timers, write the timed deadlock-freedom item
   over the package with no declaration and over the package declaring
   `terminal Done using v on M when { true };`; through `derived_zone_item`,
   settle the first with a symmetry-reduced run over the `when` package and
   the certificate request over its own, with an unreduced run over its own
   and the certificate request over the `when` package, and settle the
   second with the reduced run and certificate request over its own. Hand
   `derived_zone_item` the timers' derived deadlock-freedom item with the
   run and certificate request over a package of another model.

Tag the tests `#[trace("TC-1142", "FR-411-AC-n")]` with the AC each step
backs.

## Expected Results

- Step 1 (FR-411-AC-1): one node for `when` (one reference to `self` in `parameters`, the
  condition node of `self.value = 3`) and one for `any` (empty `parameters`
  and `condition`), each with one `claim` occurrence; no terminal node
  without a declaration; the reader returns `When`, `Any` and `None`.
- Step 2 (FR-411-AC-2): different `package_id`s for `when` and `any`; the
  same node id and `package_id` after the rename.
- Step 3: the package holds the terminal node and the replay
  settles as FR-128-AC-6 states (FR-411-AC-3 is retired).
- Step 4 (FR-411-AC-4): `invalid_package`/`invalid-value` at `mode`;
  `invalid_semantic_graph` at `condition`.
- Step 5 (FR-411-AC-5): the two obligations are equal and the two `when`
  keys differ; each mismatched run or check refuses
  `stale_dependency`/`content-mismatch` naming `v` and the two node keys
  (`none` for no declaration), with no verdict settled; the `when` item
  over the `when` package settles `proved` and the no-declaration item over
  its own package `refuted`; with the `value = 2` replay it settles
  `inconclusive`, `replay_refused`.
- Step 6 (FR-411-AC-6): `DerivedRun` from the three public adapters;
  `OtherAnchor` naming the `Rpc` item's anchor, on EN-1 and on EN-6;
  `TerminalDeclaration` naming `none` and the `when` key for the reduced
  run and for the certificate request; `Proved{Reduced}` on EN-6 for the matching item.
