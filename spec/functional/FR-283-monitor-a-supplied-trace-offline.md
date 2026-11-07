---
id: FR-283
title: "Monitor a supplied trace offline against temporal clauses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
---
# FR-283: Monitor a supplied trace offline against temporal clauses

## Description

`monitor` applies the layer-5 trace evaluator to a trace the caller supplies
(ADR-029 OP-3). It uses the semantics replay uses: ADR-014 A-4's finite-prefix
and lasso evaluation. It lives in layer A, crate `qsl-analyze`.

A runtime monitor generated from S5 IR is a `generate` target owned by the
driver with CG and RT (FR-280). It is held to the verdicts `monitor` gives on
a shared trace corpus.

## Inputs

```rust
pub fn monitor(
    package: &CheckedPackage,
    request: &MonitorRequest<'_>,
    limits: &MonitorLimits,
    cancel: &Cancel,
) -> Result<Staged<MonitorOutcome>, StageFailure<MonitorRefusal>>;

pub struct MonitorRequest<'a> {
    pub clauses: Vec<MonitorClause>,
    pub trace: MonitorTrace<'a>,
    pub snapshots: &'a DocumentProvision,
    pub packages: &'a PackageInput,
    pub sources: &'a [Source],
    pub temporal_selections: &'a [TemporalClockSelection],
}

pub enum MonitorTrace<'a> {
    Document(&'a [u8]),
    Typed(&'a TraceInput),
    Samples { subject: ValueType, samples: &'a [Value] },
}

pub struct MonitorClause {
    pub name: String,
    pub over: Option<OverObject>,
}

pub struct MonitorLimits {
    pub observation: ObservationLimits,
    pub normalization: ModelNormalizationLimits,
    pub accounting: ScalarLimits,
    pub max_window_values: u64,
}
```

- `&CheckedPackage`.
- `clauses`: one or more temporal clause selections, each a temporal
  clause's declared name and its `over` binding. For a clause over a model
  subject, run over a `Document` or `Typed` trace, `over` is the object the
  clause's `over` parameter binds, resolved in the observation at position 0,
  as FR-330's `Temporal` selection names it. For a clause over a declared
  record trace subject, run over `Samples`, `over` is absent: FR-330's
  trace-subject route binds the subject parameter to each sample and takes no
  object.
- `trace`: one of three shapes.
  - `Document`: the bytes of a trace document ("Trace document" below),
    holding a finite trace, or a lasso: prefix positions followed by a
    non-empty loop that re-enters at its first position. Each position is a
    snapshot named by its `DocumentRef`. This is the shape a command-line
    caller supplies.
  - `Typed`: a library caller's FR-330 `TraceInput` (`Finite`, `Lasso`,
    `Progressed` or `Timestamped`), read as FR-330's `Temporal` selection
    reads it. It is the shape that monitors a finite-window clause (FR-501)
    and a progressed event-position or fixed-sample clause (FR-504), because
    a trace document carries no stamp, order key, progress assertion, cover
    or `TraceBinding`. `monitor` carries no FR-502 progress ledger, so a
    `Progressed` or `Timestamped` trace is evaluated alone.
  - `Samples`: a declared record type and a non-empty finite sequence of its
    record samples in trace-position order, FR-330's trace-subject route
    input. It is the shape that monitors a window-aggregate clause, because
    FR-194 admits a window aggregate only over a trace subject and a
    snapshot position binds model objects only (FR-330-AC-8).
- `snapshots`: the FR-106 snapshot provision, document bytes by
  identity labels, that a `Document` or `Typed` trace's positions name;
  `packages`: FR-056's package input, the domain package bytes FR-106
  admission reads. A `Samples` trace reads neither.
- `sources`: the unit's `Source` and each library `Source` that `package`
  was checked from, the sources FR-109's compile reads. `monitor` renders a
  refused condition's locus and names a record's atom from them (Behavior).
- `temporal_selections`: the FR-050 `TemporalClockSelection`s, one per
  temporal declaration whose profile requires a clock configuration, that
  `monitor` emits `package` with (`emit_checked_with_temporal_selections`)
  to obtain the `profile_selections` and `temporal_bindings` rows a
  `Progressed` or `Timestamped` trace's `binding` is authenticated against
  (FR-522). That emission covers the whole package, so it needs the
  selection of every clock-bearing temporal declaration of `package`, not
  only of the selected clauses, and it is read back strictly under
  `packages`. A request with no such trace emits nothing, reads no
  selection, and may pass an empty slice.
- `limits`: the observation and model-normalization limits FR-106
  admission takes, the evaluator's accounting limits, and FR-194's
  `max_window_values` (setting `monitor.max_window_values`), the window
  limit of every window aggregate a `Samples` run reads; each field has its
  published default (FR-277, FR-194).
