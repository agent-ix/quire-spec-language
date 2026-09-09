---
id: TC-044
title: "Resolve explicit references and operation declarations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-044: Resolve explicit references and operation declarations

## Description

Integration, priority P1. Verifies FR-015-AC-5. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Link invariant parent-field dereference, one-edge reaches syntax, and pre/post step clauses through link_native with the qualified model. Inspect native/model/operation occurrence targets. Send the analogous constructs through original link with its formal-only artifact. Exercise a missing operation/field and an ordinary enum used as a reference.

## Expected Results

Native occurrences retain exact record/field/operation and role source correspondence. Dereference names resolve through the explicitly selected reference role. Missing or unadmitted mappings refuse without a package. Original link keeps its formal digest and unsupported_construct behavior for reference and operation mappings; existing linking tests remain valid.

