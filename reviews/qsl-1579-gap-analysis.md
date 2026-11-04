---
id: SR-20261104002157
title: "Gap analysis — QSL-1579 RoleSlot model read-back (PR #2226)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9605b39e1; PR #2226 diff: qsl-package/src/protocol_occurrence.rs, qsl-package/src/emit/tests/protocol_occurrence.rs, tests/it/composed_domain_models.rs; FR-384 Outputs and AC-1, AC-4, AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-384
    type: reviews
---

## Summary

Ticket: QSL-1579. Plan completion: not assessed. The scope is the PR diff only. The PR is explicitly partial: the Port key cannot reach the wire until IR-1310, and Linear records IR-1310 as blocking QSL-1579. The new production behaviour is `RoleSlot.model`. FR-384 owns the occurrence schema, but its Outputs list for `roles` does not name a model, and the new test is tagged FR-384-AC-1, whose text says nothing about role model types. The two `v`→`p` fixes restore tests tagged FR-384-AC-4 and FR-384-AC-5 without changing what they assert. Quoin 0.29.1. The trace was checked by reading the tags.

## Verdict

**CONDITIONAL.** Two medium findings. The code is real and has tests, with no stubs and no inflated coverage. But the spec does not describe the new field, and merging as linked would close the partial ticket.

## Scope

- FR-384 Outputs, `roles` bullet: examined against `RoleSlot`.
- `FR-384-AC-1`: examined. Its text covers slot count, ordering and owners, not role model types.
- `FR-384-AC-4`, `FR-384-AC-5`: examined. The profile fix leaves their assertions unchanged.
- QSL-1579 acceptance (ticket text, treated as data) and its Linear PR attachment: examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The code goes beyond the spec. FR-384 Outputs says each role slot holds "the role node's key, its declaration path and its replication". `RoleSlot` now also holds `model`, and no FR-384 AC states it. The new test `role_slots_carry_their_selected_model_type_through_read_back` is tagged `FR-384-AC-1`, but AC-1 does not mention model types, so the model assertions trace to no criterion. Fix: add "its selected model type node (the Port's owning object type until the role record carries a member)" to the Outputs `roles` bullet. Then either extend AC-1 with the W role→type expectations (Fulfillment→Shipment, Orders→OrderView, Payments→Payment) or add a new AC and retag the test. | spec/functional/FR-384-derive-the-protocol-occurrence-schema-and-runtime-binding-requirements.md:66-69; qsl-package/src/protocol_occurrence.rs:209-213; qsl-package/src/emit/tests/protocol_occurrence.rs:202 |
| FND-002 | medium | Linear links PR #2226 to QSL-1579 with `linkKind: closes`, so merging closes the ticket. The PR body calls itself partial, and none of the ticket's Port-identity acceptance items are met; they wait on IR-1310. Fix: before merging, make the link non-closing, for example with "Part of QSL-1579" in the PR body or by changing the link in Linear. Otherwise, reopen QSL-1579 right after merge. | QSL-1579 Linear attachment (PR #2226) |
