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
- `last_stage`: the last ADR-011 stage the operation reached.
- `category`: the ADR-013 O-16 category of the whole outcome, as the one
  `Category` type (FR-285); each item's category is the same type.
- `items`: for `prove`, `analyze` and `monitor`, one entry per requested item
  with its terminal record and its category. A proof item (`prove`,
  `analyze`) never carries the label `undefined`: an undefined claim
  evaluation is `refuted` with cause `UndefinedEvaluation{where, cause}`,
  category violation (FR-281). A `monitor` clause whose evaluation over the
  trace is undefined is a violation with that cause (FR-283). The label
  `undefined` appears only on a non-proof evaluation outcome, such as an
  `execute` outcome (FR-285).
- `diagnostics`: each with its typed cause, catalog code, `Locus` (ADR-013
  T-5, FR-095) and message.
- `artifacts`: the identities of the artifacts produced, such as a
  `package_id` or a generated artifact's content identity.

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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-286-AC-1 | The `check` outcome over `tests/fixtures/spine-compile.native` serializes to a document whose `format` is `quire-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, `diagnostics` empty and `artifacts` holding the package's `package_id`. | Test (TC-770) |
| FR-286-AC-2 | The `check` outcome of FR-100-AC-5's `inv` source serializes with `category` refusal and one diagnostic whose cause, catalog code `ill_typed`, `Locus` and message equal the `StageFailure`'s. | Test (TC-770) |
| FR-286-AC-3 | An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes. | Test (TC-770) |
| FR-286-AC-4 | FR-281-AC-7's `analyze` outcome serializes its item with category violation and cause `UndefinedEvaluation`, and the document holds no `undefined` label; FR-283-AC-5's `monitor` outcome serializes the same way; FR-100-AC-10's `execute` outcome serializes with category undefined and the label `undefined`. | Test (TC-770) |

## Dependencies

- ADR-029 CB-4: the document members.
- ADR-013 O-16, T-5: category and `Locus`.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): `Locus`.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit code.
- QSpec FR-300-AC-3, FR-301-AC-1: library and CLI agree.

## Overlap

The driver repository specifies and tests that `quire --format json` writes
this document to stdout with no member added or removed, by QSpec
FR-300-AC-3 and FR-301-AC-1. QSL's part is the document and its
serialization.

## References

- QSL-390 (ARCH-50): the team-leader decision ADR-029's References records,
  which keeps the `undefined` label off proof items (QSL-366 ruling).
- QSpec FR-300 (STD-141): the QSpec half.
