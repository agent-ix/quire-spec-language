---
id: SR-969
title: 'Code review of PR #2216: per-occurrence TemporalBinding and temporal_clause_names, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@06490a1a7; git diff origin/peter/qsl-1581-ir-fields...06490a1a7 (PR #2216); ticket QSL-1592; QSpec FR-322-AC-342..370'
review_set: subset
date: '2026-11-03'
---

## Summary

Ticket: QSL-1592. PR: quire-spec-language#2216, head 06490a1a7, stacked on #2213. The diff adds the per-occurrence `occurrence` on every TemporalBinding row, emits `temporal_clause_names` (clause, hyper and hybrid-claim sites), and bumps the quire-contract-model lock line to IR main 64de9ec9.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-322-AC-343 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-344 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-345 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-294 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-305 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-361 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-365 | spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `--features population-fixtures` no longer compiles: the `CheckedPackageSidecarsRef` initializer lacks the new `temporal_clause_names` field. | src/population_fixtures.rs:438 |
| FND-002 | medium | Hyper-clause, hybrid-claim and compensation occurrence/name rows are emitted but no test asserts their values. | qsl-package/src/emit.rs:5856-5990 |

## Verdict

Not mergeable until FND-001 is fixed. Evidence: `cargo check --workspace --all-targets` passes; `cargo check --features population-fixtures` fails E0063 at src/population_fixtures.rs:438; `cargo test -p qsl-package --lib temporal` 33 passed. Cargo.lock changes one line. No new `rev=` pin (the one in qsl-source/Cargo.toml predates the diff). No compat layer. Sort order is (declaration, occurrence) for both row sets. Re-emit paths (emit_timed_models, monitor_plan) pass base names through. Compensation rows use the `Declaration` occurrence, as FR-322-AC-305 requires.
