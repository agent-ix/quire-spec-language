---
id: TC-736
title: "A deep source and value replay to the proving run's verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-736: A deep source and value replay to the proving run's verdict

## Description

Verify that replay recompiles a 100,000-deep source and decodes a
100,000-deep assignment under the request's limits, and that a witness
transcript carries the same 100,000-long value in FR-070's witness value
text.

Scope: FR-263-AC-1, FR-070-AC-10. Moved from B5 (QSL-486) to QSL-640, since
a recursive list assignment needs a composite `WitnessValue`.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Compile a function whose body is a 100,000-term sum over a recursive list
   parameter under raised `s1.*` and `s3.*` settings, and record a
   counterexample whose value is a 100,000-long recursive list.
2. Replay it with those settings in the request's `stage_limits`.
3. Write the same value as a `Witness` transcript entry in FR-070's
   witness value text, with `replay.input_bytes` raised to fit the
   envelope. Decode it, then clone, compare, `Debug`-render and drop the
   decoded value. Build the envelope again with `replay.input_bytes` one byte
   below its size.

The list type is `record List { head: Int[0, 9]; tail?: List; }`. The
request's accounting limits fit the value's occurrence and node counts
(FR-098-AC-9).

Tag the tests `#[trace("TC-736", "FR-263-AC-1")]` and
`#[trace("TC-736", "FR-070-AC-10")]`.

## Expected Results

- Step 2: the replay reaches the proving run's verdict, and its recompiled
  `package_id` equals the request's.
- Step 3: the entry decodes, and every walk over the decoded value completes
  with no stack overflow. The envelope one byte over its bound refuses
  `BoundExceeded` naming `replay.input_bytes`.

