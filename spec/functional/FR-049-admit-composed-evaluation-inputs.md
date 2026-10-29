---
id: FR-049
title: "Route composed evaluation through checked-term runtime inputs"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-106, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-107, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-255, type: references }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: depends_on }
  - { target: ix://agent-ix/quire-protocol/FR-003, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-405, type: references }
  - { target: ix://agent-ix/quire-spec-language/ADR-011, type: references }
---
# FR-049: Route composed evaluation through checked-term runtime inputs

## Description

When a caller evaluates a checked expression or protocol slot, QSL SHALL route
it through the `qsl-eval` checked-term APIs, which dispatch checked-term evaluation to `quire-eval`, with the exact checked identity,
typed runtime environment and caller-owned accounting context required by that
entry point. The retired native `state` artifact-graph evaluator has no
parallel replacement API.

This requirement ties the replacement entry points together for
[FR-046](FR-046-execute-predicates-and-ordered-queries.md),
[FR-047](FR-047-evaluate-finite-object-reference-graphs.md),
[FR-048](FR-048-preserve-native-choreography-semantics.md) and
[FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md).
The retirement follows [ADR-011 §7.3, M-6c, row 1250](../decisions/ADR-011-stage-dag-and-dependency-architecture.md#L1250)
and [FR-405, “The state after the switch,” item 1, lines 156–163](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md#L156):
the artifact-graph evaluators are deleted under M-6c. The old API names below
describe the boundary being retired; they do not preserve that boundary or add
a compatibility façade.

## Inputs

Checked-term evaluation receives only the checked package and expression
identity selected by its owning entry point (`CheckedPackageEvaluation::call`,
`evaluate` or `evaluate_protocol_slot`, or the applicable checked-family
function such as `evaluate_closed`, `evaluate_state_predicate` or
`evaluate_refinement_row`). Family evaluation receives its
typed checked family form and family-specific immutable inputs. Protocol-node
evaluation receives the `ProtocolSlot`, `ProtocolSupply` and caller-supplied
`Meter` specified by FR-390. Clause execution obtains its model environment
through the clause-run contract; it does not convert a free-standing native
`StateView` or artifact graph. Model authority remains owned by
[FR-056](FR-056-admit-domain-package-model-declarations.md), and snapshot and
invocation authority remains owned by
[FR-106](FR-106-admit-snapshots-and-invocations.md) and its selected
observation contract; QSL does not mint or duplicate that authority.

The retiring public Rust surface comprises `evaluate`/`evaluate_v2`;
`EvaluationRequest`; `ObservationKey`, `ObservationIdentity`, `ObjectKey`,
`CanonicalDigest` and `ObservationDigest`; `StaticAuthority`,
`StaticAuthorityMember`, `StaticAuthorityDigestCause`, `AssessmentAuthority`,
`AuthorityEvidence`, `AuthorityAdapter`, `AdapterArtifact` and
`InvalidAdapterArtifact`; `MissingInput`, `ValuePathSegment`, `InputSlot`,
`ContextualSlot`, `ContextualValue`, `ContextualValueKind`, `FieldValue`,
`FieldInput`, `Value`, `ValueKind`, `BinderInput`, `ObjectInput`,
`PopulationInput` and `StateView`; `Refusal`, `EvaluationOutcome`,
`EvaluationReport`; and `Limits`, `Usage`, `Dimension`, `Exhaustion`,
`ExhaustionCause` and `ACCOUNTING_VERSION`. The replacement mapping is:

| Retired state capability | Replacement owner and exercised entry point |
| --- | --- |
| `evaluate` / `evaluate_v2`, `EvaluationRequest` | The checked source pipeline selects a `CheckedPackage` and checked expression; `CheckedPackageEvaluation::{call, evaluate}` plus the applicable checked-family function such as `evaluate_closed`, `evaluate_state_predicate` or `evaluate_refinement_row` (FR-046/FR-047; TC-136) |
| `StateView`, `BinderInput`, `PopulationInput`, `ObjectInput`, `FieldInput`, `FieldValue`, `InputSlot`, `ContextualSlot`, `ContextualValue`, `ContextualValueKind`, `Value`, `ValueKind`, `ValuePathSegment`, `ObjectKey`, `ObservationKey`, `ObservationIdentity` | The `ObjectEnvironment` and checked clause environment from FR-107, with the family inputs owned by FR-046/FR-047 and the observation selections owned by FR-106 (TC-136) |
| `StaticAuthority`, `StaticAuthorityMember`, `StaticAuthorityDigestCause`, `AssessmentAuthority`, `AuthorityEvidence`, `AuthorityAdapter`, `AdapterArtifact`, `InvalidAdapterArtifact`, `CanonicalDigest`, `ObservationDigest` | FR-056 selects static declarations; FR-106 owns snapshot/invocation and assessment authority; FR-107/FR-390 admit clause environments and protocol-slot supplies (TC-136, TC-1071) |
| Evaluate compensation guards, captures, retry and recovery expressions | `CheckedPackageEvaluation::evaluate_protocol_slot` with `ProtocolSlot`, `ProtocolSupply` and shared `Meter` (FR-390, TC-1070–TC-1073); TC-142's eval portion is re-homed there |
| `MissingInput`, `Refusal`, `EvaluationOutcome`, `EvaluationReport` | The owning checked-term entry point's typed `Evaluation`, `FamilyOutcome` and `CallFailure` categories; FR-107/FR-390 (TC-136, TC-1071–TC-1073) |
| Bound and account evaluation work | Caller-owned `quire-exact::Meter` and the shared accounting contract; FR-390 and NFR-009 (TC-137, TC-1073) |

TC-134 retains compiler emission and independent reader coverage only; its
evaluator calls are covered by TC-1070 through TC-1073. No old `/1` or `/2`
artifact-graph evaluator behavior remains an entry point.

## Outputs

The result shapes owned by checked-term and family APIs, as FR-107 and FR-390
define them. A non-complete result never exposes a partial completed value;
callers use the typed result and its cause rather than diagnostic text.

## Behavior

Each evaluation entry point SHALL accept only checked terms or protocol slots
whose checked identity is established by its owning compile/read path.

Family entry points SHALL preserve the evaluation semantics of their owning
requirement: ordered query stopping and partial-result discard under FR-046,
finite graph identity and closure under FR-047, and choreography expression
semantics under FR-048/FR-390. They SHALL receive no semantic meaning from a
freely constructed artifact graph or reparsed expression text.

The checked-term admission path SHALL admit runtime values and model fields
against their checked types and selected FR-056/FR-106 model and observation
authority at the relevant clause or protocol-slot boundary. Missing or
unavailable input SHALL remain distinct from a completed value, invalid
admission, resource exhaustion and an internal fault; source-order and
short-circuit behavior remains as specified by the owning family requirement.

All evaluation work SHALL use the caller's existing accounting context. A
retry that starts a new evaluation uses a fresh meter; protocol slots evaluated
as one operation share the same meter and accumulate charges in call order.
The evaluator SHALL borrow immutable checked packages and environments and
retain no mutable process-global evaluation state.

No compatibility API SHALL keep the retired `state` evaluator reachable.
Native `/1` and `/2` artifact-graph inputs are not converted into checked terms
to preserve the old entry points; the supported replacement is the checked
source pipeline and its FR-046/047/048/390 entry points.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | Each checked-term evaluation selects an exact checked declaration/expression identity; raw bytes, freely constructed wire records and a foreign or out-of-range checked handle cannot enter an evaluation session. | Test |
| FR-049-AC-2 | Runtime binders and model/observation values are admitted only through the owning checked environment and authority contracts; duplicate, surplus, foreign and wrong-type inputs refuse at admission, and concrete population/window/clock roles remain distinct. | Test |
| FR-049-AC-3 | Available Boolean, numeric, text, enum, record, option, sequence, reference and object values satisfy their checked type, unit, bound and field contracts without implicit conversion, normalization or duplicate coalescing. | Test |
| FR-049-AC-4 | Unavailable values preserve their typed missing identity and affect evaluation only when reached; they never become null, false, zero, empty or a completed output member. | Test |
| FR-049-AC-5 | Graph evaluation uses the complete observation/model/universe/object-type/object identity, rejects duplicate storage keys, permits distinct observation occurrences, and distinguishes a missing target in a complete domain from unavailable membership or closure. Static and assessment identity authority remains owned by FR-056 and the selected observation contract. | Test |
| FR-049-AC-6 | Every checked-term entry returns its owning typed result; completed values are distinct from unavailable input, admission refusal, incomplete results and internal faults, and an incomplete result caused by `resource_exhausted` retains that cause; no caller classifies results from diagnostic text. | Test |
| FR-049-AC-7 | Each applicable shared-meter counter has zero/no-work, exact-usage and one-short evidence; over-budget work produces no completed partial value and stops at the specified charge point. | Test |
| FR-049-AC-8 | Repeating an evaluation over the same immutable checked package and runtime environment with a fresh sufficient meter returns the same result and leaves inputs unchanged; sequential protocol-slot calls on one meter accumulate in call order. | Test |
| FR-049-AC-9 | A checked protocol package evaluates its exact compensation guard, capture, retry and recovery slots through FR-390's `evaluate_protocol_slot`; crossed slot/supply selections refuse before charge, unavailable inputs and resource stops remain distinct, and no `/1` or `/2` state evaluator or compatibility entry point exists. | Test |
| FR-049-AC-10 | Evaluation uses only static declaration identities selected by the FR-056 intake path and assessment identities supplied by their owning observation contract; replacing a selected identity independently is refused before evaluation. | Test |
| FR-049-AC-11 | The checked-term entry path is unreachable when FR-056 refuses static package authority; the FR-056 malformed, wrong-domain and mismatched-identity cases preserve their declared cause and locus at the evaluation boundary. | Test |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md) and
[FR-040](FR-040-check-composed-values.md) own the static declaration/type graph.
The domain package and observation system retain their own identities and
meanings. FR-046/047 own query and graph semantics; FR-390 owns checked
protocol-slot evaluation and shared-meter behavior; FR-107 owns checked-term
result categories and the clause environment; FR-056 owns static domain-package
identity, while FR-106 owns snapshot and invocation authority.

`quire-protocol` [FR-003](ix://agent-ix/quire-protocol/FR-003) consumes the
checked protocol evaluation contract. QSL owns source checking and checked-term
entry-point selection; `qsl-eval` exposes the family entry points and dispatches
checked-term execution to `quire-eval`. The consumer neither reparses source nor
mirrors a retired artifact graph.
