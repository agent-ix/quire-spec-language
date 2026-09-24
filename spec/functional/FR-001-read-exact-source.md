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

A source QSL reads itself is named by the caller, never by QSL. The caller
supplies four labels with the bytes: the **authority** that issues the
source's identity, the **identity** within that authority, and the
**revision** as a namespace and a value. The namespace names the revision
system the value belongs to, for example `git` for a commit or `semver` for
a release. QSL does not default any label and does not derive one from the
path, the bytes or another label: a path is a locator, not an identity
(QSpec FR-004), and a revision namespace is never inferred from the shape of
its value.

The source shall refuse with `invalid_source_identity` when the authority,
identity, revision namespace or revision value is empty or only whitespace,
or when the path is empty.

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
  ([FR-010](FR-010-report-native-outcomes.md)).
- A library caller supplies them in the source identity it passes to the
  source reader and to S1's `parse`.
- The replay executor recompiles each source under the `RawSourceRef` the
  replay request's package reference names (QSpec FR-323 `package`, ADR-013
  O-26). It passes that reference's four labels as the source identity, the
  reference's identity as the path, and the provided bytes as the source. A
  reference whose label is only whitespace, which QSpec's `Nonempty` admits,
  refuses the replay with `invalid_source_identity` before recompilation. It requires the recomputed
  `quire.source.bytes/v1` digest to equal the reference's digest, as
  ADR-013 C-13 requires. The recompiled package therefore names each source
  exactly as the proving run did, which keeps its declaration keys and its
  `package_id` (US-005).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-001-AC-1 | Matching selected bytes produce the expected SHA-256 digest. | Test |
| FR-001-AC-2 | Changed bytes under a selected digest receive source_digest_mismatch. | Test |
| FR-001-AC-3 | Invalid UTF-8 receives a source diagnostic. | Test |
| FR-001-AC-4 | Input beyond the selected byte ceiling receives resource_exhausted naming that ceiling, whether the ceiling is below or above the 1 MiB default. | Test |
| FR-001-AC-5 | Source admitted with authority `agent-ix`, identity `specs/a.quire`, revision namespace `git`, revision value `3f2a` and bytes `b` carries a `RawSourceRef` whose authority, identity, revision namespace and value read exactly those labels and whose digest is the `quire.source.bytes/v1` digest of `b`. Admitting the same bytes under revision value `3f2b` gives a `RawSourceRef` that differs only in the revision value. | Test (TC-424) |
| FR-001-AC-6 | Admission refuses with `invalid_source_identity`, and admits nothing, when exactly one of the authority, identity, revision namespace or revision value is empty, and again when it is a single space. | Test (TC-424) |
| FR-001-AC-7 | Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, revision (`git`, `1`), and again under the reference of bytes `b'` admitted as `a`, `u`, (`git`, `2`), give `Point` the same node key. Checked under the reference of `b` admitted as authority `c`, identity `u`, revision (`git`, `1`), they give `Point` a different key, and under the reference of `b` admitted as authority `a`, identity `v`, revision (`git`, `1`), a third key. | Test (TC-424) |

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
- [Detailed contract or implementation evidence](../../qsl-foundation/src/source.rs) supplies the scoped context.

## Status

Draft. AC-1 to AC-4 describe the existing reader. AC-5 to AC-7 are specified
under QSL-233 and not implemented: `SourceIdentity`
(`qsl-foundation/src/source.rs`) holds an identity and a one-string revision,
and no source carries a `RawSourceRef`. ADR-013 §7 slice S-4b builds them.
