---
id: Task-043
title: "Publish the compiled-protocol v1 consumer handoff"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: verifies
---
## Scope

Complete issue #112's correction to the accepted #40 handoff claim. Publish
version-explicit directory and member constants, and verify public strict-reader
admission of producer output. Keep the `/1` reader behavior and `/2` corpus
behavior independently covered.

## Subtasks

- [x] Generate the established `/1` corpus through the release Rust producer.
- [x] Publish version-explicit owner addresses and the shared `Selection` type.
- [x] Check every member, path and external package reference.
- [x] Reconstruct the selected model and admit the committed bytes through the
  public `/1` reader.
- [x] Reconcile #40, #112, Plan-009 and the Protocol #11 dependency blocker.

## Delivery

`PUBLISHED_V1_HANDOFF` and its member constants address the four-source
handoff from the producer crate. TC-121 validates its closed member inventory
and replays the offer through the public strict reader; the independently
selected Protocol intake and historical comparison remain in Protocol #11.
The first Protocol static-link execution exposed duplicate role authorities in
the published corpus; Task-044/#114 owns and completes that corpus correction
without changing this publication boundary.
