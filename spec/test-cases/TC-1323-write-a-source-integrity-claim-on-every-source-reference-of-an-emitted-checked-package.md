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

Verify FR-528-AC-1 to FR-528-AC-6 and FR-528-AC-9 to FR-528-AC-11 against the digest type, the assembly builder and the emitter's refusal, which do not depend on Contract IR, and FR-528-AC-12 to FR-528-AC-20 against the emitter paired with the IR-1273 reader (gated: they are written, and not run, until Contract IR PR #841 is merged and the lock moves). FR-528-AC-7 and FR-528-AC-8 are inspections of signatures and manifests. A wrong implementation this test catches: an emitter that hashes normalised text, that hashes at each site separately, that omits the claim for a source with no digest, or that finds the bytes by path.

Unit **U** is two sources, `a` (CRLF line ends) and `b` (LF), plus an empty source `e`. Package **P** compiles **U**, with a source map, one refinement step row and one diagnostic locus naming each source.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `SourceDigest::of` over `a\r\nb\n`, then over `a\nb\n` | `953bba9a...f39d`; a different value |
| 2 | 2 | `SourceDigest::of` over zero bytes | `e3b0c442...b855` |
| 3 | 3 | admit `Source` from bytes differing in one byte; admit CRLF bytes | different digests; the CRLF bytes' digest |
| 4 | 4 | attach a digest for a pair not among the assembly's sources | typed refusal naming the pair |
| 5 | 5 | attach two different digests for one pair; attach the same digest twice | refusal naming the pair; accepted |
| 6 | 6 | read the digest of each assembled source and of a foreign pair from the checked graph | the attached digests; none |
| 7 | 7, 8 | inspect `assemble`, `assemble_units`, `emit_checked`, `RawSourceRef` and `qsl-package`'s manifest | signatures unchanged; two members; no hashing dependency |
| 8 | 9 | emit **P** with no digest attached for `b` | `SourceBytesNotSupplied` naming `b`, code `invalid_package`, cause `source-integrity`, no bytes |
| 9 | 10 | emit with no digest for a pair first named by `source_map` and another named by a later `lock.sources` entry | the refusal names the `lock.sources` pair first |
| 10 | 11 | emit **P** with a `TemporalClockSelection` over `a` | package written; no added source reference |
| 11 | 12 | gated: emit **P**; read each `lock.sources` entry, include `e` | each claim equals the SHA-256 of its authored bytes; `e` equals the zero-byte digest |
| 12 | 13-15 | gated: read the source map, refinement row and diagnostic locus sources | each carries its pair's claim |
| 13 | 16 | gated: compare the claims of one pair across the four sites | identical |
| 14 | 17 | gated: flip one byte of `b`, emit again | only `b`'s claims change |
| 15 | 18 | gated: read `DefinitionRef` and `SourceOwner` members | two members, no claim |
| 16 | 19 | gated: read **P** with the authored bytes; with one byte of `b` flipped | admitted; `invalid_package`/`source-integrity` at `b`'s reference |
| 17 | 20 | gated: import a library compiled from its units | view built; an injected I2 `source-integrity` refusal is carried by `ImportRefusal::View` |

Tag the tests `#[trace("FR-528-AC-1", ..., "TC-1323")]`; the gated rows carry their AC tags and `#[ignore]` until IR-1273 lands; AC-7 and AC-8 are Inspection and carry no test tag.

## Expected Results

Each row's Expected column.
