---
id: TC-1335
title: "dropFirst and removeOne check, lower and evaluate"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-1335: dropFirst and removeOne check, lower and evaluate

## Description

Verify that `dropFirst(c)` and `removeOne(c, v)` map through S2, check to
QSpec FR-145's derived types, lower to the QSpec FR-322 one-occurrence
identities and evaluate with FR-145's values and charges, and that the
ineligible kinds, wrong value types and wrong arities are refused. Scope:
FR-093-AC-59 and the `dropFirst` and `removeOne` rows of FR-091-AC-3 (QSpec
FR-145-AC-10 and AC-11, TC-190).

## Test Procedure

1. Check, lower and evaluate `f1`, `f2` and `g1` of FR-093-AC-59 and the
   `removeOne` bodies over the bag, set and ordered-set parameters with the
   listed values; read each node's identity, arguments, `result_type` and
   leaves, and pass each package to IR's v2 reader.
2. Check and lower `removeOne(t, x)` over the text bag and `removeOne(u, q)`
   over the float set; evaluate the float set vectors.
3. Evaluate `dropFirst(sequence[1, 2, 3])` and `removeOne(sequence[1, 2, 3], 2)`
   and read the meter's charge log of each.
4. Check `dropFirst` over the set and bag parameters, `removeOne(s, true)` and
   `dropFirst(i)`; parse `dropFirst(s, s)` and `removeOne(s)`.

Tag the tests `#[trace("FR-093-AC-59", "TC-1335", "QSpec-TC-190",
"QSpec-FR-145-AC-10")]` for `dropFirst` and with `QSpec-FR-145-AC-11` for
`removeOne`; step 1's S2 mapping also carries `FR-091-AC-3`.

## Expected Results

- Step 1: every function checks; each node and value is the one FR-093-AC-59
  lists; IR's v2 reader admits each package.
- Step 2: the text node lists the empty-path `nfc` leaf, the float node the
  empty-path `ieee_profile` leaf with mode `null`; the NaN member is removed
  by bit identity and `+0` is not removed by `-0`.
- Step 3: the `dropFirst` log holds only the formation of two survivors; the
  `removeOne` log holds the scan of `1` and `2` with no scalar retain, then the
  formation of two survivors.
- Step 4: `dropFirst` over a set or a bag refuses
  `ill_typed`/`operator-ineligible` at the call with no node;
  `removeOne(s, true)` refuses `ill_typed`/`type-mismatch` at `true`;
  `dropFirst(i)` refuses `ill_typed`/`type-mismatch` at `i`; the wrong arities
  refuse `invalid_syntax`/`unexpected-token`.
