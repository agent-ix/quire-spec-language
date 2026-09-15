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

Complete issue #112's correction to the accepted #40 handoff claim. Commit the
existing four-source Rust producer output, publish version-explicit directory
and member constants, and prove complete checksums plus public strict-reader
admission. Preserve the separate frozen `/1`-to-`/2` compatibility fixture and
the existing `/2` corpus unchanged.

## Subtasks

- [x] Generate the established `/1` corpus through the release Rust producer.
- [x] Publish version-explicit owner addresses and the shared `Selection` type.
- [x] Check every member, path, digest and the external package reference.
- [x] Reconstruct the selected model and admit the committed bytes through the
  public `/1` reader.
- [x] Reconcile #40, #112, Plan-009 and the Protocol #11 dependency blocker.

## Delivery

`PUBLISHED_V1_HANDOFF` and its member constants address one immutable
four-source handoff from the pinned crate. The producer writes the complete
checksum inventory in Rust. TC-121 validates its closed inventory and replays
the committed offer through the public strict reader; the independently selected
Protocol intake and historical two-revision comparison remain in Protocol #11.
