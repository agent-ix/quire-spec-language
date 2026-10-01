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

Scope: FR-273-AC-1.

## Test Procedure

1. Run `cargo xtask canonical-types` over the QSL workspace.
2. Run `make ci`.

Tag the tests `#[trace("TC-748", "FR-273-AC-1")]`.

## Expected Results

- Step 1: no finding and exit 0.
- Step 2: `make ci` runs `canonical-types` among its prerequisites and
  passes.
