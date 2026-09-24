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

When the native CLI finishes a request, it shall report the observed pipeline outcome with its source identity.

## Inputs

Command, source labels/file and compiler outcome.

## Outputs

JSON parse/diagnostic output or formatted source plus a documented exit code.

## Behavior

Current parse reports parsed only. [FR-301](ix://agent-ix/quire-specification/FR-301) states the native CLI's exit status contract; this command carries it. A successful parse completes without violation and exits 0. A refused syntax request, an invalid command invocation and invalid OS encoding in commands or labels are invalid or refused input and exit 20. A construct the parser recognizes but the admitted profile does not support, or a command invocation naming a lowering target outside the published catalog, names a real capability this build lacks and exits 21. An exhausted parser request is incomplete and exits 22. A failure to write the command's own output — the parsed/formatted result on stdout or a diagnostic on stderr — is a tool failure, not a request-level disposition, and exits 30, FR-301's code for tool failure. Source diagnostics preserve original byte and scalar coordinates. Future link/evaluate outcomes cannot be inferred from parse success.

`parse` and `format` take the source reference's four labels before the file
([FR-001](FR-001-read-exact-source.md)):

```text
quire-spec <parse|format> <source-authority> <source-id> <revision-namespace> <revision> <file>
```

The CLI reads OS arguments without assuming UTF-8. The command and the four
labels must be UTF-8; invalid label/command encoding is a usage error
before opening the path. An empty or whitespace-only UTF-8 label refuses as
`invalid_source_identity` (FR-001) before the file is opened. The file operand remains an OS path for actual I/O, including Unix
non-UTF-8 paths. JSON path text is display-only and may contain replacement
characters; it cannot serve as portable source authority. The exact source
labels and digest remain distinct. Collect at most seven arguments so extra
arguments refuse without unbounded argument allocation. A successful parse
reports the source as its `RawSourceRef`: the authority, the identity, the
revision's namespace and value, and the `quire.source.bytes/v1` digest.

Native Diagnostic implements standard Display and Error,
retaining its structured phase/code/source/path/span/message fields and existing
code spellings. The Copy Code enum exposes as_str, all and from_code; unknown
spellings return None. The repository owns a stable native-code catalog,
separate from the fixture-audit catalog. The source labels are the members of
the source's `RawSourceRef` (FR-001), the reference the checked package's lock
and source regions name the source by.

Retain the public `source: SourceIdentity` field as diagnostic provenance, not
an underlying error. Implement these two standard traits directly: the pinned
thiserror derive treats any field named source as an error cause and cannot
express this existing API. Display renders `{code}: {message}` and Error has
no underlying cause. This scoped compatibility exception does not introduce
a second error envelope or alter the audit target's derived errors.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-010-AC-1 | A successful parse emits status parsed. | Test |
| FR-010-AC-2 | A refused syntax request whose construct is invalid, not merely unsupported, exits 20, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for invalid or refused input. | Test |
| FR-010-AC-3 | An invalid command invocation exits 20, other than naming an unpublished lowering target (AC-9). | Test |
| FR-010-AC-4 | An exhausted parser request exits 22, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for incomplete. | Test |
| FR-010-AC-5 | A parse result carries the actual source digest. | Test |
| FR-010-AC-6 | Invalid OS encoding in commands/labels exits 20 without panic; valid source at a non-UTF-8 Unix file path parses with the exact labels and byte digest. | Test |
| FR-010-AC-7 | A missing or unreadable selected source file exits 20 and emits no parsed output. | Test |
| FR-010-AC-8 | A native Diagnostic propagates through a standard Error-based caller; every stable code round-trips through its catalog lookup. | Test |
| FR-010-AC-9 | A construct the parser recognizes but the admitted profile does not support, or a command invocation naming a lowering target outside the published catalog, exits 21, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for unsupported. | Test |
| FR-010-AC-10 | A failure writing the command's own output exits 30, [FR-301](ix://agent-ix/quire-specification/FR-301)'s code for tool failure. | Test |
| FR-010-AC-11 | `parse agent-ix specs/a.quire git 3f2a <file>` over an admissible file reports its source with authority `agent-ix`, identity `specs/a.quire`, revision namespace `git`, revision value `3f2a` and the file's `quire.source.bytes/v1` digest. `parse` with the four labels and no file, or with one extra operand, exits 20. `parse agent-ix specs/a.quire "" 3f2a <missing-file>` exits 20 with `invalid_source_identity`, not a file error. `format` takes the same four labels: `format agent-ix specs/a.quire git 3f2a <file>` over admissible complete-V1 source exits 0, and with one extra operand exits 20. | Test (TC-425) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/main.rs) supplies the scoped context.

## Status

Draft. FR-010-AC-11 is specified under QSL-233 and not implemented: the
grammar (`src/cli.rs`) takes `<source-id> <source-revision> <file>`. ADR-013
§7 slice S-4b builds it.

Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
