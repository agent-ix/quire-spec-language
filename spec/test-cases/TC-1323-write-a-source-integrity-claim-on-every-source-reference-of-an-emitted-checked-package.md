---
id: TC-1323
title: "Write a source_integrity claim on every source reference of an emitted checked package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-528
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-322
    type: verifies
---
# TC-1323: Write a source_integrity claim on every source reference of an emitted checked package

## Description

Verify FR-528-AC-1 to AC-6, AC-8 to AC-11 and AC-16 against the digest type, `SourceDigests`, the assembly builder and the pre-activation emitter, which do not depend on Contract IR, and FR-528-AC-17 to AC-32 against the emitter paired with the IR-1273 reader (gated: written, and not run, until Contract IR PR #841 is merged and the lock moves). FR-528-AC-7 and AC-12 to AC-15 are inspections of constructors, signatures and manifests. A wrong implementation this test catches: an emitter that hashes normalised text, that hashes at each site separately, that omits the claim for a source with no digest, that accepts a digest not made from a `Source`, or that finds the bytes by path.

Unit **U** is two sources, `a` (CRLF line ends) and `b` (LF), plus an empty source `e`, and an extracted native body **X** with its Markdown document **D**. Package **P** compiles **U**, with a source map, one refinement step row and one diagnostic locus naming each source.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `SourceDigest::of_bytes` over `a\r\nb\n` | `953bba9a...f39d` |
| 2 | 2 | `SourceDigest::of_bytes` over `a\nb\n` | `911169dd...59a2`, unequal to row 1's |
| 3 | 3 | `SourceDigest::of_bytes` over zero bytes | `e3b0c442...b855` |
| 4 | 4 | `SourceDigests::of_sources` over the CRLF `Source` of `a` | the digest of those bytes |
| 5 | 5 | `of_sources` over two `Source`s differing in one byte | two different digests |
| 6 | 6 | `of_sources` entry keys | each the `Source`'s own authority and identity |
| 7 | 7 | inspect `SourceDigests`' public surface | no constructor or method taking a digest |
| 8 | 8, 9 | two `Source`s under one pair: different bytes; equal bytes | refusal naming the pair; one entry |
| 9 | 10 | attach an entry for a pair not among the assembly's sources | typed refusal naming the pair |
| 10 | 11 | read the digest of each assembled source and of a foreign pair from the checked graph | the attached digests; none |
| 11 | 12-15 | inspect `assemble`, `assemble_units`, `emit_checked`, `RawSourceRef`, `qsl-package`'s manifest and `qsl-foundation`'s hash use | signatures unchanged; two members; no hashing dependency; hash only through `SourceDigest::of_bytes` |
| 12 | 16 | emit **P** with no digest attached, before the lock moves | written as before, no claim |
| 13 | 17 | gated: emit **P** with no digest for `b` | `SourceBytesNotSupplied` naming `b`, code `invalid_package`, cause `source-integrity`, no bytes |
| 14 | 18 | gated: no digest for a pair first named by `source_map` and another named by a later `lock.sources` entry | the refusal names the `lock.sources` pair first |
| 15 | 19 | gated: emit **P** with a `TemporalClockSelection` over `a` | package written; no added source reference |
| 16 | 20, 21 | gated: emit **P**; read each `lock.sources` entry, `e` included | each claim equals the SHA-256 of its authored bytes; `e` equals the zero-byte digest |
| 17 | 22-24 | gated: read the source map, refinement row and diagnostic locus sources | each carries its pair's claim |
| 18 | 25 | gated: compare the claims of one pair across the four sites | identical |
| 19 | 26 | gated: flip one byte of `b`, emit again | only `b`'s claims change |
| 20 | 27 | gated: read `DefinitionRef` and `SourceOwner` members | two members, no claim |
| 21 | 28, 29 | gated: read **P** with the authored bytes; with one byte of `b` flipped | admitted; `invalid_package`/`source-integrity` at `b`'s reference |
| 22 | 30 | gated: compile **X**; read with the body bytes, then with **D**'s bytes under the body's identity | the claim is the body's SHA-256; admitted; refused `source-integrity` |
| 23 | 31, 32 | gated: import a library compiled from its units; inject an I2 `source-integrity` refusal | view built; the refusal carried by `ImportRefusal::View` |

Tag the tests `#[trace("FR-528-AC-1", ..., "TC-1323")]`; the gated rows carry their AC tags and `#[ignore]` until IR-1273 lands; AC-7 and AC-12 to AC-15 are Inspection and carry no test tag.

## Expected Results

Each row's Expected column.
