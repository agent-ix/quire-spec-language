---
id: TC-108
title: "Extract a selected clause body through the Quire consumer"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: verifies
---
## Description

Integration, P1: run the real Quire Rust extractor (`quire-rs`) through the
`qsl-source` consumer over immutable Markdown, and compare the exact
original and body source.

Scope: FR-030-AC-3 to FR-030-AC-5.

## Test Procedure

1. Run with a stale original digest; with a foreign context, once for each of
   source identity, path and authored package; with a missing and with a
   duplicate selected clause; and with unavailable extraction.
2. Extract an indented CRLF clause and an LF clause with Unicode
   surroundings, and map body EOF and trailing layout to original
   coordinates. Request an invalid body identity and an out-of-range mapping.
3. Run with the original byte limit and then the line limit one below the
   document's size; retry each with its default. Run the suite in a build
   with the `quire-extraction` feature and in one without it.

Tag the tests `#[trace("TC-108", "FR-030-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: each refuses without a body; each foreign context field returns its
  own typed preflight cause with actual and expected values, before
  extraction.
- Step 2: the body bytes and their original coordinates are exact; the
  invalid identity and the out-of-range request refuse.
- Step 3: each limit returns incomplete naming that limit; each retry
  succeeds; the build without the feature makes no extraction call.
