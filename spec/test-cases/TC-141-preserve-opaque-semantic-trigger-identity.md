---
id: TC-141
title: "Historical native temporal v2 trigger identity boundary"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-053, type: references }
---
# TC-141: Historical native temporal v2 trigger identity boundary

## Description

Historical M-6d test case for opaque trigger identity in the deleted native
temporal request/result path. TC-141 and its former implementation test
`tests/it/native_temporal_owner.rs` are retiring with that carrier. They do not
verify the Layer-4 successor obligation
[FR-053](../functional/FR-053-preserve-opaque-semantic-trigger-identity.md);
that behavior is assigned to
[TC-149](./TC-149-checked-package-opaque-trigger-identity.md).

## Test Procedure

Retired. The former procedure submitted trigger bytes to the versioned native
temporal request/result API. It is retained here only as historical context
and is not a current verification procedure.

## Expected Results

No current acceptance evidence is claimed by TC-141. The checked-package
trigger identity obligations remain active under FR-053-AC-7 through
FR-053-AC-9 and are verified by TC-149 when implemented.
