---
id: FR-528
title: "Write a source_integrity claim on every source reference of an emitted checked package"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: references
---
# FR-528: Write a source_integrity claim on every source reference of an emitted checked package

## Description

When the checked-package emitter writes a source reference, it SHALL write
with it a `source_integrity` claim: the SHA-256 of the raw bytes of the source
document the reference names, as 64 lowercase hexadecimal characters. A
checked package then states, for each source it was compiled from, which
bytes it vouches for, and a reader that is handed those bytes can tell
whether they are the ones the package was built from (QSpec FR-322
"Source-byte integrity"). This requirement is the writer side. The reader
side belongs to Contract IR (IR-1273) and to FR-071 for the replay request.

The claim is a function of bytes the front end already holds. QSL admits a
source once (FR-001), as a `Source` that keeps the admitted bytes unchanged,
and every later stage names it by its `RawSourceRef`, authority and
identity. The claim is computed at that one point and carried beside the
reference, so the emitter never goes back to a file, a path or a cache to
find the bytes (QSpec FR-322-AC-329 verifies against caller-supplied bytes,
not against a filesystem).

## Use case

A maintainer compiles two source units into one package and ships the
package. A consumer later receives the package with the two documents and
reads it. If one document changed by a single byte on the way, the consumer's
reader refuses the package at that document's reference. The refusal is
possible only because the emitter wrote, for each source, a claim computed
from the bytes it compiled.

## Semantic authority and boundary

QSpec FR-322 owns the claim, its syntax, its four sites and its reader
refusals (FR-322-AC-319 to FR-322-AC-341). This requirement owns where QSL
computes the claim, how the bytes' digest reaches the emitter, and what the
emitter does when it holds no digest.

The sites are the four FR-322 names, and the emitter writes the claim at
each: `lock.sources`; the `source` of every `source_map` region; the `source`
of every `refinement_step_rows` region; and the `source` of every
`diagnostics.entries` locus (FR-322-AC-319, FR-322-AC-320). A
`DefinitionRef` and a nominal `SourceOwner` stay the closed two-member
`{authority, identity}` and carry no claim (FR-322-AC-322, FR-322-AC-323):
the owner join compares the pair alone, so the claim never changes which
declarations belong to which source.

## Inputs

- The `Source` of each unit of a compile, whose admitted bytes are the
  document (FR-001), and the `RawSourceRef` the caller named it by.
- The checked graph the units assemble into (FR-091, FR-480).

## Outputs

- A `SourceDigest` for each assembled source, retained on the checked graph
  beside its `RawSourceRef`.
- The emitted package bytes, with `source_integrity` on every written source
  reference, or an `EmitRefusal`.

## Behavior

### The claim

The claim is the plain SHA-256 of the document's raw bytes, rendered as 64
lowercase hexadecimal characters. It has no algorithm prefix, no
normalisation of line ends, no Unicode normalisation and no trimming: a
source with CRLF line ends and the same source with LF line ends have
different claims (QSpec FR-322-AC-324). An empty source is a valid source and
its claim is the SHA-256 of zero bytes (FR-322-AC-331). FR-001 admits only
UTF-8 bytes with no byte-order mark and no NUL, and keeps them unchanged, so
the admitted `Source` text and the bytes the caller supplied are the same
bytes.

### Where the digest is computed and carried

`qsl_foundation::source::provenance` defines `SourceDigest`, a 32-byte value
with a constructor `SourceDigest::of(bytes)` and a lowercase-hexadecimal
`Display`, and `SourceDigests`, a map from `RawSourceRef` to `SourceDigest`.
`Source` yields its digest from the bytes it holds. The `qsl-foundation` crate
owns the hash, as the one crate every other stage already depends on; the
hash is a pure function of its input (FR-284), so it adds no input, clock or
environment read to any stage.

`PackageDeclarations` gains an additive builder step that attaches a
`SourceDigests` to the assembly, and the checked graph `check` returns
retains it and exposes it through an accessor. The `assemble`,
`assemble_units` and `emit_checked*` signatures do not change. A caller that
holds the `Source` of each unit attaches the digests of those sources
(`SourceDigests::of_sources`). The builder step refuses a digest for a pair
that is none of the assembly's sources, and two different digests for one
pair, with a typed refusal naming the pair.

### Once per pair

