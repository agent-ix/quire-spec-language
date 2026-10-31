---
id: TC-140
title: "Historical native temporal request/result owner boundary"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: references }
---
# TC-140: Historical native temporal request/result owner boundary

## Description

Historical M-6d test case for the deleted
`protocol_artifact::native_temporal::{request,result}` request/result path.
TC-140 and its former implementation test `tests/it/native_temporal_owner.rs`
are retiring with that carrier. They do not verify the Layer-4 successor
obligation [FR-052](../functional/FR-052-publish-native-temporal-evaluation-owner.md);
that behavior is assigned to [TC-144](./TC-144-checked-package-temporal-evaluation-owner.md).

## Test Procedure

Retired. The former procedure constructed and round-tripped native temporal
request/result documents. It is retained here only as historical context and
is not a current verification procedure.

## Expected Results

No current acceptance evidence is claimed by TC-140. The Layer-4 evaluation
owner obligations remain active under FR-052-AC-9 through FR-052-AC-12 and are
verified by TC-144 when implemented.
