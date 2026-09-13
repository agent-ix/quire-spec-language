---
id: SR-410
title: "Rust review of protocol-role refusal loci"
type: SpecReview
analysis: code-review
scope: "a7911875 refusal-locus hunk; src/protocol_artifact/native/runtime.rs; tests/native_protocol_emission.rs; FR-042-AC-8; TC-121"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

The PR-time Rust review found no defect in the issue #68 slice. The implementation establishes the
complete role-owned locus before either fallible model/type lookup, and the integration regression
reaches the real native admission API and checks the typed refusal, source index, and exact source
slice independently.

## Verdict

**PASS** — no Rust, test, safety, boundary, resource, or gate-integrity finding remains in the
reviewed slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | a7911875; runtime.rs:581; native_protocol_emission.rs:332 |

## Review evidence

The review isolated the five-line refusal-locus change in `a7911875`; the same commit's unrelated
state-evaluator cleanup was not attributed to issue #68. The new assignment is private,
single-threaded diagnostic state and introduces no API, unsafe code, integer conversion, panic,
allocation, loop, wire field, or error variant. Its `layout.locus(role.span)?` failure remains a
typed refusal, and every later fallible role-admission operation sees the current role's complete
source/span pair. No workflow file, lint level, `resources/native-v1/` content, tag, or dependency
pin changed.

The test is deterministic, has the repository-required `#[trace]` tags, uses two distinct source
units, and would fail if the old stale source index returned with the current role span. Its
assertions are unconditional after explicit success/failure extraction; it uses no double,
source-text inspection, clock, environment, network, or hosted service.

Local gates used the owner-required `--target-dir target-codex-backends`, `-j 1`, and
`--test-threads=1` where applicable: formatting and Clippy passed; the focused regression passed;
the complete no-default-features suite and doctests passed with only three named, pre-existing
private-packet tests ignored; the no-default-features build, fixture-audit self-test, and model-byte
audit passed.
