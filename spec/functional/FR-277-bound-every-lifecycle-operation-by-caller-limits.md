---
id: FR-277
title: "Bound every lifecycle operation by caller-configurable limits"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-277: Bound every lifecycle operation by caller-configurable limits

## Description

Every bound a QSL lifecycle operation (FR-275) enforces SHALL be a field of
the limits value the caller passes, with a published default the caller can
replace (ADR-029 LC-4). Reaching a bound SHALL refuse with `LimitExceeded`
(ADR-013 T-4), which names the limit, its configured value and the limits
field that raises it, and is never read as success.

Nesting depth is not a limit. Node, byte and work limits bound every stage,
and no stage recurses on the native stack (ADR-030 D-1). ADR-030 owns the
depth rule; this requirement states only that the lifecycle operations take
their limits from the caller.

## Inputs

The operation's limits value. Each field has a published default, available
as the limits type's `Default` value.

## Outputs

On reaching a bound, `StageFailure::Limit(LimitExceeded)`; for `execute`'s
accounting limits, the evaluation outcome `Incomplete` naming the counter;
for `analyze`, each open FR-331 item settled `incomplete` with the limit as
its cause.
`LimitExceeded` carries the limit kind, the configured value, the counter at
the failed charge, the `Locus` where the producer knows one (FR-096), and the
FR-255 setting that sets the bound.

## Behavior

- Each lifecycle operation shall read every bound it enforces from the
  limits value its caller passes.
- Each limits type shall give every field a published default that the
  caller replaces field by field.
- When an operation reaches a bound, it shall return `LimitExceeded` naming
  the limit kind, the configured value, the counter reached and the limits
  field that raises it.
- A reached limit shall never produce a success outcome.
- Each lifecycle operation shall process input at any nesting depth that
  fits its node, byte and work limits, and shall not fail by exhausting the
  native stack.

## Supplied-value phase contract

### Supplied-value phases

The following phase contract SHALL govern both public checked-call admission
and replay argument processing. It preserves FR-098's semantic conversion
counts; helper traversal events SHALL NOT redefine converted nodes.

QSL-681 review status: the following event tables and numeric defaults are a
PROPOSED contract and falsifiable planning hypothesis, not a settled norm,
production default or qualified count. The inspected released QSV has no
public union membership/type API. The union-specific event order, resource
contract and full-fixture default qualification require the reviewed QSV
union stack after IR-713's declared-registry carrier. Earlier hosted
candidate counts are research predictions, not approved source authority.
Publishing or freezing this draft's review commit does not settle those gates.

| Phase or counter | Unit and owner | Caller-visible bound | Aggregation |
| --- | --- | --- | --- |
| Admission helper work | QSV supplied-membership events, including runtime value visits and iterative type comparison, under the reviewed QSV FR-109 event contract | `supplied.admission_work_units` | One caller-owned cumulative traversal budget across all supplied arguments and admission helpers |
| Conversion helper work | QSL witness preflight, descriptor inspection, runtime type resolution and witness materialization events defined by FR-098 | `supplied.conversion_work_units` | One caller-owned cumulative budget across all supplied arguments and conversion helpers |
| Converted semantic nodes | FR-098 nodes converted, one unit per node | `accounting.work_units` | Each argument separately; neither helper traversal nor evaluation spend seeds this count |
| Semantic value occurrences | `quire.value.accounting/v1` `occ` | `accounting.value_occurrences` | Each supplied argument separately |
| Evaluation | Actual semantic charges at their existing charge points | `accounting.work_units` and other existing accounting fields | The existing evaluation Meter; pre-call counters contribute zero |

- QSL SHALL expose a typed supplied-value limits value with independent
  `admission_work_units` and `conversion_work_units` fields on its public
  checked-call/replay limits where the phase applies. Their units are the
  supplied-membership and conversion-helper events below, independently of
  whatever evaluation limit the caller chooses. Each proposed default
  hypothesis is 262,144 units. The count basis and confirmation status below
  are explicit; neither default is computed from a caller's evaluation
  allowance, an existing expression/declaration-work default, or a runtime
  measured count. They SHALL permit the unchanged full TC-830/831/906 Tree
  under the agreed event contract.
  Proposed diagnostic/setting names are `supplied.admission_work_units`
  and `supplied.conversion_work_units`, distinct from population admission's
  `admission.work_units`. Acceptance by the settings operation or replay
  `stage_limits` requires an explicit coordinated FR-255 table amendment,
  with bound owner, limits field, taxonomy and justified published default.
  This requirement SHALL NOT independently register a setting or privately
  add a closed replay-envelope member. Missing bounds inherit their own
  published defaults under ADR-014 section 2; they are never unbounded.
  It SHALL NOT derive either bound from the evaluation allowance, multiply
  that allowance, reset helper spend between arguments, or bypass a phase.