- `&Cancel`.

### Trace document

A `Document` trace holds a trace document. The trace document is a UTF-8 JSON object, format `quire.monitor-trace/1`,
with the closed member set `format`, `prefix` and `loop`:

```json
{
  "format": "quire.monitor-trace/1",
  "prefix": [
    {"identity": {"authority": "agent-ix", "identity": "ix://example/counter/run-1/0"}}
  ],
  "loop": [
    {"identity": {"authority": "agent-ix", "identity": "ix://example/counter/run-1/1"}}
  ]
}
```

- `format` is the string `quire.monitor-trace/1`.
- `prefix` and `loop` are arrays of snapshot `DocumentRef`s, each spelled
  as an FR-106 invocation's `pre` member: `identity`, the FR-001 labels,
  the snapshot's document identity.
- With `loop` absent the trace is finite, its positions the `prefix`
  elements in order, and `prefix` holds at least one element. With `loop`
  present the trace is a lasso of `n = len(prefix) >= 0` prefix positions
  and `L = len(loop)` loop positions, position `n + L` re-entering position
  `n` (FR-329); FR-330's `TraceInput::Finite` and `TraceInput::Lasso` are
  the same traces as typed values.
- Positions are numbered from 0, prefix first, then loop. Each position is
  one observation that FR-106 admits from `snapshots` by its identity labels, as
  FR-330 admits a `Temporal` trace's positions.

The document is read with `quire-canonical`'s shared reader (FR-261) at any
nesting depth, bounded by `observation.input_bytes`, `observation.objects`
and `observation.values` (FR-106). A document defect refuses the whole
request before any position is admitted, at the first defect in this order,
naming its JSON pointer:

1. Bytes above `observation.input_bytes`: `stage_limit_exceeded`/
   `input-bytes-exceeded` naming that setting.
2. Bytes the shared reader refuses as JSON, and a top-level value that is
   not an object: `invalid_runtime_input`/`invalid-value` at `""` and
   `invalid_runtime_input`/`wrong-value-kind` at `""`.
3. A missing `format` or `prefix`: `invalid_runtime_input`/`missing-member`
   naming the first missing in the order `format`, `prefix`; a member
   outside the set: `invalid_runtime_input`/`unknown-member`, naming the
   first in document order.
4. A present `format` other than the string `quire.monitor-trace/1`:
   `unknown_wire`/`unsupported-wire` at `/format`.
5. A `prefix` or `loop` that is not an array, or an element that is not a
   `DocumentRef` object: `invalid_runtime_input`/`wrong-value-kind`; an
   element missing `identity`, or holding another member:
   `missing-member` or `unknown-member`; an `identity` with a blank label:
   `invalid_source_identity`/`blank-label`.
6. An empty `prefix` with `loop` absent, and a `loop` member present and empty (in this document a present `loop` states a lasso, so it must be non-empty; the counterexample of FR-330 spells a finite prefix with an empty `loop`, a different wire):
   `invalid_runtime_input`/`invalid-value` at `/prefix` and at `/loop`.

## Outputs

`Staged<MonitorOutcome>`:

```rust
pub struct MonitorOutcome {
    pub records: Vec<MonitorRecord>,
}

pub struct MonitorRecord {
    pub clause: String,
    pub verdict: MonitorVerdict,
    pub category: Category,
}

impl MonitorRecord {
    pub fn atom(&self) -> Option<&str>;
}
```

`MonitorRecord::atom()` is the name of the atom an `InsufficientData` or
`Indeterminate` record names, and is `None` for every other record. An
`InsufficientData` record's atom is named by the source text of the
aggregate condition, read from `sources` (Behavior), such as
`count over past[0, 9] min_count 2 >= 1`. An `Indeterminate` record's atom
is the `atom` its FR-501 reason (`MissingValuation`,
`ConflictingValuation`) names, also by source text.

Each record is rendered into an FR-286 `monitor` item by
`OutcomeItem::from_clause_run`, which adds the `clause` member naming the
record's clause (FR-286 "Monitor documents from clause runs").

`records` holds one record per selected clause, in request order. Each
record's verdict is one of FR-267's temporal-disposition rows, with `tested`
in the place of `holds`, under ADR-014 A-4:

