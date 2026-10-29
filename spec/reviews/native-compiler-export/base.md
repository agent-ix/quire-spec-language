---
id: SR-175
title: "base review of standalone compiler export"
type: SpecReview
analysis: base
scope: "FR-027; TC-105; TM-007; Task-026"
review_set: all
---
## Summary

FR-027 defines source-only input, exact existing artifact bytes, profile/error behavior and consumer reread. Three criteria map to TC-105. Real binary tests cover state and operation exports, absent runtime files, exact byte/digest correspondence, verified reread and static/request refusals. Existing run and parse/format controls preserve the shared boundary. No new independent resource policy or semantic option is introduced.

Author PR-readiness review of `f0cfe7b` with the owner-selected all set.
No applicable AssuranceProfile was found; review timing follows the owner directive.

Six binary/catalog tests cover the three criteria, including a fresh directory containing only selected sources, command-specific arity, typed file-count context and a real Linux output failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-027; TC-105 |
