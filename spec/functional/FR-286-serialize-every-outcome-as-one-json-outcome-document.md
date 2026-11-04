---
id: FR-286
title: "Serialize every library outcome as one JSON outcome document"
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
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: references
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-286: Serialize every library outcome as one JSON outcome document

## Description

Each QSL lifecycle operation's outcome SHALL serialize to one JSON outcome
document (ADR-029 CB-4). A CLI in machine mode writes exactly that document
to stdout, so the same request through the library and through the CLI gives
the same document (QSpec FR-300-AC-3). The document's members are:

- `format`: the document's format identity, `quire-outcome/1`.
- `operation`: the library operation that produced it.
- `last_stage`: the last ADR-011 stage the operation reached, or `null`
  when no stage is known: a cancelled outcome that stopped before its first
  stage, or an internal-failure (fault) outcome.
- `category`: the ADR-013 O-16 category of the whole outcome, as the one
  `Category` type (FR-285); each item's category is the same type.
- `items`: for `prove`, `analyze` and `monitor`, one entry per requested item
  with its category. The member is always present;
  it is an empty array for every other operation and for an outcome with no
  items. Each item that has a terminal record, every proof result, carries its
  QSpec FR-331 terminal record with its label
  and cause in FR-331's wire spelling (its wire-spelling table): for example
  `undefined-evaluation{where, cause}` and `replay-parity`. A `monitor` item
  built from a clause run that did not settle (pending or indeterminate)
  carries the clause-run record, with the `clause` member, and no FR-331
  terminal record (see "Monitor documents from clause runs"). The scalar-parity
  cause `ScalarAgrees` (FR-357) is spelled `scalar-agrees` and carries its kind
  only: the claim identity and outcome stay on the typed `ScalarAgreement`. A proof item (`prove`,
  `analyze`) never carries the label `undefined`: an undefined claim
  evaluation is `refuted` with cause `undefined-evaluation{where, cause}`,
  category violation (FR-281). A `monitor` clause whose evaluation over the
  trace is undefined is a violation with that cause (FR-283). The label
  `undefined` belongs to the category undefined and to no other category: it
  appears on a non-proof evaluation outcome, such as an `execute` outcome, and
  on a `monitor` item for FR-283's `InsufficientData` (category undefined,
  reason `insufficient-data`), and on no item of any other category (FR-285).
- `diagnostics`: each with its typed cause, catalog code, `Locus` (ADR-013
  T-5, FR-095) and message.
- `artifacts`: always an empty array `[]`. No operation writes an artifact
  identity into the outcome document: `check` produces a `CheckedPackage` and
  no `package_id` (ADR-029 OP-1).
- `result`: the evaluation result of an `execute` outcome, the outcome
  FR-100's `spine-run-result/1` member `outcome` carries for a completed,
  undefined or incomplete call. It is `null` for every other operation, and
  for an `execute` outcome that is refused (its refusal record is a
  diagnostic), cancelled or a fault. Otherwise it is exactly one of:
  - `{"kind": "completed", "value": V}`, where `V` is FR-100's value
    rendering: `{"kind": "boolean", "value": true|false}` or
    `{"kind": "integer", "decimal": "<ASCII decimal>"}` (FR-038's integer
    spelling), or the existing canonical Reference value
    `{"kind":"reference","universe":U,"type":T,"object_identity":O}`.
    For a Reference completion, `U` and `T` are the actual returned shared
    reference's universe and most-specific effective type, each as 64
    lowercase hexadecimal characters; `O` is its exact nonempty UTF-8
    object identity, without normalization, reminting or substitution by an
    input reference. This reuses the kernel value wire Reference form; it
    carries neither object state nor a Population binding. A clause
    evaluation still completes with its `Boolean` value.
  - `{"kind": "undefined", "reason": "<reason>"}`, `reason` spelled as
    FR-100's undefined tables spell it, such as `sum-out-of-domain` or
    `precondition-false`.
  - `{"kind": "incomplete", "limit": {"kind": K, "bound": B, "counter": C,
    "field": F, "setting": S}}`, the exhausted limit as FR-277 names one: `K` the limit
    kind, the exhausted counter's `quire.value.accounting/v1` member name
    (such as `work_units`); `B` the configured value; `C` the counter at the failed
    charge as FR-277 defines it (the consumed value plus the denied amount for
    a cumulative counter, the denied size for a high-water counter); `F` the name of the `execute`
    limits field that sets the bound, which for an accounting counter is the
    `ScalarLimits` field of the same name; `S` the FR-255 setting that
    raises it, which for an accounting counter is the counter name `K`. `B` and `C` are
    ASCII decimal strings in FR-038's integer spelling.

