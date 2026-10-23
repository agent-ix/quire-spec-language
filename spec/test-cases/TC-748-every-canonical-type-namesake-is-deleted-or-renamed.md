---
id: TC-748
title: "Once every namesake is deleted or renamed, the canonical-types gate and make ci pass"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-273
    type: verifies
---
# TC-748: Once every namesake is deleted or renamed, the canonical-types gate and make ci pass

## Description

Verify FR-273: with the canonical types tagged and the verdict table carried
out, the gate finds no second definition in the QSL workspace and `make ci`
runs it and passes.

Scope: FR-273-AC-1 to FR-273-AC-51.

## Test Procedure

1. Before M-6d lands, run `cargo xtask canonical-types` on demand over the
   QSL workspace.
2. With the M-6c, M-6d and M-6e deletions FR-273's `delete` rows cite landed,
   run `cargo xtask canonical-types` over the QSL workspace.
3. Run `make ci`.
4. For each verdict-table row n of FR-273, read the row's namesake in the
   QSL workspace's shipped code and the gate's findings from step 2.

Tag the tests `#[trace("TC-748", "FR-273-AC-1")]`.

## Expected Results

- Step 1: the remaining namesakes are reported; `make ci` does not run the
  gate.
- Step 2: no finding and exit 0.
- Step 3: `make ci` runs `canonical-types` among its prerequisites and
  passes.
- Step 4: FR-273-AC-n holds for every row: a `delete` row's item does not
  exist, a `rename` row's item carries the row's name in its definition and
  every use, `qsl_replay::identity::QualifiedName` does not exist and
  `qsl-replay` names `qsl_eval::value::expression::family::QualifiedName`, which implements no `From<&str>` or `From<String>`, and a `compile_fail` doctest passing a string literal to the replay request's function selection fails to compile,
  each `keep` row's `pub use` names the row's path with no `as`, and `quire_semantic_value::location::Location` and `Origin` are named `ExpressionLocation` and `ExpressionOwner` with no `identifier` finding, and FR-401's location type resolves at the renamed path (FR-273-AC-49 to FR-273-AC-51).
