---
id: FR-044
title: "Activate temporal obligations with immutable captures"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-specification/FR-093, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-094, type: references }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
---
# FR-044: Activate temporal obligations with immutable captures

## Description

When a temporal scope activates, the evaluator SHALL create one obligation
instance per distinct admitted origin or semantic trigger identity, initialized
with every declared capture evaluated exactly once at the authored activation
anchor and immutable thereafter.

## Semantic authority and boundary

Agent E owns the activation, trigger-scope and capture meaning in
`ix://agent-ix/quire-specification/FR-093` and `FR-094`. Each Behavior
subsection restates the owned rule named beside it, and the owned rule governs on
divergence.

| Behavior subsection | Owned source |
| --- | --- |
| Instance identity — trigger-keyed instances, repeated delivery | FR-093 Behavior paragraph 1; FR-093-AC-1, FR-093-AC-2; merged `quire-specification` PR #23 |
| Instance identity — whole-execution origin | temporal-common.md "Each activation has its own exact semantic trigger/origin identity" |
| Captures | FR-093 Behavior paragraph 2; FR-093-AC-3, FR-093-AC-4; FR-094-AC-6 |
| Dispositions | FR-093 Behavior paragraph 3; FR-093-AC-5; FR-094-AC-3; merged `quire-specification` PR #23 |

### Ratified activation rulings

The merged native-temporal semantic amendment in `quire-specification` PR #23
ratifies that a false guard creates no obligation, while an unestablished guard
is incomplete or refused rather than false. It also ratifies that an identical
duplicate receipt preserves provenance without a second semantic event, while a
conflicting payload under the same semantic-trigger identity is a typed
contradiction refusal. These rules are shared activation semantics, not local
implementation selections.

The refusal of an ambient `self`, operation `result` or `pre(expr)` inside a
capture initializer is not restated here: it is owned by
[FR-036](./FR-036-link-composed-native-packages.md)'s scope checking, is already
delivered in the composed linker, and is covered by TM-002. This requirement
consumes that refusal rather than duplicating it.

Agent F owns trigger observation binding, delivery transport and
duplicate-delivery provenance; a receipt identity is a caller-supplied opaque
value here. Agent B owns protocol activation, participation and result
serialization; this requirement produces no protocol result and no wire encoding.

## Inputs

The emitted activation record — a whole-execution origin with its anchor, or a
trigger binder with its separately scoped guard and anchor — the emitted ordered
capture list with each capture's declared type and initializer handle, the
admitted trigger observations with their semantic trigger identities, receipt
identities and payloads, and the trigger scope's closure and completeness state.

## Outputs

An activation disposition of `inactive`, `unknown` or `active`; for each active
instance, its instance identity, its delivered receipt provenance and its
immutable typed capture environment; or a located incomplete or refused
activation naming the missing or inconsistent input.

A trigger whose guard evaluates false creates no instance and is not itself an
output: it contributes nothing, so the scope's disposition follows from whatever
remains. A closed-complete scope whose every admitted trigger is guard-false is
therefore `inactive`; an open scope in the same shape is `unknown` with open
completeness. Guard-false is not a sixth disposition.

Temporal truth is never produced by this requirement.

## Behavior

### Instance identity

The evaluator SHALL identify an event-triggered instance by the clause subject
and the semantic trigger-event identity, and SHALL NOT identify it by timestamp,
display name, receipt identity or equal captured payload.

The evaluator SHALL identify a whole-execution-origin instance by the clause
subject and the admitted execution-origin identity supplied with the trace, and
SHALL create exactly one such instance per admitted execution origin.

The evaluator SHALL create no second instance for a repeated delivery of the same
semantic trigger, and SHALL retain that delivery's receipt provenance on the
existing instance. The evaluator SHALL create a distinct instance for a distinct
semantic trigger even where its captured values compare equal.

If two deliveries assert the same semantic trigger identity or the same
execution-origin identity with conflicting payloads, then the evaluator SHALL
return a typed contradiction refusal naming that identity, and SHALL NOT silently
alias them or replace the retained capture.

### Activation guard

While an activation guard is present, the evaluator SHALL evaluate it once at the
authored anchor over the trigger binder and the admitted trigger, and SHALL
create an instance only where it evaluates true. A guard evaluating false SHALL
create no instance and SHALL NOT be reported as a refusal.

