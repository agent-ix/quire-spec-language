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
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-contract-codegen/FR-004
    type: traces_to
---
# FR-267: Write and read run results as native-run-result/2

## Description

The clause-run result document of the spine check route and the `run`
command's command-error envelope SHALL be `native-run-result/2` (QSpec
FR-352), as ADR-031 SW-10 and SW-11 and ADR-013 OQ-1 decide. The library
serializes `qsl_replay::spine::run_clause`'s `ClauseRunReport` (FR-109) as
the `/2` clause-run document; this requirement owns that producer. A
clause-run document carries the members QSpec FR-352 defines, including the
settlement basis label and the separating witness record of FR-266.

For CG consumption, this document reports the QSL clause run or command error. It does not
authenticate the generated source, source map or coverage producer used by a
CG campaign. CG owns that authentication through a separate producer/artifact
binding receipt, as specified under **CG source-identity boundary** below.

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
- For reading: a typed clause-run result or a typed command-error result, or
  a located wire refusal. Both result variants belong to the same QSL strict
  reader; CG consumes its typed output rather than parsing JSON itself.

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
- For FR-115's evaluated `Frame` violation, the writer SHALL encode
  `disposition.refusal.frame_witness` as the complete FR-115 `FrameWitness`
  under the closed form below. The enclosing `refusal` retains
  `frame_violation`/`unauthorized-change`, while `disposition.category` is
  `violation`, `truth` is `false`, `basis` is `closed-scope`, and neither
  `disposition.outcome` nor the top-level FR-351 `witness` is present. The
  reader SHALL restore that complete typed frame witness, refuse a missing
  or extra member in it, and refuse a `frame_witness` on any other
  disposition. Its nested `code` and `cause` SHALL equal those of the
  enclosing `refusal`.
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
- When reading `native-run-result/2`, the QSL strict reader SHALL return
  the typed clause-run or command-error variant according to the document's
  closed member shape. The command-error variant uses exactly the envelope
  defined above, including FR-100's code-specific `cause` and `details`
  rules, `basis: unavailable` and no `witness`. A mixed, unknown or invalid
  shape refuses with a located wire diagnostic. Shape selection applies
  only after the exact `format` check; it does not infer the wire version.
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

### Frame witness wire

`frame_witness` is a closed object with exactly `code`, `cause`,
`invocation`, `pre`, `post`, `population` and `change`. `code` is
`frame_violation`; `cause` is `unauthorized-change`; `population` is the
changed population's declaration identity. Each of `invocation`, `pre`
and `post` is an FR-106 `DocumentRef`, encoded as a closed object with
the four FR-001 labels `authority`, `identity`, `revision_namespace` and
`revision`, and its `sha256-jcs` `digest`. A declaration key in `change`
is a closed `{package, node}` object. The reader reconstructs the FR-115
value from these members; no document bytes, path or caller-supplied
permission is inferred from the witness.

`change` is exactly one closed tagged object:

| `kind` | Required members beside `kind` | Frame permission |
| --- | --- | --- |
| `field_write` | `object` (key), `field` (declared name), `modifies` (ordered array of declaration keys) | FR-115's checked `modifies` |
| `created` | `object` (key), `type_name` (most-specific declaration key), `creates` (ordered array of declaration keys) | FR-115's checked `creates` |
| `deleted` | `object` (key), `type_name` (most-specific declaration key), `deletes` (ordered array of declaration keys) | FR-115's checked `deletes` |
| `retyped` | `object` (key), `pre_type` and `post_type` (most-specific declaration keys) | No frame permission authorizes retype |

The reader SHALL reject an unknown `kind`, an absent required member, an
extra member, `null` in place of a member, or a declaration key that is
not the declared `{package, node}` form.

### Wire member cross-check

This table checks the full clause-run and command-error member inventory
against its owning contract. A conditional member is omitted when its
condition is false; the reader refuses `null` or a default in its place.

