---
id: FR-267
title: "Write and read run results as native-run-result/2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-266
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-014
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-352
    type: depends_on
---
# FR-267: Write and read run results as native-run-result/2

## Description

The clause-run result document of the spine check route and the `run`
command's command-error envelope SHALL be `native-run-result/2` (QSpec
QSpec FR-352), as ADR-031 SW-10 and SW-11 and ADR-013 OQ-1 decide. The library
serializes `qsl_replay::spine::run_clause`'s `ClauseRunReport` (FR-109) as
the `/2` clause-run document; this requirement owns that producer. A
clause-run document carries the members QSpec FR-352 defines, including the
settlement basis label and the separating witness record of FR-266.

This lands in the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), which also
deletes native `run` (FR-026 to FR-032) and `native-run-result/1`; from that
change `WireFormat` names `native-run-result/2` and no `native-run-result/1`
(ADR-031 R-1).

## Inputs

- For writing: an FR-109 `ClauseRunReport` (with FR-266's `basis` and
  `witness`), or a command error from FR-100's refusal and internal-failure
  paths.
- For reading: a document's bytes.

## Outputs

- One `native-run-result/2` document, encoded through ADR-013 §2's RFC 8785
  encoder.
- For reading: the decoded result, or a located wire refusal.

## Behavior

- The library serializer of an FR-109 `ClauseRunReport` SHALL write every
  clause-run result document with `format` `native-run-result/2`.
- From the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), the `run` command SHALL write every command-error envelope
  with that format; before it, FR-100's refusals use FR-026's
  `native-run-result/1` envelope. A
  function run's output is FR-100's `spine-run-result/1`, the `quire-spec`
  CLI's current output, retired when the driver's verbs land (ADR-029
  CB-1); the outcome document is FR-286's `quire-outcome/1`.
- A clause-run document SHALL carry the report's disposition and usage in
  the members QSpec FR-352 defines for them, `basis` on every
  document, and `witness` exactly when `basis` is
  `decisive-counterexample` or `decisive-witness`.
- A command-error envelope SHALL be one closed JSON object with exactly
  these members:
  - `format`: `native-run-result/2`;
  - `stage`: the stage that refused or failed (for example `request`,
    `intake`, `profile`, `call` or a spine stage);
  - `code`: the catalog code;
  - `cause`: the catalog cause, present exactly when the code's refusal has
    one;
  - `message`: a human-readable message naming the input;
  - `details`: an object holding the code's structured context, as FR-100's
    refusal table states per row;
  - `basis`: `unavailable`.
  It SHALL carry no `witness`.
- The writer SHALL write each component of the record in QSpec's spelling
  for it: the deciding quantifier, the deciding element in the kernel value
  wire form, the index, the value path, and the trace position where
  present.
- The writer SHALL write an absent component as an absent member, never as
  a default value.
- The reader SHALL refuse a document, naming the failing member:
  - whose `witness` is missing a required component, carries a member the
    record does not define, or gives an absent component a default value
    (QSpec FR-351-AC-5);
  - whose `basis` is absent or not a QSpec FR-243 label;
  - that carries `witness` with a basis that is not decisive, or a decisive
    basis with no `witness` (QSpec FR-352-AC-3);
  - whose `format` is `native-run-result/1` or any other identifier this
    reader does not support, under QSpec FR-014's unsupported-version rule
    (QSpec FR-352-AC-2). The version comes from `format` alone, never from
    the document's other content.
- Writing a report and reading the document back SHALL give a `basis` and
  `witness` equal to the report's, under QSpec FR-351 componentwise
  identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-267-AC-1 | FR-266-AC-1's `AllBelow` over `high` report writes a document with `format` `native-run-result/2`, `basis` `decisive-counterexample` and a `witness` holding the `forall`'s occurrence key, the reference to `mid`, `index` 1, the population-root value path and no `trace_position` member; the bytes equal their own RFC 8785 re-encoding, and reading them back gives an equal basis and record. The `AllBelow` over `low` report writes `closed-scope` with no `witness` member, and the exhausted-work report writes `unavailable` with no `witness` member. | Test (TC-742) |
| FR-267-AC-2 | Each of these edits of FR-267-AC-1's `high` document refuses, naming the member: `index` removed; an extra member in `witness`; `trace_position` given as `0`; `basis` removed; `basis` `closed-scope` with the `witness` kept; `basis` `decisive-counterexample` with `witness` removed; `format` set to `native-run-result/1`; `format` set to `native-run-result/3`. | Test (TC-742) |
| FR-267-AC-3 | From the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), `run` over a native-run/1 request whose source declares `edition "9-draft"` writes nothing to stdout and writes a command-error envelope to stderr whose `format` is `native-run-result/2`, with exactly the members `format`, `stage` (`profile`), `code` (`unknown_edition`), `message`, `details` and `basis` (`unavailable`), no `witness` member, exiting 20. | Test (TC-742) |

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-10,
  SW-11 and rulings R-1 to R-3; ADR-013 §2 (the encoder) and OQ-1 (`/2`
  only).
- [FR-266](FR-266-report-a-clause-run-s-witness-and-settlement-basis.md)
  (the report members), [FR-100](FR-100-run-a-named-function-through-the-spine.md)
  (the command-error paths), [FR-109](FR-109-run-a-state-clause-through-the-spine.md)
  (the spine check route and its `ClauseRunReport`).
- QSpec FR-014 (closed wire and unsupported-version rule), QSpec FR-351 (the
  record and strict reader), QSpec FR-352 (`native-run-result/2`). The spelling of
  the basis member and of the record's components is QSpec's (see
  References).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
- QSpec half (Linear STD-144): QSpec FR-352 AC-6 and AC-7 (basis label and
  witness members), FR-207 AC-9 to AC-11 (value-path spelling) and
  QSpec FR-351-AC-7 (deciding quantifier); QSpec FR-352-AC-2 refuses `/1`.
