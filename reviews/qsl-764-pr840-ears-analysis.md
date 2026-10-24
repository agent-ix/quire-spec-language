---
id: SR-9110
title: "EARS review of quire-spec-language PR #840: protocol reference statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@ae803014a9c6f6be9733a88ce9d6caedb9eeeae6; spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/functional/FR-092-key-type-parameter-and-declared-nodes.md"
review_set: subset
---
# EARS review of quire-spec-language PR #840

## Summary

Ticket: QSL-764. EARS conformance of the new requirement sentences.

Examined:
- FR-308 "Protocol reference expression": ubiquitous `S2 SHALL build ...`;
  grammar and admission sentences are definitions (examined)
- FR-218 "Protocol references": `S3 SHALL resolve ... SHALL bind`, and
  event-style refusals ("A name that ... SHALL refuse ...") (examined)
- FR-092 "Protocol reference nodes": `check SHALL build ...`, `S4 SHALL
  emit ...`, `The emitter SHALL NOT write ...` (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Each new SHALL names its system (S2, S3, S4, check, emitter) and
one response. The conditional refusals read as unwanted-behaviour clauses.
The overlap between two FR-218 refusal bullets is recorded in SR-9109
FND-003, not here.
