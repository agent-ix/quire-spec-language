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
  whatever evaluation limit the caller chooses. Numerical hypotheses under
  review are 262,144 admission units and 524,288 conversion units; neither
  is selected as a default or qualified ceiling. The count basis
  and confirmation status below
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
- Every helper event SHALL have exactly one initiating-phase budget owner.
  QSL conversion invoking QSV membership SHALL pass its conversion budget;
  direct or final checked-call admission SHALL pass its admission budget.
  A physical operation is charged once to that initiating phase, not both.
  Descriptor inspection, runtime type resolution and materialization
  initiated by conversion SHALL use the conversion budget.
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

### Unselected numerical hypotheses and confirmation

| Counter | Unselected numerical hypothesis | Planning basis, not measured work | Planned headroom |
| --- | --- | --- | --- |
| Supplied admission helper units | 262,144 | Research predicts 69,997 membership events for the unchanged full Tree; conservatively allow two such passes if conversion membership and final call admission both actually run | 192,147 above one predicted pass; 122,150 above two (139,994), about 87% above the two-pass planning total |
| Replay conversion helper units | 524,288 | Research predicts 129,994 events for the whole converter; QSV's additional hypothetical 32D+128 schedule at D=10,000 gives 320,128, not a proved event bound | 394,294 above the research prediction; 204,160 above the hypothetical schedule (about 64%) |

These fixed powers of two are chosen from the supplied-fixture research
scale with explicit room for the still-reviewed helper/event boundary.
The corresponding hypothetical admission schedule 16D+64 gives 160,064
at D=10,000, leaving 102,080 units (about 64%) below 262,144. These formulas
are planning hypotheses, not source-proven upper bounds or default
recommendations from QSV. The released trusted boundary and original
QSL-503 fixture must determine the approved schedule; TC-817's different
Leaf/Node-with-two-Options fixture SHALL NOT substitute for it.
They are candidates for independent pre-call ceilings, NOT selected
defaults or an increase to the evaluator's allowance. The 39,997 evaluator
prediction justifies neither helper default and remains subject to the fixed TC-831 trace
controls. No predicted total becomes a normative event count, observed
result, mandatory extra pass or cache bypass. If the reviewed actual event
contract or qualification fails these defaults, the settlement SHALL be
reconsidered explicitly; no automatic multiplier/reset is allowed.

Peer confirmation of event definitions, union/helper source baseline and
default adequacy remains outstanding. Exact/one-less acceptance uses actual
approved event sequences A and H, not these predictions. The full unchanged
fixture's default success is required independently of this planning rationale.
That check is falsifiable: failure of the approved event trace to fit either
262,144/524,288 hypotheses, failure to reach evaluation at 39,996/39,997, or failure
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

The selected proposal is logical events plus independently bounded helper
inputs. Each following actual logical step SHALL cost one unit in the owning
helper budget; it is not a measurement of internal limb/key comparisons.
This table assigns QSL integration ownership; QSV's supplied-membership
event ordering, trusted-value boundary and union baseline remain its
reviewed FR-109 contract. It SHALL NOT prescribe a nonexistent union API.

| Actual step | Owner and count boundary | Cache/delegation rule |
| --- | --- | --- |
| Logical value visit | Initiating phase: one per runtime occurrence actually inspected by QSV supplied membership | Existing trusted nested-value boundary and invalid-stop order; unvisited descendants cost no visit |
| Logical type-pair/link step | Initiating phase: one per actual QSV structural type-pair or link step under FR-109 | No duplicate expected-type-entry charge; cache optimization preserves the logical count |
| Logical child schedule | Initiating phase: one per actual child scheduling operation prescribed by QSV FR-109 | Count before fallible worklist reservation/mutation; do not also hide it inside a value visit |
| Logical descriptor query | Initiating phase: one per descriptor query prescribed by QSV FR-109 | Cache hits preserve logical query count; QSL/QSV delegation does not count the same query twice |
| Unhooked numeric/enum/registry subordinate operation | Included in its initiating logical check/query, with independently bounded helper inputs | No invented per-limb/key-comparison events; source proof must bound input-dependent work, temporary storage and cancellation behavior |

These are descriptive event labels, not invented public enum/API spellings.
QSL SHALL use QSV FR-109's actual published event definitions and ordering.
QSL's separate witness preflight/materialization work remains FR-098-owned;
its delegated QSV events use that same conversion phase budget. A later
final-admission invocation is new actual work owned by admission, not a
transfer of conversion spend or a reset within either phase.

