---
id: NFR-008
title: "Bound temporal evaluation work and required retained state"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: constrains
  - target: ix://agent-ix/quire-specification/NFR-040
    type: depends_on
---
# NFR-008: Bound temporal evaluation work and required retained state

## Statement

If a temporal horizon computation overflows or a selected work, active-instance
or retention ceiling is reached, then the temporal evaluator shall stop with a
typed incomplete or refused result that retains the affected obligation and limit
identity, and shall not emit a Boolean truth for that obligation.

## Scope

- Applies to native temporal admission, whole-trace evaluation and incremental
  re-evaluation of an emitted `quire.compiled-protocol/1` temporal body.
- The semantic minimum retains every supplied valuation and capture still
  required by an unsettled bounded obligation.
- Numeric ceilings are explicit caller-lowered inputs. This requirement invents no
  universal maximum and defines no observation storage or replay mechanism; those
  remain agent F's.
- Mutation-adequacy measurement of the suite is recorded as outstanding assurance
  work rather than claimed here.

## Rationale

Bounded interval syntax does not make horizon arithmetic, operator nesting,
concurrent trigger instances or retained histories safe. A resource stop must stay
distinguishable from a property violation and from successful closure; collapsing
either into `false` would report a passing or failing obligation that was never
evaluated.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Accepted horizon or history computations that overflowed the checked domain | 0 | 0 | Property Test |
| Exhausted evaluations emitting a Boolean truth | 0 | 0 | Fault Injection |
| Unsettled instances whose required valuation or capture was silently evicted | 0 | 0 | Fault Injection |
| Results reused after a changed limit, profile or clock binding | 0 | 0 | Integration Test |

## Verification

Enumerate nested future and past formulas, boundary intervals at the checked
integer extrema and concurrent trigger instances around each declared ceiling.
Independently force horizon overflow, work exhaustion, active-instance exhaustion
and required-state eviction, and require a typed incomplete or refused result
retaining the affected obligation, position and limit identity. Separately mutate
each exhaustion path to return a Boolean and require the suite to fail.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-008-AC-1 | Future horizon, past-history need and nested interval composition use checked arithmetic and reject overflow before evaluation begins. | Test (TC-124) |
| NFR-008-AC-2 | An exhausted work, active-instance or retention ceiling produces an incomplete or refused result naming the limit, and never `true` or `false`. | Test (TC-124) |
| NFR-008-AC-3 | Valuations and captures required by an unsettled obligation remain retained until it settles, or an explicit incomplete result records their loss. | Test (TC-124) |
| NFR-008-AC-4 | Limit configuration and any admitted restoration state participate in result identity, so a changed ceiling refuses reuse of an earlier result. | Test (TC-124) |
| NFR-008-AC-5 | The first unaffordable operation remains unperformed, usage reflects only successful charges, and a retry under sufficient ceilings produces the full result. | Test (TC-124) |

## Dependencies

- **Upstream**: [FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md)
  and [FR-044](../functional/FR-044-activate-temporal-obligations.md) define the
  outcomes this requirement bounds.
- **Peer**: [NFR-006](./NFR-006-bound-native-runtime.md) bounds the historical
  native runtime; its ceilings are not shared with the temporal evaluator.
- Shared source requirement: `ix://agent-ix/quire-specification/NFR-040`.
