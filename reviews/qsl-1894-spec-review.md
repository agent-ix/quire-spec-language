---
id: SR-2026111216593803
title: "spec review of PR2692 FR-519, TC-1314 and ADR-031 amendments"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-519-*.md, spec/test-cases/TC-1314-*.md, spec/decisions/ADR-031-*.md; PR2692 e2a6140cf9aac9c7c6a8d8f9ab8a5d0dc9fd5fdd"
review_set: subset
---

## Summary

Ticket: QSL-1894 (ADR-036 slice 2). The review covers the three spec edits with integrity and EARS checks. FR-519 gains four accessors with AC-21 to AC-23, which match ADR-036 PA-3, and AC-23 records the PA-24 decision that no value-path accessor is needed. The ADR-031 SW-5 and SW-7 amendments match PA-24. They depend on a "canonical string" of a node id that no spec defines, and SW-7 says more about the wire than the code does.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ADR-031 SW-5 amendment (like ADR-036 PA-24 and FR-574) uses "the canonical string of a protocol element's checked node id", but no QSL, QSpec or quire-protocol text defines that string. QSpec FR-351 gives the deciding element as "the exact value", and quire-protocol FR-302 copies it byte-exactly. Pin it: the RFC 8785 canonical text of the `CheckedNodeId` V2 wire object. | spec/decisions/ADR-031-state-forall-separating-witness.md:92 |
| FND-002 | low | The ADR-031 SW-7 amendment says "the wire accepts an absent index and an absent quantifier for the protocol element". Only the cause codec does. The `native-run-result/2` reader (`validate_state`, `read_state`) is still state-only until slice 3. Name the cause codec, or say that the /2 reader follows in slice 3. | spec/decisions/ADR-031-state-forall-separating-witness.md:94 |
| FND-003 | low | TC-1314 row 22 omits two checks FR-519-AC-22 requires: `queue_channel`'s `MalformedMember` on a non-reference `channel`, and a selector on `block`. | spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:41 |

## Scope and evidence

FR-519 now lists 22 view functions in "The record-to-node map", and the new paragraph matches the signatures in `protocol_view.rs`. The TC-1314 scope moves to AC-23. EARS: the new ACs are testable and atomic per accessor. ADR-031 SW-5 and SW-7 keep the state-clause text unchanged and append the amendment, as PA-24 asks.

- FR-519-AC-21 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): For R's FIFO channel Charges, channel_ordering returns ChannelOrdering::Fifo and fifo_key returns the message binder node and the key expression node. (examined)
- FR-519-AC-22 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): queue_overflow returns Overflow::Reject, Block and Loss of that selector; queue_channel returns the channel node. (examined)
- FR-519-AC-23 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): The protocol read surface adds no accessor that renders or derives a value path for a protocol node. (examined)
- SW-5 (spec/decisions/ADR-031-state-forall-separating-witness.md): Amended by ADR-036 PA-24: the deciding quantifier is absent for a family with no quantifier occurrence, and the deciding element is either the kernel value or the canonical string of a protocol element's checked node id. (examined)
- SW-7 (spec/decisions/ADR-031-state-forall-separating-witness.md): Amended by ADR-036 PA-24: quantifier is Option, deciding_element is a DecidingElement, and the wire accepts an absent index and quantifier for the protocol element. (examined)
- TC-1314 (spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): rows 21-23. (examined)
