---
id: SR-269
title: "Evidence review of owned runtime decoding"
type: SpecReview
analysis: evidence
scope: "FR-024 amendment; TC-099/100; Task-035; implementation 3c6a0e6"
review_set: all
---
## Summary

`quoin advise --json` was run and failed CLI version detection despite Quire 0.31.0 (engine 0.46.0@ca7362d4). The catalog was read; the choices below are author judgment, not deterministic recommendations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-024-AC-1: retain Test using integration-testing; actual artifact round trips preserve selected bytes and execute reread inputs. | TC-099 |
| FND-002 | low | FR-024-AC-2: retain Test using negative-abuse-testing; existing tests distinguish selection and decode stages with typed causes. | TC-100 |
| FND-003 | low | FR-024-AC-3: retain Test using negative-abuse-testing plus integration-testing; new direct-record/variant tests failed on positional-array acceptance before the fix. | TC-099; TC-100 |
| FND-004 | low | FR-024-AC-4: retain Test using negative-abuse-testing and integration-testing; existing structural-limit, retry and runtime controls remain required. | TC-099; TC-100 |

## Method limits

Both catalog methods have class Test and evidence kind Integration; the repository's Rust harness supplies that boundary. Required-nullable and digest cases are finite adverse controls, not a claim of complete malformed-JSON coverage. Property testing, fuzzing and broader mutation measurement remain later assurance work. Loom adds no relevant evidence to this pure synchronous conversion change. The failed advisor and #28 status-column mismatch remain disclosed.

