---
id: SR-16716
title: "Code review of QSL-1063 EN-1 footprint enumeration"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1458 diff against merged main: qsl-eval/src/simulation/model/cells.rs, qsl-eval/src/simulation/protocol/access.rs, qsl-eval/src/simulation/protocol.rs, qsl-analyze/tests/it/weak_memory.rs"
review_set: subset
---

## Summary

Ticket: QSL-1063, the first ordered implementation slice under QSL-1049. The change derives enabled EN-1 footprints by applying each attempt with its selected effective preconditions, deduplicates footprints by transition identity, retains guard-agnostic potential footprints for memory backing, and makes the FR-220-AC-15 fixture use a real `Cell z` excluded by its immutable id guard. The successor issues own state-local thread observations, attempt-constraint filtering, refusal reporting, and dynamic fork/runtime integration.

The review covers the four changed files listed in `scope`. No workflow files changed. No applicable `AssuranceProfile` was found under `spec/assurance` or `spec/`. The behavior follows ADR-025 MA-1 and the EN-1 distinction between potential and enabled footprints in FR-220.

## Verdict

**PASS.** No actionable findings in this slice. Potential footprints are used only to reserve memory backing; V-8 shape checks use the enabled set. The test exercises the real `Cell` fixture and retains the FR-220-AC-15 trace tag.

The full QSL-1049 behavior remains incomplete by design and is tracked in ordered successor issues; this verdict applies only to QSL-1063.

Commands run:

- `cargo fmt --check` — passed.
- `cargo clippy -p qsl-eval -p qsl-analyze --all-targets -- -D warnings` — passed.
- `cargo test -p qsl-analyze --test it weak_memory::` — 32 passed, 0 failed.
- `git diff --check` — passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder). | Scoped files listed above |
