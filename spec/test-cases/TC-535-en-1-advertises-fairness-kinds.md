---
id: TC-535
title: "Candidates carry the fairness kinds their backends advertise"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-134
    type: verifies
---
# TC-535: Candidates carry the fairness kinds their backends advertise

## Description

Verify that EN-1 registers advertising weak and strong fairness and that
the registry carries each backend's fairness kinds to its candidates.

Scope: FR-134-AC-1.

## Test Procedure

1. Register EN-1 and a test temporal backend advertising only `Weak`;
   compute the candidates for an (`temporal-satisfaction`, `unbounded`)
   item.

Tag the tests `#[trace("TC-535", "FR-134-AC-n")]`.

## Expected Results

- Step 1: two candidates, EN-1 with `{Weak, Strong}` and the test backend
  with `{Weak}`.
