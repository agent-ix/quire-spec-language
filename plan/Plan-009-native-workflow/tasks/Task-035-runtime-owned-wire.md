---
id: Task-035
title: "Own native runtime wire decoding at type boundaries"
type: Task
status: done
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

Implemented at 3c6a0e6. Three new public-API tests cover eleven records and all
ten value variants; two tests reproduced direct positional-array acceptance
before the fix. Local suites pass 353/337 ordinary tests with all/minimal features,
plus three compile-fail doctests in each. PR-readiness reviews are SR-265–274;
matrix status verification remains limited by #28. This completes Task-035's
decoder slice, not the broader #27 inventory or native-profile reconciliation.
