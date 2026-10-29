---
id: SR-002
title: "failure-domain review of quire-spec-language"
type: SpecReview
analysis: failure-domain
scope: "spec/spec.md and indexed requirements"
review_set: all
evaluated_revision: "a80a17d1dd303b91712df2023fdba8aba83e89c1"
review_date: "2026-09-07"
---

## Summary

Trust boundaries, entity keys, purity, and graph/resource failure handling were reviewed. The standard explicitly distinguishes static refusal, missing observation and work incompleteness; the current native CLI/formatter findings are recorded only in the language scope.

## Verdict

**CONDITIONAL** — Findings require disposition before the affected implementation or acceptance gate. This is not owner acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An OS argument containing byte FF reaches std::env::args and panics (exit 101), escaping the documented usage/I/O outcome. Specify the argument and path encoding boundary. | ../../src/main.rs:22; FR-010; [probe](data/cli-nonutf8.json) |
| FND-002 | medium | Formatter performs the append before checking its ceiling and has no caller-selected output budget. Define inclusive budget behavior and check prospective growth. | ../../src/format.rs:8; ../../src/format.rs:46; FR-003; NFR-001 |

## Scope and provenance

Reviewed `quire-spec-language@a80a17d1dd303b91712df2023fdba8aba83e89c1`. The owner selected base plus all seven Quoin analyses, and declined the optional intent↔test↔code semantic step in `/gap-analysis`. Ordinary requirements consistency, EARS conformance, and the separate current-code review remain in scope. The assignment prohibits spawning additional agents, so these analyses were performed sequentially by Agent A; no independent reviewer acceptance is implied.

The installed Quoin 0.20.0 `spec-review/SKILL.md` and its analysis skills govern these artifacts. The SpecReview authoring pack was fetched once for this repository and its process skeleton/schema was used. [Provenance](data/provenance.json), [coverage output](data/coverage.json), [advisor output](data/advice.json), and [actual method catalog](data/verification-methods.json) preserve the deterministic inputs. IDs in this report are local to this repository unless qualified.

These are new repositories. Missing formal plans, TC records, suites, and matrices are workflow setup/readiness debt. Unimplemented LC02–LC05 stages and unqualified shared consumers are known remaining work; they are not reported as regressions or hidden stubs. No completion status is fabricated.

## Boundary analysis

| Boundary | Policy and examined edge cases | Requirements |
| --- | --- | --- |
| External model / typed adapter | Strict refusal on missing, foreign, stale or ambiguous static bindings; no partially linked package. A owns native linking; C coordinates existing consumers. | FR-005 |
| Entity and evidence identity | Artifact key includes kind, authority, identity and structured revision; byte digest and semantic definition/feature selection are checked separately. Local syntax labels do not grant portable identity. | FR-001, FR-004, FR-005 |
| User predicates | Read-only expressions over immutable populations, snapshot-qualified optional facts and invocation parameters; no callback side effects or hidden domain mutation admitted. | FR-006, FR-007, FR-008 |
| Cycles, depth and unavailable data | Reachability tests the target before visited suppression; cycles terminate, disconnected nodes do not invent reachability, work exhaustion remains incomplete. Missing observations are not empty populations. | FR-007, FR-008, NFR-001, NFR-003 |
| Backend / extraction | Unsupported obligations remain addressable. Extraction provides opaque bodies and correspondence; the native compiler owns parsing and lowering uses the existing binder. | FR-009, FR-011; IT-003 |

## Required follow-through

Use generated identity collisions, stale bytes with recomputed digests, cyclic/disconnected graph cases, exact and exceeded budgets, and interrupted evaluation to exercise these policies. Existing example packets supply candidates, not an independent oracle. The optional gap semantic comparison was not run.
