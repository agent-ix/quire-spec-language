---
id: TC-1273
title: "Lower and read a checked present option"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: verifies
---

# TC-1273: Lower and read a checked present option

## Description

Verify S4 lowering and FR-264's QSL I2 read-back for the present option
expression used by FR-140-AC-11, plus the S3 type-mismatch refusal.

## Test Procedure

Compile a checked function whose result is `Option<Integer>` and whose body is
`Some(x)` for `x: Integer`. Read its emitted checked package through FR-264.
Have the test obtain the package id computed by that read from the checked
package contents, then assert that the emitted package id equals that computed
id and that the read returns `Verified`. Confirm the option value and its key,
type, payload dependency, and absence of a `Some` application follow the
carrier contract in QSpec FR-322; this test does not redefine that wire
contract or record a copied package digest.

Check a second function with `x: Integer`, result `Option<Boolean>`, and body
`Some(x)`. It must refuse at the source expression `Some(x)` with code
`ill_typed` and cause `type-mismatch`, before S4 emits a package; the exact
refusal locus is the `Some(x)` expression span.

Tag the tests `#[trace("TC-1273", "FR-093-AC-48", "FR-140-AC-11")]`.

## Expected Results

The admitted function's emitted package id equals the id FR-264 computes from its checked package contents, and FR-264 returns `Verified`. The mismatched function has no emitted package and refuses `ill_typed`/`type-mismatch` at the `Some(x)` source span. The present-option carrier is governed by QSpec FR-322.