A source's digest is computed once, when its `Source` is admitted and its
digests are attached, and the emitter reads that value at every site. Two
sites that name one pair therefore write one claim, which cannot differ. The
emitter computes no digest of its own, and `qsl-package` has no hashing
dependency.

### A source with no digest

When the emitter is about to write a source reference whose pair has no
digest on the graph, it SHALL refuse the whole emission with
`EmitRefusal::SourceBytesNotSupplied`, naming the pair, and write no bytes.
The refusal's code is `invalid_package` and its cause `source-integrity`,
the pair the reader gives a reference whose bytes the caller did not supply
(QSpec FR-322-AC-332, FR-272-AC-26), so the writer and the reader use one
vocabulary and no new catalog cause is introduced. The emitter visits the
sites in the order `lock.sources`, `source_map`, `refinement_step_rows`,
`diagnostics.entries`, array order within a site, and the first pair with no
digest decides.

This covers every pair a site writes, whatever put it there. A temporal
clock selection's `source` (`TemporalClockSelection`) selects a declaration
of an assembled source and writes no reference of its own, so it adds no
requirement; a selection naming a source with no digest still refuses under
the existing selection rules before the emitter reaches a site. A dependency
library's source never appears at the four sites of the importing package: the
library's own emitted package carries its own claims, and the replay request
copies them (FR-071).

### Library import

The S4 source resolution reads each library's emitted bytes back through the
I2 read (FR-099). The library units were compiled from sources the importer
holds, so the read is handed those bytes. A refusal of that read for
`invalid_package`/`source-integrity` surfaces as the existing
`ImportRefusal::View`, which keeps the read's code, cause and pointer; no
`ImportRefusal` variant is added. It cannot arise from QSL's own emission,
because the claim and the bytes come from one `Source`, so it states that the
writer and the reader agree.

## Design decisions

The claim has to reach the emitter from the point where the bytes exist. The
alternatives considered, and why the builder was chosen:

- Carry the digest in a sidecar map attached to the assembly and retained on
  the checked graph (chosen). It is additive: no existing `assemble*` or
  `emit_checked*` signature changes, so the many callers that never emit a
  source claim keep compiling, and the value is computed where the bytes
  live.
- Add a digest or bytes argument to `PackageDeclarations::assemble`,
  `assemble_units` and their limit, cancel and execution-point variants
  (rejected). The value is read only by the emitter, yet every assembly
  entry point and every caller would change, including the tests that
  assemble without emitting.
- Make `RawSourceRef` carry the claim (rejected). It is the QSpec
  `quire:canonical` two-member type that orders, hashes and joins owners
  (FR-322-AC-323); with a claim in it, equality, ordering and the owner join
  would depend on bytes, and a `DefinitionRef` or `SourceOwner` built from it
  would carry a member the schema forbids.
- Add a source-bytes map parameter to every `emit_*` entry point (rejected).
  There are many emit entry points and the emitter would have to hash bytes
  itself, breaking "once per pair"; the bytes would also reach the emitter
  separately from the units they belong to, so a map could name a pair the
  graph does not hold.
- Look the bytes up from the filesystem at emission (rejected). The claim
  must be the digest of the bytes compiled, not of whatever a path holds
  later (QSpec FR-322), and no QSL stage reads by path (FR-071, FR-098).

A pair with no digest refuses rather than omitting the claim or writing a
placeholder, because QSpec FR-322 admits a reference only with a valid claim
(FR-322-AC-326): omitting would write a package every reader refuses, and a
placeholder would write a false claim.

## Gating

