---
id: SR-160302
title: "Gap analysis of PR #2229: restore the qsl-semantics lib suite on main"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c809ff263; PR #2229; FR-135-AC-10, FR-136-AC-6, FR-383-AC-17, TC-541, TC-663; ticket QSL-1603"
review_set: subset
date: "2026-11-03"
---

## Summary

Ticket: QSL-1603. Planless. The PR changes one production file (no behavior change) and test fixtures. No criterion lost its tagged test: the traces on the edited tests are unchanged.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136-AC-6 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-135-AC-10 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-383-AC-17 | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md | examined |
| TC-663 await under no temporal profile | qsl-semantics/src/check/protocol_clause/tests/graph.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No finding (placeholder) | - |

## Verdict

Mergeable, no gaps. Every edited test keeps its trace tags and its own refusal assertion; the S3 helpers still return every non-emission refusal, so exact-list assertions (`only(&found)`, `[missing_alias, missing_selection]`) still fail on a masked or extra refusal.
