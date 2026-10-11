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
  with its terminal record and its category. The member is always present;
  it is an empty array for every other operation and for an outcome with no
  items. Each item carries its QSpec FR-331 terminal record with its label
  and cause in FR-331's wire spelling (its wire-spelling table): for example
  `undefined-evaluation{where, cause}` and `replay-parity`. The scalar-parity
  cause `ScalarAgrees` (FR-357) is spelled `scalar-agrees` and carries its kind
  only: the claim identity and outcome stay on the typed `ScalarAgreement`. A proof item (`prove`,
  `analyze`) never carries the label `undefined`: an undefined claim
  evaluation is `refuted` with cause `undefined-evaluation{where, cause}`,
  category violation (FR-281). A `monitor` clause whose evaluation over the
  trace is undefined is a violation with that cause (FR-283). The label
  `undefined` appears only on a non-proof evaluation outcome, such as an
  `execute` outcome (FR-285).
- `diagnostics`: each with its typed cause, catalog code, `Locus` (ADR-013
  T-5, FR-095) and message.
- `artifacts`: the identities of the artifacts produced, such as a
  `package_id` or a generated artifact's content identity. An operation that
  mints no identity writes an empty array: `check` produces a
  `CheckedPackage` and no `package_id` (ADR-029 OP-1), so a `check` document's
  `artifacts` is empty, and the `package` outcome carries the `package_id`.
- `result`: the evaluation result of an `execute` outcome, the outcome
  FR-100's `spine-run-result/1` member `outcome` carries for a completed,
  undefined or incomplete call. It is `null` for every other operation, and
  for an `execute` outcome that is refused (its refusal record is a
  diagnostic), cancelled or a fault. Otherwise it is exactly one of:
  - `{"kind": "completed", "value": V}`, where `V` is FR-100's value
    rendering: `{"kind": "boolean", "value": true|false}` or
    `{"kind": "integer", "decimal": "<ASCII decimal>"}` (FR-038's integer
    spelling). A clause evaluation completes with its `Boolean` value.
  - `{"kind": "undefined", "reason": "<reason>"}`, `reason` spelled as
    FR-100's undefined tables spell it, such as `sum-out-of-domain` or
    `precondition-false`.
  - `{"kind": "incomplete", "limit": {"kind": K, "bound": B, "counter": C,
    "field": F}}`, the exhausted limit as FR-277 names one: `K` the limit
    kind, the exhausted counter's `quire.value.accounting/v1` member name
    (such as `work_units`); `B` the configured value; `C` the counter at the failed
    charge as FR-277 defines it (the consumed value plus the denied amount for
    a cumulative counter, the denied size for a high-water counter); `F` the name of the `execute`
    limits field that sets the bound, which for an accounting counter is the
    `ScalarLimits` field of the same name. `B` and `C` are ASCII decimal
    strings in FR-038's integer spelling.

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
- The serialization shall write the label `undefined` on no `prove`,
  `analyze` or `monitor` item.
- The serialization shall write each item's label and cause in QSpec
  FR-331's wire spelling.
- The serialization shall write `last_stage` `null` for a cancelled outcome
  that reached no stage and for a fault outcome.
- The serialization shall write the `items` member on every document, as an
  empty array when the outcome has no items.
- `OutcomeDocument::from_run` shall serialize `qsl_replay::spine::run`'s
  result as one `execute` document: a call that ran is `from_call`'s
  document with the compiled package's `package_id` as its artifact; a
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
  the selected profile prohibits), in category unsupported (FR-285, exit 21).
- The serialization shall write an `execute` outcome's completed value,
  undefined reason or exhausted limit as the `result` member, and `result`
  `null` on every other document.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-286-AC-1 | The `check` outcome over `tests/fixtures/spine-compile.native` serializes to a document whose `format` is `quire-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, `items`, `diagnostics` and `artifacts` each an empty array, and `result` `null`. The `package` outcome over that `check` outcome serializes with `operation` `package`, `last_stage` S4, `category` success and `artifacts` holding exactly the emitted package's `package_id`. | Test (TC-770) |
| FR-286-AC-2 | The `check` outcome of FR-100-AC-5's `inv` source serializes with `category` refusal and one diagnostic whose cause, catalog code `ill_typed`, `Locus` and message equal the `StageFailure`'s. | Test (TC-770) |
| FR-286-AC-3 | An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes. | Test (TC-770) |
| FR-286-AC-4 | FR-281-AC-7's `analyze` outcome serializes its item with category violation and cause `undefined-evaluation`, and the document holds no `undefined` label; FR-283-AC-5's `monitor` outcome serializes the same way; FR-100-AC-10's `execute` outcome serializes with category undefined and the label `undefined`. | Test (TC-770) |
| FR-286-AC-5 | The `execute` outcome of FR-100-AC-1's `seven` serializes with `category` success and `result` `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`; FR-100-AC-10's empty `sum` with `category` undefined and `result` `{"kind": "undefined", "reason": "sum-out-of-domain"}`; FR-100-AC-6's `seven` with `work_units` 0 with `category` incomplete and `result` `{"kind": "incomplete", "limit": {"kind": "work_units", "bound": "0", "counter": "1", "field": "work_units"}}`. A `check` called with a `Cancel` already cancelled serializes with `category` incomplete, `last_stage` `null`, `items` `[]` and `result` `null`. | Test (TC-770) |
| FR-286-AC-6 | `OutcomeDocument::from_run` over a completed run holds the call's document and one `package_id` artifact; over `run` refusing `ill_typed` source, `category` refusal, `last_stage` S3 and one `ill_typed` diagnostic; over an unknown function, `category` refusal at S6a with `missing_declaration`; over `run` with a cancelled handle, `category` incomplete, `last_stage` null and one `cancelled` diagnostic with cause `requested`; over an internal fault, internal failure with its catalog code; and a driver's `OutcomeDocument::new(Execute, None, Unsupported)` for an unbuilt engine holds one diagnostic whose code serializes as `unimplemented_capability`, with `category` unsupported, `last_stage` `null`, and exits 21. | Test (TC-770) |

AC-3 and AC-4's `analyze` and `monitor` halves are tested over items
built with the public outcome builders, because layer A has no
`AnalyzeOutcome` or `MonitorOutcome` yet. QSL-596 (FR-281, `analyze`) and
QSL-597 (FR-283, `monitor`) each re-run TC-770 steps 3 and 4 over their
real outcomes.

## Dependencies

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