QSpec FR-322 makes the claim a member of the checked-package wire, which the
emitter writes through Contract IR's `CheckedSourceRef`. Contract IR PR #841
(`code/ir1273-source-integrity`, draft) adds `source_integrity` to
`CheckedSourceRef` and the reader that verifies it (IR-1273). The wire-level
acceptance criteria below (marked gated) cannot pass until that change is
merged and QSL's lock moves to it, and they are written for the emitter paired
with it. The digest, builder, retention and refusal criteria do not depend on
Contract IR and are verified first.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-528-AC-1 | `SourceDigest::of` over the bytes `a`, CR, LF, `b`, LF renders `953bba9ac9726eaea07e844abcf144a0afe998039257c7a88b6665819597f39d`, and over the same text with LF line ends renders a different value. | Test (TC-1323) |
| FR-528-AC-2 | `SourceDigest::of` over zero bytes renders `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. | Test (TC-1323) |
| FR-528-AC-3 | A `Source` admitted from bytes that differ in one byte yields a different digest, and a `Source` admitted from CRLF bytes yields the digest of those CRLF bytes. | Test (TC-1323) |
| FR-528-AC-4 | The builder step that attaches `SourceDigests` refuses a digest for a pair that is none of the assembly's sources, naming the pair. | Test (TC-1323) |
| FR-528-AC-5 | The builder step refuses two different digests for one pair, naming the pair, and accepts the same digest twice for it. | Test (TC-1323) |
| FR-528-AC-6 | The checked graph returned by `check` exposes the attached digest of each assembled source and none for a pair that is not one. | Test (TC-1323) |
| FR-528-AC-7 | The signatures of `PackageDeclarations::assemble`, `assemble_units` and `emit_checked` are those they had before this requirement, and `RawSourceRef` has exactly its two members. | Inspection |
| FR-528-AC-8 | `qsl-package` has no hashing dependency, and `qsl-foundation`'s hash is reached only through `SourceDigest::of`. | Inspection |
| FR-528-AC-9 | Emitting a graph whose source has no attached digest returns `EmitRefusal::SourceBytesNotSupplied` naming the pair, with code `invalid_package` and cause `source-integrity`, and no bytes. | Test (TC-1323) |
| FR-528-AC-10 | Of several pairs with no digest, the refusal names the first in the order `lock.sources`, `source_map`, `refinement_step_rows`, `diagnostics.entries`, array order within a site. | Test (TC-1323) |
| FR-528-AC-11 | Emitting with a `TemporalClockSelection` over an assembled source whose digest is attached writes the package, and the selection adds no source reference. | Test (TC-1323) |
| FR-528-AC-12 | Gated on IR-1273: every `lock.sources` entry of an emitted package carries a `source_integrity` equal to the SHA-256 of the authored bytes of its source, and an empty source's entry carries the digest of zero bytes (QSpec FR-322-AC-319, FR-322-AC-331). | Test (TC-1323) |
| FR-528-AC-13 | Gated on IR-1273: the `source` of every `source_map` region of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-14 | Gated on IR-1273: the `source` of every `refinement_step_rows` region of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-15 | Gated on IR-1273: the `source` of every `diagnostics.entries` locus of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-16 | Gated on IR-1273: a pair named at all four sites carries one identical claim at each. | Test (TC-1323) |
| FR-528-AC-17 | Gated on IR-1273: changing one byte of one source of a two-source package and emitting again changes the claim of that pair at every site and changes no other pair's claim. | Test (TC-1323) |
| FR-528-AC-18 | Gated on IR-1273: `DefinitionRef` and `SourceOwner` members of an emitted package carry no `source_integrity` (QSpec FR-322-AC-322, FR-322-AC-323). | Test (TC-1323) |
| FR-528-AC-19 | Gated on IR-1273: the reader, handed the authored bytes of each source, admits an emitted package, and handed bytes that differ in one byte of one source refuses `invalid_package`/`source-integrity` at that source's reference (QSpec FR-322-AC-329, FR-322-AC-330). | Test (TC-1323) |
| FR-528-AC-20 | Gated on IR-1273: a library compiled for import is read back by the I2 read with its own bytes and builds its import view, and an I2 refusal for `invalid_package`/`source-integrity` is carried by `ImportRefusal::View` with its code, cause and pointer. | Test (TC-1323) |

## Dependencies

- **Contract IR:** IR-1273, `CheckedSourceRef` carries reader-verified
  source-byte integrity (PR #841, draft). Gates FR-528-AC-12 to
  FR-528-AC-20 and the QSL lock bump that compiles the claim.
- [FR-001](FR-001-read-exact-source.md): the one admission of source bytes.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md):
  the assembly the digests attach to.
- [FR-062](FR-062-implement-checked-family-contract.md): the checked family
  the emitter writes.
- [FR-518](FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md):
  the handoff writer's packages carry these claims, and its case 28m
  mutates the bytes the reader is handed.
- [FR-071](FR-071-implement-typed-replay-request.md): the replay request copies the
  claims of the lock's `sources` and checks them against its byte provision.
- QSpec FR-322 "Source-byte integrity" (FR-322-AC-319 to FR-322-AC-341),
  FR-272-AC-26.
