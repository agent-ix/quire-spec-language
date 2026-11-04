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
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-526
    type: references
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
| `trigger: SemanticTriggerIdentity` | `trigger: Option<SemanticTriggerIdentity>` (non-empty opaque bytes when present) | selects the instance the call assesses among the resolved instances; for a clause that activates `on origin` it is the execution-origin identity; required when the request carries a delivery or an origin, and `None` when it carries none |
| `correspondence: EvidenceRef` | `correspondence: EvidenceRef` | `EvidenceRef` keeps `identity`, `authority_identity`, `authority_revision`, `scope`, `population`, and carries `contract` (text), `schema_digest`, `digest` and `authority_digest` (byte strings) as opaque caller-supplied members: QSL does not compute, verify or interpret them (it does not authenticate observation evidence), echoes each byte for byte in the result's support references, and no truth, settlement or support value depends on them. Package content digests stay removed (QSL-1270); these are the observation owner's identifiers, not QSL's |
| `positions: Vec<ObservedPosition>` | `positions` | each `{ observation: EvidenceRef, position, instance_valuations }`; `instance_valuations` is FR-524's map from a trigger identity to that instance's valuations at the position |
| `anchor` | `anchor` | the authored activation anchor: the `EvidenceRef` identity of one of the request's positions, which fixes the activation position ("Assessed instance") |
| `triggers: Vec<TriggerInput>` | `deliveries: Vec<TriggerDelivery>` (clause `on each`) or `origins: Vec<ExecutionOrigin>` (clause `on origin`, each `{ identity, payload, captures }`) | the ordered deliveries, each `{ identity, receipt, anchor, payload, guard: Option<bool>, captures }`, so that a repeated delivery of one identity and several instances are expressible (FR-524). The retired native request admitted exactly one trigger; the legacy evaluator (`temporal::Trace.triggers`) admitted many, and this field is the successor of that list |
| `trigger_evidence`, `trigger_scope` | same | `Evidence` and `Closure` of the retained `temporal` vocabulary, independent of the decision scope, with the completeness of the trigger record (`Complete` or `Incomplete`) as a third member of the scope (FR-524) |
| `decision_progress`, `decision_closure`, `surrounding_progress`, `surrounding_closure` | same | four independent axes (FR-052-AC-10) |
| `execution` | `execution` | `Completed`/`Failed` disposition |
| `completeness` | `completeness` | `{ reference, state, facts }` |
| `authoritative_origin`, `evicted` | same | |
| none | `progress_ledger: Option<ProgressLedger>` | FR-502's ledger of watermark, completeness and closure per binding; the call applies FR-502 to the request's progress assertion and returns the updated ledger |
| none | `ledger: Option<EvidenceLedger>` | the caller-held ledger of retained instances (FR-524) and latest results (FR-526); the result returns the updated ledger |
| `relation` | `relation` | `Original`, `Superseding(prev)` or `Invalidating(prev)`, `prev` the predecessor's `result_identity` (FR-526) |
| none | `occurrence: Option<{ role, ordinal }>` | selects the clause occurrence of a node that several structurally equal clauses share, with the producer-assigned `{ role, ordinal }` of QSpec FR-322; the occurrence's clock row is the one keyed `(declaration, occurrence)`, never the k-th row by position. May be omitted for a node with one occurrence |
| none | `control: Option<CheckedNodeId>` | the protocol clause the caller selected; when present, the package's FR-449 activation-map row of that clause must name the call's clause (below) |
| none (the retired document asserted the profile and clock) | `binding: TraceBinding` | the profile, clock binding name and clock parameters the request asserts; the evaluator compares them with the clause's rows before any position (FR-522) |

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
| `predecessor()`, `lineage()`, `relation_kind()` | `relation` echoed with `prev`; `lineage: u64`; `input_identity` (FR-526); the updated `ledger` and `progress_ledger` |
| none | `authenticated: AuthenticatedBinding` (FR-522), present on every result because a refused binding returns no result |
| none | `instances`: every instance the lists resolved, each with its identity and activation state (`Active`, or `Unavailable` with its fault, a guard fault included), so that the siblings of the assessed instance stay inspectable (FR-524); a delivery with a false guard is not listed |
| none | `incomplete: Option<EvidenceIncomplete>`: the counter, meter, lineage or evicted-valuation stop, or the clause stop (`formula_nodes`, `history_span`, a tick sum that does not fit), that ended the call, naming what stopped it (a tick sum that does not fit names `history_span`); `None` when the call ran to its result, and `None` for a withdrawn anchor, which is a `Missing` non-value (Independent facts) |
| `request_identity()`, `subject_identity()`, `document().*` | `result_identity` as FR-043-AC-13 states it (an evaluator identity, not a digest); the rest end with the documents |