| Class | Member | Owner FR | Encoding | Decoding |
| --- | --- | --- | --- | --- |
| Clause run | `format` | QSpec FR-352 | Exact `native-run-result/2` | Check before variant selection |
| Clause run | `package_id` | FR-109 | Compiled package's hex identity when compile completed | Restore identity or absence |
| Clause run | `disposition` | FR-109 | Closed report disposition below | Restore typed disposition |
| Clause run | `usage` | FR-109, QSpec FR-352 | Closed accounting object below | Restore all counters |
| Clause run | `basis` | QSpec FR-352, FR-266 | FR-243 label on every report | Validate label and verdict relation |
| Clause run | `witness` | QSpec FR-352, FR-266 | FR-351 record only for decisive basis | Restore record or absence |
| Disposition | `stage` | FR-109 | Compile, select, admit or evaluate label | Restore stage |
| Disposition | `category` | FR-109 | Report category label | Restore category |
| Disposition | `truth` | FR-109 | Boolean when the report assigns it | Restore Boolean or absence |
| Disposition | `refusal` | FR-109 | FR-096 record or FR-115 frame finding below, when assigned | Restore complete record |
| Disposition | `outcome` | FR-100, FR-109 | FR-100 outcome when evaluation assigns one | Restore complete outcome |
| Disposition | `internal_fault` | FR-100, FR-109 | Fault object only for internal failure | Restore stable fault identifiers |
| Refusal | `code` | FR-096, FR-115 | Catalog code | Validate catalog code |
| Refusal | `cause` | FR-096, FR-115 | Catalog cause when assigned | Restore cause or absence |
| Refusal | `fields` | FR-096, FR-115 | Catalog field-name to rendered-string map when assigned | Restore all fields or absence |
| Refusal | `locus` | FR-096, FR-109 | FR-096 located source region when assigned | Restore locus or absence |
| Refusal | `frame_witness` | FR-115, FR-267 | Complete frame witness only for evaluated Frame violation | Restore typed witness; reject other placements |
| Outcome | `kind` | FR-100 | Exact completed, refused, undefined or incomplete tag | Select FR-100 variant |
| Outcome | `value` | FR-100 | Completed value's tagged form | Restore exact value |
| Outcome | `code` | FR-100 | Refused catalog code | Restore code |
| Outcome | `cause` | FR-100 | Refused catalog cause | Restore cause |
| Outcome | `fields` | FR-100 | Refused catalog fields when assigned | Restore all fields or absence |
| Outcome | `locus` | FR-100 | Refused source region when assigned | Restore locus or absence |
| Outcome | `location` | FR-100 | Refused semantic location when assigned | Restore location or absence |
| Outcome | `reason` | FR-100 | Undefined reason when assigned | Restore reason |
| Outcome | `limit` | FR-100 | Incomplete accounting counter when assigned | Restore counter |
| Completed value | `kind` | FR-100 | Boolean or integer tag | Select value kind |
| Completed value | `value` | FR-100 | Boolean for Boolean kind | Restore Boolean |
| Completed value | `decimal` | FR-100 | Exact decimal string for integer kind | Restore integer |
| Outcome locus | `file` | FR-100 | Authored source path | Restore source |
| Outcome locus | `span` | FR-100 | Start and end byte, line and column | Restore exact span |
| Outcome location | `origin` | FR-100 | Semantic origin variant | Restore origin |
| Outcome location | `path` | FR-100 | Child-index array | Restore path |
| Internal fault | `stage` | FR-100 | Stable fault stage | Restore stage |
| Internal fault | `invariant` | FR-100 | Stable invariant identifier | Restore invariant |
| Usage | `admission` | FR-109, QSpec FR-352 | Closed admission counters | Restore counters |
| Usage | `evaluation_admissions` | FR-109, QSpec FR-352 | Unsigned count | Restore count |
| Usage | `evaluation_consumed` | FR-109, QSpec FR-352 | Ordered counter array | Restore array |
| Admission | `document_bytes` | QSpec FR-352 | Unsigned count | Restore count |
| Admission | `objects` | QSpec FR-352 | Unsigned count | Restore count |
| Admission | `values` | QSpec FR-352 | Unsigned count | Restore count |
| Evaluation counter | `kind` | QSpec FR-352 | Counter name | Restore name |
| Evaluation counter | `consumed` | QSpec FR-352 | Unsigned count | Restore count |
| Frame witness | `code` | FR-115 | `frame_violation` | Match enclosing refusal |
| Frame witness | `cause` | FR-115 | `unauthorized-change` | Match enclosing refusal |
| Frame witness | `invocation` | FR-115 | FR-106 document reference | Restore invocation selection |
| Frame witness | `pre` | FR-115 | FR-106 document reference | Restore pre selection |
| Frame witness | `post` | FR-115 | FR-106 document reference | Restore post selection |
| Frame witness | `population` | FR-115 | Declaration identity | Restore population |
| Frame witness | `change` | FR-115 | One tagged change from table above | Restore exact change and permission |
| Document reference | `authority` | FR-106 | FR-001 label | Restore label |
| Document reference | `identity` | FR-106 | FR-001 label | Restore label |
| Document reference | `revision_namespace` | FR-106 | FR-001 label | Restore label |
| Document reference | `revision` | FR-106 | FR-001 label | Restore label |
| Document reference | `digest` | FR-106 | `sha256-jcs` digest | Restore digest |
| Change | `kind` | FR-115 | One of four tags above | Select typed variant |
| Change | `object` | FR-115 | Object key | Restore key |
| Change | `field` | FR-115 | Declared field name on field write | Restore field |
| Change | `type_name` | FR-115 | Declaration key on create or delete | Restore type |
| Change | `pre_type` | FR-115 | Declaration key on retype | Restore pre type |
| Change | `post_type` | FR-115 | Declaration key on retype | Restore post type |
| Change | `modifies` | FR-115 | Ordered declaration-key array on field write | Restore checked grant |
| Change | `creates` | FR-115 | Ordered declaration-key array on creation | Restore checked grant |
| Change | `deletes` | FR-115 | Ordered declaration-key array on deletion | Restore checked grant |
| Declaration key | `package` | FR-056, FR-115 | Package identity | Restore key package |
| Declaration key | `node` | FR-056, FR-115 | IR node identity | Restore key node |
| Separating witness | `quantifier` | QSpec FR-352 | FR-351 occurrence key when assigned | Restore component |
| Separating witness | `deciding_element` | QSpec FR-352 | Canonical typed value | Restore component |
| Separating witness | `index` | QSpec FR-352 | Unsigned position when assigned | Restore component |
| Separating witness | `value_path` | QSpec FR-352 | FR-207 runtime value path | Restore component |
| Separating witness | `trace_position` | QSpec FR-352 | Unsigned position when assigned | Restore component |
| Command error | `format` | FR-267 | Exact `native-run-result/2` | Check before variant selection |
| Command error | `stage` | FR-100, FR-267 | Command stage | Restore stage |
| Command error | `code` | FR-100, FR-267 | Catalog code | Restore code |
| Command error | `cause` | FR-100, FR-267 | Catalog cause when assigned | Restore cause or absence |
| Command error | `message` | FR-100, FR-267 | Human-readable diagnostic | Restore message |
| Command error | `details` | FR-100, FR-267 | Code-specific closed object | Restore details |
| Command error | `basis` | FR-267 | `unavailable` | Require unavailable |

