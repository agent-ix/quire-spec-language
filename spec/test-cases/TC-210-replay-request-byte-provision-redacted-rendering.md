---
id: TC-210
title: "The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-073
    type: verifies
---
# TC-210: The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes

## Description

Verify that neither `Debug` nor `Display` rendering of a constructed
replay request (FR-071) contains a byte-provision entry's full raw bytes,
even when that entry is a multi-kilobyte source file. A wrong
implementation this test would catch: a request type that stores each
byte-provision entry as `Vec<u8>` and derives `Debug` without a custom
implementation, so formatting the request for a log line dumps every
dependency source file's full bytes into the log. Scope: FR-073-AC-2.

## Test Procedure

1. Construct a replay request whose byte provision carries one entry of at
   least 4 KiB of distinctive, non-repeating bytes (for example a
   pseudo-random fill seeded so the test can check for its presence).
2. Render the request with `{:?}` (`Debug`) and capture the output.
3. Render the request with `{}` (`Display`), if the type implements
   `Display`, and capture the output.
4. Search both captured outputs for any contiguous run of the step 1
   entry's bytes longer than a short bounded excerpt, and compare each
   output's total length against the entry's raw byte length.
5. Look up the same byte-provision entry directly through the request's
   typed accessor (by its digest) and confirm it returns the full,
   unredacted 4 KiB of bytes constructed in step 1.

## Expected Results

- Neither captured output (steps 2, 3) contains a long contiguous run of
  the entry's raw bytes; each shows at most the entry's digest or a short
  bounded excerpt.
- Neither captured output's length scales with the entry's raw byte
  length.
- Step 5's direct accessor lookup returns the full, correct 4 KiB of
  bytes, unaffected by the rendering redaction in steps 2-3.