Entry: `CheckedPackageEvaluation::evaluate_temporal_evidence(&self, clause:
&CheckedNodeId, request: &EvidenceRequest, meter: &mut Meter) ->
Result<EvidenceResult, CallFailure>`; **Refusal has one outcome.** Every refusal of the call itself is
`Err(CallFailure::Admission)`, nothing is charged and no result exists: a clause
not held by the package or not a temporal clause, a request that fails
construction (an empty trigger identity), a selection or supply refused before S6a
(FR-390), a refused binding (FR-522), a delivery contradiction or a surplus
capture fact (FR-524), an occurrence selector that is absent for a node with several occurrences (`occurrence-required`) or names no occurrence of the node (`occurrence-unknown`), a
progress contradiction (FR-502), a `trigger-required` or `trigger-unknown`
refusal, a control that does not map to the clause (below) and each relation
refusal (FR-526). `activation = Unavailable` is a result,
not a refusal: the call ran and a fact the clause needs was withdrawn, or the
obligation was never activated. An `Incomplete` is likewise a result, never an
error: errors are for malformed input only, and a call that a counter, the meter,
the lineage ceiling or an evicted valuation stops returns `Ok` with
`activation = Unavailable`, `truth = None`, `settlement = None` and `incomplete`
naming the stop. The same holds for a stop at the clause (`formula_nodes`,
`history_span`, a tick sum that does not fit), which assesses no instance.
`activation` is never unset: every `Ok` result holds exactly one of the four
states. An `Incomplete` result returns the request's own `ledger` and
`progress_ledger` unchanged for every stop kind (counter, meter, evicted
valuation, clause stop, lineage): no instance, receipt, latest-result entry or
progress entry the call made before the stop is kept, so a retry under sufficient
ceilings from the same inputs is byte-equal to a call that never stopped
(FR-523).

## Behavior

- **Activation.** `Inactive` when no trigger was admitted within a closed,
  complete trigger scope; `Unknown` when the scope was open or trigger evidence
  was missing or refused; `Active` when one semantic trigger was admitted or the
  whole-execution origin activated the obligation; `Unavailable` when a fact
  that decides activation (the anchor or the clock row) was withdrawn from the
  request, when the assessed instance has a capture or guard fault (FR-524), or
  when the obligation was never activated, or when the call stopped as
  `Incomplete` (Outputs). A fact withdrawn after
  activation (decision closure, completeness record, a support fact) leaves
  `activation` as derived and yields a `Missing` non-value for that fact
  (Independent facts below). No other path
  assigns an activation state (FR-044).
- **Execution and trigger record.** The request's `execution` (`Completed` or
  `Failed`) is carried into an `Unknown` disposition (FR-524) and echoed in the
  result's `execution` field; it changes no `Inactive`, `Active` or `Unavailable`
  state and no truth. The trigger record's completeness decides whether a
  `Closed`, admitted scope with no instance is `Inactive` (`Complete`) or
  `Unknown(open)` (`Incomplete`).