## Monitor documents from clause runs

A `monitor` document is built from clause runs. The producer lives with the
outcome types in `qsl-replay` (the one renderer a core crate may hold,
FR-284): `OutcomeItem::from_clause_run(request_index, clause, &ClauseRunReport)`
builds one item from one FR-109 / FR-330 clause run, and
`OutcomeDocument::from_clause_runs(last_stage, runs)` builds the document
with operation `monitor` through `OutcomeDocument::settled`, one item per
selected clause in request order. `qsl-inspect` and the driver read the
document and build no item of their own. The `monitor` operation's records
(FR-283) are rendered into items through this producer, and no other code path
builds an item for a clause run.

Each item takes its category from `ClauseRunReport::category()` (the catalog's
`Category`, FR-285) and from nowhere else, and its `result` label from that
category:

| Category of the run | `result` label | `cause` |
| --- | --- | --- |
| success (`Completed(true)`, FR-283's `Tested`) | `tested` | none |
| violation (`Completed(false)`, FR-283's `Violated`) | `refuted` | none: FR-331 gives a `refuted` result a cause only for `undefined-evaluation` |
| violation (an `Undefined` evaluation, FR-283's `Undefined`) | `refuted` | `undefined-evaluation{where, cause}` |
| pending (a finite-trace or finite-window `Pending`) | `pending` | none |
| indeterminate (FR-501's `Indeterminate`) | `indeterminate` | FR-501's reason, as its result report spells it |
| incomplete | `incomplete` | the exhausted limit |
| unsupported (for example a missing fairness premise) | `unsupported` | the refusal cause |
| refusal | `declined` | the refusal cause |
| undefined (FR-283's `InsufficientData`) | `undefined` | `insufficient-data` |

A `monitor` item carries the members of FR-283's record for its verdict, besides
`clause`, `result`, `category` and `cause`:

| Verdict | Members besides `clause`, `result`, `category` and `cause` |
| --- | --- |
| `Violated` | `position`; `counterexample`, FR-283's `TemporalCounterexample` with its `prefix`, its `loop` (`[]` for a finite trace, as FR-330 and QSpec FR-364 spell it) and `trace_position`; no `witness` |
| `Undefined` | `position`; `counterexample` as for `Violated`; no `witness` |
| `InsufficientData` | `position`, `count` and `min_count`; no `counterexample` and no `witness` |
| `Pending`, `Tested` | none |
| `Indeterminate` | `position` where FR-501's reason names one |

`pending` and `indeterminate` are catalog labels (`Category::as_str`), not
FR-331 terminal-record labels: a clause run that did not settle has no terminal
record, so these two items carry no FR-331 terminal record, only the clause-run record, and the label is the catalog's.
Every item built from a clause run also carries `clause`, the clause's
qualified name, so that a consumer names the pending clauses (FR-285's status
`pending` and the ids of the pending clauses) from the document alone. An
evaluator fault is not an item: it fails the whole request (FR-283).

The document's category is the most severe item's under FR-285's order, so a
`monitor` document whose only item is a pending clause has category pending,
exit 0, and a document with a pending and an indeterminate item has category
indeterminate, exit 11, and with a violated item beside either it has category
violation, exit 10.

`quire-outcome/1` is the outcome document of every operation, `execute`
included; it replaces FR-100's `spine-run-result/1` when the driver's verbs
land (ADR-029 CB-1).

The serialization lives with the outcome types in the core; it is the one
renderer a core crate may hold (FR-284).

## Inputs

A lifecycle operation's outcome value.

## Outputs

The outcome document's bytes.

## Behavior

- Each lifecycle operation's outcome type shall serialize to an outcome
  document with the members the Description lists.
- The same outcome value shall serialize to the same bytes every time.
- The `category` member shall be the outcome's O-16 category, from which
  FR-285 derives the exit code.
- The serialization shall write the label `undefined` on an item exactly when
  the item's category is undefined, so on no `prove` or `analyze` item and on
  a `monitor` item only for FR-283's `InsufficientData`.
- The serialization shall write each item's label and cause in QSpec
  FR-331's wire spelling.
- The serialization shall write `last_stage` `null` for a cancelled outcome
  that reached no stage and for a fault outcome.
- The serialization shall write the `items` member on every document, as an
  empty array when the outcome has no items.
- `OutcomeDocument::from_run` shall serialize `qsl_replay::spine::run`'s
  result as one `execute` document: a call that ran is `from_call`'s
  document with `artifacts` an empty array; a
  compile refusal carries the stage it reached, the refusal's category and its
  diagnostic; a call-stage refusal (FR-100) carries `S6a`, its category and its
  code; a cancelled run is incomplete with `last_stage` `null`, whichever
  stage the cancel stopped, and the cancel's cause; a
  fault is internal failure at no stage with its catalog code.
- A driver failure before any operation runs shall be one
  `OutcomeDocument::new(operation, None, category)` with one diagnostic. An
  engine the request names that the driver has not built (AOT or JIT, where
  only the interpreter exists) is valid meaning the selected producer does
  not implement, so its diagnostic code is QSpec FR-271's
  `unimplemented_capability`, never `unsupported_construct` (a source form
  the selected profile prohibits), with QSpec FR-272's cause
  `backend-not-supplied` and `backend` the engine's name (`aot` or `jit`), in
  category unsupported (FR-285, exit 21).
- `OutcomeItem::from_clause_run` shall give each item the members the
  "Monitor documents from clause runs" member table lists for its verdict and
  no other, reading `counterexample` from `ClauseRunReport::counterexample`
  (FR-330 Outputs; the same record FR-267 writes in its `temporal` member).
- `OutcomeItem::from_clause_run` shall write no `witness`: FR-266 gives a
  temporal violation no FR-351 witness record.
- `OutcomeItem::from_clause_run` shall write no `counterexample` on an item
  whose report holds none.
- `OutcomeDocument::from_clause_runs` shall build a `monitor` document with
  one item per clause run, the item's category read from
  `ClauseRunReport::category()` and its label and cause as the "Monitor
  documents from clause runs" table states, the pending and indeterminate
  items included.
- A `monitor` item for a pending clause shall have `result` `pending` and
  `clause` naming the clause.
- A `monitor` document whose only item is a pending clause shall have category
  pending and exit 0.
- The serialization shall write an `execute` outcome's completed value,
  undefined reason or exhausted limit as the `result` member, and `result`
  `null` on every other document.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-286-AC-1 | The `check` outcome over `tests/fixtures/spine-compile.native` serializes to a document whose `format` is `quire-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, `items`, `diagnostics` and `artifacts` each an empty array, and `result` `null`. The `package` outcome over that `check` outcome serializes with `operation` `package`, `last_stage` S4, `category` success and `artifacts` an empty array `[]`. | Test (TC-770) |
| FR-286-AC-2 | The `check` outcome of FR-100-AC-5's `inv` source serializes with `category` refusal and one diagnostic whose cause, catalog code `ill_typed`, `Locus` and message equal the `StageFailure`'s. | Test (TC-770) |
| FR-286-AC-3 | An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes. | Test (TC-770) |
| FR-286-AC-4 | FR-281-AC-7's `analyze` outcome serializes its item with category violation and cause `undefined-evaluation`, and the document holds no `undefined` label; FR-283-AC-5's `monitor` outcome serializes the same way; FR-100-AC-10's `execute` outcome serializes with category undefined and the label `undefined`. | Test (TC-770) |
| FR-286-AC-5 | The `execute` outcome of FR-100-AC-1's `seven` serializes with `category` success and `result` `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`; FR-100-AC-10's empty `sum` with `category` undefined and `result` `{"kind": "undefined", "reason": "sum-out-of-domain"}`; FR-100-AC-6's `seven` with `call.accounting` `{"work_units": 0}` with `category` incomplete and `result` `{"kind": "incomplete", "limit": {"kind": "work_units", "bound": "0", "counter": "1", "field": "work_units", "setting": "accounting.work_units"}}`. A `check` called with a `Cancel` already cancelled serializes with `category` incomplete, `last_stage` `null`, `items` `[]` and `result` `null`. | Test (TC-770) |
| FR-286-AC-6 | `OutcomeDocument::from_run` over a completed run holds the call's document and `artifacts` an empty array; over `run` refusing `ill_typed` source, `category` refusal, `last_stage` S3 and one `ill_typed` diagnostic; over an unknown function, `category` refusal at S6a with `missing_declaration`; over `run` with a cancelled handle, `category` incomplete, `last_stage` null and one `cancelled` diagnostic with cause `requested`; over an internal fault, internal failure with its catalog code; and a driver's `OutcomeDocument::new(Execute, None, Unsupported)` for an unbuilt engine holds one diagnostic whose code serializes as `unimplemented_capability` with cause `backend-not-supplied` and `backend` naming the engine, with `category` unsupported, `last_stage` `null`, and exits 21. | Test (TC-770) |
| FR-286-AC-7 | TC-792's present lookup through the public spine run entry serializes through `OutcomeDocument::from_run` as one `quire-outcome/1` execute document, with category success, one compiled package artifact and `result` `{"kind":"completed","value":{"kind":"reference","universe":U,"type":T,"object_identity":O}}`. All three components equal the actual returned shared Reference. A second source-compiled Reference-valued function returning a different admitted reference from its input serializes the returned reference, not the input. The existing Boolean/Integer completions remain unchanged; no Reference completion is emitted as a `/2` clause truth or a Population payload. | Test |
| FR-286-AC-8 | `OutcomeDocument::from_clause_runs` over one clause run of `always p` over a finite supplied trace that ends pending serializes to a document with operation `monitor`, `category` pending, exactly one item with `result` `pending`, `category` pending and `clause` naming the clause, no `cause`, no terminal record and no `undefined` label, and FR-285 maps the document to exit 0. | Test (TC-770) |
| FR-286-AC-9 | `OutcomeDocument::from_clause_runs` over FR-501-AC-4's `Indeterminate` run serializes one item with `result` `indeterminate`, category indeterminate and FR-501's reason, and the document exits 11; over that run and a pending run it is category indeterminate; over those two and a `Completed(false)` run (item `refuted`, category violation) it is category violation, exit 10; over a `Completed(true)` run it holds the item `tested`, category success. Every item's category equals `ClauseRunReport::category()` and equals the label and exit FR-285-AC-10 reads. | Test (TC-770) |
| FR-286-AC-10 | Over FR-283-AC-1's clause run `always p` violated at position 2, `from_clause_run` gives an item with `result` `refuted`, category violation, `clause`, `position` 2, a `counterexample` with a `prefix` of three identities, an empty `loop` (`[]`, the finite spelling of FR-330 and QSpec FR-364) and `trace_position` 2, no `witness` and no `cause` member. | Test (TC-770) |
| FR-286-AC-11 | Over FR-283-AC-5's clause run, `from_clause_run` gives an item with `result` `refuted`, category violation, `position` 1, a `counterexample` with `trace_position` 1, cause `undefined-evaluation{where, cause}` with `cause` `division-by-zero`, no `witness` and no `undefined` label. | Test (TC-770) |
| FR-286-AC-12 | Over FR-283-AC-11's clause run, `from_clause_run` gives an item with `result` `undefined`, category undefined, cause `insufficient-data`, `position` 0, `count` 1 and `min_count` 2, and neither `counterexample` nor `witness`; its document exits 10. | Test (TC-770) |
| FR-286-AC-13 | Each `MonitorRecord` the FR-283 `monitor` operation returns for the clause runs of AC-8 to AC-12 (the pending run, FR-501-AC-4's indeterminate run, and the violated, undefined-evaluation and insufficient-data runs of FR-283-AC-1, AC-5 and AC-11) renders through `OutcomeItem::from_clause_run`, and the document built from the records equals `from_clause_runs`'s document for the same runs, byte for byte. | Test (TC-770) |

## Status

The Reference-completion amendment under QSL-665 is source-only; producer,
serializer and consumer qualification for AC-7 remain UNRUN. Existing
accounting-limit rendering and AC-5 remain their accounting owner's scope.

Partial (QSL-592). AC-1, AC-2 and AC-5 are tested over real outcomes. AC-3
and AC-4's `analyze` and `monitor` halves are tested over items built with
the public outcome builders, because layer A has no `AnalyzeOutcome` or
`MonitorOutcome` yet. Remaining: QSL-596 (FR-281, `analyze`) and QSL-597
(FR-283, `monitor`) each re-run TC-770 steps 3 and 4 over their real
outcomes.

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md), [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md), [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md) and [FR-283](FR-283-monitor-a-supplied-trace-offline.md): the clause runs and verdicts the monitor document reports.
- **Delivery:** FR-286-AC-8 to FR-286-AC-12 (the clause-run item and document producer) are delivered by QSL-1622. FR-286-AC-13 (the monitor operation's records render through that producer) is delivered by the ticket that builds the `monitor` operation (FR-283, QSL-597).
- ADR-029 CB-4: the document members.
- ADR-013 O-16, T-5: category and `Locus`.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): `Locus`.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit code.
- QSpec FR-300-AC-3, FR-301-AC-1: library and CLI agree.
- QSpec FR-331: the terminal record and its wire spellings.
- QSpec FR-271: the diagnostic code catalog, `unimplemented_capability` among
  it.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the value,
  undefined reason and limit spellings of `result`.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): the
  exhausted limit's members.

## Overlap

The driver repository specifies and tests that `quire --format json` writes
this document to stdout with no member added or removed, by QSpec
FR-300-AC-3 and FR-301-AC-1. QSL's part is the document and its
serialization.

## References

- QSL-390 (ARCH-50): the team-leader decision ADR-029's References records,
  which keeps the `undefined` label off proof items (QSL-366 ruling).
- QSpec FR-300 (STD-141): the QSpec half.
