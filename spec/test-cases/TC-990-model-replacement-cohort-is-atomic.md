---
id: TC-990
title: "Model normalization refuses every mixed replacement cohort"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-990: Model normalization refuses every mixed replacement cohort

## Description

Verify FR-081-AC-12: the strict model reader accepts only one coherent
replacement selection and never makes an old or mixed cohort appear current
through compatibility behavior.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_cohort.rs::mixed_cohorts_refuse_without_view`.

1. Start with a coherent replacement selection and a valid complete model
   correspondence; confirm it reaches complete-view admission.
2. In independent cases, pair the replacement definition with one old
   selection, schema, reader contract, accounting contract or identity-domain
   member. Also try the old contract through a compatibility alias and an old
   identity relabelled as a replacement identity.
3. Observe the strict reader's typed disposition and whether any effective
   view was exposed.

## Expected Results

The coherent cohort reaches admission. Every mixed, legacy, aliased or
relabeled case refuses at the selection/strict-reader boundary with a typed
reason and exposes no effective view. No mixed case succeeds merely because
its individual files parse independently.
