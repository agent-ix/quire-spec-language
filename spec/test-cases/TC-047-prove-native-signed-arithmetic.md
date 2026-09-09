---
id: TC-047
title: "Prove signed arithmetic through the actual IR API"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-047: Prove signed arithmetic through the actual IR API

## Description

Integration, priority P1. Verifies FR-016-AC-3. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Check the named negative-division, negative-remainder, possible-zero-divisor, nonzero-guard and minimum-divided-by-minus-one rule cases using the real qualified model. Inspect retained discharged IR obligations and native locations. Include guarded boundary addition from TC-027 and a late guard after unsafe arithmetic.

## Expected Results

Signed constant division/remainder and the sufficient nonzero/bound guard cases are accepted. Possible zero division, signed minimum/-1, missing/weakened range guards and right-hand late guards return undefined_expression with the actual structured IR diagnostic. No sampled runtime value supplies a proof range, and no logical counterexample is claimed.

