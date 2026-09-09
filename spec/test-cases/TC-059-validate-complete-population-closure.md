---
id: TC-059
title: "Validate complete population closure"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-059: Validate complete population closure

## Description

Integration, priority P1. Verifies FR-007-AC-1, FR-007-AC-2, FR-007-AC-8, FR-007-AC-9. Qualified at 45ed1b4 by SR-097 through the public runtime validation tests. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Validate complete/missing/incomplete populations containing absent optional parents, dangling parents, wrong target model/type/universe, duplicate keys, cycles and equal-valued distinct keys. Include empty and Unicode object keys at their authored carrier maximum. Make unavailable populations contain independently malformed supplied fields. Exercise references through structural records, skipped fields and unused invocation inputs.

## Expected Results

Complete dangling targets refuse; incomplete/missing required populations are incomplete, without invented dangling evidence. Known invalid supplied data remains diagnosed. Equal-valued objects retain separate identity; object cycles terminate validation. Key content follows exact model text bounds with no normalization or extra nonempty restriction.
