---
id: SR-2026111216593801
title: "code review of PR2692 protocol analysis key, premises and witness record amendment"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/protocol_analysis.rs, qsl-replay/src/result.rs, qsl-replay/src/result/wire.rs, qsl-replay/src/execute/state_clause.rs, qsl-package/src/protocol_view.rs and their tests; PR2692 e2a6140cf9aac9c7c6a8d8f9ab8a5d0dc9fd5fdd"
review_set: subset
---

## Summary

Ticket: QSL-1894 (ADR-036 slice 2). Code review with the rust-review lane folded in. The key builder and reader refuse every top-level absent member as `invalid_runtime_input`/`missing-member`, `None` and `Some(empty)` are distinct present values, there is no digest, and root and run limits are not key fields. The witness record amendment keeps state-clause records unchanged. A bare JSON string is an unambiguous protocol element because every kernel element encoding is a JSON object with a `kind`. The node-id spelling is not canonical, premise node ids are untyped strings, and `from_view` swallows one refusal and has no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `node_text` says it returns the canonical JSON object of a `CheckedNodeId`, but it uses `serde_json::to_string`. That writes members in declaration order (`package`, `path`, `ordinal`), which is neither RFC 8785 order (`ordinal`, `package`, `path`) nor the spelling this repo already uses for node ids (`qsl_foundation::node_wire::node_key_to_text`, used by `OccurrenceWire.node_id`). quire-protocol FR-302-AC-3 copies the deciding element byte-exactly, so the form must be fixed before slice 3 emits it. Use `quire_canonical::to_vec(id)`. | qsl-replay/src/protocol_analysis.rs:243-251 |
| FND-002 | medium | The premise node members (`ChannelPremise` channel, sender, receiver and message_type, `ChoicePremise`, `OrderingPremise::Fifo`) are `String`s of JSON text, not typed `CheckedNodeId`s (FR-572-AC-3 requires typed values). Sorting therefore runs lexicographically over JSON text, not by `CheckedNodeId` order (PA-15), so ordinal 10 sorts before ordinal 9. | qsl-replay/src/protocol_analysis.rs:86-205,276-305 |
| FND-003 | medium | `from_view` skips any control where `control_operation(node) != Ok(Choice)`, so a control whose operation read refuses is silently dropped from premises group (b). The key changes instead of the call refusing. Propagate the error with `?` and skip only `Ok(non-choice)`. | qsl-replay/src/protocol_analysis.rs:283-289 |
| FND-004 | medium | `ProtocolPremises::from_view`, a slice-2 deliverable ("premises (a) and (b) built from protocol_view"), and every `PremiseRefusal` variant have no test. Add a test over a real package (the FR-517 **R** fixture has FIFO channel `Charges`, its queue and choice `Outcome`) plus the OrphanQueue and ChannelWithoutQueue cases. | qsl-replay/src/protocol_analysis.rs:254-306 |
| FND-005 | low | `from_json` refuses a present but unknown or non-string `kind` as `missing-member` naming `kind`, which is the wrong cause. It also accepts and discards `bounds.max_history` on a projection key, so a stored projection key that carries it reads equal to one that does not. | qsl-replay/src/protocol_analysis.rs:529-544,456 |
| FND-006 | medium | A nested premise with a missing field, such as a channel premise without `delivery`, fails as `KeyReadError::Malformed`, a serde error with no catalog code. A nested bounds member (`max_depth`, `max_history`) is refused as `missing-member`. FR-572 Outputs lists the FR-054 channel members as key content, and PA-13 says a reader refuses an absent member "the same way", so refuse it as `missing-member` naming the member path. At minimum, give `Malformed` a catalog code. | qsl-replay/src/protocol_analysis.rs:47-56,86-205 |
| FND-007 | low | `read_deciding` accepts any JSON string as a protocol element (the test uses `"node"`). FR-574-AC-3 makes it the canonical string of a checked node id, so the reader should refuse a string that does not read back as a `CheckedNodeId` and re-encode to the same bytes, as `OccurrenceWire::read` does for `node_id`. | qsl-replay/src/result/wire.rs:229-237 |
| FND-008 | low | `RecordWire::of` writes a protocol record, but the `native-run-result/2` reader (`validate_state`, `read_state`) refuses it with `Shape("quantifier")`, so the writer can emit what its own reader refuses. Both stay state-only until slice 3 (QSL-1895) carries the FR-574 record. QSL-1895 should name that reader extension. The new `separate()` branch for a protocol record (`Failed(Element, Unmet)`) is untested. | qsl-replay/src/result/wire.rs:795-880; qsl-replay/src/execute/state_clause.rs:551-559 |

## Scope and evidence

The review covered the diff `origin/main...HEAD` at e2a6140cf only. The rust-review lane found no `unsafe`, no new panics in library code, no unbounded recursion, and no Tokio. Every new public item has docs. `#[must_use]` is on the code and cause accessors. The roughly 61 `Some(..)`/`DecidingElement::Value(..)` reference updates are mechanical and correct, and `witness_member.rs` uses `.value()`. The new tests reference new API (`DecidingElement`, `ProtocolAnalysisKey`, `channel_ordering`, `queue_overflow`), so they cannot pass without the code.

Wire ambiguity check: `encode_element` and `validate_element_shape` give every kernel element an object form with `kind` (ScalarWire and option/composite/collection), so `Node::String` at the deciding-element slot cannot be a kernel value. The protocol branch of the cause codec refuses a present quantifier, a present index and an absent trace_position. The kernel branch still refuses an absent index or quantifier, and the state record round-trips byte-equal.

Recommended canonical node-id form: the RFC 8785 canonical text (`quire_canonical::to_vec`) of the `CheckedNodeId` V2 wire object. That is the same object the checked package carries, with members in `ordinal`, `package`, `path` order. Pin it in ADR-036 PA-24 and FR-574. quire-protocol (FR-302-AC-1/3 and FR-152, origin/main 56baf23) never parses the string. It copies it byte-exactly and joins to the graph through retained source identities, so QSL alone fixes the bytes.

- FR-572 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): Every projection and realizability result SHALL carry the structural key of QSpec FR-174-AC-6 or FR-175-AC-6 as typed members, SHALL never stand in a digest for a member, and SHALL be a function of its key. (examined)
- FR-572-AC-3 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): The result carries every key member as a typed value, and the key holds no digest. (examined)
- FR-572-AC-4 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): For each key member in turn, building a key without it, and reading a stored result with it removed, are each refused invalid_runtime_input/missing-member naming that member. (examined)
- FR-574 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): The QSL record type is amended by ADR-036 PA-24; a state-clause record shall still round-trip unchanged. (examined)
- FR-519-AC-21 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): channel_ordering and fifo_key over Charges and an unordered channel. (examined)
- FR-519-AC-22 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): queue_overflow and queue_channel. (examined)
