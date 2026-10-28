---
id: SR-1720001
title: "code-review of PR #1720 (QSL-1144 follow-up, STD-525): memory drain row mandatory"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1720 git diff origin/main...8e5e03309; qsl-semantics/src/check/refinement/protocol.rs and protocol/tests.rs; QSpec FR-432-AC-3, FR-432-AC-22 (as amended by QSpec #471)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1720001: code-review of PR #1720

## Summary

Ticket: QSL-1144. Head reviewed: 8e5e03309, on main e142cc19a, 2 files. QSPEC_DIR is ~/dev/quire-specification (48373530, which contains #471 a550e345).

Each check, verified:
- **Code:** `MemoryStepKind::need` returns `Need::Row` for `drain` and `flush`, and `Need::Optional` is gone. The memory classes come only from the outermost `parallel` controls through `MemoryStepKind::of_model`: sc and ra give `[Drain]`, tso gives `[Drain, Flush]`.
- **Refusal:** a missing drain row refuses `missing_declaration`/`missing-name` with `missing(RowClass::Memory, "memory drain")` at `FillRefinesAcc`. Under tso the test asserts that the missing-flush refusal has the same locus and code.
- **Doubled row:** `invalid_model_binding`/`conflicting-binding` at the second row, `Conflicting{name: "memory drain", first}`, with `first` resolving to the first row.
- **No `parallel`:** such a protocol (`REFUNDS`) still checks without a drain row. Every refinement fixture in the workspace outside this file still passes.
- **Catalog order:** drain(ra), drain(sc), drain(tso), flush(tso) is unchanged and already asserted by `drains_are_keyed_by_model_ordered_and_unique` (qsl-semantics tests/it/refinement_protocol.rs:1672).

Gates (locked-build, jobs 4, `--all-features`, debug info off):
- fmt: exit 0.
- qsl-semantics: lib 802, it 1323, doc 30, all passing.
- qsl-analyze `weak_memory`/`refinement`/`protocol`: 242/242.
- qsl-package `refinement`/`protocol`: 41/41.
- root `refinement`/`protocol`: 94/94.
- `cargo clippy -p qsl-semantics --all-targets -D warnings`: exit 0. This is scoped to the package because workspace clippy is red on main from #1715.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean. Mergeable.**
