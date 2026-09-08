---
id: SR-001
title: "base review of quire-spec-language"
type: SpecReview
analysis: base
scope: "spec/spec.md and indexed requirements"
review_set: all
evaluated_revision: "a80a17d1dd303b91712df2023fdba8aba83e89c1"
review_date: "2026-09-07"
---

## Summary

The draft is structurally organized and keeps source/model authority and incomplete outcomes explicit. Review findings concern evidence readiness, prerequisite visibility, and the concrete implementation discrepancies identified where syntax already exists.

## Verdict

**CONDITIONAL** — Findings require disposition before the affected implementation or acceptance gate. This is not owner acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Formatter Inputs permit an output budget and NFR-001 requires checking before an operation; current API takes no budget and checks the fixed default after appending. | [FR-003](../functional/FR-003-format-native-source.md); [NFR-001](../non-functional/NFR-001-bound-syntax-work.md); ../../src/format.rs:8 |
| FND-002 | medium | The CLI argument boundary can panic before reporting a phase outcome. Its non-UTF-8 OS argument/path policy needs an explicit contract and regression case. | [FR-010](../functional/FR-010-report-native-outcomes.md); ../../src/main.rs:22; [code review](../../reviews/26-09-07-native-code-review.md) |
| FND-003 | medium | No recognized tags bind the 21 existing Rust evidence symbols; TC/matrix setup is absent. Twelve NFR method cells also use uncatalogued labels. | [coverage](data/coverage.json); [evidence review](evidence.md) |
| FND-004 | low | Installed catalog diagnostics prevent an error-free validation claim. Expected missing future stages are kept separate from current implementation findings. | ../../docs/spec-workflow.md |

## Scope and provenance

Reviewed `quire-spec-language@a80a17d1dd303b91712df2023fdba8aba83e89c1`. The owner selected base plus all seven Quoin analyses, and declined the optional intent↔test↔code semantic step in `/gap-analysis`. Ordinary requirements consistency, EARS conformance, and the separate current-code review remain in scope. The assignment prohibits spawning additional agents, so these analyses were performed sequentially by Agent A; no independent reviewer acceptance is implied.

The installed Quoin 0.20.0 `spec-review/SKILL.md` and its analysis skills govern these artifacts. The SpecReview authoring pack was fetched once for this repository and its process skeleton/schema was used. [Provenance](data/provenance.json), [coverage output](data/coverage.json), [advisor output](data/advice.json), and [actual method catalog](data/verification-methods.json) preserve the deterministic inputs. IDs in this report are local to this repository unless qualified.

These are new repositories. Missing formal plans, TC records, suites, and matrices are workflow setup/readiness debt. Unimplemented LC02–LC05 stages and unqualified shared consumers are known remaining work; they are not reported as regressions or hidden stubs. No completion status is fabricated.

## Checklist results

| Check | Observed result |
| --- | --- |
| Artifact identities and local links | 23 identified artifacts plus the master; unique local IDs and previously checked local paths/relationship targets resolve. Review artifact validation is recorded in docs/spec-workflow.md. |
| Stories | Four stories each state actor, goal, value, priority, and two Given/When/Then illustrative examples. The ISO catalog uses US-EX identifiers; these are not invented formal TC coverage. |
| FR structure and lineage | 11 FRs each state input/output/behavior, have Test-class ACs and trace through a story to StR-001. |
| Options and constraints | No formal option permutation set is declared. Profile and feature choices are closed; unknown selections refuse. Absence of an Options section alone is not a defect. |
| Quality constraints | All four NFRs have scope, measurement rows and affected-FR edges. Method labels and reverse discoverability need disposition; see integrity/evidence. |
| Six coverage rules | Formal TC coverage, option combinations, constraint boundaries, error paths, transitions and edge cases cannot receive a matrix verdict without TC/matrix artifacts. Concrete current tests are assessed separately. |
| Completion honesty | Quire reports zero status_lies. Authored expectations and syntax checks do not claim the full healthy/violating/incomplete workflow. |

## Review set

| Analysis | Artifact |
| --- | --- |
| base | [base](base.md) |
| failure-domain | [failure-domain](failure-domain.md) |
| integrity | [integrity](integrity.md) |
| dependency | [dependency](dependency.md) |
| evidence | [evidence](evidence.md) |
| risk-complexity | [risk-complexity](risk-complexity.md) |
| scope-boundary | [scope-boundary](scope-boundary.md) |
| ears-conformance | [ears-conformance](ears-conformance.md) |

## Disposition order

First reconcile current syntax contracts and boundary failures in the language repository, and specify TC identities, evidence methods and applicable suites. Then resolve findings and repeat the affected reviews. Use the dependency report for LC02–LC05 ordering after the shared FS02/FS03/FS05 and B/C contract gates are satisfied. Public release and standard-artifact terms remain separate owner decisions. No runtime implementation was changed by this review.
