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
FR-075-AC-7 and FR-075-AC-14.

## Test Procedure

1. Register backend `p` with a fixed origin and manifest digest, advertising
   `probabilistic-satisfaction` with `{statistical}` and a fixed valid
   `domains` value.
2. Repeat the same identity, origin, digest, advertised pair, and `domains`,
   changing only the evidence-kind set to `{exact}`.
3. Inspect the registry, its duplicate-backend refusal, and candidates for
   probabilistic items naming `statistical` and `exact`.

## Expected Results

- The descriptors are unequal because their evidence-kind sets differ; the
  identity is refused with `invalid_capability`/`duplicate-backend` according
  to FR-075-AC-4.
- The identity is withdrawn and is a candidate for neither evidence kind.
- A repeat whose every descriptor member is equal remains idempotent under
  FR-075-AC-7 and TC-448.
