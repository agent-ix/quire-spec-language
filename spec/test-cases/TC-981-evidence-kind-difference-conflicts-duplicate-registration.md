---
id: TC-981
title: "An evidence-kind difference conflicts a repeated backend identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-981: An evidence-kind difference conflicts a repeated backend identity

## Description

Verify complete descriptor equality includes the evidence-kind set. Scope:
FR-075-AC-7 and FR-075-AC-13.

## Test Procedure

1. Register backend `p` advertising `probabilistic-satisfaction` with
   `{statistical}`.
2. Repeat the same identity and advertised pair with `{exact}` instead.
3. Inspect the registry, its duplicate-backend refusal, and candidates for
   probabilistic items naming `statistical` and `exact`.

## Expected Results

- The descriptors are unequal because their evidence-kind sets differ; the
  identity is refused with `invalid_capability`/`duplicate-backend` according
  to FR-075-AC-4.
- The identity is withdrawn and is a candidate for neither evidence kind.
- A repeat whose identity, advertised pairs and evidence-kind set are all
  byte-for-byte equal remains idempotent under FR-075-AC-7 and TC-448.
