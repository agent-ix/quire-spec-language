---
id: FR-516
title: "Carry the temporal evidence request and result on the checked-package route"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-516: Carry the temporal evidence request and result on the checked-package route

## Description

FR-052 and FR-053 state that the checked-package route owns temporal truth,
settlement, support and opaque trigger identity, and say they "do not prescribe a
new public API". quire-protocol needs one: its assessment builds a
`native_temporal::v2::Input` from observation evidence, receives a
`ValidatedResult` with activation state, truth, settlement and decision support,
and joins them to its own result record (`src/assessment.rs`, `src/conformance/kernel.rs`,
IT-002). QSL-303 deleted the documents (`quire.native-temporal-request/v1`, `/v2`,
`quire.native-temporal-result/v1`, `/v2`) and nothing replaced the typed input and
output. This requirement specifies them as Rust values on the evaluator, with no
JSON document, schema or content-derived identity string.

The semantics stay where they are: FR-043 (truth), FR-044 (activation and
captures), FR-052 and FR-053 (ownership and opaque trigger identity), FR-501 and
FR-502 (finite-window profile and progress ledger). This requirement states the
shape that carries them and the retained facts, and ends the document layer.

## Semantic authority and boundary

QSL derives truth, settlement and support. The observation owner selects the
trigger identity and the evidence references; QSL treats both as opaque (FR-053).
QSL does not authenticate observation evidence.

## Inputs

`qsl_eval::temporal::evidence::EvidenceRequest`, built by the caller, with no
result field:

| Retired `v2::Input` field | Successor field | Note |
| --- | --- | --- |
| `trigger: SemanticTriggerIdentity` | `trigger: SemanticTriggerIdentity` (non-empty opaque bytes) | unchanged |
| `correspondence: EvidenceRef` | `correspondence: EvidenceRef` | `EvidenceRef` keeps `identity`, `authority_identity`, `authority_revision`, `scope`, `population`, and carries `contract` (text), `schema_digest`, `digest` and `authority_digest` (byte strings) as opaque caller-supplied members: QSL does not compute, verify or interpret them (it does not authenticate observation evidence), echoes each byte for byte in the result's support references, and no truth, settlement or support value depends on them. Package content digests stay removed (QSL-1270); these are the observation owner's identifiers, not QSL's |
| `positions: Vec<ObservedPosition>` | `positions` | each `{ observation: EvidenceRef, position }` |
| `anchor` | `anchor` | the authored activation anchor |
| `triggers: Vec<TriggerInput>` | `delivery: TriggerDelivery` | exactly one: `{ receipt, anchor, payload, guard: Option<bool>, captures }`; the retired vector admitted exactly one, and the type now states it |
| `trigger_evidence`, `trigger_scope` | same | `Evidence` and `Closure` of the retained `temporal` vocabulary, independent of the decision scope |
| `decision_progress`, `decision_closure`, `surrounding_progress`, `surrounding_closure` | same | four independent axes (FR-052-AC-10) |
| `execution` | `execution` | `Completed`/`Failed` disposition |
| `completeness` | `completeness` | `{ reference, state, facts }` |
| `authoritative_origin`, `evicted` | same | |

The request names the clause by `CheckedNodeId` (FR-515); the evaluator takes the
profile, clock row, activation, captures and formula from the package, never from
the request (FR-337). The retired `instance` text of `/1` is not carried: an
instance is identified by the trigger identity (FR-053).

## Outputs

`EvidenceResult`:

| Retired result accessor | Successor field |
| --- | --- |
| `activation()` | `activation: ActivationState` = `Inactive`, `Unknown`, `Active`, `Unavailable` |
| `execution()` | `execution` |
| `truth()` | `truth: Option<Truth>` = `True`, `False`, `Pending` |
| `non_value()` | `non_value: Option<NonValue>` with kind `Inactive`, `ActivationUnknown`, `Missing`, `Refused`, `Unactivated`, a code and an optional dimension |
| `settlement()` | `settlement: Option<Settlement>` = `ClosedScope`, `DecisiveWitness`, `DecisiveCounterexample`, `Unsettled`, `Unavailable` |
| `support()` | `support: Vec<SupportRef>` (the fact references inside the decision support) |
| `decision_progress()` … `surrounding_closure()`, `completeness()` | the four axes and the completeness record, echoed exactly |
| `limits()` | `usage`: the meter charges of the call, over the thirteen counters of the Work limits item |
| `predecessor()`, `lineage()`, `relation_kind()` | `relation: Original | Superseding(prev) | Invalidating(prev)`; `lineage: u64` |
| `request_identity()`, `subject_identity()`, `document().*` | `result_identity` as FR-043-AC-13 states it (an evaluator identity, not a digest); the rest end with the documents |

Entry: `CheckedPackageEvaluation::evaluate_temporal_evidence(&self, clause:
&CheckedNodeId, request: &EvidenceRequest, meter: &mut Meter) ->
Result<EvidenceResult, CallFailure>`; **Refusal has one outcome.** Every refusal of the call itself is
`Err(CallFailure::Admission)`, nothing is charged and no result exists: a clause
not held by the package or not a temporal clause, a request that fails
construction (an empty trigger identity), a selection or supply refused before S6a
(FR-390), and each lineage refusal below. `activation = Unavailable` is a result,
not a refusal: the call ran and a fact the clause needs was withdrawn, or the
obligation was never activated.

## Behavior