One performed logical comparison or lookup SHALL appear once, in its
explicit owner budget. A facade/helper delegation that reaches the same
lookup is one event, not a facade event plus a helper event. Repeating an actual operation
counts a new event; returning an already computed result cannot trigger a
fabricated second visit. Successful helper spend SHALL not reset on a
cache hit, helper return or argument boundary. Original Cancel SHALL be
polled before each scheduled logical step and after an invoked helper
returns, before any subsequent mutation or event. This promises no callback
inside an unhooked helper and no wall-clock cancellation latency. The public ceiling SHALL remain a caller
choice, never a derived multiplier of those sizes or the evaluation bound.

### Selected independently bounded helper-input contract

Released kernel Boolean
numeric membership and QSV declaration-map/ancestor search expose no
per-limb/per-comparison WorkBudget or original-Cancel hooks. Their logical
event limits bound invocation counts only. The separate helper-input contract
must establish a finite bound on each invocation's internal work before the
logical schedule can claim bounded traversal. QSL SHALL NOT copy their algorithms or extend private
declaration budgets to manufacture those hooks.

| Helper input | Required independent bound and source proof | Owner boundary |
| --- | --- | --- |
| Numeric membership | Finite stored numeric size for every value and domain endpoint, and finite scale/exponent inputs; prove the released operation terminates with work bounded by those sizes | QSV identifies the actual kernel operation and public/admitted size access; kernel owns semantics, QSL supplies the invoking phase context |
| Decimal membership temporary storage and cancellation | Establish a separate temporary-storage bound, actual fallible reservation/failure path and original-Cancel observation boundary; numeric input sizes alone do not establish these guarantees | Luna's IR/kernel helper-contract plan owns the missing capability; QSL projects its actual native cause and phase without invoking an unbounded substitute |
| Enum / registry / ancestor search | Finite admitted member/declaration/ancestor entry counts and key-content sizes; connect actual immutable environment sizes to its effective caller-configured admission/byte limits | QSV registry and membership contract, after IR-713; no guessed descriptor/signature |
| Type resolution / structural comparison | Finite reachable type-node/link counts and bounded scalar/key content, with source-proven cycle/termination behavior; no type nesting-depth ceiling | QSV's existing admitted-type boundary; QSL conversion owns its actual descriptor/materialization invocations |
| Witness helper inputs | Finite replay encoded bytes and complete semantic occurrence/node counts; subordinate numeric/type/registry helpers additionally satisfy the rows above | QSL FR-098/263 and QSV's helper-input contract; occurrence/node limits alone do not bound an arbitrarily large numeric leaf |

These bounds must identify the actual existing published limits fields or
an explicitly owned helper-input amendment, their effective finite values,
and a bounded way to establish the input sizes before the unhooked operation.
A claim that inputs are merely admitted is not that proof. Replay's byte
limit alone is not proof about a direct caller's kernel values. Missing
size access/bounds or an unproved terminating type chain remains an explicit
source gate; no implicit unbounded input or free cached work is allowed.

Released decimal membership's `compare_shifted` performs BigInt decimal-digit
formatting and shifted multiplication (`abs * 10^shift`) with no helper
budget, original-Cancel hook or demonstrated fallible temporary reservation.
The input-size reasoning above does not prove bounded temporary storage or
the required cancellation/storage-failure behavior for that operation.
Candidate integer cost of at most two limb comparisons and rational cost
of at most four limb comparisons are positive cost-bound hypotheses from
the peer packet, not qualified claims about every numeric helper or decimal
storage. No default numeric/type bound is inferred from them.

Where the current native helper lacks the required bounded capability, the
supplied phase SHALL return category unsupported with a typed native cause
identifying the actual helper/operation and missing bounded temporary-storage
or cancellation capability before invoking that helper. It SHALL NOT claim
limit exhaustion, caller cancellation, invalid value or allocator failure
that did not occur. An actual storage/capacity failure or cancellation
retains its original typed cause. No default admission path bypasses this
check, and no partially admitted value or evaluation consumption is returned.
The concrete shared native cause/carrier/catalog projection is an explicit
owner mapping gate; this draft creates no kernel variant, catalog spelling
or guessed capacity default. If that mapping is unavailable, QSL reports
the bounded capability/projection unavailable rather than manufacturing a
kernel cause or successful admission.

QSV must establish a finite work function for each bounded helper input.
Then a finite number of admitted logical invocations, each on independently
bounded inputs, bounds total helper work without pretending to count its
internal comparisons. This reasoning is a proposal, not an established
released-source proof or a request for a new kernel API. If QSV's source
proof cannot establish it, main SHALL route the precise bounded-helper API
gap to Luna's IR plan and preserve the QSL-503 blocking edge. It SHALL NOT
implement a copied helper or silently change event units. Draft publication
and independent review may proceed while that enabling gap is unresolved.
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
