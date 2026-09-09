---
id: TC-078
title: "Retain complete package inventories"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
---
# TC-078: Retain complete package inventories

## Description

Integration, priority P1. Verifies FR-019-AC-1, FR-019-AC-2, FR-019-AC-3. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Construct the smallest admitted source with one imported model and one constant clause, followed by real checked sources with multiple authored owners, invariants and pre/post clauses, multiple imports and unused selected declarations. Independently enumerate expected names, owners, clause kinds/anchors, original/formal source identities and selected model artifact bytes. Change only the local display path, then change an unused selected declaration and its actual import digest.

## Expected Results

Every source clause/import is retained in source order with exact typed-model/source correspondence. Display path alone leaves artifact bytes unchanged; selected declaration/role content changes them. A source/model setup error fails this test before packaging.

A header-only or import-only source is not positive setup: the adopted grammar
requires both an import and a clause. Its actual parser refusal belongs to
TC-087's adverse reconstruction controls, not a fabricated empty checked package.