- **Pending.** A finite-window verdict of `Pending` and one of `PendingNextEvent`
  (FR-504) both map to `Truth::Pending`; `PendingNextEvent` has no truth value of
  its own, and the result keeps no difference between them in `truth`.
- **No supplied truth.** The request has no truth, settlement or support field;
  the result's three are derived from the package and the evidence
  (FR-052-AC-9).
- **Anchor.** The `anchor` is the `EvidenceRef` identity of one observed position of
  the request. The activation position of a clause `on each` is that observation's
  index in the request's positions (the canonical order of FR-501 for a
  timestamped trace, the listed order for an event-position or fixed-sample
  trace), and the activation position of a clause `on origin` is 0. When the
  `anchor` is absent or names no observed position, the anchor is withdrawn and the
  result is `Unavailable` with a `Missing` non-value naming the anchor.
- **Assessed instance.** The instance activates once, at the activation position,
  so the call returns that one activation's outcome and combines no activations.
  `trigger` names the assessed instance among the resolved ones.
- A withdrawn anchor takes precedence: whatever the trigger scope and the
  deliveries, a request whose anchor is withdrawn returns `Unavailable`, and the
  scope disposition below applies only to a request whose anchor names an
  observed position.
- When the request carries no delivery and no origin, `trigger` is `None` and the
  anchor names an observed position, the call returns the scope disposition of FR-524 (`Inactive` or `Unknown`), no truth
  and the matching `non_value`.
- When the request carries a delivery or an origin and `trigger` is `None`, the call
  is refused with cause `trigger-required` as `Err(CallFailure::Admission)`.
- When `trigger` is present and names an identity that no delivery or origin of the
  request and no entry of the ledger holds, the call is refused with cause
  `trigger-unknown` as `Err(CallFailure::Admission)`.
- When `trigger` names a delivery whose guard is false, there is no instance and
  the result holds the activation disposition FR-524 gives a scope with no
  instance, no truth and the matching `non_value`.
- **Progress.** A regressing watermark, a conflicting completeness revision and a
  closure contradiction in the request's progress assertion are refused as FR-502
  states, as `Err(CallFailure::Admission)` with an empty charge log and the
  contradicted dimension in the failure; the progress ledger is unchanged.
- **Independent facts.** Each of the four axes, the trigger scope, the
  completeness record, the anchor, the clock row and the execution disposition is
  returned as its own field; an absent fact yields a non-value, never a default
  false, closed, settled or complete (FR-052-AC-10).
- **Opaque trigger.** The result echoes the trigger identity byte for byte;
  an empty identity is refused at construction; repeated delivery of one identity
  with an equal payload, in `deliveries` or against the ledger, is the existing
  instance; a different payload under one identity is the FR-044 contradiction
  (FR-053-AC-7 to AC-9; FR-524).
- **Lineage.** A superseding or invalidating result names its direct
  predecessor's `result_identity`, which must be the latest result the request's
  ledger holds for the obligation instance (FR-526); the predecessor is unchanged
  (FR-052-AC-12).
- **Control.** When `control` is present, the evaluator reads the package's FR-449
  activation-map row of that protocol clause. When the map has no row for it the
  call is refused with cause `activation-mapping-missing`, and when the row names
  another temporal clause it is refused with cause `activation-mapping-mismatch`,
  each as `Err(CallFailure::Admission)` (the successors of the legacy
  `MissingActivationMapping` and `ActivationMappingMismatch`).
