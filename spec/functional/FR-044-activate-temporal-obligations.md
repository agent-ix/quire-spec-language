---
id: FR-044
title: "Activate temporal obligations with immutable captures"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-specification/FR-093, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
  - { target: ix://agent-ix/quire-specification/FR-095, type: references }
---
# FR-044: Activate temporal obligations with immutable captures

## Description

When a temporal scope activates, the evaluator SHALL create one obligation
instance per distinct semantic trigger identity, initialized with every declared
capture evaluated exactly once at the authored activation anchor and thereafter
immutable.

## Inputs

The emitted activation record (whole-execution origin or trigger binder with its
separately scoped guard), the emitted ordered capture list with each capture's
declared type and initializer handle, the admitted trigger observations with
their semantic trigger identities and delivery provenance, and the trigger
scope's progress and completeness state.

## Outputs

An activation disposition of `inactive`, `unknown` or `active`; for each active
instance, its trigger identity and its immutable typed capture environment; or a
located incomplete or refused activation naming the missing or inconsistent
input. Temporal truth is never produced by this requirement.

## Behavior

The evaluator SHALL identify an event-triggered instance by the clause subject
and the semantic trigger-event identity, and SHALL NOT identify it by timestamp,
display name, delivery receipt or equal captured payload.

The evaluator SHALL create no second instance for a repeated delivery of the same
semantic trigger, and SHALL retain that delivery's provenance on the existing
instance. The evaluator SHALL create a distinct instance for a distinct semantic
trigger even when its captured values compare equal.

The evaluator SHALL evaluate captures in authored source order at the activation
anchor exactly once, and SHALL retain each capture's declared expression handle,
type, value, source binding and anchor. A later mutable observation SHALL NOT
replace a captured value, and one instance's captures SHALL NOT satisfy another
instance's propositions.

If a declared capture cannot be established because its input is missing,
nullable, wrongly typed, stale or anchor-mismatched, then the evaluator SHALL
report an incomplete or refused activation naming that capture, and SHALL NOT
begin temporal evaluation for that instance.

The evaluator SHALL report `inactive` only for a trigger scope that is
authoritatively closed and complete with no admitted trigger. The evaluator SHALL
report `unknown` with an open completeness disposition for an open trigger scope,
and `unknown` with its incomplete or refused assessment-execution disposition for
missing or refused trigger evidence. Neither disposition SHALL become temporal
truth or evidence that recovery behavior was exercised.

Where a declaration is source-valid but its requested backend mapping is
unsupported, the evaluator SHALL retain the native declaration subject, its
selected profile and its activation record on the refusal, and SHALL NOT
substitute a profile, clock or reduced native form.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-044-AC-1 | Two distinct semantic triggers carrying equal captured amounts create two distinct obligation instances, and a value captured by one instance cannot satisfy the other instance's proposition. | Test (TC-123) |
| FR-044-AC-2 | A duplicate delivery of one semantic trigger creates no second instance and retains the duplicate's receipt provenance on the existing instance. | Test (TC-123) |
| FR-044-AC-3 | Mutating the source observation after activation leaves the already captured value, reference and predicate valuation environment unchanged. | Test (TC-123) |
| FR-044-AC-4 | A missing, nullable, wrong-type, stale or anchor-mismatched capture input reports incomplete or refused activation naming that capture, before any temporal evaluation runs for the instance. | Test (TC-123) |
| FR-044-AC-5 | A closed-complete trigger scope with no trigger reports `inactive`; an open scope reports `unknown` with open completeness; missing or refused trigger evidence reports `unknown` with its own incomplete or refused execution disposition; none of the three carries temporal truth. | Test (TC-123) |
| FR-044-AC-6 | Captures evaluate in authored source order exactly once at the anchor, and a forward read of a later capture from an earlier initializer refuses. | Test (TC-123) |
| FR-044-AC-7 | A source-valid past-time or timestamped declaration whose requested mapping reports unsupported retains its native declaration subject, selected profile and activation record on the refusal, with no profile or clock substitution. | Test (TC-123) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) emits the
  activation record, ordered captures and their binding requirements.
- **Peer**: [FR-043](./FR-043-evaluate-bounded-native-temporal.md) consumes the
  activated instance's immutable capture environment.
- **Constrained by**:
  [NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md) for the
  active-instance and retained-capture limits.
- Shared source requirement: `ix://agent-ix/quire-specification/FR-093`. Agent B
  owns protocol activation and participation result contracts; agent F owns
  trigger observation binding and duplicate-delivery provenance transport.
