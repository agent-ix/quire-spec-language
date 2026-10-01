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

## Inputs

The source's authority, identity and revision (a namespace and a value), a
path, UTF-8 bytes and caller ceilings.

## Outputs

Immutable source with its digest, its source reference and indexed
locations, or a diagnostic.

## Behavior

The input ceiling is the caller's, used as given; it defaults to 1 MiB and may be set above or below that. Invalid UTF-8, BOM and NUL refuse. Verified intake checks the selected raw SHA-256 digest. Byte spans remain original and half open; line/scalar-column coordinates are derived without changing source text.

### The caller names the source it hands QSL

This section governs every source admission, on the spine and in the
native-v1 lane alike. Both admit source through the one F type
`qsl_foundation::SourceIdentity` and the one F `Source`, which ADR-011 names
as S0's input; the native-v1 parser reads through them and through the
spine's lexer (ADR-013 §6 Spans row).

A source QSL reads itself is named by the caller, never by QSL. The caller
supplies four labels with the bytes: the **authority** that issues the
source's identity, the **identity** within that authority, and the
**revision** as a namespace and a value. The namespace names the revision
system the value belongs to, for example `git` for a commit or `semver` for
a release. QSL does not default any label and does not derive one from the
path, the bytes or another label: a path is a locator, not an identity
(QSpec FR-004), and a revision namespace is never inferred from the shape of
its value.

A label is blank when it is empty or consists only of Unicode `White_Space`
scalars. Admission checks the labels first, in the order `authority`,
`identity`, `revision_namespace`, `revision`:

- When a label is blank, the source shall refuse with
  `invalid_source_identity`, cause `blank-label`, and field `label` naming
  the first blank label in that order, spelled exactly `authority`,
  `identity`, `revision_namespace` or `revision`.
- When every label is non-blank and the path is empty, the source shall
  refuse with `invalid_source_identity`, cause `empty-path`, and no field.

These are the two causes `quire.native.diagnostics/v1` revision `1-draft.8`
(QSpec STD-110) closes `invalid_source_identity` to. Neither names a source
region.

### Edit and binding refusals are not source-identity refusals