If a guard cannot be established from its inputs, then the evaluator SHALL report
an incomplete or refused activation naming the guard, and SHALL NOT treat an
unestablished guard as false.

### Captures

The evaluator SHALL evaluate captures in authored source order at the activation
anchor exactly once per instance, and SHALL retain each capture's declared
expression handle, type, value, source binding and anchor.

The evaluator SHALL NOT re-evaluate a capture initializer during incremental
re-evaluation, restoration or replay. A retained capture that has been evicted
SHALL make its instance incomplete rather than be recomputed at a later anchor.

A later mutable observation SHALL NOT replace a captured value, and one
instance's captures SHALL NOT satisfy another instance's propositions.

If a declared capture cannot be established because its input is missing,
nullable, wrongly typed, stale or anchor-mismatched, then the evaluator SHALL
report an incomplete or refused activation naming that capture, and SHALL NOT
begin temporal evaluation for that instance.

### Dispositions

The evaluator SHALL report `inactive` only for a trigger scope that is
authoritatively closed and complete with no instance-creating trigger. That
disposition is not a true obligation and is not evidence that recovery behavior
was exercised.

The evaluator SHALL report `unknown` with an open completeness disposition for an
open trigger scope, and `unknown` with its incomplete or refused
assessment-execution disposition for missing or refused trigger evidence.

Neither `inactive` nor `unknown` SHALL become temporal truth, and
[FR-043](./FR-043-evaluate-bounded-native-temporal.md) SHALL evaluate no position
for either.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-044-AC-1 | Two distinct semantic triggers carrying equal captured amounts create two distinct obligation instances, and a value captured by one cannot satisfy the other's proposition; the reported mismatch names both instance identities rather than the equal amount. | Test (TC-123) |
| FR-044-AC-2 | A duplicate delivery of one semantic trigger under a different receipt identity creates no second instance and retains that receipt's provenance on the existing instance with its capture values unchanged; two deliveries of one trigger identity with conflicting payloads return a typed contradiction refusal instead of aliasing. | Test (TC-123) |
| FR-044-AC-3 | A captured value, reference and predicate valuation environment are unreachable for mutation after activation, and a later observation carrying a changed amount or reference leaves the retained capture record byte-identical. | Test (TC-123); Analysis |
| FR-044-AC-4 | A missing, nullable, wrong-type, stale or anchor-mismatched capture input reports incomplete or refused activation naming that capture before any temporal position is evaluated for that instance, while an unrelated healthy instance in the same evaluation retains its activation. | Test (TC-123) |
| FR-044-AC-5 | A closed-complete trigger scope with no instance-creating trigger reports `inactive`; an open scope reports `unknown` with open completeness; missing and refused trigger evidence report `unknown` with their distinct incomplete and refused execution dispositions; none of the four carries temporal truth or evaluates a position. | Test (TC-123) |
| FR-044-AC-6 | Captures evaluate in authored source order exactly once at the anchor, and an initializer reading a later capture's name refuses; the recorded per-instance evaluation count stays one across an incremental re-evaluation and an admitted restoration. | Test (TC-123); Demonstration |
| FR-044-AC-7 | A guard evaluating false creates no instance and is not a refusal; a closed-complete scope whose every admitted trigger is guard-false reports `inactive`; a guard that cannot be established reports incomplete or refused activation naming the guard rather than treating it as false. | Test (TC-123) |
| FR-044-AC-8 | A whole-execution origin creates exactly one instance keyed by the clause subject and the admitted execution-origin identity; two admitted executions produce two instances that never alias, re-evaluating one execution does not split it, and two deliveries of one origin identity with conflicting payloads return a typed contradiction refusal. | Test (TC-123) |
| FR-044-AC-9 | An evicted retained capture makes its instance incomplete and names the evicted capture; no initializer is re-evaluated at a later anchor during incremental re-evaluation, restoration or replay. | Test (TC-123) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) emits the
  activation record, ordered captures and their binding requirements.
- **Peer**: [FR-043](./FR-043-evaluate-bounded-native-temporal.md) evaluates only
  an obligation this requirement reports `active`.
- **Constrained by**:
  [NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md) for the
  active-instance, capture and retention ceilings.