- Every helper event SHALL have exactly one helper-budget owner. A QSL
  conversion invoking QSV membership SHALL pass the same admission budget
  used for final checked-call admission. Descriptor inspection, runtime
  type resolution and materialization SHALL use the conversion budget.
  A bounded reusable QSV helper for these tasks SHALL receive that explicit
  owner budget; it SHALL NOT also spend admission work for the same event.
- Admission SHALL preserve the reviewed released QSV trust boundary for
  already admitted nested Option/Composite/Collection values. A runtime
  occurrence visit is counted only when that membership contract actually
  inspects the occurrence; semantic `occ` counting remains a separate
  complete occurrence count. No universal descendant revalidation is
  introduced by a work counter. FR-321's explicit union payload admission
  rules still require their reviewed union baseline.
- Each event SHALL be checked before its work. A denied event SHALL remain
  unspent. Limits of zero SHALL be enforced as zero, and checked counter
  overflow SHALL deny the event without wrapping or performing its work.
- QSL SHALL pass the original caller Cancel to every phase and helper,
  including later arguments. It SHALL NOT replace it with a fresh token,
  reinterpret cancellation as limit exhaustion, or reset cumulative spend.
- Public pre-call failure SHALL be a typed QSL phase outcome, identifying
  `admission` or `conversion` and the parameter position where known, with
  distinct causes for invalid input, limit, cancellation, measured storage
  failure and capacity failure. A limit SHALL carry its typed counter/event,
  setting, configured ceiling, successful spend, denied next amount and
  checked attempted count (or explicit counter overflow), plus available
  locus. Cancellation SHALL retain its original cause. Storage/capacity
  SHALL retain the failed operation and its available measured quantities;
  unavailable measurements SHALL NOT be invented. Reservation SHALL be
  fallible and precede mutation; no partial admitted value is success.
- This outcome SHALL NOT be a declaration `EnvironmentFailure` or `Stop`,
  an evaluator `ChargePoint::FunctionCall`, or an evaluation Meter debit.
  Invalid kind/domain input SHALL retain `WrongValueKind` and its existing
  refusal boundary. Replay SHALL project a pre-call limit as
  `inconclusive`/`NoValue` with phase-marked QSL `Incomplete`, preserving the
  full limit payload; other failures SHALL retain their distinct typed
  phase cause. Every pre-call failure SHALL report zero evaluation spend,
  no evaluated call and no fabricated semantic charge.
- QSL's union-specific membership integration SHALL require a reviewed QSV
  declaration/member/type-comparison contract supporting unions. Released
  bool-only `TypeEnvironment::admits` and private declaration work budgets
  SHALL NOT substitute for that public contract. No union descriptor or
  private API signature is prescribed here.

### Applicability of existing validation accounting

NFR-009 governs one `state::evaluate` or `state::evaluate_v2` call under
`quire.state.evaluation-work/1`, including its input validation. Its node,
aggregate-entry, text and other counters SHALL keep their specified units,
default values and before-inspection/retention charge points. NFR-006
governs the historical native artifact construction, validation request and
reference evaluation through FR-018/007/008; its arena-node/child-edge,
validation-visit and text rules SHALL remain unchanged. Neither accounting
identity is reinterpreted as `quire.value.accounting/v1` or as QSV membership.

The present settlement governs supplied-value admission to the S6a Value
checked-call seam and replay witness conversion (FR-090/098/321/322). It
does not exempt a composed/native API from its applicable validation rules.
If an operation invokes one of those APIs, that API SHALL still perform
its prescribed charges against its own caller-configured limits; those
counters SHALL NOT be merged into S6a evaluation or conversion-node units.
QSV's trusted admitted nested-value boundary governs membership only; it
SHALL NOT suppress a node/entry inspection that NFR-009 or NFR-006 requires
at their own public boundaries.

The supplied helper limits are proposed execution-resource bounds B-2
because their carrying type is a supplied-value accounting limits type,
not a compiler/reader stage-limits type. This classification requires a
coordinated amendment of ADR-014 B-2 and its outcome projection rule:
pre-call exhaustion has a phase/event, not a semantic evaluator charge
point. Merely storing a bound in replay `stage_limits` SHALL NOT reclassify
it as B-3 (ADR-014 section 1). The retained per-argument node/occurrence
bounds remain their existing B-2 `ScalarLimits` fields.

### Proposed default basis and confirmation

