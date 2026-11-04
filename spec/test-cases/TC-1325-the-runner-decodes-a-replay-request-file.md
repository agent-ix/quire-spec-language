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

Verify FR-350-AC-11 to FR-350-AC-14 and FR-350-AC-16 to FR-350-AC-18 (AC-15 is retired) over fixture `replay-request` files. A wrong implementation this test catches: an adapter that executes no replay vector, one that keeps revision labels, one that reports a malformed corpus file as a QSL refusal.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 11 | a conforming `replay-request` file with its sibling byte files | decoded into `ReplayRequestWire` and run through `qsl_replay::replay`; the vector settles |
| 2 | 12 | a `sources` entry of the package and of a dependency | two members as written; no revision label; `bytes_path` bytes read raw |
| 3 | 13 | `object_environment` `{identity: {authority, identity}}`; the same with another member | the closed identity; the second is a `tool-failure` |
| 4 | 14 | an unknown member; a `sources` entry without `identity`; a wrong JSON kind | each `tool-failure` holding cause `corpus-decode` and the JSON pointer of the defect (Settlement) |
| 5 | 16 | a file with `package`; the same without it; with `package` the string `"orders"` | decoded to the package reference with no package-name member; each of the others a `tool-failure`, `corpus-decode`, `/package` |
| 6 | 17 | `bytes_path` `missing.bin`, `unlisted.bin`, `/etc/hostname`, `../x.bin`; a listed path | each of the four a `tool-failure`, `corpus-decode`, `/byte_provision/0/bytes_path`, no request; the listed path read as raw bytes |
| 7 | 18 | `stage_limits` with `depth`; with `s1.input_bytes` `"1024"` | a `tool-failure`, `corpus-decode`, `/stage_limits/depth`; the entry `("s1.input_bytes", 1024)` |

Tag each test with its AC ids and `TC-1325`.

## Expected Results

Each row's Expected column.