Three complete-V1 host refusals concern a source already admitted, not the
labels it was admitted under: an incremental edit whose revision is not the
exact predecessor's or that changes the authority, identity or revision
namespace (`EditPredecessor`), a CST node rendered against a parsed source it
does not belong to (`ForeignNode`), and an editor request bound to another
document revision (`RequestRevision`). They SHALL refuse with
`invalid_source_map`, the retained host code for an invalid correspondence,
source binding or queried range, whose causes revision `1-draft.8` does not
close, keeping their host cause. They SHALL name no region: byte 0 cannot
stand in for a location (the catalog's common structured context). They
never use `invalid_source_identity`, whose causes are the two above.
(QSL team-lead ruling, 2026-09-26, on the review of PR #487.)

### Where an S0 refusal is located

An S0 refusal names its position by a `SourceRegion` when one exists, and by
no region otherwise (`quire.native.diagnostics/v1` common structured
context: "Unavailable context is explicitly unavailable; byte zero, an empty
path or a name-search match cannot masquerade as a located failure"):

- Invalid UTF-8, a BOM and NUL refuse at a region under a `RawSourceRef`
  minted for the refusal over the offered bytes: the four labels and the
  `quire.source.bytes/v1` digest of the bytes offered. Only these refusals
  mint a reference over bytes that were not admitted. Invalid UTF-8 is the
  empty region at the end of the longest valid prefix; a BOM is the region
  of its three bytes at 0; NUL is the one-byte region of the first NUL.
- A blank label or an empty path, input beyond the byte ceiling, and a
  digest mismatch under verified intake refuse with no region. The first has
  no complete, non-blank label set to name the source by, and an empty path
  names no location. The second is
  refused without hashing the offered bytes. The third concerns the bytes
  as a whole, not a position in them.

This changes current behaviour, which places the unnamed-source, byte-budget
and digest-mismatch refusals at byte 0. The native-v1 `Diagnostic` hosted in
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
QSpec FR-322): the four labels exactly as supplied and the
`quire.source.bytes/v1` digest of the admitted bytes. The source reference
is the one value every later stage names this source by:

- A region in this source is a `SourceRegion` under this `RawSourceRef`
  (ADR-013 O-12).
- The checked package's lock lists this `RawSourceRef` among its `sources`
  (QSpec FR-322 `PackageLock`).
- A declaration of this source is owned by
  `SourceOwner{authority, identity}`, taken from this `RawSourceRef`
  ([FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-092](FR-092-key-type-parameter-and-declared-nodes.md)).

The revision and digest do not enter a declaration's node key: two
revisions of one source keep the keys of their unchanged declarations
(QSpec `proposals/checked-package-v2/README.md`). A different authority or
identity gives every source-owned declaration a different key.

### Who supplies the labels

- The command line supplies them as operands of `parse` and `format`
  ([FR-010](FR-010-report-native-outcomes.md)). `parse` reads native-v1
  source and `format` complete-V1 source, under the same four-label grammar.
- A native-v1 request supplies them in each source's wire identity: the
  run, compile and lower requests
  ([FR-026](FR-026-run-standalone-native-workflow.md),
  [FR-027](FR-027-export-compiled-native-package.md)) and the extraction
  body record ([FR-031](FR-031-run-extracted-native-source.md)), which share
  one identity definition.
- A library caller supplies them in the source identity it passes to the
  source reader and to S1's `parse`.
- A supplied library's source carries its own four labels in the
  dependency input (FR-099, ADR-015 D-1): the library caller sets them, the
  native-compile/1 `libraries` member carries them in each library's source
  selection, and the replay executor takes them from the dependency entry's
  source reference, exactly as it does for the proved source below. Those
  labels give the library's declarations their `SourceOwner`, and so its
  `package_id`.
- The replay executor recompiles each source under the `RawSourceRef` the
  replay request's package reference names (QSpec FR-323 `package`, ADR-013
  O-26). It passes that reference's four labels as the source identity, the
  reference's identity as the path, and the provided bytes as the source. A
  reference whose label is only whitespace, which QSpec's `Nonempty` admits,
  refuses the replay with `invalid_source_identity`/`blank-label`, naming
  that label, at S1, before any form is built. It requires the recomputed
  `quire.source.bytes/v1` digest to equal the reference's digest, as
  ADR-013 C-13 requires. The recompiled package therefore names each source
  exactly as the proving run did, which keeps its declaration keys and its
  `package_id` (US-005). The executor is ADR-013 TK-01, `qsl_replay::replay`
  ([FR-098](FR-098-execute-a-replay-request.md)), and this is the
  replay module's recompilation path, the S8 row of ADR-011 §2. TC-444
  checks the whitespace-only label.

### Runtime artifacts are not sources

A native runtime input artifact (a snapshot or an invocation,
`native-state-input/1`) is not a source read under this requirement. Its
caller names it with the same four labels, because QSpec FR-004 makes the
revision namespace and value part of every immutable key
([FR-018](FR-018-construct-native-runtime-inputs.md),
[FR-024](FR-024-read-native-runtime-artifacts.md)).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-001-AC-1 | Matching selected bytes produce the expected SHA-256 digest. | Test |
| FR-001-AC-2 | Changed bytes under a selected digest receive source_digest_mismatch. | Test |
| FR-001-AC-3 | Invalid UTF-8 receives a source diagnostic. | Test |
| FR-001-AC-4 | Input beyond the selected byte ceiling receives resource_exhausted naming that ceiling, whether the ceiling is below or above the 1 MiB default. | Test |
| FR-001-AC-5 | Source admitted with authority `agent-ix`, identity `specs/a.quire`, revision namespace `git`, revision value `3f2a` and bytes `b` carries a `RawSourceRef` whose authority, identity, revision namespace and value read exactly those labels and whose digest is the `quire.source.bytes/v1` digest of `b`. Admitting the same bytes under revision value `3f2b` gives a `RawSourceRef` that differs only in the revision value. | Test (TC-424) |
| FR-001-AC-6 | Admission refuses with `invalid_source_identity`, cause `blank-label`, and admits nothing, when exactly one of the authority, identity, revision namespace or revision value is empty, again when it is a single space, and again when it is U+3000 IDEOGRAPHIC SPACE; field `label` is `authority`, `identity`, `revision_namespace` or `revision` respectively. A label that is U+200B ZERO WIDTH SPACE, which is not `White_Space`, is not blank. | Test (TC-424) |
| FR-001-AC-11 | With both the revision namespace and the authority blank, admission refuses `invalid_source_identity`/`blank-label` with `label` `authority`. With the identity blank and the path empty, it refuses `blank-label` with `label` `identity`, not `empty-path`. With all four labels non-blank and the path empty, it refuses `invalid_source_identity`/`empty-path` with no field. | Test (TC-424) |
| FR-001-AC-12 | An incremental edit whose expected revision is not the source's revision, rendering a CST node against a parsed source it does not belong to, and an editor request bound to another document revision each refuse with `invalid_source_map`, keeping the host cause `EditPredecessor`, `ForeignNode` or `RequestRevision` respectively, and name no region; none refuses with `invalid_source_identity`. | Test (TC-424) |
| FR-001-AC-7 | Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, revision (`git`, `1`), and again under the reference of bytes `b'` admitted as `a`, `u`, (`git`, `2`), give `Point` the same node key. Checked under the reference of `b` admitted as authority `c`, identity `u`, revision (`git`, `1`), they give `Point` a different key, and under the reference of `b` admitted as authority `a`, identity `v`, revision (`git`, `1`), a third key. | Test (TC-424) |
| FR-001-AC-8 | Bytes `a\xffb` admitted as (`a`, `u`, `git`, `1`) refuse with the region `[1, 1)` under the `RawSourceRef` of those three bytes, and bytes `ab\0c` with the region `[2, 3)` under theirs. | Test (TC-424) |
| FR-001-AC-10 | Admission with an empty revision namespace, admission of five bytes under a four-byte ceiling, and verified intake of bytes whose digest differs from the selected one each refuse with no region, not a region at byte 0. | Test (TC-424) |
| FR-001-AC-9 | A renderer given the region `[4, 7)` of admitted source `ab\ncdéf` reports start line 2, column 2 and end line 2, column 4, derived from the admitted bytes; the region itself holds only its `RawSourceRef`, 4 and 7. | Test (TC-424) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md):
  the same identity and source location on both sides of the checked-package
  boundary, which the replay executor's recompilation needs.
