---
id: TC-054
title: "Bind decoded JSON occurrences to exact original source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-017
    type: verifies
---
# TC-054: Bind decoded JSON occurrences to exact original source

## Description

Integration/property, priority P1. Verifies FR-017-AC-1 using the actual pinned
serde_json decoder and FormalSource, with independent expected byte positions.

## Test Procedure

Decode repeated declaration/reference names and repeated identical JSON objects
from one source. Reformat whitespace and reorder keys; include JSON-escaped
names and Unicode. Map the selected borrowed values to IR source spans and
compare each with its independent expected original byte range. Supply an
identical value from a separate allocation/source as an adverse control.
Decode invalid JSON and duplicate/unknown typed fields through the same decoder.

## Expected Results

Each successful typed value carries its own occurrence, including a declaration
whose name appeared earlier as a reference. Coordinates use actual source bytes,
not formatted name searches. Malformed data and foreign buffers return setup
errors; neither an IR model nor a native checker judgment is manufactured.
The bounded source and existing Serde recursion guard remain active. No build
fanout, fuzz campaign or complete model qualification is implied.
