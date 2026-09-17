---
id: Plan-001
title: "Rust fixture audit remediation"
type: Plan
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
---
# Plan-001: Rust fixture audit remediation

## Requirements Summary

- [x] FR-012: real Rust fixture identity, correspondence and syntax checks.
- [x] NFR-005: Rust-owned verification and explicit unapproved producer refusal.

US-004/StR-001 supply the existing evidence/integration need; the full native
assessment workflow is not owned or marked complete by this bounded plan.
Readiness comes from the [scoped review](../../spec/reviews/rust-verification/base.md)
at specification revision 11a9128. Fresh producer approval is not assumed.

## Dependency Graph

Existing FR-001/FR-002 outputs feed the audit's syntax mode. Within the audit,
fallible bounded I/O, strict JSON and contextual error helpers precede mode
checks. NFR-005 constrains all paths; actual Rust outcomes precede CI adoption
and Python retirement. No new external adapter or model authority is required.
The authoritative task edges are in the task files; no cycle exists.

## Test Plan

| TC | Boundary | Setup / expected outcome |
| --- | --- | --- |
| TC-001 | Self-contained identity negative controls | Run the actual audit control function for three distinct artifact roles. All six stale/recomputed-digest controls and a duplicate-key control reject with the expected code. |
| TC-002 | Selected invocation packet audit | Run review over the pinned standard fixture directory and independently corrupt a temporary selected input. The real packet reports 23 files and seven cases; the corrupted input refuses before success. |
| TC-003 | Selected role and source correspondence audit | Run roles over the pinned packet, then exercise mismatched region/digest/role membership in isolated fixtures. The real packet reports 17 artifacts and four regions; the changed correspondence refuses. |
| TC-005 | Real native rule syntax outcomes | Use rule-syntax on the selected standard packet and exercise malformed case metadata. 50 cases parse and one is unsupported; metadata changes refuse; no evaluator result is claimed. |
| TC-006 | Malformed JSON and field types | Generate bounded malformed JSON and field-type mutations including escaped duplicate keys, invalid UTF-8 and trailing JSON. Each invalid input errors without panic or a success summary; valid neighboring inputs remain accepted. |
| TC-007 | Fixture root containment | Generate nested relative and escaping/absolute paths; include a Unix symlink escape in an isolated temporary tree. Contained paths are read; foreign targets refuse and are never used as evidence. |
| TC-008 | Audit resource ceilings | Generate sizes around lowered file/aggregate/record limits and verify the default oversized-file boundary. At-ceiling valid input succeeds; beyond-ceiling work reports resource-exhausted without partial success. |
| TC-009 | CLI encoding and Rust-only execution | Invoke the real binary with unknown/invalid OS arguments and run self-test with external runtimes unavailable. Usage exits 2 without panic; Rust audits execute with no Python/Node. |
| TC-010 | Owned verification language inventory | Inspect all owned executable audit files, CI steps and documented verification commands after migration. No Python audit executable/CI invocation or embedded non-Rust verifier remains; historical records retain their provenance. |

## Delivered Work

### Track A: serial critical path

- **Task-001: Rust audit implementation** — medium, roughly one implementation session; exit: real audit modes and adverse cases report only performed checks, with producer refusal.
- **Task-002: CI adoption and verification** — medium, roughly one verification session; gate: all Rust tests/lints, real local packet audits and language inventory pass before Python helpers are removed and the scope is marked implemented.

## Parallel Execution Summary

Agent A executes Task-001 then Task-002. The assignment prohibits spawning
additional agents. B/C/TL continue their own work; this plan does not allocate it.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-001 | A | FR-012, NFR-005 | TC-001–TC-009 | done |
| Task-002 | A | FR-012, NFR-005 | TC-002–TC-010 | done |

## Coordination Rules

The campaign tracks this work as [Agent A #58 / LR02](https://github.com/agent-ix/quire-research/issues/58),
coordinated with [LC01 / language#2](https://github.com/agent-ix/quire-spec-language/issues/2).
This plan uses the active Agent A branch; it creates no competing implementation
or duplicate tracking issue. Commit the reviewed spec/plan before implementation.
Record actual test trace binding and selected packet revision. Keep existing
fixture bytes, native default CLI, grants, unrelated review findings and the
separate external-producer approval status intact. Merge/publication is not
authorized by a passing audit.
