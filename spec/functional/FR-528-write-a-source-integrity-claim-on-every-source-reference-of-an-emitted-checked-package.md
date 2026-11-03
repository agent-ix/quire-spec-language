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
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-016
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
with `SourceDigest::of_bytes(bytes)` and a lowercase-hexadecimal `Display`,
and `SourceDigests`, a map from `RawSourceRef` to `SourceDigest`. A bare
`SourceDigest` is a digest of whatever bytes it was given, which the replay
request needs for the raw bytes of its byte provision (FR-071). `SourceDigests`
is the value the emitter trusts, and it has exactly one constructor,
`SourceDigests::of_sources`, over admitted `Source`s: each entry's pair is the
`Source`'s own authority and identity and each digest is the digest of the
bytes that `Source` holds. No public method inserts a digest, so an entry
cannot hold a digest that is not that of an admitted `Source` for its pair.
Two `Source`s admitted under one pair with different bytes refuse construction
with a typed refusal naming the pair; two with equal bytes are one entry.

`PackageDeclarations` gains an additive builder step that attaches a
`SourceDigests` to the assembly, and the checked graph `check` returns retains
it and exposes it through an accessor. The step refuses an entry whose pair is
none of the assembly's sources, with a typed refusal naming the pair. The
`assemble`, `assemble_units` and `emit_checked*` signatures do not change.

What the digest binds is the bytes admitted under a pair, as FR-001 has the
caller name it. The pairing of a unit's forms with the `Source` of the same
pair is the caller's, exactly as naming the unit's `RawSourceRef` already is;
the spine, the CLI and the library import take both from one parsed unit, so
they cannot differ.

The hash is SHA-256 from the `sha2` crate, taken by `qsl-foundation` with
default features off and its portable software implementation, as NFR-015's
row for `qsl-foundation` states with the reason and the unsafe status. The hash
is a pure function of its input (FR-284), so it adds no input, clock or
environment read to any stage. The alternative of hashing in a `std` crate
and carrying an opaque value into `qsl-foundation` was rejected: the digest
could then be made without a `Source`, so a caller could attach a digest of
other bytes, and the replay request would need the same hash in a second
place.

### Once per pair

A source's digest is computed once, when its `Source` enters
`SourceDigests::of_sources`, and the emitter reads that value at every site.
Two sites that name one pair therefore write one claim, which cannot differ.
The emitter computes no digest of its own, and `qsl-package` has no hashing
dependency.

### Which bytes the claim covers

A claim covers the bytes of the `Source` admitted under the pair the
reference names, which is the unit the compiler parsed. For a source read from
a file it is the file. For an extracted native body (FR-030, FR-031) the
unit compiled is the verified body, under the body's own identity, so the
package's references name that identity and the claim is the SHA-256 of the
body bytes, not of the Markdown document it came from. The Markdown document's
pair is not named by any package source reference (its regions are reported in
the extraction record, FR-031), so it carries no claim, and a reader handed the
Markdown bytes under the body's pair refuses at that reference. The reader of
such a package is handed the body bytes under the body's identity. The claim
therefore differs from the digest of the authored document by construction,
and Quire's extraction record, not this claim, vouches for the correspondence
between document and body.

### A source with no digest, and when it takes effect

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

The refusal changes the behavior of every emit caller, so it takes effect
together with the claim, when the lock moves to the Contract IR change that
lets the wire carry it (Gating). Until then the emitter writes no claim, because
the wire has no member to hold it, and it refuses nothing for want of a
digest: emission behaves as before this requirement, and attaching digests is
optional. When the refusal takes effect, every caller that emits must have
attached digests: the CLI compile and run paths, the S4 library import
(FR-099), the replay executor's recompile (FR-098), the checked-handoff
writer (FR-518), the emission paths of `qsl-package`, `qsl-semantics`,
`qsl-eval`, `qsl-inspect` and `qsl-bench`, and the tests and fixtures that
call `emit_checked*` (the QSL-1560 fixture and the xtask conformance fixtures
included). The coder's first step on activation is to attach digests at each
of those and to run the suite with the refusal enabled.

The refusal covers every pair a site writes, whatever put it there. A
temporal clock selection's `source` (`TemporalClockSelection`) selects a
declaration of an assembled source and writes no reference of its own, so it
adds no requirement; a selection naming a source with no digest still refuses
under the existing selection rules before the emitter reaches a site. A
dependency library's source never appears at the four sites of the importing
package: the library's own emitted package carries its own claims, and the
replay request copies them (FR-071).

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
alternatives considered, and why the sidecar was chosen:

