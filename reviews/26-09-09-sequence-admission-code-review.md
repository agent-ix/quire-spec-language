---
id: SR-263
title: "Code and Rust review of sequence declaration admission"
type: SpecReview
analysis: code-review
scope: "Task-034; src/native_model/admission.rs; three new admission tests; TC-065 fixture"
review_set: subset
---
## Summary

Author PR review of implementation fd69a60 using the actual agent-skills
code-review, rust-review, rust-style and implementation-gap discovery workflows.
No required AssuranceProfile, repository-specific Rust idiom override or deny.toml
was found. The final spec wording clarifies existing preflight precedence.

## Verdict

PASS for the scoped sequence-admission implementation. This is author review,
not an independent approval or whole-profile qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No outstanding code defect in this slice. The full run exposed an oversized historical stress declaration; the repaired fixture preserves its logical occurrence count and exact hard-limit assertions. | tests/runtime_validation_cases/limits.rs:499; TC-065 |

## Rust and discovery checks

One private u32 constant and a comparison in the existing bounded type walk own
the rule. The walk visits every declaration and nested wrapper before artifact
publication. The source frontend delegates admission to this same function.
The existing typed Code::UnsupportedConstruct carries the refusal; its displayed
maximum is explanatory, not parsed to choose a result kind. Earlier resource
failures retain their established code. No new cast, panic, unsafe block, I/O,
async work, lock, trait seam, parser, public API or serialization shape is added.

Three public-API tests construct valid IR before judging admission and cover
21 boundary scenarios, including used/unused fields, unused values, inner/outer
nesting, maximum u32 and attempted elevation of every model work limit. Positive
branches inspect admitted types or exact source correspondence; negative branches
require the actual unsupported code, absent incomplete status and original
provenance. JSON failures require ModelSourceCause::Admission. No assertion is
guarded by a field whose absence would skip its verification. Real trace
attributes bind TC-041/FR-015-AC-2 and TC-102/FR-025-AC-3.

Removing the gate was observed to fail all three new tests before implementation:
the old code admitted maximum 10,001. TC-065 retains exactly 40,000 logical text
occurrences through two 200-wide levels and still asserts Unicode 8,388,608 and
work 1,000,000. No runtime threshold was relaxed. Reverse discovery maps the new
constraint/refusal to FR-015 and the fixture topology to TC-065; no source/test
stub was found. Original frozen model/package vectors remain in the full suite.

## Local checks

- Before the fix: source boundary test failed; both new direct-Rust boundary
  tests failed. The existing nested-wrapper test passed.
- Focused model/source regressions: 28 passed.
- Strict all-targets Clippy, all features: exit 0.
- Full all-feature tests: exit 0; 350 ordinary + 3 compile-fail doctests, 4 existing ignored.
- Strict all-targets Clippy, minimal features: exit 0.
- Full minimal-feature tests: exit 0; 334 ordinary + 3 compile-fail doctests, 4 existing ignored.
- Cached minimal binary/example build, warnings-denied all-feature rustdoc,
  formatting and both Rust fixture-audit commands: exit 0.

Commands use the locked offline dependencies, worktree target cache, nice 10,
one Cargo job and one test thread. Logs are /tmp/agent-a-sequence-*.txt. Quire
validation/coverage is recorded by SR-262/264. The workflow diff is empty;
existing CI remains workflow_dispatch-only with both feature lanes, and no run
was dispatched. No dependency or license changed. Broader ruling reconciliation,
matrix status repair and deferred activation assurance remain outside this pass.
