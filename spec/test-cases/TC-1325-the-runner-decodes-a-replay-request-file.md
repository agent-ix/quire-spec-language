---
id: TC-1325
title: "The runner decodes a replay-request file into the replay wire"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-336
    type: verifies
---
# TC-1325: The runner decodes a replay-request file into the replay wire

## Description

Verify FR-350-AC-11 to FR-350-AC-15 over fixture `replay-request` files. A wrong implementation this test catches: an adapter that executes no replay vector, one that keeps revision labels, one that reports a malformed corpus file as a QSL refusal, or one that decides a source-integrity result itself.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 11 | a conforming `replay-request` file with its sibling byte files | decoded into `ReplayRequestWire` and run through `qsl_replay::replay`; the vector settles |
| 2 | 12 | a `sources` entry of the package and of a dependency | three members as written; no revision label; `bytes_path` bytes read raw |
| 3 | 13 | `object_environment` `{identity: {authority, identity}}`; the same with `source_integrity` | the closed identity; the second is a `tool-failure` |
| 4 | 14 | an unknown member; a `sources` entry without `source_integrity`; a wrong JSON kind | each `tool-failure` holding cause `corpus-decode` and the JSON pointer of the defect (Settlement) |
| 5 | 15 | a schema-valid request whose claim mismatches its bytes, expecting `source-integrity` at `/package/sources/0`; the same request expecting a success | `passed` by `qsl_replay::replay`'s refusal, the adapter not pre-refusing; `failed` with the expected and actual results |

Tag each test with its AC ids and `TC-1325`.

## Expected Results

Each row's Expected column.
