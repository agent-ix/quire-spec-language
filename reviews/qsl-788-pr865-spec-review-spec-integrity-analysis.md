---
id: SR-11259
title: "Spec review (integrity) — QSL-788 FR-070/FR-106 native forms (PR 865)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ade0c9373be87d02d260bad8ef5006759e4372ef; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-056-admit-domain-package-model-declarations.md (lines 236-314)"
review_set: subset
---

## Summary

Ticket: QSL-788. This is a spec-integrity review of the PR's spec edits and of the document forms the code adds. The brief says FR-106 was edited, but it was not: the only spec file this PR's own commits touch is FR-070 (commit 7a33368df), which gains two rows. FR-106 is unchanged. The oracles are FR-056's closed wire objects (lines 236-240 and 274-281) and QSpec FR-181's typed canonical value list, read from the private repository and cited by id only.

## Verdict

**Two blocking findings, both spec contradictions.** Neither edit was weakened by changes to existing tests. No test that enumerates the closed value grammar or the `RawNode` forms was changed or removed, and no existing test sends a `kind` object.

The two parts belong to different tickets:

- **QSL-791** (planner ruling: a spec author goes first and the canonical spelling is flagged): the snapshot and invocation document forms `{"kind":"uuid","text":..}` and `{"kind":"timestamp","nanoseconds":..}`. These are `RawNode::Uuid` and `RawNode::Timestamp`, `SnapshotValue::uuid` and `SnapshotValue::timestamp`, and the changes to tree.rs, encode.rs, `admit_leaf` and `field_kind_matches`, together with the three tests in observation_composites.rs.
- **QSL-788**: the replay witness path, which is `WitnessValue::Uuid` and `WitnessValue::Timestamp`, value_text, `witness_type`, the converter, and `NativePayload`. It also includes the FR-070 rows those need.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The code adds two snapshot and invocation value forms that FR-106 excludes. FR-106 lines 124-127 say a value is exactly one of boolean, integer, absent, present, reference, sequence or record. The PR now admits `{"kind":"uuid","text":..}` and `{"kind":"timestamp","nanoseconds":..}` as leaves, and encodes them, in tree.rs, encode.rs and `admit_leaf`. FR-106 was not edited, although the coder reports that it was. Under the QSL-791 ruling this part needs the FR-106 form authored first, with the canonical spelling taken to the planner. Fix: land the FR-106 form edit (QSL-791) with this code, or move the document-path code and its three tests to the QSL-791 change. | spec/functional/FR-106-admit-snapshots-and-invocations.md:124-127; qsl-semantics/src/model/observation/tree.rs:201-217 |
| FND-002 | high | The new FR-070 UUID and Timestamp rows contradict FR-070's own definition of value text and give FR-056's inputs a second spelling. FR-070 lines 127-128 define the value text as "QSpec FR-181's typed canonical form, limited to the shapes below". QSpec FR-181 has no UUID or Timestamp form, so these two shapes are QSL-invented while the text says they are FR-181's. They also differ from FR-056's closed wire objects (`kind`/`text` and `kind`/`nanoseconds`, lines 239-240 and 274), which leaves two spellings for the same runtime input. Fix: either take the shapes to the planner for QSpec FR-181 (as with QSL-791), or say in FR-070 that these two rows are a QSL extension outside FR-181 and why they are not FR-056's closed object. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:127-128,149-150 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 1359be715 | The document forms were removed, so the code no longer exceeds FR-106. The forms move to QSL-791 / PR 866, which spells them `{"kind":"uuid","text":..}` and `{"kind":"timestamp","nanoseconds":..}`, the same as FR-056. |
| FND-002 | fixed 1359be715 | The FR-070 rows now use FR-056's closed wire objects as the one spelling, say they are not QSpec FR-181 forms, and link FR-056. The codec writes and reads exactly those objects. |