- **Activation.** `Inactive` when no trigger was admitted within a closed,
  complete trigger scope; `Unknown` when the scope was open or trigger evidence
  was missing or refused; `Active` when one semantic trigger was admitted or the
  whole-execution origin activated the obligation; `Unavailable` when a fact the
  clause needs was withdrawn from the request or the obligation was never
  activated. No other path
  assigns an activation state (FR-044).
- **No supplied truth.** The request has no truth, settlement or support field;
  the result's three are derived from the package and the evidence
  (FR-052-AC-9).
- **Independent facts.** Each of the four axes, the trigger scope, the
  completeness record, the anchor, the clock row and the execution disposition is
  returned as its own field; an absent fact yields a non-value, never a default
  false, closed, settled or complete (FR-052-AC-10).
- **Opaque trigger.** The result echoes the trigger identity byte for byte;
  an empty identity is refused at construction; repeated delivery of one identity
  with an equal payload is the existing instance; a different payload under one
  identity is the FR-044 contradiction (FR-053-AC-7 to AC-9).
- **Lineage.** A superseding or invalidating result names its direct
  predecessor's `result_identity`; a self-edge, a predecessor of another
  obligation and an identity mismatch are each `Err(CallFailure::Admission)`; the
  predecessor is unchanged
  (FR-052-AC-12).
- **Work limits.** None of the retired fourteen dimensions is dropped. Four
  bounded the JSON documents and become counters on the typed values:
  `input_bytes` becomes `request_bytes` (the total of the request's opaque byte
  members), `output_bytes` becomes `result_bytes` (the same over the result), and
  `string_bytes` becomes `member_bytes` (the longest single opaque member).
  `json_depth` has no successor counter because a typed value has the fixed
  nesting its type gives, so there is no depth to bound. Ten
  are carried unchanged: `formula_nodes`, `formula_depth`, `positions`, `valuations`,
  `captures`, `support`, `history_span`, `evaluation_steps`, `lineage`, `visited`
  are named temporal work counters (QSL-1502, NFR-008). Each of the thirteen
  counters completes at exactly
  its measured usage, and one below it returns `Incomplete` naming the counter
  with no partial truth.
- **Contract ids.** The four contract strings have no producer after this
  requirement; quire-protocol's producer-contract registry rows for them end with
  QSL-1549, not before (FR-521 row 4).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-516-AC-1 | A compile-fail test that sets `truth`, `settlement`, `support` or `instance` on `EvidenceRequest` fails to compile, and the type has no member holding a QSL document or a QSL-computed digest. | Test (TC-1311) |
| FR-516-AC-2 | Over **R**'s event-position clause with a closed complete trigger scope and no delivery, the result has `activation = Inactive`, `truth = None` and `non_value.kind = Inactive`; with an open trigger scope `Unknown` and `ActivationUnknown`; with one admitted delivery `Active` and a truth; with the decision closure withdrawn `Unavailable`; a request naming a clause the package does not hold returns `Err(CallFailure::Admission)` with an empty charge log and no result. | Test (TC-1311) |
| FR-516-AC-3 | With one fact withdrawn from an otherwise Active request (decision closure, completeness record, a support fact), the result truth, settlement and support are the FR-043 values of the remaining evidence, the result carries a `Missing` non-value for the withdrawn fact, and no field defaults to false, closed, settled or complete. | Test (TC-1311) |
| FR-516-AC-4 | Two requests that differ only in one byte of the trigger identity give two different `result_identity` values; equal timestamps, receipts, payloads and captures under two identities give two instances, a repeated delivery with an equal payload gives the existing instance, a different payload under one identity is the FR-044 contradiction, and an empty identity is refused at construction. | Test (TC-1311) |
| FR-516-AC-5 | The decision and surrounding progress and closure axes, the trigger scope and the completeness record are each echoed in the result exactly as supplied, and changing one of them changes only its field and the derived truth where FR-043 says it does. | Test (TC-1311) |
| FR-516-AC-6 | A superseding result carries its direct predecessor's `result_identity` and `lineage + 1`; a self-predecessor, a predecessor of another obligation and a mismatched identity each return `Err(CallFailure::Admission)` with an empty charge log, and the predecessor result is unchanged. | Test (TC-1311) |
| FR-516-AC-7 | For each of the thirteen work counters (the ten carried and `request_bytes`, `result_bytes`, `member_bytes`), the call at exactly the measured usage completes and the call one below returns `Incomplete` naming that counter with `truth = None`; a sibling clause evaluated in the same package keeps its own result. | Test (TC-1311) |
| FR-516-AC-8 | Over **R**'s timestamped-event clause, the request/result path of FR-501 and FR-502 (finite window, regressing watermark, conflicting completeness revision) returns the outcomes those requirements state through `evaluate_temporal_evidence`, and the result for the in-process package equals the result for the read-back package, charge log included. | Test (TC-1311) |
| FR-516-AC-9 | A request whose `EvidenceRef` members `contract`, `schema_digest`, `digest` and `authority_digest` are arbitrary byte strings, including empty ones, is evaluated; the result echoes each byte for byte, and changing any one of them changes no truth, settlement, support or activation value, only the echoed member. | Test (TC-1311) |

## Dependencies

- **Upstream:** FR-043, FR-044, FR-052, FR-053, FR-501, FR-502, FR-515; the
  evaluator ports QSL-1502, QSL-1503 and QSL-1504 (FR-499 rows), which this
  requirement does not replace.
- **Downstream:** quire-protocol `src/assessment.rs`, `src/conformance/kernel.rs`,
  `tests/it002_observation_temporal.rs` (QSL-1549).
