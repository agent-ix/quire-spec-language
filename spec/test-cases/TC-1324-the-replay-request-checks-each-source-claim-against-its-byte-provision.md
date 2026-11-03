---
id: TC-1324
title: "The replay request checks each source claim against its byte provision in one total order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-323
    type: verifies
---
# TC-1324: The replay request checks each source claim against its byte provision in one total order

## Description

Verify FR-071-AC-18 to FR-071-AC-32 and FR-098-AC-30 and FR-098-AC-31, tagging QSpec FR-323-AC-29 to FR-323-AC-35. Requests are built in process as `ReplayRequestWire` values; the vector ids are QSpec's RP-P1, RP-R1 to RP-R7 (TC-277's rows BP-60 to BP-63 state the same cases on the QSpec side). A wrong implementation this test catches: a check that runs at the executor after the completeness rule, one that reports a pointer at the `source_integrity` member, one that lets absent bytes win over a malformed later claim, or one that keeps revision labels in the entry.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | FR-071-AC-18 | round-trip triple entries; empty source with its zero-byte digest; inspect accessors | equal; decodes; no revision accessor (RP-P1) |
| 2 | FR-071-AC-19 | claim differs from the digest at `package.sources[0]` | `invalid_package`/`source-integrity` at `/package/sources/0` (RP-R1) |
| 3 | FR-071-AC-20 | claim differs at `dependencies[0].sources[0]` | pointer `/package/dependencies/0/sources/0` (RP-R2) |
| 4 | FR-071-AC-21 | an entry with no claim | `source-integrity` at that entry (RP-R3) |
| 5 | FR-071-AC-22 | a claim of JSON kind null, number, object | `source-integrity` at that entry |
| 6 | FR-071-AC-23 | uppercase; 63 characters; 65 characters; non-hexadecimal | each `source-integrity` at the entry |
| 7 | FR-071-AC-24 | pair absent from the provision, claims well-formed | `missing_import`/`missing-selection` naming the pair (RP-R4) |
| 8 | FR-071-AC-25 | first entry without bytes, later dependency entry malformed | `source-integrity` at the dependency entry (RP-R7) |
| 9 | FR-071-AC-26 | mismatch at `/package/sources/0`, absent bytes at a dependency entry | `source-integrity` at `/package/sources/0` (RP-R5) |
| 10 | FR-071-AC-27 | absent bytes at `/package/sources/0`, mismatch at a dependency entry | `missing_import`/`missing-selection` (RP-R6) |
| 11 | FR-071-AC-28 | two failing entries across `sources` and dependencies `[0]`, `[1]` | the first in order decides |
| 12 | FR-071-AC-29 | each earlier rule's refusal together with a mismatching claim | the earlier rule's refusal |
| 13 | FR-071-AC-30 | a domain-package entry and observed-document entry with arbitrary bytes | no source-integrity refusal |
| 14 | FR-071-AC-31 | `object_environment` identity with `source_integrity` | `unknown-member` |
| 15 | FR-071-AC-32 | read code, cause, pointer, authority and identity of the refusal | `invalid_package`, `source-integrity`, the entry's pointer, the pair |
| 16 | FR-098-AC-30 | `frame_operation` target and a mismatching claim | `source-integrity` at decode, before the target refusal |
| 17 | FR-098-AC-31 | all claims hold; a failing claim | recompile under two labels; no `Recompile` refusal |

Tag each test with its AC ids and `TC-1324`.

## Expected Results

Each row's Expected column.