- **Work limits.** Of the retired fourteen dimensions, twelve have a successor and
  two are retired with their reasons below, and one counter, `instances`, is added.
  Three bounded the JSON documents and become counters on the typed values:
  `input_bytes` becomes `request_bytes` (the total of the request's opaque byte
  members), `output_bytes` becomes `result_bytes` (the same over the result), and
  `string_bytes` becomes `member_bytes` (the longest single opaque member).
  `json_depth` has no successor counter because a typed value has the fixed
  nesting its type gives, so there is no depth to bound, and `formula_depth` has
  none because formula nesting is not a limit (ADR-030 D-1; NFR-008 Scope). Nine
  are carried unchanged: `formula_nodes`, `positions`, `valuations`,
  `captures`, `support`, `history_span`, `evaluation_steps`, `lineage`, `visited`;
  and `instances` is NFR-008's concurrent-instance counter, a real ceiling now that
  `deliveries` is a list. The thirteen are named temporal work counters whose units,
  settings and stop scope FR-523 defines. Each of the thirteen
  counters completes at exactly
  its measured usage, and one below it returns an `Incomplete` result naming the
  counter with no partial truth. A lookup at a silent sample (FR-504: a fixed-sample
  watermark sample with no observation, at which every atom reads false) is a
  `holds` lookup that finds no valuation: it charges the `valuations` counter and
  its `visited` unit and the meter at the `temporal.visit` charge point as every
  lookup does, and it charges no `evaluation_steps` unit and no `positions` unit, because it is not an evaluated position visit (FR-523).
  It adds no counter: the thirteen stay thirteen.
- **The in-process package.** The call reads the same facts from the in-memory
  checked package that the I2 write holds before any lock is emitted as from the
  package read back from the emitted bytes: the in-memory package carries, at
  construction, the `profile_selections` row of each clause and one
  `TemporalBinding` (clock) row per clause occurrence, the rows FR-515 reads.
  A `CheckedNodeId` plus the request's `occurrence` selects the clock row by its
  key `(declaration, occurrence)` (QSpec FR-322), not by position among the node's
  rows, so producer-assigned ordinals that differ from position pair correctly. For
  a node with one occurrence the selector may be omitted; for a node with several
  occurrences an absent selector is refused with `occurrence-required` and a
  selector naming no occurrence with `occurrence-unknown`. Two clauses with
  different nodes never read each other's rows.
- **Contract ids.** The four contract strings have no producer after this
  requirement; quire-protocol's producer-contract registry rows for them end with
  QSL-1549, not before (FR-521 row 4).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-516-AC-1 | A compile-fail test that sets `truth`, `settlement`, `support` or `instance` on `EvidenceRequest` fails to compile, and the type has no member holding a QSL document or a QSL-computed digest. | Test (TC-1311) |
