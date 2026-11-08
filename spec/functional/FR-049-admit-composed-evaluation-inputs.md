---
id: FR-049
title: "Route composed evaluation through checked-term runtime inputs"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-027, type: references }
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
entry point.

This requirement ties the entry points together for
[FR-046](FR-046-execute-predicates-and-ordered-queries.md),
[FR-047](FR-047-evaluate-finite-object-reference-graphs.md),
[FR-048](FR-048-preserve-native-choreography-semantics.md) and
[FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md).
Under [ADR-011 §7.3, M-6c](../decisions/ADR-011-stage-dag-and-dependency-architecture.md#L1250)
and [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md) the
artifact-graph evaluators are deleted, and no parallel or compatibility
evaluation API exists.

## Inputs

Checked-term evaluation receives only the checked package and expression
identity selected by its owning entry point (`CheckedPackageEvaluation::call`,
`evaluate` or `evaluate_protocol_slot`, or the applicable checked-family
function such as `evaluate_closed_with_frame`, `evaluate_state_predicate` or
`evaluate_refinement_row`). Family evaluation receives its
typed checked family form and family-specific immutable inputs. Protocol-node
evaluation receives the `ProtocolSlot`, `ProtocolSupply` and caller-supplied
`Meter` specified by FR-390. Clause execution obtains its model environment
through the clause-run contract, never from a free-standing artifact graph. Model authority remains owned by
[FR-056](FR-056-admit-domain-package-model-declarations.md), and snapshot and
invocation authority remains owned by
[FR-106](FR-106-admit-snapshots-and-invocations.md) and its selected
observation contract; QSL does not mint or duplicate that authority.

Compile-time identities are established at their owning boundaries, never
supplied again at evaluation: the source and compile-configuration provenance
by [FR-036](FR-036-link-composed-native-packages.md) and FR-510, the model
package identity by FR-056, the header profile identity by FR-110, and the
snapshot and invocation document identity by FR-106. Their refusal causes and
loci apply before evaluation, and checked evaluation accepts no free-standing
document, model, profile or configuration artifact. Typed temporal clock
configurations are a distinct FR-050 contract.

The entry points and their owners are:

| Concern | Owner and exercised entry point |
| --- | --- |
| Selecting a checked expression and evaluating it | The checked source pipeline selects a `CheckedPackage` and checked expression; `CheckedPackageEvaluation::{call, evaluate}` plus the applicable checked-family function such as `evaluate_closed_with_frame`, `evaluate_state_predicate` or `evaluate_refinement_row` (FR-046/FR-047; TC-136) |
| Runtime binders, populations, objects, fields and values | `ObjectEnvironment` and the checked clause environment from FR-107; family inputs are owned by FR-046/FR-047 and observation selections by FR-106 (TC-136) |
| Object and observation identity | The full object and observation identities admitted by FR-106 and retained by FR-107; duplicate and cross-observation cases are exercised in TC-136 (FR-106 AC-1/AC-2 and FR-107 AC-1/AC-2) |
| Document, model, profile and configuration identity | The checked source package (FR-036), the selected domain `ModelSelection` (FR-056), the `ProfileSelection` resolved against the closed definition catalog (FR-110) and the retained compile configuration provenance (FR-036-AC-5); each owner keeps its own typed refusal (TC-136) |
| Snapshot and invocation evidence | FR-106's selected `AdmittedObservations`, invocation identity and population manifests; snapshot and invocation documents enter only through FR-106's typed admission (TC-136) |
| Compensation guards, captures, retry and recovery expressions | `CheckedPackageEvaluation::evaluate_protocol_slot` with `ProtocolSlot`, `ProtocolSupply` and shared `Meter` (FR-390, TC-1070–TC-1073; TC-142) |
| Typed results | The owning entry point's `Evaluation`, `FamilyOutcome` and `CallFailure` categories; FR-107/FR-390 (TC-136, TC-1071–TC-1073) |
| Unpublished or unadmitted packages | Checked evaluation selects only a `CheckedPackage` with the identity established by its compile/read path. FR-027/FR-264 own checked-package publication and reading; FR-107 owns the evaluation result. TC-142 verifies strict read-back and evaluation through that admitted package |
| Bounding and accounting evaluation work | Caller-owned `quire-exact::Meter` and the shared `quire.value.accounting/v1` contract; FR-390 and NFR-009 (TC-137, TC-1073) |

TC-134 covers compiler emission and independent reader coverage; evaluator calls
are covered by TC-1070 through TC-1073.

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

No evaluation entry point SHALL accept a native artifact graph. Evaluation is
reached only through the checked source pipeline and its FR-046/047/048/390
entry points.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | Each checked-term evaluation selects an exact checked declaration/expression identity; raw bytes, freely constructed wire records and an out-of-range checked handle cannot enter an evaluation session. A valid `CheckedExpression` produced by another checked graph is refused before evaluation as `invalid_runtime_input`/`foreign-checked-expression`. | Test (TC-136) |
| FR-049-AC-2 | Runtime binders and model/observation values are admitted only through the owning checked environment and authority contracts; duplicate, surplus, foreign and wrong-type inputs refuse at admission, and concrete population/window/clock roles remain distinct. The `call`, `call_supplied` and `evaluate` entries admit the caller's object environment whole at entry, before any charge: an object of a type the package does not declare, or a missing, surplus or ill-typed attribute, refuses `invalid_runtime_input`/`wrong-value-kind` naming the object and attribute. The `evaluate_holds`, `evaluate_guard`, `evaluate_hyper_holds`, `evaluate_protocol_slot` and clause entries evaluate over state FR-106 admitted, and refuse an ill-typed attribute value with the same code and cause when it is read, never as an internal fault or a value of another type or unit. | Test (TC-136) |
| FR-049-AC-3 | Available Boolean, numeric, text, enum, record, option, sequence, reference and object values satisfy their checked type, unit, bound and field contracts without implicit conversion, normalization or duplicate coalescing. | Test (TC-129, TC-136) |
| FR-049-AC-4 | Unavailable values preserve their typed missing identity and affect evaluation only when reached; they never become null, false, zero, empty or a completed output member. | Test (TC-136) |
| FR-049-AC-5 | Graph evaluation uses the complete observation/model/universe/object-type/object identity, rejects duplicate storage keys, permits distinct observation occurrences, and distinguishes a missing target in a complete domain from unavailable membership or closure. Static and assessment identity authority remains owned by FR-056 and the selected observation contract. | Test (TC-129, TC-136) |
| FR-049-AC-6 | Every checked-term entry returns its owning typed result; completed values are distinct from unavailable input, admission refusal, incomplete results and internal faults, and an incomplete result caused by `resource_exhausted` retains that cause; no caller classifies results from diagnostic text. | Test (TC-136, TC-1071–TC-1073) |
| FR-049-AC-7 | Each applicable `accounting.<counter>` setting in FR-255's shared `quire.value.accounting/v1` table has zero/no-work, exact-usage and one-short evidence; over-budget work produces no completed partial value and stops at the owning requirement's charge point. | Test (TC-137, TC-1073) |
| FR-049-AC-8 | Repeating an evaluation over the same immutable checked package and runtime environment with a fresh sufficient meter returns the same result and leaves inputs unchanged; sequential protocol-slot calls on one meter accumulate in call order. | Test (TC-137, TC-1073) |
| FR-049-AC-9 | A checked protocol package evaluates its exact compensation guard, capture, retry and recovery slots through FR-390's `evaluate_protocol_slot`; crossed slot/supply selections refuse before charge, unavailable inputs and resource stops remain distinct, and no artifact-graph evaluator or compatibility entry point exists. | Test (TC-142, TC-1070–TC-1073) |
| FR-049-AC-10 | The compile path retains the selected source, model, profile and configuration identities under FR-036/FR-056/FR-110/FR-050; observation identities are admitted by FR-106. No runtime API accepts independent document, model, profile or configuration identity members. | Test (TC-115, TC-136, TC-138, TC-465, TC-490) |
| FR-049-AC-11 | Model and observation document defects are refused by their owning intake boundary with that owner's cause and locus before evaluation; no separate four-member identity validator runs at evaluation. A checked evaluation uses only the identities from the successful owning admissions. | Test (TC-136, TC-465) |

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