- Carry the digests in a sidecar built only from admitted `Source`s, attached
  to the assembly and retained on the checked graph (chosen). It is additive:
  no existing `assemble*` or `emit_checked*` signature changes, and the value
  is computed where the bytes live and cannot be forged.
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
acceptance criteria and the emit refusal (marked gated) cannot pass until that
change is merged and QSL's lock moves to it, and they are written for the
emitter paired with it. The digest, `SourceDigests`, builder, retention and
pre-activation criteria do not depend on Contract IR and are verified first.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-528-AC-1 | `SourceDigest::of_bytes` over the bytes `a`, CR, LF, `b`, LF renders `953bba9ac9726eaea07e844abcf144a0afe998039257c7a88b6665819597f39d`. | Test (TC-1323) |
| FR-528-AC-2 | `SourceDigest::of_bytes` over the bytes `a`, LF, `b`, LF renders `911169ddaaf146aff539f58c26c489af3b892dff0fe283c1c264c65ae5aa59a2`, a value different from AC-1's. | Test (TC-1323) |
| FR-528-AC-3 | `SourceDigest::of_bytes` over zero bytes renders `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. | Test (TC-1323) |
| FR-528-AC-4 | `SourceDigests::of_sources` over a `Source` admitted from CRLF bytes holds the digest of exactly those bytes. | Test (TC-1323) |
| FR-528-AC-5 | `SourceDigests::of_sources` over two `Source`s that differ in one byte holds two different digests. | Test (TC-1323) |
| FR-528-AC-6 | Each entry of `SourceDigests::of_sources` is keyed by the authority and identity of the `Source` it was built from. | Test (TC-1323) |
| FR-528-AC-7 | `SourceDigests` has no public constructor or method that takes a digest value, and `SourceDigests::of_sources` is its only constructor. | Inspection |
| FR-528-AC-8 | `SourceDigests::of_sources` over two `Source`s admitted under one pair with different bytes refuses, naming the pair. | Test (TC-1323) |
| FR-528-AC-9 | `SourceDigests::of_sources` over two `Source`s admitted under one pair with equal bytes holds one entry for it. | Test (TC-1323) |
| FR-528-AC-10 | The builder step that attaches `SourceDigests` refuses an entry whose pair is none of the assembly's sources, naming the pair. | Test (TC-1323) |
| FR-528-AC-11 | The checked graph returned by `check` exposes the attached digest of each assembled source and none for a pair that is not one. | Test (TC-1323) |
| FR-528-AC-12 | The signatures of `PackageDeclarations::assemble`, `assemble_units` and `emit_checked` are those they had before this requirement. | Inspection |
| FR-528-AC-13 | `RawSourceRef` has exactly its two members, authority and identity. | Inspection |
| FR-528-AC-14 | `qsl-package` has no hashing dependency. | Inspection |
| FR-528-AC-15 | `qsl-foundation` reaches the hash only through `SourceDigest::of_bytes`. | Inspection |
| FR-528-AC-16 | Before the lock moves to IR-1273, emitting a graph with no digests attached succeeds and writes the package it wrote before this requirement, with no claim. | Test (TC-1323) |
| FR-528-AC-17 | Gated on IR-1273: emitting a graph whose source has no attached digest returns `EmitRefusal::SourceBytesNotSupplied` naming the pair, with code `invalid_package` and cause `source-integrity`, and no bytes. | Test (TC-1323) |
| FR-528-AC-18 | Gated on IR-1273: of several pairs with no digest, the refusal names the first in the order `lock.sources`, `source_map`, `refinement_step_rows`, `diagnostics.entries`, array order within a site. | Test (TC-1323) |
| FR-528-AC-19 | Gated on IR-1273: emitting with a `TemporalClockSelection` over an assembled source whose digest is attached writes the package, and the selection adds no source reference. | Test (TC-1323) |
| FR-528-AC-20 | Gated on IR-1273: every `lock.sources` entry of an emitted package carries a `source_integrity` equal to the SHA-256 of the bytes of the `Source` admitted under its pair (QSpec FR-322-AC-319, FR-322-AC-324). | Test (TC-1323) |
| FR-528-AC-21 | Gated on IR-1273: an empty source's `lock.sources` entry carries the digest of zero bytes (QSpec FR-322-AC-331). | Test (TC-1323) |
| FR-528-AC-22 | Gated on IR-1273: the `source` of every `source_map` region of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-23 | Gated on IR-1273: the `source` of every `refinement_step_rows` region of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-24 | Gated on IR-1273: the `source` of every `diagnostics.entries` locus of an emitted package carries the claim of its pair (QSpec FR-322-AC-320). | Test (TC-1323) |
| FR-528-AC-25 | Gated on IR-1273: a pair named at all four sites carries one identical claim at each. | Test (TC-1323) |
| FR-528-AC-26 | Gated on IR-1273: changing one byte of one source of a two-source package and emitting again changes the claim of that pair at every site and changes no other pair's claim. | Test (TC-1323) |
| FR-528-AC-27 | Gated on IR-1273: `DefinitionRef` and `SourceOwner` members of an emitted package carry no `source_integrity` (QSpec FR-322-AC-322, FR-322-AC-323). | Test (TC-1323) |
| FR-528-AC-28 | Gated on IR-1273: the reader, handed the bytes of the `Source` admitted under each pair, admits an emitted package (QSpec FR-322-AC-330). | Test (TC-1323) |
| FR-528-AC-29 | Gated on IR-1273: the reader, handed bytes that differ in one byte of one source, refuses `invalid_package`/`source-integrity` at that source's reference (QSpec FR-322-AC-329). | Test (TC-1323) |
| FR-528-AC-30 | Gated on IR-1273: a package compiled from an extracted native body carries, for the body's identity, the SHA-256 of the body bytes, and the reader handed the original Markdown bytes under that identity refuses `invalid_package`/`source-integrity`. | Test (TC-1323) |
| FR-528-AC-31 | Gated on IR-1273: a library compiled for import is read back by the I2 read with its own bytes and builds its import view. | Test (TC-1323) |
| FR-528-AC-32 | Gated on IR-1273: an I2 refusal for `invalid_package`/`source-integrity` is carried by `ImportRefusal::View` with its code, cause and pointer. | Test (TC-1323) |

## Dependencies

- **Contract IR:** IR-1273, `CheckedSourceRef` carries reader-verified
  source-byte integrity (PR #841, draft). Gates FR-528-AC-17 to
  FR-528-AC-32 and the QSL lock bump that compiles the claim.
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
