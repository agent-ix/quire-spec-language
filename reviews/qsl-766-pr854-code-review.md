---
id: SR-11087
title: "QSL-766 quire-spec-language PR 854 code review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@00aefc8ba60d0dd4e612eee38c387caab9ce1b7f; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/emit/tests/envelope_adoption.rs"
review_set: subset
---

## Summary

Ticket: QSL-766. One commit on `main`. `emit_package_inner` now builds the
`quire.checked-package/v2` envelope with IR's
`quire_contract_model::encode_checked_package_v2`. The local `WireV2`
struct, its `Encode` impl and `encode_array` are deleted, along with the
`quire_canonical::{Encode, Sink, Writer}` import and the `CHECKED_PACKAGE_V2`
import. There is no wrapper and no second path.

Byte equality (checked by reading the code; nothing was built here). IR's
encoder at the locked rev f7dd613 writes the same eight members:
`contract_version` (= `CHECKED_PACKAGE_V2`), `identity_preimage`,
`package_id`, `lock`, `semantic_graph`, `source_map`, `capability_report`,
`diagnostics`. The two writers differ in three places, and none of them
changes the output:
- The nested members (preimage, graph, diagnostics) go through the same
  `Encode` impls in both versions.
- The fixed members go through `writer.serialize`. The old code called
  `encode_into`, and quire-canonical 5dc4e12 `lib.rs:170` has a blanket
  `impl<T: FixedShape> Encode for T` that calls the same serializer.
- The two slices are serialized as serde sequences, where the old code
  wrote begin_array plus one encode per item. Both produce the same array
  events.

The writer sorts object members (RFC 8785), so member order cannot matter
anyway. Both versions call `quire_canonical::to_vec` with
`crate::checked::EMIT_LIMITS`.

The encoder takes IR-owned typed values by reference. No JSON is built by
hand. The closure borrows locals, as before, and returns
`Result<Vec<u8>, quire_canonical::Error>` mapped through the existing
`EmitRefusal::from`, so the error path is typed and unchanged.

Rust lane: no new `unwrap`, `expect`, `panic`, indexing or integer
conversion in src. `quire-canonical` is still used by `checked.rs`,
`checked_v2.rs` and `emit.rs` (the error type), so no dependency became
unused. `quire-contract-model` was already a normal dependency, so TC-390
layering is unaffected. The FR-284 direction check passes in the gate log. No
golden or fixture changed in the diff. `git grep WireV2` finds only a
historical review record (`reviews/qsl-480b-code-review.md`).

Gate: `/home/peter/dev/worktrees/logs/qsl766-ci.log` (written 06:04 PDT,
after the 04:19 PDT commit) ends with EXIT=0 and has no `test result:
FAILED`. The new test ran and passed. Its four `error:` lines are the
negative-test fixture outputs, and the same lines appear in the green
`qsl774-final-ci.log`.

## Verdict

PASS. The emitted bytes are unchanged by construction, the local envelope
type is gone, and nothing in the diff panics or leaks. One low finding is
recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The adoption scan skips a directory named `extent_agreement`, but no such directory exists. `emit/extent_agreement.rs` is a file and is scanned. The skip clause does nothing. | qsl-package/src/emit/tests/envelope_adoption.rs:15 |
