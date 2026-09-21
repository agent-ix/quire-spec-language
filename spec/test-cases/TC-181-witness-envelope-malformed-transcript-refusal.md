---
id: TC-181
title: "The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-181: The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding

## Description

Verify that constructing a witness envelope from a cover-playback
transcript, an untrimmed transcript, or a transcript with zero or two
assertion blocks refuses, that none of these attempts leaves a
partially-built envelope reachable by the caller; and, separately, that
construction refuses a `RawSourceRef` or `package_id` digest whose domain
falls outside the closed FR-201 set, and refuses an envelope whose encoded
size exceeds the configured reader bound. A wrong implementation this test
would catch: a constructor that accepts a two-block transcript and silently
selects the first block, producing a "valid-looking" envelope that actually
witnesses the wrong assertion; a constructor that accepts any 64-hex-digit
string as a digest regardless of its declared domain, silently admitting a
digest computed under the wrong algorithm or domain; or a reader that
streams and returns a partially-decoded envelope once it exceeds the size
bound instead of refusing the whole construction. Scope: FR-070-AC-2,
FR-070-AC-6, FR-070-AC-7.

## Test Procedure

1. Construct a witness envelope from a cover-playback transcript (not an
   `assertion` check kind).
2. Construct a witness envelope from an untrimmed transcript (leading or
   trailing bytes outside the selected assertion block).
3. Construct a witness envelope from a transcript containing zero assertion
   blocks.
4. Construct a witness envelope from a transcript containing two assertion
   blocks.
5. For each of steps 1-4, attempt to read back any field of the
   would-be envelope (transcript, harness symbol, concrete values).
6. Construct a witness envelope whose `RawSourceRef` digest is well-formed
   64-lowercase-hex bytes but declares a digest domain outside the closed
   FR-201 set (e.g. an unregistered domain string).
7. Construct a witness envelope whose encoded size exceeds the reader's
   configured bound (pad the profile-selection or bounds members with
   repeated valid entries until the bound is crossed), and inspect whatever
   value the constructor returns.

## Expected Results

- Each of the four constructions in steps 1-4 refuses with a structured,
  typed cause naming the admission failure.
- No field of a refused construction is readable; there is no
  partially-built envelope object exposed to the caller in any of the four
  cases, and in particular the two-block case (step 4) does not silently
  select either block.
- Step 6 refuses with a structured, typed cause naming the digest-domain
  mismatch (`stale_dependency`/`digest-domain-mismatch`); no envelope is
  constructed with the out-of-domain digest stored.
- Step 7 refuses with a bound-exceeded cause; the constructor returns no
  envelope value at all, in particular no envelope holding a truncated
  prefix of the padded entries.
