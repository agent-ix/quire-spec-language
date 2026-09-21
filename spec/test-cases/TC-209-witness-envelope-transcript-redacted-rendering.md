---
id: TC-209
title: "The witness envelope's Debug and Display rendering never reproduces the full transcript"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-073
    type: verifies
---
# TC-209: The witness envelope's Debug and Display rendering never reproduces the full transcript

## Description

Verify that neither `Debug` nor `Display` rendering of a constructed
witness envelope (FR-070) contains the full transcript text, and that
whatever the rendering does show is a bounded descriptor rather than an
excerpt long enough to reconstruct the transcript. A wrong implementation
this test would catch: a `#[derive(Debug)]` witness envelope whose one
field is the private `transcript: String`, which by default renders the
field's full contents in `{:?}` output — exactly the shape that would
leak an entire Kani assertion-playback transcript into any log line or
panic message that formats the envelope. Scope: FR-073-AC-1.

## Test Procedure

1. Construct a witness envelope from a transcript long enough (well above
   the reader's configured size bound for a rendering excerpt, if the
   implementation uses one) to make an accidental full inclusion visually
   obvious, and containing a distinctive marker string not otherwise
   likely to appear in a bounded descriptor (for example a UUID literal
   embedded mid-transcript).
2. Render the envelope with `{:?}` (`Debug`) and capture the output.
3. Render the envelope with `{}` (`Display`), if the type implements
   `Display`, and capture the output.
4. Search both captured outputs for the step 1 marker string and for the
   transcript's full length in bytes.
5. Call the envelope's typed accessors (`harness_symbol()`,
   `concrete_values()`, `decode()`) directly and confirm each still
   returns its full, correct, unredacted result.

## Expected Results

- Neither captured output (steps 2, 3) contains the step 1 marker string.
- Neither captured output is as long as the full transcript; each is
  bounded to a fixed size or shows only a descriptor (digest, byte length,
  or a short fixed-length excerpt).
- Step 5's typed accessors return their full, correct results, unaffected
  by the rendering redaction in steps 2-3.
