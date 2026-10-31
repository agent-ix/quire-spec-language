---
id: FR-001
title: "Read bounded immutable native source"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/US-005"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/ADR-013"
    type: depends_on
---
# FR-001: Read bounded immutable native source

## Description

When source is admitted, the compiler shall retain its exact bytes and the
source reference that names them.

`SourceIdentity` and the
label check still carry the revision namespace and revision labels (four
labels in all); they drop to the authority and identity as AC-6, AC-7,
AC-8, AC-10 and AC-11 state. The `RequestRevision` and
`EditPredecessor` checks are removed (AC-12): an edit or editor request is
made against the one immutable `Source` instance it names, so no predecessor
or revision value is carried or compared.

## Inputs

The source's authority and identity, a path, UTF-8 bytes and caller
ceilings.

## Outputs

Immutable source with its source reference and indexed
locations, or a diagnostic.

## Behavior

The input ceiling is the caller's, used as given; it defaults to 1 MiB and may be set above or below that. Invalid UTF-8, BOM and NUL refuse. Byte spans remain original and half open; line/scalar-column coordinates are derived without changing source text.

### The caller names the source it hands QSL

This section governs every source admission, on the spine and in the
native-v1 lane alike. Both admit source through the one F type
`qsl_foundation::SourceIdentity` and the one F `Source`, which ADR-011 names
as S0's input; the native-v1 parser reads through them and through the
spine's lexer (ADR-013 §6 Spans row).

A source QSL reads itself is named by the caller, never by QSL. The caller
supplies two labels with the bytes: the **authority** that issues the
source's identity, and the **identity** within that authority. The source's
identity is that document identity (authority and identity). QSL does not default either label and does not derive one
from the path, the bytes or the other label: a path is a locator, not an
identity (QSpec FR-004).

A label is blank when it is empty or consists only of Unicode `White_Space`
scalars. Admission checks the labels first, in the order `authority`,
`identity`:

- When a label is blank, the source shall refuse with
  `invalid_source_identity`, cause `blank-label`, and field `label` naming
  the first blank label in that order, spelled exactly `authority` or
  `identity`.
- When every label is non-blank and the path is empty, the source shall
  refuse with `invalid_source_identity`, cause `empty-path`, and no field.

These are the two causes `quire.native.diagnostics/v1`
(QSpec STD-110) closes `invalid_source_identity` to. Neither names a source
region.

### Edit and binding refusals are not source-identity refusals

