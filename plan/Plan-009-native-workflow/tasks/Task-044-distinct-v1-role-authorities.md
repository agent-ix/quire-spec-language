---
id: Task-044
title: "Publish distinct version-1 protocol role authorities"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: verifies
---
## Scope

Complete issue #114's producer-corpus correction after Protocol #11 executed
the newly published handoff through its required static linker and found both
authored roles selecting one model authority. Preserve the accepted reader and
handoff contracts while regenerating the owner-produced bytes from corrected
native/model inputs.

## Subtasks

- [x] Give the Provider role its own admitted object/model authority while
  retaining Service as the authority for `Workflow::apply`.
- [x] Regenerate the published version-1 offer, selection, original inputs,
  dependency bytes, external reference and complete checksum inventory with the
  existing Rust producer.
- [x] Prove the regenerated offer admits through the strict public reader and
  its two authored role records carry distinct exact model coordinates.
- [x] Preserve the version-2 handoff, frozen synthetic compatibility fixture,
  embedded native resources and tags unchanged.

## Delivery

`PUBLISHED_V1_HANDOFF` continues to expose the same versioned handoff contract.
Its regenerated `Flow` declaration retains `Service` and `Provider` as distinct
model authorities, allowing Protocol to enforce FR-050 without weakening its
duplicate-authority refusal.
