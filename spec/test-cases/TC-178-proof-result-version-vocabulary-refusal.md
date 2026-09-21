---
id: TC-178
title: "The proof-result reader refuses an unknown version, vocabulary, or oversized envelope before consumption"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-178: The proof-result reader refuses an unknown version, vocabulary, or oversized envelope before consumption

## Description

Verify that the proof-result reader refuses an envelope whose
`contract_version` is not exactly `quire.backend-provider/v1`, or whose
`capability_vocabulary` is not exactly `quire.capability-kind/v1`, with a
structured cause, and that it reads no `results`, `dispositions`,
`counterexamples` or `accounting` member first; and separately, that it
refuses an envelope whose encoded size exceeds the configured reader bound
without returning a truncated or partially-populated envelope. A wrong
implementation this test would catch: a reader that falls through to a
default/legacy parse path on an unrecognized version and returns a
best-effort envelope instead of refusing; one that reads and discards the
mismatched-vocabulary `results` member before checking the vocabulary
field, leaving a partially consumed envelope observable through a side
channel (e.g. a populated cache); or a reader that streams and returns
whatever it decoded up to the configured bound instead of refusing the
whole envelope, silently truncating `results`/`dispositions` rather than
reporting the bound violation. Scope: FR-069-AC-2, FR-069-AC-4.

## Test Procedure

1. Construct a well-formed FR-331 envelope with `contract_version` set to an
   unrecognized string (e.g. `quire.backend-provider/v2-draft`) and read it.
2. Construct a well-formed FR-331 envelope with `capability_vocabulary` set
   to an unrecognized string and read it.
3. Construct a well-formed FR-331 envelope with `capability_vocabulary`
   absent entirely and read it.
4. Instrument or otherwise observe whether any `results`, `dispositions`,
   `counterexamples` or `accounting` member was consumed before each
   refusal in steps 1-3.
5. Construct a well-formed FR-331 envelope whose encoded size exceeds the
   reader's configured bound by padding `dispositions` with repeated valid
   entries until the bound is crossed, and read it. Inspect whatever value
   the reader returns for a populated `results` or `dispositions` field.

## Expected Results

- Each of the three malformed envelopes from steps 1-3 refuses with a
  structured, typed cause (`unknown_wire`/`unsupported-wire` for the
  version case, `invalid_capability`/`unsupported-version` for the
  vocabulary cases).
- No `results`, `dispositions`, `counterexamples` or `accounting` member is
  read or otherwise observably consumed before any of the three refusals.
- The oversized envelope in step 5 refuses with a bound-exceeded cause, and
  the reader returns no envelope value at all — in particular, no envelope
  whose `results` or `dispositions` field holds a truncated prefix of the
  padded entries.