| `MonitorVerdict` | Evaluator result (FR-330) | Members | O-16 category | Exit (FR-285) |
| --- | --- | --- | --- | --- |
| `Violated` | `Completed(false)`, finite or lasso | `position`, `counterexample`, `decisive` when the report holds one (FR-330) | violation | 10 |
| `Undefined` | `Undefined` with a kernel or family cause | `position`, `counterexample`, `cause` (`UndefinedEvaluation{where, cause}`) | violation | 10 |
| `InsufficientData` | FR-327's insufficient-data outcome, `InsufficientData{where, atom, count, min_count}` (a window aggregate short of data, QSpec FR-414) over a `Samples` trace, not an `Undefined` | `position`, `atom`, `count`, `min_count` | undefined, with the label `undefined` and reason `insufficient-data` | 10 |
| `Pending` | `Pending`, finite trace | none | pending | 0 |
| `Indeterminate` | `Indeterminate`, a finite-window clause (FR-501) over a `Typed` `Timestamped` trace, whose window is closed and whose evidence stays unresolved | the FR-501 reason, with its `atom` where the reason names one (`MissingValuation`, `ConflictingValuation`), and `position` where that reason names one | indeterminate | 11 |
| `Tested` | `Completed(true)` | none | success | 0 |
| `MissingFairnessPremise` | `MissingFairnessPremise`, a clause with a non-empty fairness set | `constraint`, the first constraint of the fairness set | unsupported | 21 |
| `Incomplete` | `Incomplete` | the charge point and limit | incomplete | 22 |
| `WindowLimitReached` | `WindowLimitReached`, a window aggregate over a `Samples` trace holding more than `max_window_values` values | `limit`, `value`, `setting` | incomplete | 22 |
| `ConditionRefused` | a `holds` condition refused (FR-327), other than `CheckedInvariant` | `position`, the refusal record with its locus rendered from `sources` | the record code's category | its code |
| `TraceRefused` | a `TemporalRefusal` other than `Fault` that the evaluator returns at stage `evaluate`, such as FR-501's refusal of equal-stamp positions without comparable order keys over a `Timestamped` trace | the refusal | the refusal code's category | its code |

- `position` is the `TemporalPosition` (FR-327) of the trace.
- `counterexample` is the `TemporalCounterexample` FR-330's report holds:
  QSpec FR-364's observed arm, built from the trace's `DocumentRef`
  identities, with `trace_position` equal to `position`. It lets QSpec FR-364's
  replayer re-run the trace. A `Samples` trace has no document identities,
  so a record over it carries no `counterexample`; its `position` is the
  sample's zero-based index and its `decisive` is FR-330's trace-subject
  `decisive`.
- `decisive` is a member of the record beside `position` and `counterexample`,
  not a member of the counterexample, and it carries the decisive information
  of a temporal violation: `{ position, quantifier, element, index, value_path }`,
  as FR-330 defines it, including which atom is decisive, when there is none,
  and its `position`. The clause run report carries it as
  `ClauseRunReport::decisive`. The `counterexample` stays QSpec FR-364's closed
  record, and FR-267's `temporal` member and the FR-364 wire are unchanged by
  `decisive`.
- A temporal `Violated` or `Undefined` record has no `witness` member: FR-266
  gives a temporal run the basis `closed-scope` and no FR-351 separating
  witness record, and FR-267 writes the counterexample in its `temporal`
  member. The FR-351 `witness` appears only on a non-temporal decisive
  violation.
