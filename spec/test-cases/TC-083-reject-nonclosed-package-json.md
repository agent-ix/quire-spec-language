---
id: TC-083
title: "Reject nonclosed package JSON"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-083: Reject nonclosed package JSON

## Description

Property, priority P1. Verifies FR-020-AC-2. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Starting from a real emitted package, systematically mutate every closed record with an extra field, missing required field, duplicate decoded member or wrong primitive kind. Include escaped duplicate keys, UTF-8 failures, BOM, trailing documents, malformed/overflowed integers, fractions/exponents in integer fields and invalid surrogate escapes. Recompute byte selectors so digest checks do not mask the target defect.

Compile the structural schema in a qualified Rust Draft 2020-12 validator and
check real producer outputs plus each structurally invalid record. Include
prefixItems/items closure for the two projections and exact large integer
limits; the already locked jsonschema crate needs its draft feature selected
and verified before being used for this qualification. Keep lexical integer,
duplicate-key, byte-budget and reconstruction tests on the actual reader: a
generic parsed JSON value has already erased some of those distinctions.

## Expected Results

Every malformed/unknown/duplicate/omitted shape refuses before an accepted package exists. Required null members cannot be omitted. IR-shaped identity/span/anchor records also enforce closure rather than inheriting permissive Deserialize behavior.