| FR-516-AC-2 | Over **R**'s event-position clause with a closed complete trigger scope and no delivery, the result has `activation = Inactive`, `truth = None` and `non_value.kind = Inactive`; with an open trigger scope `Unknown` and `ActivationUnknown`; with one admitted delivery `Active` and a truth; with a delivery whose capture is missing `Unavailable` and a `Missing` non-value naming the capture; with the anchor withdrawn (absent or naming no observed position) `Unavailable`; a request naming a clause the package does not hold returns `Err(CallFailure::Admission)` with an empty charge log and no result. | Test (TC-1311) |
| FR-516-AC-3 | With one fact withdrawn from an otherwise Active request (decision closure, completeness record, a support fact), the result truth, settlement and support are the FR-043 values of the remaining evidence, the result carries a `Missing` non-value for the withdrawn fact, and no field defaults to false, closed, settled or complete. | Test (TC-1311) |
| FR-516-AC-4 | Two requests that differ only in one byte of the trigger identity give two different `result_identity` values, and an empty identity is refused at construction. | Test (TC-1311) |
| FR-516-AC-5 | The decision and surrounding progress and closure axes, the trigger scope and the completeness record are each echoed in the result exactly as supplied, and changing one of them changes only its field and the derived truth where FR-043 says it does. | Test (TC-1311) |
| FR-516-AC-6 | A superseding result carries its direct predecessor's `result_identity` and `lineage + 1`, and the predecessor result is unchanged. | Test (TC-1311) |
| FR-516-AC-7 | For each of the thirteen work counters (the nine carried, `instances` and `request_bytes`, `result_bytes`, `member_bytes`), the call at exactly the measured usage completes and the call one below returns `Incomplete` naming that counter with `truth = None`. | Test (TC-1311) |
| FR-516-AC-8 | Over **R**'s timestamped-event clause, FR-501's finite-window outcomes are returned through `evaluate_temporal_evidence`, the call's `temporal.visit` charges being FR-501-AC-12's plus one for each delivery or origin it resolves (FR-523). | Test (TC-1311) |
| FR-516-AC-9 | A request whose `EvidenceRef` members `contract`, `schema_digest`, `digest` and `authority_digest` are arbitrary byte strings, including empty ones, is evaluated; the result echoes each byte for byte, and changing any one of them changes no truth, settlement, support or activation value, only the echoed member. | Test (TC-1311) |
| FR-516-AC-10 | A request whose `deliveries` hold `t1` and `t2` with equal timestamps, receipts, payloads and captures resolves two instances in `instances`. | Test (TC-1311) |
| FR-516-AC-11 | A request whose `deliveries` hold `t1` twice with an equal payload resolves one instance holding both receipts. | Test (TC-1311) |
| FR-516-AC-12 | A request whose `deliveries` hold `t1` twice with different payloads returns `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1311) |
| FR-516-AC-13 | With the ledger of AC-6, a request that leaves the input unchanged, a request naming a predecessor that is not the latest, and a request for another obligation each return `Err(CallFailure::Admission)` with an empty charge log (FR-526 names the causes). | Test (TC-1311) |
| FR-516-AC-14 | A sibling clause evaluated in the same package after the AC-7 stop keeps its own result. | Test (TC-1311) |
| FR-516-AC-15 | A request whose `control` names the protocol clause whose FR-449 row names the call's clause is evaluated. | Test (TC-1311) |
| FR-516-AC-16 | A request whose `control` names a protocol clause with no FR-449 row is refused with cause `activation-mapping-missing`, and one whose row names another temporal clause is refused with `activation-mapping-mismatch`, each as `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1311) |
| FR-516-AC-17 | Through `evaluate_temporal_evidence` with a `progress_ledger`, a regressing watermark and a conflicting completeness revision are refused as FR-502-AC-1 and AC-2 state, with an empty charge log and the progress ledger unchanged. | Test (TC-1311) |
| FR-516-AC-18 | The result for the in-process package equals the result for the read-back package, charge log included. | Test (TC-1311) |
| FR-516-AC-19 | A request that carries `deliveries` and names a `trigger` that no delivery or ledger entry holds is refused with cause `trigger-unknown` as `Err(CallFailure::Admission)`. | Test (TC-1311) |
| FR-516-AC-20 | A request that carries `deliveries` and a `trigger` of `None` is refused with cause `trigger-required`. | Test (TC-1311) |
| FR-516-AC-21 | A request whose `trigger` names a delivery with a false guard returns no truth and the disposition of FR-524. | Test (TC-1311) |
| FR-516-AC-22 | For a clause `on each` whose `anchor` names the observation at index 2, the activation outcome reports position 2, and for a clause `on origin` it reports position 0 whatever the `anchor` names among the positions. | Test (TC-1311) |
| FR-516-AC-23 | A request whose `anchor` names no observed position returns `Unavailable` with a `Missing` non-value naming the anchor. | Test (TC-1311) |
| FR-516-AC-24 | A request with a closed complete trigger scope, no delivery, `trigger` `None` and an `anchor` naming no observed position returns `Unavailable`, not `Inactive`. | Test (TC-1311) |
| FR-516-AC-25 | A call stopped by a counter ceiling, by the meter, by the `lineage` ceiling, by an evicted required valuation, by `formula_nodes` and by `history_span` (including a tick sum that does not fit) each returns `Ok` with `activation = Unavailable`, `truth = None` and `incomplete` naming the stop (`history_span` for the last), and none returns `Err` or an unset activation. A request whose anchor names no observed position returns `Ok` with `activation = Unavailable`, a `Missing` non-value naming the anchor and `incomplete = None`. | Test (TC-1311) |
| FR-516-AC-26 | A finite-window verdict `PendingNextEvent` and a verdict `Pending` each return `truth = Pending`. | Test (TC-1311) |
| FR-516-AC-27 | Over a fixed-sample clause whose silent samples are read, the `evaluation_steps` usage equals the count of evaluated position visits, the meter's `temporal.visit` usage exceeds it by exactly the number of silent-sample lookups, and `usage` reports exactly thirteen counters. | Test (TC-1311) |
| FR-516-AC-28 | A request with a guard-`None` delivery and a healthy delivery lists both in `instances` (the first `Unavailable` with a guard fault), assessing the healthy one by `trigger`, and a request with a surplus capture fact returns `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1311) |
| FR-516-AC-29 | Over the in-memory package of the I2 write, before any emit, the clause's profile selection and `TemporalBinding` row equal the rows of the package read back from its bytes, and `evaluate_temporal_evidence` over it authenticates the clause's own `binding`. | Test (TC-1311) |
| FR-516-AC-30 | In a package with two temporal clauses of different formulas and different clock rows, each clause's `CheckedNodeId` pairs with its own row: a `binding` asserting the other clause's row is refused with the dimension FR-522 names, in the in-memory and in the read-back package alike. | Test (TC-1311) |
| FR-516-AC-31 | For each stop kind of AC-25 other than the withdrawn anchor, the result's `ledger` and `progress_ledger` equal the request's byte for byte, even when instances were created or a receipt was added before the stop, and the retry of FR-523-AC-11 equals the call that never stopped. | Test (TC-1311) |
| FR-516-AC-32 | Over twins of one node whose `occurrences()` lists ordinal 7 then ordinal 3 (declaration order) while the lock's clock rows ascend 3 then 7, with different clock rows, a request with `occurrence` ordinal 3 authenticates a `binding` equal to the row keyed ordinal 3 and refuses one equal to the row keyed ordinal 7 with the dimension FR-522 names, and the reverse for ordinal 7; a pairing by position fails this criterion. | Test (TC-1311) |
| FR-516-AC-33 | Over the twins of AC-32, a request with no `occurrence` returns `Err(CallFailure::Admission)` with cause `occurrence-required` and a request with ordinal 5 returns cause `occurrence-unknown`, each with an empty charge log; over a node with one occurrence a request with no `occurrence` is evaluated. | Test (TC-1311) |
| FR-516-AC-34 | `execution` `Failed` and `execution` `Completed` over an otherwise equal request with open trigger scope return `Unknown` carrying `Failed` and `Completed` respectively, and over an admitted delivery the two requests return the same activation and truth. | Test (TC-1311) |
| FR-516-AC-35 | Over a closed, admitted scope with no delivery, trigger record `Complete` returns `Inactive` and trigger record `Incomplete` returns `Unknown(open)`, each with the request's `execution` echoed. | Test (TC-1311) |

## Dependencies

- **Upstream:** FR-043, FR-044, FR-052, FR-053, FR-501, FR-502, FR-515; the
  evaluator ports QSL-1502 (FR-522, FR-523), QSL-1503 (FR-524, FR-526) and QSL-1504
  (FR-527), the successor map FR-525 and the FR-499 rows, which this requirement
  does not replace. FR-522 supplies `binding`, `authenticated` and the
  `result_identity` rule, FR-523 the thirteen counters, FR-524 the activation
  states and faults, FR-526 the relation checks.
- **Downstream:** quire-protocol `src/assessment.rs`, `src/conformance/kernel.rs`,
  `tests/it002_observation_temporal.rs` (QSL-1549).
