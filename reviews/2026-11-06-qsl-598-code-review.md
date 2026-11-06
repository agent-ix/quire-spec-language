---
id: SR-20261105101076
title: "Code and Rust review of QSL-598 ToolFailure terminal records"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-598; PR #2464 head fffc763d6; qsl-replay/src/proof_result/tool_failure.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/outcome.rs, qsl-replay/src/statistical/settle.rs, qsl-replay/src/model_check/step_safety.rs, qsl-replay/src/lib.rs, qsl-replay/tests/it/tool_failure.rs, qsl-replay/tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: reviews
---

## Summary

Ticket: QSL-598. Code review with the rust-review lane folded in, scoped to the one commit PR #2464 adds (fffc763d6) over origin/main.

## Verdict

**MERGEABLE (code).** No high or medium code defects. The fourteen `ToolFailure` variants and their `defect` spellings match QSpec FR-462's failure table exactly, and only `FrameLength` and `TotalBytes` carry `limit`/`value`. `TerminalValue::ToolFailed` maps to `Category::InternalFailure` (exit 30) and `ItemLabel::Failed`. The exhaustive matches are updated honestly: outcome.rs (label, cause, value/measured arms), statistical/settle.rs (not a statistical verdict, `None`), model_check/step_safety.rs (`OtherBackendResult::Failed`, same as plain `Failed`), proof_result.rs (category, measured bytes, other arms). No wildcard match absorbs the new variant. No stubs. FR-293 is retired and nothing was built for it, which is correct. Three low findings, none blocking.

## Scope

- FR-291 (examined): typed `ToolFailure` cause, serialization, category, sibling independence.
- FR-291-AC-1 (examined): fourteen variants, cause shape, exit 30, read-back.
- FR-291-AC-2 (examined): `TimedOut` and cancelled are incomplete, exit 22.
- FR-291-AC-3 (examined): a plugin failure leaves a sibling record unchanged, outcome exits 30.
- FR-293 (context_only): retired; confirmed nothing is built for it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ItemCause` now has two fields that both serialize as JSON `limit` (`limit: Option<&'static str>` and `byte_limit: Option<u64>` renamed); only a doc comment keeps them apart, so a future cause that sets both writes a duplicate key. `defect` is also a `&'static str` where `ToolFailure` itself could be held | qsl-replay/src/outcome.rs:358-366 |
| FND-002 | low | `ToolFailure::read_cause` doc says it reads "exactly FR-331's `tool-failure` cause", but it reads the outcome document's `kind` discriminator; an FR-331 wire cause (`type`) returns `None` | qsl-replay/src/proof_result/tool_failure.rs:82-87 |
| FND-003 | low | AC-3 test overwrites `request_index` with an identical value (a no-op), compares `serde_json::Value`s rather than bytes, and builds the `proved` sibling with the same plugin backend `acme` instead of a compile-time provider | qsl-replay/tests/it/tool_failure.rs:111-121 |

## Evidence and limits

Built only through locked-build.sh with the shared target, jobs 4, debug 0, after touching the changed .rs files. `cargo test -p qsl-replay --test it tool_failure`: 3 passed. `cargo clippy -p qsl-replay --all-targets -- -D warnings`: clean (the coder had not run clippy). `cargo check -p qsl-analyze -p quire-spec-language --all-targets`: clean, so no downstream exhaustive match breaks. No full gate run.
