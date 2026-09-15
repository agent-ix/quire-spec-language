---
id: Task-039
title: "Evaluate admitted v2 compensation expressions"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-141
    type: verifies
---
## Scope

Complete issue #101's exact QSL-owned evaluation path for guard, retry and
recovery value handles in a strictly admitted compiled-protocol `/2` package.
Reuse the existing state evaluator and preserve typed refusal, incomplete and
resource-exhaustion outcomes. Do not add a conversion, copied evaluator,
Protocol-owned truth callback or synthetic artifact authority.

## Subtasks

- [x] Add TC-141 over a real strictly read v2 compensation package.
- [x] Share one evaluator across the existing v1 and new v2 public entry points.
- [x] Admit compensation-specific binder anchors, subjects and model authority.
- [x] Exercise unpublished, crossed, unavailable, wrong-authority and bounded paths.
- [x] Run the required local Rust gates and reconcile the Test Matrix and capability records.
- [x] Complete self `/rust-review` and `/gap-analysis` at PR readiness.

## Delivery

The focused test first failed because `state::evaluate_v2` and
`Refusal::UnpublishedArtifact` did not exist. TC-141 now executes non-constant
activation, retry and recovery expressions with exact binder and population
inputs, deterministic accounting and fresh retries. Formatting, strict Clippy,
the focused evaluator suites and the full default- and all-feature Cargo suites
pass with the isolated `target-codex-backends` directory. SR-432 records the
passing Rust review after both findings were fixed, and SR-433 records the
passing targeted gap analysis. Issue closure and the dependent quire-protocol
pin follow the admin merge.
