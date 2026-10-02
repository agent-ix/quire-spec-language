---
id: SR-156
title: "failure-domain review of public rule-model source"
type: SpecReview
analysis: failure-domain
scope: "FR-025; TC-101; TC-102; TM-007; Task-024"
review_set: all
---
## Summary

Unknown profiles stop before model decoding; invalid JSON, foreign occurrences, constructor errors and budget stops retain the original FormalSource. Duplicate scalar names refuse; the existing IR and native admission own declaration/reference validity. Recursive types have an explicit lowering ceiling and Serde retains its decode ceiling. There are no callbacks, I/O, shared mutation or graph traversals in this frontend.

Author PR-readiness re-review of `fbdf687`, using the owner-selected all set.
No applicable AssuranceProfile was found. Review timing follows the owner directive.

Duplicate and unknown scalar names now have separate causes. Byte, category and depth budgets retain effective context; malformed group values and a malformed later operation cannot mask exhaustion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-025; TC-101; TC-102 |
