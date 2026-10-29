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
    request: &MonitorRequest,
    limits: &MonitorLimits,
    cancel: &Cancel,
) -> Result<Staged<MonitorOutcome>, StageFailure<MonitorRefusal>>;

pub struct MonitorRequest {
    pub clauses: Vec<MonitorClause>,
    pub trace: Vec<u8>,
    pub snapshots: SnapshotProvision,
    pub packages: PackageInput,
}

pub struct MonitorClause {
    pub name: QualifiedName,
    pub over: Option<ObjectRef>,
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
  clause's declared name and, when the clause has an `over` parameter, the
  object it binds, resolved in the observation at position 0, as FR-330's
  `Temporal` selection names them.
- `trace`: the bytes of a trace document ("Trace document" below), holding
  a finite trace, or a lasso: prefix positions followed by a non-empty loop
  that re-enters at its first position. Each position is a snapshot named
  by its `DocumentRef`.
- `snapshots`: the FR-106 snapshot provision, document bytes by
  identity labels, that the positions name; `packages`: FR-056's
  package input, the domain package bytes FR-106 admission reads.
- `limits`: the observation and model-normalization limits FR-106
  admission takes, the evaluator's accounting limits, and FR-194's
  `max_window_values` (setting `monitor.max_window_values`); each field has
  its published default (FR-277, FR-194).
- `&Cancel`.

### Trace document

The trace document is a UTF-8 JSON object, format `quire.monitor-trace/1`,
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
6. An empty `prefix` with `loop` absent, and an empty `loop`:
   `invalid_runtime_input`/`invalid-value` at `/prefix` and at `/loop`.

## Outputs

`Staged<MonitorOutcome>`:

```rust
pub struct MonitorOutcome {
    pub records: Vec<MonitorRecord>,
}

pub struct MonitorRecord {
    pub clause: QualifiedName,
    pub verdict: MonitorVerdict,
    pub category: Category,
}
```

`records` holds one record per selected clause, in request order. Each
record's verdict is one of FR-267's temporal-disposition rows, with `tested`
in the place of `holds`, under ADR-014 A-4:

| `MonitorVerdict` | Evaluator result (FR-330) | Members | O-16 category | Exit (FR-285) |
| --- | --- | --- | --- | --- |
| `Violated` | `Completed(false)`, finite or lasso | `position`, `counterexample`, `witness` | violation | 10 |
| `Undefined` | `Undefined` with a kernel or family cause | `position`, `counterexample`, `cause` (`UndefinedEvaluation{where, cause}`) | violation | 10 |
| `InsufficientData` | FR-327's insufficient-data outcome, `InsufficientData{where, atom, count, min_count}` (a window aggregate short of data, QSpec FR-414), not an `Undefined` | `position`, `count`, `min_count` | undefined, with the label `undefined` and reason `insufficient-data` | 10 |
| `Pending` | `Pending`, finite trace | none | inconclusive | 0 |
| `Tested` | `Completed(true)` | none | success | 0 |
| `MissingFairnessPremise` | `MissingFairnessPremise`, a clause with a non-empty fairness set | `constraint`, the first constraint of the fairness set | unsupported | 21 |
| `Incomplete` | `Incomplete` | the charge point and limit | incomplete | 22 |
| `WindowLimitReached` | `WindowLimitReached` | `limit`, `value`, `setting` | incomplete | 22 |
| `ConditionRefused` | a `holds` condition refused (FR-327), other than `CheckedInvariant` | `position`, the refusal record | the record code's category | its code |
| `TraceRefused` | a `TemporalRefusal` other than `Fault` | the refusal | the refusal code's category | its code |

- `position` is the `TemporalPosition` (FR-327) of the trace.
- `counterexample` is the `TemporalCounterexample` FR-330's report holds:
  QSpec FR-364's observed arm, built from the trace's `DocumentRef`
  identities, with `trace_position` equal to `position`. It lets QSpec FR-364's
  replayer re-run the trace.
- `witness` is the QSpec FR-351 separating witness record of the violation,
  the record that backs a decisive temporal counterexample (QSpec FR-351,
  FR-094). Its trace position is `position`. Its deciding quantifier,
  deciding element, index and FR-207 value path are the ones FR-107's clause
  evaluator retains, through FR-267's witness channel, for the deciding
  atom: the first `holds` atom, in clause-node order, that evaluates false
  at `position` for the failing activation, evaluated over the observation
  at `position`. When that atom's evaluation is decided by no collection
  quantifier, the record has no deciding quantifier and no index, its
  deciding element is the atom's value `false`, and its value path is the
  empty path at the observation's root.
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

`MonitorRefusal`, the cause of a whole-request `StageFailure::Refused`, is
`TraceDocument { record, pointer }` for a "Trace document" defect, or
`Admission { position, failure }` for the first position whose FR-106
admission fails, with FR-106's `AdmissionFailure`. Neither holds a clause
record.

## Behavior

- The `monitor` operation shall read the trace document under "Trace
  document" and refuse the request with `MonitorRefusal::TraceDocument` at
  its first defect, before it admits any position.
- The `monitor` operation shall admit every trace position as an FR-106
  snapshot, in position order, before it evaluates any clause.
- If any position fails admission, then `monitor` shall refuse the whole
  request with `MonitorRefusal::Admission` holding that position's index and
  admission failure, with no clause record.
- The `monitor` operation shall evaluate each selected clause over the
  admitted trace with the layer-5 trace evaluator replay uses.
- When the trace is finite and ADR-014 A-4's finite-prefix rule makes the
  clause false for every extension, `monitor` shall report a violation.
- When the trace is finite and the evaluator decides the clause neither
  false nor true on it (FR-328 `Pending`), `monitor` shall report pending.
- If a lasso's loop is empty, then `monitor` shall refuse the request with
  `invalid_runtime_input`/`invalid-value` at `/loop`, with no clause record.
- When `monitor` reports `Violated`, it shall attach the QSpec FR-351
  separating witness that "Outputs" defines, with trace position equal to
  the violation's position.
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
| FR-283-AC-1 | Over a three-position finite trace whose `p` is false at position 2, `always p` reports a violation at position 2 with its `counterexample`, category violation. The same clause over a trace whose `p` is true at every position reports pending, category inconclusive, and FR-285 maps it to exit 0. | Test (TC-765) |
| FR-283-AC-2 | Over a lasso of a two-position prefix and a two-position loop, `always eventually q`, whose fairness set is empty, reports `tested` when `q` holds at a loop position and a violation when `q` holds at no loop position. | Test (TC-765) |
| FR-283-AC-3 | A trace whose second position's snapshot fails FR-106 admission is refused with that admission cause and position index 1, and the outcome holds no clause record. A lasso with an empty loop is refused with `invalid_runtime_input`/`invalid-value`, and the outcome holds no clause record. | Test (TC-766) |
| FR-283-AC-4 | For each AC-1 and AC-2 trace, the verdict `monitor` reports equals the verdict layer-6 replay's trace evaluation reports for the same clause and trace. | Test (TC-766) |
| FR-283-AC-5 | Over a three-position finite trace whose `x` is 0 at position 1, `always (10 / x > 0)` reports a violation at position 1, category violation, with cause `UndefinedEvaluation` whose `cause` is `division-by-zero`; the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-765) |
| FR-283-AC-6 | Over AC-2's lasso, `always eventually q` under `fair weak inc` settles its record unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`, and FR-285 maps the outcome to exit 21; in a request selecting it beside AC-2's clause without fairness, the second clause still reports `tested`. | Test (TC-766) |
| FR-283-AC-7 | AC-1's three-position trace as the document `{"format":"quire.monitor-trace/1","prefix":[R0,R1,R2]}`, each `Ri` a snapshot `DocumentRef` in `snapshots`, gives the AC-1 records; AC-2's lasso as `{"format":"quire.monitor-trace/1","prefix":[R0,R1],"loop":[R2,R3]}` gives the AC-2 records. Each record is a `MonitorRecord` naming its clause; AC-1's violation is `Violated` at position 2 whose `counterexample` is FR-330's `TemporalCounterexample` with `prefix` the three identities, an empty `loop` and `trace_position` 2. | Test (TC-765) |
| FR-283-AC-8 | Each of these trace documents refuses the whole request with `MonitorRefusal::TraceDocument`, no position admitted and no record: bytes `[` (`invalid_runtime_input`/`invalid-value` at `""`); the JSON array `[]` (`invalid_runtime_input`/`wrong-value-kind` at `""`); `{"prefix":[]}` with no `format` (`invalid_runtime_input`/`missing-member` naming `format`, not `unknown_wire`); `format` `quire.monitor-trace/2` (`unknown_wire`/`unsupported-wire` at `/format`); no `prefix` (`missing-member`); an extra member `fairness` (`unknown-member`); `prefix` a string (`wrong-value-kind` at `/prefix`); an element with a blank `authority` label (`invalid_source_identity`/`blank-label` at `/prefix/0/identity`); `{"format":"quire.monitor-trace/1","prefix":[]}` (`invalid-value` at `/prefix`); and a document one byte above `observation.input_bytes` (`stage_limit_exceeded`/`input-bytes-exceeded` naming `observation.input_bytes`). | Test (TC-766) |
| FR-283-AC-9 | Over AC-1's trace, a request selecting `always p` and `always (10 / x > 0)` returns two records in request order; over FR-327-AC-1's closed `Counter` trace (`c.value` 0, 1, 2) as a three-position finite document, `eventually[0,2] holds(c.value = 2)` under event-position false-extension, `on origin`, reports `Tested`, category success, exit 0, and `eventually[0,1] holds(c.value = 2)` reports `Violated` at position 0. | Test (TC-765) |
| FR-283-AC-10 | AC-1's `always p` violation carries a `witness` with trace position 2, no deciding quantifier and no index, deciding element `false` and the empty value path at the observation's root. Over a three-position finite trace whose position-2 snapshot holds a collection field `items` of values 3, 0, 5, the clause `always holds(forall i in self.items : i > 0)` reports `Violated` at position 2 whose `witness` names the `forall` occurrence key as deciding quantifier, deciding element 0, index 1, the FR-207 value path to `items` and trace position 2, equal to the record FR-107's evaluator retains for that `holds` atom over the position-2 observation. | Test (TC-765) |
| FR-283-AC-11 | Over FR-327-AC-1's closed `Counter` trace (`c.value` 0, 1, 2) as a three-position finite document under event-position false-extension, `always[0,2] (count over past[0, 9] min_count 2 >= 1)` `on origin` reports `InsufficientData` at position 0 with `count` 1 and `min_count` 2, category undefined, the label `undefined` with reason `insufficient-data`, and exit 10. | Test (TC-765) |
| FR-283-AC-12 | Over AC-1's trace: `always p` with accounting `work_units` 0 reports `Incomplete` naming `work_units`, category incomplete, exit 22; a window aggregate clause whose window holds 2 values with `max_window_values` 1 reports `WindowLimitReached` with `limit` `max_window_values`, `value` 1 and `setting` `monitor.max_window_values`, category incomplete, exit 22. Over FR-327-AC-6's trace, its `holds` atom refused at position 1 reports `ConditionRefused` at position 1 with the record `integer_out_of_domain`/`outside-domain` and its locus, category refusal, exit 20. | Test (TC-766) |
| FR-283-AC-13 | Over AC-2's lasso, `eventually[0,2] holds(q)` under event-position false-extension, a bounded profile, reports `TraceRefused` with the `TemporalRefusal::LassoUnderBoundedProfile` refusal naming that profile and the category and exit of its code. With a test trace evaluator that returns `TemporalRefusal::Fault` for one selected clause, the request fails with `StageFailure::Fault`, no record for any clause, and exit 30. | Test (TC-766) |

## Dependencies

- ADR-029 OP-3: the operation.
- ADR-014 A-4: finite-prefix and lasso evaluation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): position admission.
- FR-330: the `Temporal` selection, `TraceInput` and `TemporalCounterexample`; FR-327 to FR-329: the trace evaluator; FR-267: the temporal-disposition rows the verdict follows; FR-261: the shared reader.
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
