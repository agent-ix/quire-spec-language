---
id: TC-041
title: "Refuse missing or inconsistent native model roles"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-041: Refuse missing or inconsistent native model roles

## Description

Integration, priority P1. Verifies FR-015-AC-2. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Starting from the valid producer, independently remove a scalar site, repeat a site/role, change one site's integer representation, select a nonprimitive site, use unsigned/saturating/rational formal data, exceed the authored text ceiling, supply an invalid object record/reference enum, or break an operation parameter/result/frame target. Include unused declarations and duplicate frame entries. Construct IR inputs successfully before testing native-only adverse cases.

## Expected Results

Each invalid model returns the specified invalid_model_binding or unsupported_construct diagnostic with no NativeModel. IR constructor failures cannot replace the native refusal assertions. A valid bounded text role including maximum zero is accepted; missing text role is refused. Reusing valid source/roles after each failure succeeds.

