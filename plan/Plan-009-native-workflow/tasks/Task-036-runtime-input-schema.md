---
id: Task-036
title: "Publish the native runtime input structural schema"
type: Task
status: in_progress
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-099
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-100
    type: verifies
---
## Scope

Resolve the native-state-input/1 schema follow-up under #27. Cover both envelope
kinds, every closed record and value variant, with real Rust producer/reader
controls using the existing jsonschema dev dependency. Preserve source bytes,
Serde decoding and runtime admission semantics. QUOIN all-set and actual
code/Rust reviews occur at PR readiness; no producer edits or hosted CI.