### CG source-identity boundary

- The QSL producer SHALL emit only the members defined by
  [QSpec FR-352](ix://agent-ix/quire-specification/FR-352) and this requirement;
  generated-source identity, source-map identity and coverage producer or tool
  version are not additional `/2` members.
- A consumer SHALL NOT treat successful `/2` decoding, its disposition, basis,
  witness or [FR-109](FR-109-run-a-state-clause-through-the-spine.md)'s
  `package_id` as authentication of CG generated source, source maps or
  coverage-producer execution.
- When CG consumes `/2` for a campaign, CG SHALL pair the result with a
  separate CG-owned producer/artifact binding receipt that authenticates the
  association between that exact result, the producer execution, and the
  exact generated source and source map used by the campaign. CG owns the
  receipt's representation, trusted producer authority and verification under
  [CG FR-004](ix://agent-ix/quire-contract-codegen/FR-004) and
  [CG interface-001](ix://agent-ix/quire-contract-codegen/interface-001).
  Caller declarations, matching paths and self-declared producer metadata
  alone do not authenticate this association.
- If the receipt is absent or unauthenticated, any required association is
  absent, or its result, producer execution, source or map does not match the
  supplied campaign inputs, then CG SHALL refuse the binding with a
  structured non-success analysis outcome identifying the missing,
  unauthenticated or mismatched association and retaining available input
  identities. Such an outcome cannot qualify the campaign, emit a measured
  coverage classification from those inputs or discharge its coverage
  obligation.
- When the receipt's associations authenticate and match, CG SHALL retain the
  decoded QSL disposition, basis and witness without upgrading the outcome;
  the receipt establishes the association only. Coverage and campaign
  success remain subject to CG's own obligations.
- When the strict reader returns a command-error variant, CG SHALL retain
  its stage, code, optional cause, message, details and unavailable basis as
  a non-success execution outcome. Even a matching authenticated receipt
  cannot make a command error qualify a campaign, produce a measured
  coverage classification or discharge its coverage obligation.
- When CG refuses the binding of a valid `/2` document, CG SHALL preserve
  that document's decoded outcome separately from the binding diagnostic.
  Missing external identity is not a malformed `/2` document or a QSL
  execution refusal. The strict wire reader continues to reject an invalid
  document or unsupported format before consumer binding; it adds no
  compatibility reader or inferred wire members.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-267-AC-1 | FR-266-AC-1's `AllBelow` over `high` report writes a document with `format` `native-run-result/2`, `basis` `decisive-counterexample` and a `witness` holding the `forall`'s occurrence key, the reference to `mid`, `index` 1, the population-root value path and no `trace_position` member; the bytes equal their own RFC 8785 re-encoding, and reading them back gives an equal basis and record. The `AllBelow` over `low` report writes `closed-scope` with no `witness` member, and the exhausted-work report writes `unavailable` with no `witness` member. | Test |
| FR-267-AC-2 | Each of these edits of FR-267-AC-1's `high` document refuses, naming the member: `index` removed; an extra member in `witness`; `trace_position` given as `0`; `basis` removed; `basis` `closed-scope` with the `witness` kept; `basis` `decisive-counterexample` with `witness` removed; `format` set to `native-run-result/1`; `format` set to `native-run-result/3`. | Test |
| FR-267-AC-3 | From the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), `run` over a native-run/1 request whose source declares `edition "9-draft"` writes nothing to stdout and writes a command-error envelope to stderr whose `format` is `native-run-result/2`, with exactly the members `format`, `stage` (`profile`), `code` (`unknown_edition`), `message`, `details` and `basis` (`unavailable`), no `witness` member, exiting 20. | Test |
| FR-267-AC-4 | A valid `/2` document with no CG receipt is accepted by the wire reader but CG refuses its campaign binding, retaining the decoded outcome and an absent-receipt diagnostic; no campaign qualification, measured coverage classification or coverage discharge results. | Test |
| FR-267-AC-5 | With an authenticated CG receipt associating the supplied result, producer execution, generated source and source map, CG accepts the binding and retains the result's original disposition, basis and witness. A bound violation, refusal, unsupported or incomplete result remains that outcome and cannot become success through receipt validation. | Test |
| FR-267-AC-6 | Starting with AC-5's matching inputs, independently replacing the result, producer execution, generated source or source map causes CG to refuse the binding and identify the mismatched association, even when the replacement has the same path or QSL `package_id`. The original decoded outcome and available input identities remain available, with no campaign qualification, measured coverage classification or coverage discharge. | Test |
| FR-267-AC-7 | Independently omitting each required receipt association, or supplying only caller-declared identities and producer metadata without authentication, causes the same non-success binding outcome as AC-4 with the missing or unauthenticated association identified. | Test |
| FR-267-AC-8 | The QSL producer emits no added generated-source, source-map or coverage-producer members; the strict reader rejects an injected member outside the declared `/2` shape, and an otherwise matching external receipt cannot make `/1`, an unsupported identifier or an invalid `/2` document readable. | Test |
| FR-267-AC-9 | The same QSL strict reader returns the clause-run variant for AC-1's document and the command-error variant for AC-3's envelope. CG consumes both typed variants without its own JSON parser and retains AC-3's envelope as non-success with no campaign qualification, measured coverage classification or coverage discharge, including when the binding receipt authenticates and matches. | Test |
| FR-267-AC-10 | Independently removing a required command-error member, adding a clause-run-only member, replacing the unavailable basis with a decisive basis, adding a witness, or violating FR-100's code-specific cause/details rules makes the strict reader refuse with a located diagnostic. Changing an otherwise valid command-error envelope to `/1` or another unsupported format refuses before variant selection. | Test |
| FR-267-AC-11 | FR-115's forbidden-parent-change Frame report writes `disposition.refusal.frame_witness` with its invocation, pre, post, population and `field_write` change naming `child`, `parent` and the checked `modifies` grant; reading the `/2` document restores an equal FR-115 `FrameWitness`. A creation, deletion and retype each round-trip their tagged change. The document has `violation`, `truth: false`, `basis: closed-scope`, no `disposition.outcome` and no top-level `witness`. Independently dropping a frame-witness member or change member, adding an unknown member, mismatching its code or cause with the enclosing refusal, or adding `frame_witness` to a non-Frame disposition refuses with a located wire diagnostic. | Test |

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
- CG source-identity ownership: Linear QSL-688, choosing a CG-owned separate
  producer/artifact binding receipt; CG consumer specification: Linear IR-514.
  The receipt and CG consumer criteria above are specified obligations, not
  claims that CG receipt verification is implemented.
- QSpec half (Linear STD-144): QSpec FR-352 AC-6 and AC-7 (basis label and
  witness members), FR-207 AC-9 to AC-11 (value-path spelling) and
  QSpec FR-351-AC-7 (deciding quantifier); QSpec FR-352-AC-2 refuses `/1`.
