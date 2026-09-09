---
id: TC-048
title: "Solve exact native contextual types"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-048: Solve exact native contextual types

## Description

Integration, priority P1. Verifies FR-016-AC-4, FR-016-AC-5. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Exercise literal constraints through arithmetic, conditional branches and let uses; size(self.items)=self.count; mismatched Distance/Duration; dimensioned multiply/divide/remainder; equal-representation but distinct nominal scalars; record/object/ref/enum/Bool comparison eligibility; text maximum and ordering; numeric roots and unequal conditional branch types.

## Expected Results

Unique explicit contexts infer the exact scalar identity. Unconstrained or inconsistent constraints refuse ill_typed, including incompatible nominal identities/units and forbidden equality/ordering. Every branch remains type-checked. Contextual size uses Count only when its interval covers all possible lengths; no IR unsigned-length coercion or model-wide first-fit search occurs.

