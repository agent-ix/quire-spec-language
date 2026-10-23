---
id: FR-010
title: "Report phase-specific CLI outcomes"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-010: Report phase-specific CLI outcomes

## Description

When QSL's `parse` or `format` operation finishes, QSL shall report the observed pipeline outcome with its source identity, as a typed outcome whose FR-285 exit code is the code a frontend exits with.

A CLI frontend reaches these operations through the driver's `parse` and
`format` verbs (FR-287). The frontend's arguments, their OS encoding, its
refusal of a bad invocation and its writes to the process streams are the
driver's, specified by FR-287-AC-3 to FR-287-AC-9 (FR-287, Overlap).

## Inputs

Source labels, a source path or bytes, and the operation's outcome.

## Outputs

The `parse` outcome document or the formatted source, with FR-285's exit code of the outcome's category.

## Behavior

The `parse` operation's outcome serializes as `OutcomeDocument::from_parse`, one `quire-outcome/1` document, and its category's FR-285 exit code is the code a frontend exits with ([FR-002](FR-002-parse-native-units.md)). [FR-301](ix://agent-ix/quire-specification/FR-301) states the exit status contract. A successful parse completes without violation, exit code 0. A refused syntax request is invalid or refused input, exit code 20. A construct the parser recognizes but the admitted profile does not support names a real capability this build lacks, exit code 21. An exhausted parser request is incomplete, exit code 22. Source diagnostics preserve original byte and scalar coordinates. Future link/evaluate outcomes cannot be inferred from parse success.

`parse` and `format` take the source reference's two labels with the source
([FR-001](FR-001-read-exact-source.md)). An empty or whitespace-only label
refuses as `invalid_source_identity`/`blank-label`, naming the first blank
label (FR-001), before the source path is opened. The source path is an OS
path for actual I/O, including Unix non-UTF-8 paths. JSON path text is
display-only and may contain replacement characters; it cannot serve as
portable source authority. The exact source labels and digest remain
distinct. The `ParsedSource`
a successful parse returns names the source by its `RawSourceRef`: the
authority, the identity and the `quire.source.bytes/v1` digest (QSpec
STD-150).

Native Diagnostic implements standard Display and Error,
retaining its structured phase/code/source/path/span/message fields and existing
code spellings. The Copy Code enum exposes as_str, all and from_code; unknown
spellings return None. The repository owns a stable native-code catalog.
The source labels are the members of
the source's `RawSourceRef` (FR-001), the reference the checked package's lock
and source regions name the source by.

Retain the public `source: SourceIdentity` field as diagnostic provenance, not
an underlying error. Implement these two standard traits directly: the pinned
thiserror derive treats any field named source as an error cause and cannot
express this existing API. Display renders `{code}: {message}` and Error has
no underlying cause. This scoped compatibility exception does not introduce
a second error envelope or alter the audit target's derived errors.

The refusal line's keys are in one fixed order, the field order of the refusal record: `status`, `phase`, `code`, `source`, `path`, `span`, `message`, then `cause` and `label` when present; `source`'s own keys read `authority`, `identity`, `revision_namespace`, `revision`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-010-AC-1 | `OutcomeDocument::from_parse` over a successful `parse` operation is one `quire-outcome/1` document with operation `parse` and category `success`, whose FR-285 exit code is 0. | Test (TC-015) |
| FR-010-AC-2 | A refused syntax request whose construct is invalid, not merely unsupported, returns an outcome whose FR-285 exit code is 20, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for invalid or refused input. | Test (TC-015) |
| FR-010-AC-4 | An exhausted parser request returns an outcome whose FR-285 exit code is 22, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for incomplete. | Test (TC-015) |
| FR-010-AC-5 | The `ParsedSource` of a successful parse carries the `quire.source.bytes/v1` digest of the bytes actually read. | Test (TC-015) |
| FR-010-AC-6 | Valid source at a non-UTF-8 Unix file path, read through FR-001's source intake, parses, and the `ParsedSource` the `parse` operation returns for it carries the exact labels and byte digest. | Test (TC-017) |
| FR-010-AC-7 | A missing or unreadable source path refuses at FR-001's source intake, exit code 20, before any parse, and no `parse` outcome document is produced. | Test (TC-015, TC-017) |
| FR-010-AC-8 | A native Diagnostic propagates through a standard Error-based caller; every stable code round-trips through its catalog lookup. | Test (TC-018) |
| FR-010-AC-9 | A construct the parser recognizes but the admitted profile does not support returns an outcome whose FR-285 exit code is 21, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for unsupported. | Test (TC-015) |
| FR-010-AC-11 | The `parse` operation, called in process over an admissible file with source authority `agent-ix` and identity `specs/a.quire`, returns a `ParsedSource` that names its source with authority `agent-ix`, identity `specs/a.quire` and the file's `quire.source.bytes/v1` digest, and no other source member. Source intake with authority `agent-ix`, an empty identity and a missing path refuses `invalid_source_identity`, cause `blank-label`, `label` `identity`, exit code 20, not a file error. `format` takes the same two labels: `format` with authority `agent-ix` and identity `specs/a.quire` over an admissible complete-V1 file returns the formatted source, exit code 0. No call writes a byte to stdout or stderr. | Test (TC-425) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/main.rs) supplies the scoped context.
