---
id: SR-195
title: "base review of standalone projection export"
type: SpecReview
analysis: base
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

FR-029 defines the lower command over the existing closed source-only request, its fixed Boolean target, exact IR output and located refusal behavior. All three acceptance criteria map to TC-107. The three binary tests cover exact producer/consumer bytes, complete backend generation, later-clause refusal, fresh retry and shared request limits. Existing commands remain exercised.

Author PR-readiness review of `d1fcf16`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

The fourth binary test exercises command-specific arity. Two adapter unit tests
check absent/invalid Unicode coordinates and their typed serialized status/code;
the unsupported-clause binary test checks the actual located source span.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-029; TC-107 |
