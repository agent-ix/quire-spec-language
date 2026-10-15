---
id: SR-11257
title: "Code review (Rust lane) — QSL-788 UUID/Timestamp runtime inputs (PR 865)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ade0c9373be87d02d260bad8ef5006759e4372ef; diff vs 9d1674069 (stack base content): qsl-replay/src/execute.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/composite_parity.rs, qsl-replay/src/outcome.rs, qsl-replay/src/witness/value.rs, qsl-replay/src/witness/value_text.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/encode.rs, qsl-semantics/src/model/observation/tree.rs, tests"
review_set: subset
---

## Summary

Ticket: QSL-788. Code review with the rust-review lane folded in, over PR 865's own commits (203752e65..ade0c9373); the stacked base is open PR 862. No builds run (brief). Oracles: FR-056 lines 236-314 (closed wire objects, adapter mapping), FR-056-AC-35, TC-1028 E8, and quire-exact origin/main `src/value/native.rs` (the locked rev d18927e8 is the same commit).

## Verdict

**One blocking finding.** The replay path is correct: `witness_type` maps UUID and Timestamp to `Canonical`, `Converter` builds them through `Uuid::from_canonical_text`/`Timestamp::from_canonical_text`, and `ReplayRefusal::NativePayload` reports `invalid_runtime_input`/`invalid-value` with the parameter index and the typed kernel cause. Every edge case in the tests matches quire-exact's rules: empty text, leading whitespace, uppercase, `-0`, `+1`, `01`, `-01`, fraction, exponent and RFC 3339 text are noncanonical, and the endpoint +/-1 values are out of domain. The tests assert typed fields (parameter, code, cause, kernel class), so a stub that always admitted or always refused would fail them. Every exhaustive match on `WitnessValue`, `RawNode`, `RawForm` and `ReplayRefusal` names the new variants. The search found no wildcard arm that swallows one (`settle` is `#[deny(clippy::wildcard_enum_match_arm)]`, and `outcome.rs`, `composite_parity.rs`, `encode.rs`, `collect_references` and `normalize` are updated by name). Non-test code has no new unwrap or panic.

The blocking finding is the document reader's envelope handling. Recorded, not blocking: `value_parity` sends `NativePayload` to `Refused` rather than `RefusedInput`, because `input_refusal()` returns None for it. The code is still `invalid_runtime_input`, so the result is consistent but differs from the composite-parity routing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The snapshot/invocation reader refuses every FR-056 envelope fault as `wrong-value-kind`. FR-056 lines 293-298 require `missing-member`, `unknown-member` and `duplicate-member` for envelope member faults and `invalid-value` for an invalid tag. In `read_raw_value`, an object with a string `kind` returns `Ok(None)` on an unknown kind (`{"kind":"guid","text":..}`), an extra or missing member (`keys().count() != 2`, `{"kind":"uuid"}`) or a wrong payload member name. Its callers map `None` to `wrong_kind(name)`, which gives the wrong cause. Only the non-string payload case (`"nanoseconds":0`) is correctly `wrong-value-kind`. The coder disclosed this. Fix: return typed member and tag refusals from this branch and add one test per cause. Alternatively, split it to a ticket with the FR-106 form decision (QSL-791). | qsl-semantics/src/model/observation/tree.rs:201-217 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | The document reader was removed from this PR in 1359be715. Its code is on branch uuid-timestamp-document-form, and the spec is QSL-791 / PR 866. Whoever lands that branch must map envelope faults to FR-056's causes. Re-checked at 2db8accaa: no qsl-semantics diff and no RawNode/RawForm/SnapshotValue Uuid or Timestamp references remain. |
