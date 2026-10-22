---
id: TC-974
title: "Statistical evidence kind is admitted for probabilistic backends"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-974: Statistical evidence kind is admitted for probabilistic backends

## Description

Verify a probabilistic backend may advertise the exact `statistical` evidence
kind and that descriptor admission retains it. Scope: FR-075-AC-10.

## Test Procedure

Pass a descriptor advertising `probabilistic-satisfaction` and the singleton
evidence-kind set `{statistical}` to `descriptor_from_manifest`; inspect the
admitted descriptor.

## Expected Results

Admission succeeds. The descriptor retains exactly `{statistical}` for the
advertised probabilistic capability.
