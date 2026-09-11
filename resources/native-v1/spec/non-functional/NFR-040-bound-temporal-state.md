---
id: NFR-040
title: "Bound temporal work and required retained state"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-specification/FR-091
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-092
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-093
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-094
    type: constrains
---
## Statement

The temporal evaluator shall use checked compositional horizon/history bounds
and explicit finite work, active-instance, capture and retention limits without
turning exhaustion or required-state eviction into a Boolean result.

## Scope

- Applies to native temporal admission, offline evaluation, incremental
  evaluation and native-to-TL lowering.
- The semantic minimum retains every observation and capture still required by
  an unsettled bounded obligation. Deployment-specific numeric limits are
  explicit inputs; this specification does not invent one universal maximum.

## Rationale

Bounded syntax does not make arithmetic, nesting, concurrent trigger instances
or retained histories automatically safe. A resource stop must remain distinct
from a property violation or successful closure.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checked bound overflows accepted | 0 | 0 | property-based-testing |
| Exhausted evaluations emitting true or false | 0 | 0 | fault-injection |
| Unsettled instances whose required capture/history is silently evicted | 0 | 0 | model-based-test-generation |
| Results reused after a relevant limit/profile/binding change | 0 | 0 | integration-testing |

These catalog methods implement the composed-review disposition. A generic
performance benchmark cannot prove that no overflow, Boolean fallback, silent
eviction or stale reuse exists merely because each metric has threshold zero;
the method must falsify the corresponding semantic failure mode.

## Verification

Generate nested future/past formulas, boundary intervals and concurrent trigger
instances around each declared limit. Independently force arithmetic overflow,
work exhaustion and required-state eviction. Require a typed incomplete/refused
result that retains the affected obligation and limit identity, and mutate each
failure path to attempt a Boolean fallback. Fault injection establishes the
original exhaustion behavior; mutation testing separately measures whether the
suite detects an illicit Boolean fallback.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-040-AC-1 | Future horizon, past-history need and nested composition use checked arithmetic and reject overflow before evaluation. | Test (TC-118) |
| NFR-040-AC-2 | Exhausted work, active-instance or retention limits produce incomplete/refused results and no Boolean fallback. | Test (TC-118) |
| NFR-040-AC-3 | Required captures/history remain until the obligation settles or an explicit incomplete result records their loss. | Test (TC-118) |
| NFR-040-AC-4 | Limit configuration and any admitted restoration state participate in result identity and freshness. | Test (TC-118) |

## Dependencies

- Agent F owns observation storage/replay mechanisms and supplies declared
  retention/lateness limits; this requirement constrains semantic outcomes when
  those limits are insufficient.
