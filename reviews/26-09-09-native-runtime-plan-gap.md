---
id: SR-100
title: "Gap analysis of Plan-006 native runtime qualification"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-006-native-runtime/, spec/native-runtime/tests.md at fbaf29a7baf3e54b6a7c35d52e19d5bc17be94e3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-006
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-004
    type: references
---

## Summary

The native runtime requirements and all 23 matrix cases have executed backing
tests. Task-015 remains in progress because final plan reconciliation and private
handoff are not yet completed.

## Verdict

**FAIL** at the initial audit: Task-015 is an incomplete P1 critical-path task.
No runtime implementation or integration-test gap was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Task-015 remains in_progress pending final reconciliation and private handoff | plan/Plan-006-native-runtime/tasks/Task-015-runtime-qualification.md:5 |

## Coverage

Applied QUOIN 0.22.5 gap-analysis and its target, plan, matrix, reverse-gap and
SpecReview procedures. This audit changed only this review artifact. Target:
Plan-006; spec root: spec/; matrix: spec/native-runtime/tests.md (TM-004);
identity prefix: ix://agent-ix/quire-spec-language; source: src/runtime.rs and
src/runtime/; tests: tests/runtime_inputs.rs, runtime_validation.rs,
runtime_evaluation.rs and its case modules, runtime_workflow.rs and support.
Optional semantic comparison was skipped under the owner's existing choice.

Reconciliation: actual `quire coverage --scope
/home/peter/dev/worktrees/formalization-a-language --json`, without --strict,
using Quire 0.31.0 / engine ca7362d4. This meets the >=0.16 split-root contract;
no grep fallback was used. All four Task documents were inspected. Tasks done:
3/4; the dependency chain is satisfied in order. IT-006 is checked and qualified;
the unchecked StR-001 explicitly retains the broader backend/Quire requirement,
so it is not a claim that these four runtime tasks finish the original assignment.

TM-004 backing: 23/23. FR-018: 7/7; FR-007: 15/15; FR-008: 20/20.
Rust binding census: 205/205 candidates tagged and bound. Root backing: 196/208.
There are no untracked symbols, reported status lies or unbacked runtime rows.
Remaining root FR-009/011 projection/extraction rows are downstream work.
Manual TC-010 and Inspection FR-017-AC-2 are declared no-symbol rows, not
missing Rust tests. Three old IT-004 unmatched tags are outside this runtime
scope and are not included as evidence of runtime integration.

The engine retains twenty existing classifier/catalog diagnostics, including
the functional matrix's Coverage Status versus configured Status mismatch,
NFR heading/metric limitations, absent optional catalog archetypes and broad
property-shape advice. Six registry duplicate notices remain. Empty status_lies
is therefore not sufficient evidence of qualification: every runtime matrix
status was manually reconciled with SR-096/097/098/099 and the executed logs.
NFR-006's seventeen metrics are reviewed evidence, not minted metric IDs.

## Reverse code-to-spec inventory

| Behavior family | Actual implementation | Owning requirements |
| --- | --- | --- |
| Snapshot construction | Snapshot::new, SnapshotDraft, construction::Artifact | FR-018 |
| Invocation construction | Invocation::new, InvocationDraft | FR-018 |
| Exact bytes and role-specific references | immutable bytes/digest/reference getters; SnapshotRef and InvocationRef | FR-018 |
| Exact artifact/model/clause selection | validate and validation/inventory.rs | FR-007 |
| Typed values and finite population closure | validation/values.rs; private context indexes | FR-007 |
| Frames, captures and population deltas | validation/frames.rs and operation selection | FR-007 |
| Original-AST execution | evaluate, evaluation/walk.rs, value.rs and comparison.rs | FR-008 |
| Actual events and retained provenance | EvaluationReport and evaluation/budget.rs | FR-008 |
| Limits and immutable retries | construction/validation/evaluation budgets and private per-call state | NFR-006, FR-018, FR-007, FR-008 |

Inventoried behavior families: 9; untraced behaviors: 0; confirmed source stubs:
0; confirmed test stubs: 0. Public constructors, validated context and borrowed
execution are real implementations. Re-export facades expose those actual
implementations. Defensive checks have specified refusal/limit owners; there is
no unowned I/O, concurrency or second expression interpreter. The five new
pipeline tests use actual APIs and independent expected truth/costs; prior
generated graph, sequence, scalar and boundary controls remain qualified by
SR-098. No line-coverage or mutation-adequacy percentage is inferred.

## Executed evidence and limits

SR-099 records the exact commands, source pins, Rust 1.98.1, IR 690bde7 and
adopted standard e897f81. At 46f5e70, 202 ordinary tests, one compile-fail
doctest, three private audits, strict Clippy/rustdoc, formatting, minimal build
and CLI/audit checks passed. The comment/status-only 4ac3597 revision reran all
five affected integration tests and formatting successfully. All gates were
serial, offline/local and reused target caches; no hosted CI was dispatched.

The attempted quoin advise run failed its version probe; SR-099 records the
actual diagnostic and explicit supplemental method assessment. No absent
recommendation is treated as approval or missing executable evidence. Backend
parity, strict linked packages, FS03 acceptance and Quire integration remain
outside this runtime milestone and required in the original assignment.
