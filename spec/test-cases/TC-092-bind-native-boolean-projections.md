---
id: TC-092
title: "Bind complete native Boolean projections"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: verifies
---
# TC-092: Bind complete native Boolean projections

## Description

Check source, authored identity, declaration and observation correspondence at
the real executable binder boundary.

## Test Procedure

Build native packages with Boolean state reads and operation inputs. Lower all
clauses and compare the complete authored census, kinds, anchors, exact source
spans and read mappings. Change a referenced state to an operation input, and
separately change an unused native declaration. Remove one binding from actual
emitted wire and submit it to the strict binder.

## Expected Results

Healthy projections bind completely. The referenced declaration change alters
bound identity; the unrelated model change alters native identity. The strict
binder refuses missing population. Native artifacts remain unchanged.