An incremental edit that changes the authority or identity of a source
already admitted (`EditPredecessor`) SHALL refuse with
`invalid_source_map`, the retained host code for an invalid correspondence,
source binding or queried range, and keep its host cause. It SHALL name no
region: byte 0 cannot stand in for a location (the catalog's common
structured context). It never uses `invalid_source_identity`, whose causes
are the two above.
(QSL team-lead ruling, 2026-09-26, on the review of PR #487.)

### Where an S0 refusal is located

An S0 refusal names its position by a `SourceRegion` when one exists, and by
no region otherwise (`quire.native.diagnostics/v1` common structured
context: "Unavailable context is explicitly unavailable; byte zero, an empty
path or a name-search match cannot masquerade as a located failure"):

- Invalid UTF-8, a BOM and NUL refuse at a region under a `RawSourceRef`
  minted for the refusal: the two labels. Only these refusals
  mint a reference for bytes that were not admitted. Invalid UTF-8 is the
  empty region at the end of the longest valid prefix; a BOM is the region
  of its three bytes at 0; NUL is the one-byte region of the first NUL.
- A blank label or an empty path and input beyond the byte ceiling refuse
  with no region. The first has
  no complete, non-blank label set to name the source by, and an empty path
  names no location. The second is
  refused without reading the offered bytes.

This changes current behaviour, which places the unnamed-source and byte-budget
refusals at byte 0. The native-v1 `Diagnostic` hosted in
F requires a span. For a refusal with no region it keeps rendering byte 0.
That is retained lane-private debt (ADR-013 §6), recorded here so no
implementer mistakes it for this requirement's behaviour; this requirement
neither fixes nor extends it.

### Line and column are derived when rendered

A `SourceRegion` holds bytes only. A renderer derives the one-based line and
Unicode scalar column of each end from the admitted source bytes the
region's `RawSourceRef` names, as FR-010's "original byte and scalar
coordinates" require. No region stores them. A refusal region over offered
bytes that were not admitted (invalid UTF-8, BOM, NUL) renders over the
longest valid UTF-8 prefix of those bytes, which contains the region.

### Admission mints the source reference

On admission the source shall carry its `RawSourceRef` (ADR-013 O-07,
QSpec FR-322): the two labels exactly as supplied.
The source reference
is the one value every later stage names this source by:

- A region in this source is a `SourceRegion` under this `RawSourceRef`
  (ADR-013 O-12).
- The checked package's `sources` list this `RawSourceRef` (QSpec FR-322).
- A declaration of this source is owned by
  `SourceOwner{authority, identity}`, taken from this `RawSourceRef`
  ([FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-092](FR-092-key-type-parameter-and-declared-nodes.md)).

A declaration's node key does not depend on the source content: two contents of one
source keep the keys of their unchanged declarations
(QSpec `proposals/checked-package-v2/README.md`). A different authority or
identity gives every source-owned declaration a different key.

### Who supplies the labels

- The command line supplies them as operands of `parse` and `format`
  ([FR-010](FR-010-report-native-outcomes.md)). `parse` reads native-v1
  source and `format` complete-V1 source, under the same two-label grammar.
- A native-v1 request supplies them in each source's wire identity: the
  run and compile requests
  ([FR-026](FR-026-run-standalone-native-workflow.md),
  [FR-027](FR-027-export-compiled-native-package.md)), which share one
  identity definition.
- A library caller supplies them in the source identity it passes to the
  source reader and to S1's `parse`.
- A supplied library's source carries its own two labels in the
  dependency input (FR-099, ADR-015 D-1): the library caller sets them, the
  native-compile/1 `libraries` member carries them in each library's source
  selection, and the replay executor takes them from the dependency entry's
  source reference, exactly as it does for the proved source below. Those
  labels give the library's declarations their `SourceOwner`; the library
  package is identified by its package name, which the labels do not enter.
- The replay executor recompiles each source under the `RawSourceRef` the
  replay request's package reference names (QSpec FR-323 `package`, ADR-013
  O-26). It passes that reference's two labels as the source identity, the
  reference's identity as the path, and the provided bytes as the source. A
  reference whose label is only whitespace, which QSpec's `Nonempty` admits,
  refuses the replay with `invalid_source_identity`/`blank-label`, naming
  that label, at S1, before any form is built. The recompiled package therefore names each source
  exactly as the proving run did, which keeps its declaration keys and its
  `SourceOwner`s; the package keeps its package name (US-005). The executor is ADR-013 TK-01, `qsl_replay::replay`
  ([FR-098](FR-098-execute-a-replay-request.md)), and this is the
  replay module's recompilation path, the S8 row of ADR-011 §2. TC-444
  checks the whitespace-only label.

### Runtime artifacts are not sources

A native runtime input artifact (a snapshot or an invocation,
`native-state-input/1`) is not a source read under this requirement. Its
caller names it with the same two labels
(QSpec FR-004)
([FR-018](FR-018-construct-native-runtime-inputs.md),
[FR-024](FR-024-read-native-runtime-artifacts.md)).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-001-AC-1 | **RETIRED (source content check removed).** Admission carries only the authority and identity of the source. | Test (TC-011), retired |
| FR-001-AC-2 | **RETIRED (verified intake removed).** There is no verified intake and no content-mismatch refusal. | Test (TC-011), retired |
| FR-001-AC-3 | Invalid UTF-8 receives a source diagnostic. | Test (TC-011) |
| FR-001-AC-4 | Input beyond the selected byte ceiling receives resource_exhausted naming that ceiling, whether the ceiling is below or above the 1 MiB default. | Test (TC-011) |
| FR-001-AC-5 | Source admitted with authority `agent-ix`, identity `specs/a.quire` and bytes `b` carries a `RawSourceRef` whose authority and identity read exactly those labels, and no other member. Admitting bytes `b'` under the same labels gives an equal `RawSourceRef`. | Test (TC-424) |
| FR-001-AC-6 | Admission refuses with `invalid_source_identity`, cause `blank-label`, and admits nothing, when exactly one of the authority or identity is empty, again when it is a single space, and again when it is U+3000 IDEOGRAPHIC SPACE; field `label` is `authority` or `identity` respectively. A label that is U+200B ZERO WIDTH SPACE, which is not `White_Space`, is not blank. | Test (TC-424) |
| FR-001-AC-11 | With both the identity and the authority blank, admission refuses `invalid_source_identity`/`blank-label` with `label` `authority`. With the identity blank and the path empty, it refuses `blank-label` with `label` `identity`, not `empty-path`. With both labels non-blank and the path empty, it refuses `invalid_source_identity`/`empty-path` with no field. | Test (TC-424) |
| FR-001-AC-12 | An incremental edit whose expected source identity differs, and a complete-editor request bound to another source revision, each refuse with `invalid_source_map`, keeping host causes `EditPredecessor` and `RequestRevision` respectively, and naming no region; neither refuses with `invalid_source_identity`. The edit and the request name the one `Source` instance they act on. | Test (TC-424) |
| FR-001-AC-7 | Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: ["Point"], ordinal: 0}` each time: the source reference is no part of a node key. | Test (TC-424) |
| FR-001-AC-8 | Bytes `a\xffb` admitted as (`a`, `u`) refuse with the region `[1, 1)` under the `RawSourceRef` (`a`, `u`), and bytes `ab\0c` with the region `[2, 3)` under it. | Test (TC-424) |
| FR-001-AC-10 | Admission with an empty identity, and admission of five bytes under a four-byte ceiling each refuse with no region, not a region at byte 0. | Test (TC-424) |
| FR-001-AC-9 | A renderer given the region `[4, 7)` of admitted source `ab\ncdéf` reports start line 2, column 2 and end line 2, column 4, derived from the admitted bytes; the region itself holds only its `RawSourceRef`, 4 and 7. | Test (TC-424) |

The cause and `label` reach the native `Diagnostic`,
the `parse`/`format` refusal line, the native-run output, the
native-state-input construction error and the replay recompile refusal, each
tested for `blank-label` with its label. `empty-path` is tested at the reader
and the native `Diagnostic`; the other outputs cannot reach it: `parse` and
`format` and the run request open the file before any source is admitted (an
empty file operand is a file error), a native-state-input artifact has no
path, and the replay path is the reference's identity, which a non-blank
label set makes non-empty. `EditPredecessor`
refuses with `invalid_source_map` and no region
(`qsl-cst/src/diagnostic.rs` `HostCause::code`, `error_without_region`).

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md):
  the same identity and source location on both sides of the checked-package
  boundary, which the replay executor's recompilation needs.
- ADR-013 O-07 and O-12: `RawSourceRef` and `SourceRegion`. §7 slice S-4b
  builds this requirement's source reference.
- QSpec FR-004: a path or display label is a locator. QSpec FR-322
  `RawSourceRef`, `SourceOwner` and `PackageLock.sources`.
- QSpec STD-150 defines `RawSourceRef` with an extra content member; this requirement's
  `RawSourceRef` is the authority and the identity only (QSpec must follow before the wire matches).
- `quire.native.diagnostics/v1` (QSpec STD-110,
  `proposals/quire-v1/definitions/native-diagnostics.md`, cited by
  reference): the `invalid_source_identity` row, its causes `blank-label`
  and `empty-path`, and the `label` payload spelling; QSpec FR-272-AC-12.
- [Detailed contract or implementation evidence](../../qsl-foundation/src/source.rs) supplies the scoped context.
