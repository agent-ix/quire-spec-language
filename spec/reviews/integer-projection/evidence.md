---
id: SR-239
title: "evidence review of bounded integer IR lowering"
type: SpecReview
analysis: evidence
scope: "FR-033; TC-111; TM-006; Task-032"
review_set: all
---
## Summary

The previous quoin advise attempt failed while detecting installed Quire 0.31.0, yielding no recommendations. From the installed catalog, author judgment selects integration-testing (Test): actual native compilation, both pinned IR readers, explicit expected operator/type/source assertions, exact limit controls and command/backend refusal. Runnable integer files produce true/false natively and identical IR bytes. Numeric backend execution and independent assurance remain unclaimed.

Author PR-readiness review of `b0c02c7`, using the owner-selected all set.
No applicable AssuranceProfile exists; timing follows the owner directive.

The focused correction suite passes 60 tests: actual integer/native/command
paths, target parsing/display and wrong-command/non-UTF-8 operands, with located
private wire-conversion controls. Full feature-lane results are recorded in
SR-243; these observations do not assert numeric codegen acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Adviser version detection is unavailable; the methods are catalog-grounded author judgment. | FR-033-AC-1–5 |

The new filesystem criterion is checked through both public fixture paths and
the actual built example executable. Invalid-directory and late Markdown controls retain the exit-2 contract.