- ADR-013 O-07 and O-12: `RawSourceRef` and `SourceRegion`. §7 slice S-4b
  builds this requirement's source reference.
- QSpec FR-004: a path or display label is a locator, and a key includes the
  revision namespace and value. QSpec FR-322 `RawSourceRef`, `Revision`,
  `SourceOwner` and `PackageLock.sources`.
- `quire.native.diagnostics/v1` revision `1-draft.8` (QSpec STD-110,
  `proposals/quire-v1/definitions/native-diagnostics.md`, cited by
  reference): the `invalid_source_identity` row, its causes `blank-label`
  and `empty-path`, and the `label` payload spelling; QSpec FR-272-AC-12.
- [Detailed contract or implementation evidence](../../qsl-foundation/src/source.rs) supplies the scoped context.

## Status

Draft. AC-1 to AC-4 describe the existing reader. AC-5 to AC-10 are
implemented (ADR-013 §7 slice S-4b) and backed by TC-424:
`SourceIdentity` (`qsl-foundation/src/source.rs`) carries the four labels,
admission mints the source's `RawSourceRef`, S0 refusals carry a
`SourceRegion` or none, `Source::render` and `render_offered` derive line and
column, and `PackageDeclarations::new` takes the unit's `RawSourceRef`. The
native-v1 `Diagnostic` still renders a region-less refusal at byte 0 (the
debt recorded above). The replay executor's recompilation under the
reference's labels is ADR-013 TK-01's.

QSL emits the catalog revision `1-draft.8` causes:
`SourceReadCause` carries `BlankLabel { label }` and `EmptyPath`, and
`Source::read_typed` checks the four labels in order before the path
(`qsl-foundation/src/source.rs`). The revision claim
(`src/linking/composed/definition_source.rs`) reads `1-draft.8`.
The cause and `label` reach the native `Diagnostic`,
the `parse`/`format` refusal line, the native-run output, the
native-state-input construction error and the replay recompile refusal, each
tested for `blank-label` with its label. `empty-path` is tested at the reader
and the native `Diagnostic`; the other outputs cannot reach it: `parse` and
`format` and the run request open the file before any source is admitted (an
empty file operand is a file error), a native-state-input artifact has no
path, and the replay path is the reference's identity, which a non-blank
label set makes non-empty. `EditPredecessor`, `ForeignNode` and
`RequestRevision` refuse with `invalid_source_map` and no region
(`qsl-cst/src/diagnostic.rs` `HostCause::code`, `error_without_region`).
AC-6, AC-11 and AC-12 are backed by TC-424.

The native-run output and the `parse`/`format` refusal line render the span
of a region-less refusal as byte 0 only where the lane-private native
`Diagnostic` does (the debt recorded in "Where an S0 refusal is located");
that byte 0 is not this requirement's behaviour.
