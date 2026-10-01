---
id: TC-868
title: "A seeded profile-layering regression fails the gate naming the case and its edge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: verifies
---
# TC-868: A seeded profile-layering regression fails the gate naming the case and its edge

## Description

Verify `xtask refinement layering` end to end over a layering corpus whose
edges and per-side selections follow QSpec's answer to ADR-017 Q-5. The
corpus layout is fixed when that answer lands; this test's oracle is fixed
now.

Scope: FR-345-AC-4.

## Test Procedure

1. Build a test-only layering corpus holding one seeded case whose unit its
   parent side refuses with a typed refusal and its child side admits, and
   control cases that both sides refuse and that the parent prohibits.
2. Run `xtask refinement layering` over it twice.

Each expected result is a literal in the test. Tag the test
`#[trace("TC-868", "FR-345-AC-4")]`.

## Expected Results

- Verdict violation, exit 10; exactly one `regression`, the seeded case,
  naming its unit's `RawSourceRef` and its edge's two definition
  identities; the both-refuse controls `holds`; the parent-prohibits
  control `not applicable`.
- The two reports are byte-equal.
