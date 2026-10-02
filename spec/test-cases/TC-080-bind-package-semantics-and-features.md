---
id: TC-080
title: "Bind package semantics and features"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
---
# TC-080: Bind package semantics and features

## Description

Property, priority P1. Verifies FR-019-AC-6. Qualified; SR-111 records actual evidence. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Generate admitted source/model variants that add each known feature through used expressions, unreachable branches, unused declarations and nested wrappers. Independently compute the expected set from the named variant rather than by calling the exporter. Inspect every language/model/checking selector.

Cover every UnaryOp, BinaryOp and Builtin mapping, inferred expression types,
multiple aliases and recursive object references. Cycles must terminate through
exact declaration identity; expanding reference targets recursively without a
visited set is not an acceptable derivation. Existing source/model limits apply.

## Expected Results

Features are duplicate-free and sorted with no omissions or inventions.
