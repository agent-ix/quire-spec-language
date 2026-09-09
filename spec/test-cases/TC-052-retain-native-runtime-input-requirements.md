---
id: TC-052
title: "Retain population and invocation obligations after checking"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-052: Retain population and invocation obligations after checking

## Description

Integration, priority P1. Verifies FR-016-AC-9. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Check current parent-dereference/reaches clauses and pre/post step clauses, including constant true and captured pre/post aliases. Inspect checked expression types and required model/universe/observation/context/frame correspondence. Attempt reaches with mismatched type/universe/observation and compare identity equality across pre/post of one universe.

## Expected Results

Valid checked clauses retain all required input-validation assumptions and original source, with no runtime Boolean or executable IR package. Post self and pre/post frame validation remain required even for a constant postcondition. Reachability requires one exact observation; cross-observation identity equality can type-check within the same type/universe. No reference existence or closed population is asserted before FR-007 validation.

