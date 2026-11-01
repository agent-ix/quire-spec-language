---
id: SR-1988010
title: "Code review — QSL-1426 protocol compensation enum identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it/protocol_compensation.rs, qsl-replay/src/witness/value_text.rs, qsl-replay/src/id_text.rs, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md"
review_set: subset
---

## Summary

Reviewed QSL-1426 PR #2005 at `06579d7518ad41977175341859f74eab70bb986e`. The assertion now matches FR-120's canonical declaration identity text and verifies it names the expected enum declaration.

## Verdict

**PASS** — the corrected assertion checks the current wire contract without weakening coverage of the enum's declaration identity or member values.

## Examined scope

- `spec/functional/FR-120-simulate-a-checked-package-s-state-family.md`: declaration identity text and enum value encoding.
- `qsl-analyze/tests/it/protocol_compensation.rs`: the existing `a_string_attempt_field_requires_a_bound_and_an_enum_one_is_named` integration test and changed assertion.
- `qsl-replay/src/witness/value_text.rs`: enum `name` is emitted as `node_text(declaration)`.
- `qsl-replay/src/id_text.rs`: `node_text` delegates to the shared `node_key_to_text`; `node_from_text` parses the same wire form.

## Evidence

The focused test passed 1/1 on this review head. The coder also reports `cargo fmt --check`, all-target `qsl-analyze` Clippy with `-D warnings`, and `git diff --check` passing. No broad suite was run for this test-only correction.

## Findings

| ID      | Severity | Summary                    | Refs |
| ------- | -------- | -------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)  | -    |
