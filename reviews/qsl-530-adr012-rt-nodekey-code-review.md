---
id: SR-13364
title: 'Code review of PR #1070: ADR-012 RT holds shared NodeKeys'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@e1b5ec811; git diff refs/remotes/origin/main...e1b5ec811 (PR #1070): spec/decisions/ADR-012-semantic-family-extension-contracts.md rows at 875 and 1099; read against main 891825c45 merged with the PR head'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
---

## Summary

Ticket: QSL-530 (planner-designated trail; the change is IR-882's QSL side). The diff is two ADR-012 table cells. No production code, test or tool changed. The arch-lint tool the rows depend on for enforcement was read to answer what stops RT constructing a `NodeKey`: `Role` has only `Qsl` and `Cg`, and `--qsl` refuses a tree that is not quire-spec-language, so no QSL gate can scan RT. That is a spec-consistency matter and is recorded in SR-13366 FND-004, not here.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-012-875-RT-lookup-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-012-1099-RT-NodeKey-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| arch-lint-Role | tools/arch-lint/api_surface.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for this lane. No code changed; spec findings are in SR-13365 and SR-13366.
