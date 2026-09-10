---
id: Task-035
title: "Own native runtime wire decoding at type boundaries"
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

Resolve PR #16's decoder-ownership and digest-codec findings under issue #27.
Preserve native-state-input/1 bytes and the public draft field types. Replace
per-field object adapters with closed type-owned decoding, retain validated
foreign identifiers, and test direct/nested records and required nullable results.
The standalone runtime-input schema and upstream IR resource classification
remain separate follow-ups. Run QUOIN all-set and code/Rust reviews at PR readiness.
