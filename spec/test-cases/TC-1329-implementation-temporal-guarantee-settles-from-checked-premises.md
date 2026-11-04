---
id: TC-1329
title: "Implementation temporal guarantee settles from checked premises"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-529
    type: verifies
---
# TC-1329: Implementation temporal guarantee settles from checked premises

## Description

Verify QSL's settlement of QSpec TC-982's cases through real native packages,
each defect seeded independently.

## Test Procedure

Run QSpec TC-982's nine cases (its premises and mutations are read from QSpec,
not copied) through compile, read-back, admission and settlement over real
native packages. Seed the transition, divergence and fairness mutations
independently, one per run, over the positive chain. Assert each result's
label, cause, extent, trust basis and source correspondence, and that sibling
items settle unchanged. Tag the tests `#[trace("TC-1329", "<AC id>")]`.

## Expected Results

QSpec TC-982's expected results, with the conclusion record bound to the
checked package's source map; the seeded transition, divergence and fairness
mutations each defeat only their own premise.
