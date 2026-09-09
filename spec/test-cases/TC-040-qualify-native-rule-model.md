---
id: TC-040
title: "Qualify the source-derived native rule model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-040: Qualify the source-derived native rule model

## Description

Integration, priority P1. Verifies FR-015-AC-1. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Produce the separately authored rule-model fixture in Rust through actual IR constructors and NativeModel::new. Read exact source bytes; bind every declaration/role locus through FormalSource. Inspect all scalar site mappings, wrappers, object/reference/universe roles, operation parameters/result and frame.

## Expected Results

The public model preserves Version 0..1000, Signed -10..10, Count 0..3, Wide signed i64 bounds, Distance/metre and Duration/second. Node.n/parent/peer/items and step match the independent FS03 hypotheses. Seq retains max 3 and duplicate-preserving ordered semantics; the Bool result is a real Input declaration assigned to step. Model source identity, digest, IR ownership and all declaration/role loci remain inspectable. Setup failure fails this case.