| Counter | Proposed fixed default hypothesis | Planning basis, not measured work | Planned headroom |
| --- | --- | --- | --- |
| Supplied admission helper units | 262,144 | Research predicts 69,997 membership events for the unchanged full Tree; conservatively allow two such passes if conversion membership and final call admission both actually run | 192,147 above one predicted pass; 122,150 above two (139,994), about 87% above the two-pass planning total |
| Replay conversion helper units | 262,144 | Research predicts 129,994 events for the whole converter, including descriptor and final membership work; use this unsplit total conservatively for planning the QSL-owned conversion subset | 132,150 above that planning total, about 102% above it |

These fixed powers of two are chosen from the supplied-fixture research
scale with explicit room for the still-reviewed helper/event boundary.
They are new independently configurable pre-call defaults, not an increase
to the evaluator's allowance. The 39,997 evaluator prediction justifies
neither helper default and remains subject to the fixed TC-831 trace
controls. No predicted total becomes a normative event count, observed
result, mandatory extra pass or cache bypass. If the reviewed actual event
contract or qualification fails these defaults, the settlement SHALL be
reconsidered explicitly; no automatic multiplier/reset is allowed.

Peer confirmation of event definitions, union/helper source baseline and
default adequacy remains outstanding. Exact/one-less acceptance uses actual
approved event sequences A and H, not these predictions. The full unchanged
fixture's default success is required independently of this planning rationale.
That check is falsifiable: failure of the approved event trace to fit either
262,144 hypothesis, failure to reach evaluation at 39,996/39,997, or failure
of the unchanged fixture under proposed defaults refutes the proposal.

Normative-settlement gates: peer confirmation of the proposed defaults,
accepted FR-255 registry rows,
ADR-014's phase-aware B-2 projection and the coordinated carrier/wire
amendments below remain unresolved proposals. They must be settled and
independently reviewed before the normative contract is approved. A frozen
draft PR may be independently reviewed in parallel; it does not accept a
setting, waive a shared contract or qualify execution.

### Projection into existing QSL carriers

This is an explicit amendment to the QSL pre-call carrier contract, not a
new kernel cause. FR-090's `CallFailure` seam SHALL retain its existing
`Input`, `Fault` and FR-276 `Cancelled` arms and gain a typed supplied-phase
failure arm for helper limits and storage/capacity failures. Invalid input
SHALL remain `Input(InputRefusal)` with the original kind/parameter or
reference refusal; cancellation SHALL remain `Cancelled(CancelCause)`
(`Requested` or `Deadline`), category incomplete, never a limit. Phase
context and successful helper counts SHALL accompany these existing causes
as QSL diagnostic context without changing their catalog fields.

The QSL pre-call limit record SHALL preserve the numerical meanings of
existing `Incomplete`: `limit_kind`, `limit` (configured ceiling),
`consumed` (successful spend) and `next_charge` (denied amount).
`counter` SHALL be the checked attempted count, with typed overflow rather
than a wrapped count; `field` SHALL name the caller field. It SHALL add
typed `phase`, `parameter` when known, and `event` for the actual next
helper/node/occurrence step. It SHALL contain no `charge_point`. An
evaluation limit SHALL keep its existing kernel record and actual
`charge_point`. QSL's limit projection SHALL distinguish these two origins;
it SHALL NOT fill a mandatory kernel charge point with FunctionCall or
create a kernel `Outcome::Incomplete` for work that never evaluated.

FR-098's existing no-call settlement SHALL retain FR-072's arm, category
incomplete, `inconclusive`, `NoValue`, absent value/witness and zero replay
charges, carrying the QSL pre-call limit record instead of a fabricated
kernel limit. Wrong-kind input SHALL remain the existing typed replay
admission refusal, not a settled evaluator refusal. Cancellation SHALL
return the existing cancelled operation failure with its original cause
and no partial replay result (FR-276). Measured storage and capacity
failures SHALL carry their original shared typed cause in the supplied-phase
failure arm, not a limit or an InternalFault.

FR-096's `RefusalRecord` code/category/locus/catalog-field rule and FR-072's
shared result-envelope authority SHALL remain in force. A QSL phase limit
is not a kernel refusal record. The public wire projection of the
discriminated limit origin requires the reviewed shared result contract;
until that contract supports phase/event and the numerical fields above,
QSL SHALL report this projection unavailable rather than serialize a fake
charge point, drop phase information or privately extend QSpec's envelope.

### Logical events and bounded subordinate work

Each following actual step SHALL cost one unit in the owning helper budget.
This table assigns QSL integration ownership; QSV's supplied-membership
event ordering, trusted-value boundary and union baseline remain its
reviewed FR-109 contract. It SHALL NOT prescribe a nonexistent union API.