- `constraint` is QSpec FR-364's fairness member `{kind, operation,
  granularity}`.
- An internal failure (an evaluator fault, a `TemporalRefusal::Fault`, a
  refused condition holding `Refusal::CheckedInvariant`) fails the whole
  request with `StageFailure::Fault` and no record.

`tested` is evidence for that trace only, and is never `proved`. A clause
still pending when the trace ends observed no violation, so it exits 0, the
QSpec FR-301 code for a run completed without violation. A clause record
of an undefined evaluation (`Undefined`) never carries the label
`undefined` (FR-286): it is a violation. `InsufficientData` is QSpec
FR-414's window outcome, not an undefined evaluation; its record is O-16
category undefined and keeps the label `undefined` with reason
`insufficient-data`, as FR-285 requires of a category-undefined item.

A supplied trace carries states and steps and no enabledness, so it gives a
fairness constraint no premise to read. A clause with a non-empty fairness
set settles unsupported over it, as QSpec FR-362 states; a clause with an
empty fairness set admits every lasso.

`MonitorRefusal` is the cause of a whole-request `StageFailure::Refused`:

```rust
pub enum MonitorRefusal {
    TraceDocument { record: Box<AdmissionRecord>, pointer: String },
    Admission { position: usize, failure: Box<AdmissionFailure> },
    UnknownClause { name: String },
    ClauseRefused { clause: String, failure: Box<AdmissionFailure> },
    SourceNotSupplied { clause: String, source: SourceIdentity },
}
```

- `TraceDocument`: a "Trace document" defect, its catalog record and JSON
  pointer.
- `Admission`: the first position of a `Document` or `Typed` trace whose
  FR-106 admission fails, its index and FR-106's `AdmissionFailure`. The
  index is the trace's position: for a `Timestamped` trace, its FR-501
  canonical-order position.
- `UnknownClause`: a selected name that resolves to no temporal clause of
  `package`, FR-330's stage-`select` `missing_declaration`/`missing-name`.
- `ClauseRefused`: a stage-`admit` refusal of one selected clause that is
  not one position's FR-106 admission, with its record. FR-330's own:
  a bounded-profile clause over a lasso, a profile and trace shape that do
  not fit, an FR-522 binding that differs from the clause's rows, a
  `Samples` refusal of FR-330's trace-subject route, each
  `invalid_runtime_input`/`invalid-value`, and an `over` that resolves to no
  object of the observation at position 0,
  `invalid_runtime_input`/`wrong-role-mapping`. `monitor`'s own subject
  checks (Behavior): an `over` absent for a clause over
  a model subject, `invalid_runtime_input`/`wrong-role-mapping`; an `over`
  present over `Samples`, a record-trace-subject clause over a `Document` or
  `Typed` trace, and a model-subject clause over `Samples`, each
  `invalid_runtime_input`/`invalid-value`. And the request defects of a
  `Progressed` or `Timestamped` run's FR-522 authentication, each a QSpec
  FR-272 closed cause variant, its record fields as listed:
  - A clock-bearing temporal declaration of `package` with no element of
    `temporal_selections`: `missing_declaration`/`missing-selection`, field
    `member` `temporal_selections`. Emission refuses the whole package, so
    this holds also for a declaration no selected clause owns (another
    clause, or a temporal declaration that is not a clause). The refusal
    names the declaration's clause when it is selected, and otherwise the
    first selected clause.
  - `temporal_selections` that do not fit `package`, FR-050's duplicate,
    surplus and wrong-profile configurations and an invalid clock value: a
    selection repeated for one declaration, a selection at a span no
    declaration holds, a configuration of another profile than its
    declaration's: `invalid_runtime_input`/`invalid-value`, field `member`
    `temporal_selections`, naming the first selected clause. `monitor`
    reports these as defects of its request input, in place of the
    emitter's own FR-050 record.
  - `packages` whose domain package documents do not admit the emitted
    package (a document missing, truncated, or another document under the
    package's identity): `invalid_runtime_input`/`invalid-value`, fields
    `member` `packages` and `stage` `closure` when the dependency closure
    does not read back under them, or `strict-read` when the strict read of
    the emitted package refuses, naming the first selected clause.
  - A trace `binding` that differs from a selected clause's emitted rows in
    one dimension (FR-522): `invalid_runtime_input`/`invalid-value`, field
    `mismatched_dimension` the FR-522 dimension name (`Profile`, `Clock`,
    `ClockParameters`, `Epoch`, `SamplePeriod`, `ClockUnit`,
    `SequenceAuthority` or `TimestampUnit`), field `asserted` the binding's value and, when the
    clause's rows hold one, field `expected` the rows' value, naming that
    clause. This record has no `member` field. A selection whose
    `timestamp_unit` differs from the unit the binding asserts fits the
    package, emits, and is refused here, with `mismatched_dimension`
    `TimestampUnit`, `asserted` the binding's unit and `expected` the
    selection's.
- `SourceNotSupplied`: a refused condition's locus, or a record's named
  atom, whose region names a source that no element of `sources` has as its
  reference, or whose matching source's bytes do not hold the region's span
  on UTF-8 boundaries: category refusal, `invalid_runtime_input`/
  `missing-member` naming `sources`, exit 20, with the clause and the
  region's source identity. For `monitor` this replaces FR-100's internal
  failure `locus-source-supplied`: there the spine compiled the sources it
  renders from, while here `sources` is the caller's input.

None holds a clause record. These are refusals of the request as supplied:
each names the selection or input to correct, and a request refused for one
clause reports no verdict for the others.

## Behavior

- The `monitor` operation shall read a `Document` trace under "Trace
  document" and refuse the request with `MonitorRefusal::TraceDocument` at
  its first defect, before it admits any position.
- When a `Typed` trace is `Progressed` or `Timestamped`, the `monitor`
  operation shall emit `package` with `temporal_selections` and
  authenticate the trace's `binding` by FR-522, for every selected clause,
  against the `profile_selections` and `temporal_bindings` rows of that
  emission (`Emission::temporal_rows`), before it admits any position.
- If emission refuses for want of the `TemporalClockSelection` of any
  clock-bearing temporal declaration of `package`, then `monitor` shall
  refuse the whole request with `MonitorRefusal::ClauseRefused`, with
  `missing_declaration`/`missing-selection` and `member`
  `temporal_selections`, naming the declaration's clause when it is
  selected and otherwise the first selected clause, with no clause record.
- If emission refuses `temporal_selections` as not fitting `package` (a
  duplicate or surplus selection, a configuration of another profile, an
  invalid clock value), then `monitor` shall refuse the whole request with
  `MonitorRefusal::ClauseRefused` naming the first selected clause, with
  `invalid_runtime_input`/`invalid-value` and `member`
  `temporal_selections`, with no clause record.
- If the emitted package does not read back under `packages`, then
  `monitor` shall refuse the whole request with
  `MonitorRefusal::ClauseRefused` naming the first selected clause, with
  `invalid_runtime_input`/`invalid-value`, `member` `packages` and `stage`
  `closure` or `strict-read`, the step that refused, with no clause
  record.
- If the `binding` of a `Progressed` or `Timestamped` trace differs from a
  selected clause's emitted rows in any FR-522 dimension, then the
  `monitor` operation shall refuse the whole request with
  `MonitorRefusal::ClauseRefused` naming that clause, with
  `invalid_runtime_input`/`invalid-value` and the `mismatched_dimension`,
  `asserted` and `expected` fields "Outputs" gives, before it admits any
  position. A forged profile, clock name, clock parameters or timestamp
  unit is thus refused even when a position's snapshot would fail FR-106
  admission.
- The `monitor` operation shall admit every position of a `Document` or
  `Typed` trace as an FR-106 snapshot, in position order, before it
  evaluates any clause.
- If any position fails admission, then `monitor` shall refuse the whole
  request with `MonitorRefusal::Admission` holding that position's index and
  admission failure, with no clause record.
- The `monitor` operation shall evaluate each selected clause over the
  admitted trace with the layer-5 trace evaluator replay uses.
- The `monitor` operation shall evaluate a clause over a `Typed` trace as
  FR-330's `Temporal` selection evaluates that `TraceInput`, with no ledger.
- The `monitor` operation shall evaluate a clause over a `Samples` trace
  through FR-330's trace-subject evaluation over `package`, without
  compiling the unit again, with `limits.max_window_values` as FR-194's
  window limit.
- If a selected clause over a model subject has no `over`,
  a `Samples` run's clause has an `over`, or the clause's subject
  kind does not fit the trace shape (a record-trace-subject clause over a
  `Document` or `Typed` trace, a model-subject clause over `Samples`), then
  `monitor` shall refuse the whole request with
  `MonitorRefusal::ClauseRefused` and the code "Outputs" gives, before
  FR-330 admits that clause, with no clause record.
- If FR-330 refuses a selected clause at stage `select`, then the `monitor`
  operation shall refuse the whole request with
  `MonitorRefusal::UnknownClause`, with no clause record.
- If FR-330 refuses a selected clause at stage `admit` for a reason other
  than one position's FR-106 admission, then the `monitor` operation shall
  refuse the whole request with `MonitorRefusal::ClauseRefused` holding the
  clause's name and FR-330's record, with no clause record.
- If several selected clauses refuse or fault, then the `monitor`
  operation shall fail the request with the refusal or fault of the first
  of them in request order, after the trace is read and every position
  admitted.
- When a record is `ConditionRefused`, the `monitor` operation shall render
  its locus as FR-100's `locus` (`file` and `span`) over the element of
  `sources` whose reference equals the region's.
- When a record names an atom (`InsufficientData`'s `atom`, an
  `Indeterminate` reason's `atom`), the `monitor` operation shall name it by
  the source text of the atom's condition, read from the element of
  `sources` whose reference equals the condition's region, and never by a
  formula node index, and shall expose that name as the record's
  `MonitorRecord::atom()`.
- If no element of `sources` has the source of a record's locus or named
  atom as its reference, then the `monitor` operation shall refuse the
  whole request with `MonitorRefusal::SourceNotSupplied` naming the clause
  and that source's identity, with no clause record.
- If the matching element of `sources` does not hold that region's span on
  UTF-8 boundaries, then the `monitor` operation shall refuse the whole
  request with `MonitorRefusal::SourceNotSupplied` naming the clause and
  that source's identity, with no clause record.
- When the trace is finite and ADR-014 A-4's finite-prefix rule makes the
  clause false for every extension, `monitor` shall report a violation.
- When the trace is finite and the evaluator decides the clause neither
  false nor true on it (FR-328 `Pending`), `monitor` shall report pending.
- If a trace document's lasso loop is empty, then `monitor` shall refuse
  the request with `invalid_runtime_input`/`invalid-value` at `/loop`, with
  no clause record.
- If a `Typed` `Lasso`'s loop is empty, then the `monitor` operation shall
  return that clause's `TraceRefused` record with FR-329's evaluate-stage
  refusal (FR-330-AC-3).
- When `monitor` reports `Violated` over a `Document` or `Typed` trace, the
  `monitor` operation shall attach the counterexample that "Outputs"
  defines, with `trace_position` equal to the record's `position`, its
  `decisive` member beside it, and no `witness`. The record's `position` of
  a `Completed(false)` violation is the failing activation's position (FR-327);
  it does not move to the position where the deciding atom fails, which
  `decisive.position` names.
- When `monitor` reports `Violated` over a `Samples` trace, the `monitor`
  operation shall attach FR-330's trace-subject `decisive`, when the run
  holds one, and no `counterexample` and no `witness`.
- The `monitor` operation shall return one `MonitorRecord` per selected
  clause, in request order, whose verdict and category are the row of the
  Outputs table its evaluator result selects.
- If a selected clause's fairness set is non-empty, then `monitor` shall
  settle that clause's record unsupported, cause
  `unsupported_projection`/`missing-fairness-premise`, naming the first
  constraint of the fairness set, without evaluating the clause (ADR-014
  A-4, QSpec FR-362).
- When the clause's fairness set is empty and the clause evaluates true
  over the trace, `monitor` shall report `tested`; when it evaluates false
  over a lasso, a violation.
- If a clause evaluates to undefined at a trace position, then
  `monitor` shall report a violation at that position, with cause
  `UndefinedEvaluation{where, cause}` naming the position and the undefined
  reason.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-283-AC-1 | Over a three-position finite trace whose `p` is true at positions 0 and 1 and false at position 2, `always p` with activation `on origin` reports a violation at position 0, the activation's position, with its `counterexample`, category violation. The same clause over a trace whose `p` is true at every position reports pending, category pending, and FR-285 maps it to exit 0 and records the status and the pending clause ids. | Test (TC-765) |
| FR-283-AC-2 | Over a lasso of a two-position prefix and a two-position loop, `always eventually q`, whose fairness set is empty, reports `tested` when `q` holds at a loop position and a violation when `q` holds at no loop position. | Test (TC-765) |
| FR-283-AC-3 | A trace whose second position's snapshot fails FR-106 admission is refused with that admission cause and position index 1, and the outcome holds no clause record. A lasso with an empty loop is refused with `invalid_runtime_input`/`invalid-value`, and the outcome holds no clause record. | Test (TC-766) |
| FR-283-AC-4 | For each AC-1 and AC-2 trace, the verdict `monitor` reports equals the verdict layer-6 replay's trace evaluation reports for the same clause and trace. | Test (TC-766) |
| FR-283-AC-5 | Over a three-position finite trace whose `x` is 0 at position 1, `always (10 / x > 0)` reports a violation at position 1, category violation, with cause `UndefinedEvaluation` whose `cause` is `division-by-zero`; the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-765) |
| FR-283-AC-6 | Over AC-2's lasso, `always eventually q` under `fair weak inc` settles its record unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`, and FR-285 maps the outcome to exit 21; in a request selecting it beside AC-2's clause without fairness, the second clause still reports `tested`. | Test (TC-766) |
| FR-283-AC-7 | AC-1's three-position trace as the document `{"format":"quire.monitor-trace/1","prefix":[R0,R1,R2]}`, each `Ri` a snapshot `DocumentRef` in `snapshots`, gives the AC-1 records; AC-2's lasso as `{"format":"quire.monitor-trace/1","prefix":[R0,R1],"loop":[R2,R3]}` gives the AC-2 records. Each record is a `MonitorRecord` naming its clause; AC-1's violation is `Violated` at position 0 whose `counterexample` is FR-330's `TemporalCounterexample` with `prefix` the three identities, an empty `loop` and `trace_position` 0. | Test (TC-765) |
| FR-283-AC-8 | Each of these trace documents refuses the whole request with `MonitorRefusal::TraceDocument`, no position admitted and no record: bytes `[` (`invalid_runtime_input`/`invalid-value` at `""`); the JSON array `[]` (`invalid_runtime_input`/`wrong-value-kind` at `""`); `{"prefix":[]}` with no `format` (`invalid_runtime_input`/`missing-member` naming `format`, not `unknown_wire`); `format` `quire.monitor-trace/2` (`unknown_wire`/`unsupported-wire` at `/format`); no `prefix` (`missing-member`); an extra member `fairness` (`unknown-member`); `prefix` a string (`wrong-value-kind` at `/prefix`); an element with a blank `authority` label (`invalid_source_identity`/`blank-label` at `/prefix/0/identity`); `{"format":"quire.monitor-trace/1","prefix":[]}` (`invalid-value` at `/prefix`); and a document one byte above `observation.input_bytes` (`stage_limit_exceeded`/`input-bytes-exceeded` naming `observation.input_bytes`). | Test (TC-766) |
| FR-283-AC-9 | Over AC-1's trace, a request selecting `always p` and `always (10 / x > 0)` returns two records in request order; over FR-327-AC-1's closed `Counter` trace (`c.value` 0, 1, 2) as a three-position finite document, `eventually[0,2] holds(c.value = 2)` under event-position false-extension, `on origin`, reports `Tested`, category success, exit 0, and `eventually[0,1] holds(c.value = 2)` reports `Violated` at position 0. | Test (TC-765) |
| FR-283-AC-10 | AC-1's `always p` violation carries a `counterexample` with `trace_position` 0, a `decisive` beside it with `position` 2, no `quantifier` and no `index`, `element` `false` and the empty `value_path` at the observation's root, and no `witness`. Over a three-position finite trace whose position-0 and position-1 snapshots hold a collection field `items` of values 1, 2, 3 and whose position-2 snapshot holds `items` of values 3, 0, 5, the clause `always holds(forall i in self.items : i > 0)` with activation `on origin` reports `Violated` at position 0 with a `counterexample` of `trace_position` 0 and a `decisive` of `position` 2 naming the `forall` occurrence key as `quantifier`, `element` 0, `index` 1 and the FR-207 `value_path` to `items`, equal to the values FR-107's evaluator retains for that `holds` atom over the position-2 observation, and no `witness`. | Test (TC-765) |
| FR-283-AC-11 | Over FR-327-AC-7's non-model subject `c: Counter` (a record with the attribute `level`, `Int[0, 9]`) as a `Samples` trace with `level` samples 0, 1, 2 and no `over`, under event-position false-extension: `always[0,2] (count over past[0, 9] min_count 2 >= 1)` `on origin` reports `InsufficientData` at position 0 with `count` 1 and `min_count` 2, `MonitorRecord::atom()` the source text `count over past[0, 9] min_count 2 >= 1`, category undefined, the label `undefined` with reason `insufficient-data`, and exit 10; `always[0,0] (count where holds(c.level > 9) over past[0, 0] min_count 0 >= 1)` `on origin` reports `Violated` at position 0 with no `counterexample` and a `decisive` naming its aggregate occurrence at position 0 (FR-330-AC-17); the first clause selected with an `over` present refuses the whole request with `MonitorRefusal::ClauseRefused`, `invalid_runtime_input`/`invalid-value`, and no record. | Test (TC-765) |
| FR-283-AC-12 | Over AC-1's trace, `always p` with accounting `work_units` 0 reports `Incomplete` naming `work_units`, category incomplete, exit 22. Over AC-11's samples, `always[1,1] (count over past[0, 9] min_count 1 >= 1)` `on origin`, whose window at position 1 holds the 2 values of positions 0 and 1, reports `WindowLimitReached` with `limit` `max_window_values`, `value` 1 and `setting` `monitor.max_window_values`, category incomplete, exit 22, when `max_window_values` is 1, and `Tested` when it is 2. FR-330-AC-7's clause `always[0,1] holds(InUnit(c.value))` over a two-position `Document` trace with `c.value` 0, 2, with `sources` holding the unit's source, reports `ConditionRefused` at position 1 with the record `integer_out_of_domain`/`outside-domain`, its `expected` field and a `locus` whose `file` is the unit's authored path and whose `span` is the `holds` condition's span, category refusal, exit 20; the same request with `sources` empty refuses the whole request with `MonitorRefusal::SourceNotSupplied` naming the clause and the unit's source identity, `invalid_runtime_input`/`missing-member` naming `sources`, no record and exit 20; and so does the request whose `sources` holds a `Source` with the unit's reference and bytes shorter than the condition's span. | Test (TC-766) |
| FR-283-AC-13 | Each of these refuses the whole request with no record: over AC-2's lasso, `eventually[0,2] holds(q)` under event-position false-extension, a bounded profile, `MonitorRefusal::ClauseRefused` naming it with `invalid_runtime_input`/`invalid-value` (FR-330-AC-3), also as the second clause of a request whose first clause evaluates; a selection `Absent`, `MonitorRefusal::UnknownClause` naming `Absent`; AC-2's clause with an `over` naming key `ghost`, and the same clause with no `over`, `ClauseRefused` with `invalid_runtime_input`/`wrong-role-mapping`; FR-330-AC-17's record-trace-subject clause `always[0,2] A` over AC-1's `Document` trace, `ClauseRefused` with `invalid_runtime_input`/`invalid-value`. A request selecting `Absent` and then the bounded clause reports `UnknownClause`, the first in request order. Over FR-501-AC-7's trace (stamps 0, 1, 1, no order keys) as a `Typed` `Timestamped` trace with an agreeing `binding`, `holds(p) until[0,1] holds(q)` reports a `TraceRefused` record with `invalid_runtime_input`/`invalid-value` at position 2, category refusal, exit 20. With a test trace evaluator that returns `TemporalRefusal::Fault` for one selected clause, the request fails with `StageFailure::Fault`, no record for any clause, and exit 30. | Test (TC-766) |
| FR-283-AC-14 | Each `MonitorRecord` this operation returns renders through `OutcomeItem::from_clause_run` (FR-286), and the document built from the records equals `OutcomeDocument::from_clause_runs` over the same clause runs, for the runs FR-286-AC-13 names. | Test (TC-765) |
| FR-283-AC-15 | Over FR-501-AC-4's trace (stamps 0, 2, 5, `W` 5, `Complete { through: 5 }`, scope `Closed`, `p` false at stamps 0 and 5 and its valuation absent at stamp 2) as a `Typed` `Timestamped` trace, with `temporal_selections` holding the clause's `TimestampedEvent` selection and a `binding` that agrees with the emitted rows, `eventually[0,5] holds(p)` under the finite-window profile reports `Indeterminate` at position 0 with reason `MissingValuation { position: 1, atom: p }`, its `atom` named by the source text `p`, category indeterminate, exit 11; with `sources` empty the same request refuses with `MonitorRefusal::SourceNotSupplied`; with `W` 3 it reports `Pending`, exit 0. The same trace whose `binding` asserts another clock name refuses the whole request with `MonitorRefusal::ClauseRefused`, `invalid_runtime_input`/`invalid-value` with `mismatched_dimension` `Clock`, and no record, also when its position 1 snapshot fails FR-106 admission; with `temporal_selections` empty it refuses with `ClauseRefused`, `missing_declaration`/`missing-selection`; the clause over AC-1's `Document` trace refuses with `ClauseRefused`, `invalid_runtime_input`/`invalid-value` (FR-330-AC-9). Over the same `Timestamped` trace (binding unit `ms`), each of these refuses the whole request with `ClauseRefused` naming the clause and no record: `temporal_selections` lacking the selection of a clock-bearing declaration the request does not select, `missing_declaration`/`missing-selection` with `member` `temporal_selections`; `temporal_selections` with a selection repeated, with a surplus selection at a span no declaration holds, or with the clause's selection replaced by an `EventPosition` configuration, each `invalid_runtime_input`/`invalid-value` with `member` `temporal_selections`; the clause's selection with `timestamp_unit` `us`, `invalid_runtime_input`/`invalid-value` with `mismatched_dimension` `TimestampUnit`, `asserted` `ms` and `expected` `us`; `packages` empty, its package document truncated to one byte, or another document under its identity, each `invalid_runtime_input`/`invalid-value` with `member` `packages` and `stage` `strict-read`; and a `binding` whose profile, clock parameters or timestamp unit is forged, `invalid_runtime_input`/`invalid-value` with `mismatched_dimension` `Profile`, `ClockParameters` and `TimestampUnit` respectively, though position 1 names a snapshot `snapshots` does not hold, so no position is admitted. | Test (TC-765) |

## Dependencies

- ADR-029 OP-3: the operation.
- ADR-014 A-4: finite-prefix and lasso evaluation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): position admission.
- FR-330: the `Temporal` selection, `TraceInput`, the trace-subject route and `TemporalCounterexample`; FR-327 to FR-329: the trace evaluator; FR-267: the temporal-disposition rows the verdict follows; FR-261: the shared reader.
- FR-194: window aggregates and `max_window_values`; FR-501 and FR-504: the finite-window and progressed evaluations; FR-522 and FR-515: binding authentication against the package's rows; FR-100: the rendered `locus`.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- ADR-018 (draft): the temporal clauses.
- QSpec FR-300: the `monitor` operation; QSpec FR-362: the missing fairness
  premise over a supplied trace.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSpec FR-300 (STD-141): the QSpec half.
- QSpec FR-362: the missing fairness premise.
- QSL-366: an undefined evaluation settles as a violation with cause
  `UndefinedEvaluation`, and the team-leader decision ADR-029's References
  records applies it to `monitor`.
