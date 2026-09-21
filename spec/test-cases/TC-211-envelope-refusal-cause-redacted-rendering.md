---
id: TC-211
title: "A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-073
    type: verifies
---
# TC-211: A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable

## Description

Verify that a refusal cause produced during construction of the
proof-result envelope (FR-069), the witness envelope (FR-070), the replay
request (FR-071), or the replay result (FR-072) never renders unredacted
transcript, byte-provision, or full concrete-value content through its own
`Debug`/`Display` output, and that this redaction is a rendering-only
restriction — the same content stays fully reachable through its typed
accessor called directly. A wrong implementation this test would catch: a
`stale_dependency`/`byte-digest-mismatch` cause that embeds the offending
entry's full raw bytes as a field (rather than its digest) so that logging
the cause reproduces the bytes; or, in the opposite direction, an
implementation that "fixes" this by making the offending content
unreachable even through the typed accessor, which would break FR-071's
and FR-070's own round-trip and refusal-inspection acceptance criteria.
Scope: FR-073-AC-3.

## Test Procedure

1. Produce a `stale_dependency`/`byte-digest-mismatch` refusal from the
   replay request (FR-071, TC-186 step 3's construction) whose mismatched
   entry carries several kilobytes of distinctive bytes.
2. Produce a malformed-transcript refusal from the witness envelope
   (FR-070, TC-181's construction) whose rejected transcript carries a
   distinctive marker string.
3. Render each refusal cause from steps 1-2 with `{:?}` (`Debug`) and, if
   implemented, `{}` (`Display`), and capture the output.
4. Search each captured output for the step 1 entry's raw bytes and the
   step 2 transcript's marker string.
5. For each refusal, separately obtain the referenced content through its
   own typed accessor (the byte-provision entry by digest for step 1; the
   pre-refusal transcript bytes the test itself constructed for step 2)
   and confirm it is the full, correct, unredacted content.

## Expected Results

- No captured output (step 3) contains the step 1 entry's raw bytes or
  the step 2 transcript's marker string; each cause's rendering shows at
  most a digest, locus, or other bounded descriptor.
- Step 5's typed-accessor lookups each return the full, correct,
  unredacted content, confirming the redaction in steps 3-4 applies to
  rendering only and blocks no typed access path.