| Actual step | Owner and count boundary | Cache/delegation rule |
| --- | --- | --- |
| Expected-type entry / actual-type comparison | Admission: one entry for the expected type, or one comparison of the expected/actual type pair at that site, not both for the same check | Iterative structural child pairs count when compared; a trusted nested value has no invented descendant checks |
| Runtime value visit | Admission: one per occurrence actually inspected by the membership contract | Stop at the first invalid occurrence in the specified order; no event for an unvisited descendant |
| Type link / declaration lookup | Conversion when resolving a witness's runtime type; admission when resolving membership types: one per actual link followed or lookup attempted | A cache hit still spends its logical lookup unit; no second descriptor debit for that same read |
| Enum rank scan | Budget of the invoking conversion/admission helper: one per candidate rank/member compared | Cache hit spends lookup; a skipped scan has no invented scan units |
| Numeric range helper | Budget of the invoking conversion/admission helper: one per elementary digit/limb comparison or arithmetic step, in the shared helper's reviewed order | Variable-size work cannot hide behind one logical comparison |
| Declaration-map / ancestor search | Budget of the invoking helper: one per actual key comparison or binary-search comparison, alongside the distinct logical lookup | Admitted declaration size bounds storage/search domain; it is not a free-work allowance or a seed for any counter |
| Scheduling / reservation | Included in the event being scheduled, with fallible reservation before mutation | No extra scheduling debit; failed reservation retains the measured storage/capacity cause |

One performed comparison or lookup SHALL appear once, in the explicit
owner budget passed to its shared helper. Repeating an actual operation
counts a new event; returning an already computed result cannot trigger a
fabricated second visit. Successful helper spend SHALL not reset on a
cache hit, helper return or argument boundary. Cancellation SHALL be polled
before every elementary step, including uncached link walks and scans.
Before freeze, shared helper contracts SHALL establish the deterministic
ordering and bound for these variable-size steps against admitted type,
declaration and numeric sizes. The public ceiling SHALL remain a caller
choice, never a derived multiplier of those sizes or the evaluation bound.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-277-AC-1 | For each field of the limits types of all twelve operations (`parse`, `format`, `select`, `check`, `check_fences`, `package`, `execute`, `analyze`, `monitor`, `replay`, `inspect` and `render`), running the operation over its TC-755 step 6 input with that field set to one below the counter the input reaches names that field: `LimitExceeded` with the field's limit kind, configured value and limits-field name; for `execute`, the outcome `Incomplete` naming that counter; for `analyze`, each open item `incomplete` with that limit as its cause. Setting the field to the counter reached succeeds. | Test (TC-758) |
| FR-277-AC-2 | On a thread with the platform's default stack, a source whose body is a 100,000-term sum checks successfully when the caller raises `s3.nodes` to fit it, and with `s3.nodes` one below its node count refuses with `LimitExceeded` naming `s3.nodes`; no outcome names a depth. | Test (TC-758) |
| FR-277-AC-3 | `execute` over an input whose work, with `work_units` set to one below what it needs, is denied returns the outcome `Incomplete` carrying the limit kind, the configured value, the counter at the failed charge (the consumed value with the denied amount) and the `accounting` field name that raises it. | Test (TC-758) |
| FR-277-AC-4 | TC-830 and TC-906 distinguish admission/conversion limits, invalid input, caller cancellation, measured storage failure and capacity failure through the typed phase outcome above. Limits preserve the setting, configured ceiling, successful spend and denied event/amount without spending the denied step; all pre-call failures have zero evaluation consumption and no FunctionCall projection. Zero ceilings and checked overflow cannot admit a value. | Test |
| FR-277-AC-5 | Across multiple arguments and nested helpers, TC-906 retains the same original Cancel and cumulative admission/conversion budgets. No helper event is debited twice or seeded into converted-node counts; cancellation at a later argument remains cancellation, and a failed reservation precedes mutation with no partial success. | Test |

## Dependencies

- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the
  accepted setting registry and cross-entry-point mapping; its owner must
  add the proposed rows before those names are accepted.
- [ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md):
  B-2 carrying-type classification and the coordinated phase projection.
- [NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md) and
  [NFR-006](../non-functional/NFR-006-bound-native-runtime.md): their
  separately scoped validation accounting remains binding.
- [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md) and
  [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  coordinated carrier/context mapping; the frozen QSL-675 paths remain with
  their existing author/reviewer.
- [QSpec FR-323](ix://agent-ix/quire-specification/FR-323): reviewed shared
  envelope authority for the proposed phase-aware limit projection.
- ADR-029 LC-4: caller limits.
- ADR-030 D-1: depth is not a limit.
- ADR-013 T-4: `LimitExceeded`.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md): the locus a limit carries.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- QSpec FR-300: explicit limits on every request.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06).
- QSL-381: the depth ruling ADR-030 records.
- QSpec FR-300, FR-460, FR-461 (STD-141, STD-143): the QSpec half.
