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
restriction on the *cause* type — the offending content itself is neither
destroyed nor made unreachable in general, only kept out of the cause's
default rendering. A wrong implementation this test would catch: a
`stale_dependency`/`byte-digest-mismatch` cause that embeds the offending
entry's full raw bytes as a field (rather than its digest) so that logging
the cause reproduces the bytes; or, in the opposite direction, an
implementation that "fixes" this by making the content generally
unconstructible or unreadable everywhere, which would break FR-071's and
FR-070's own round-trip acceptance criteria for a validly-formed value
carrying the same content. Each half of this test therefore exercises a
typed accessor on a *separately, validly constructed* value carrying the
same content the refusal named — never the refused construction's own
local variable, which a refused call returns no object to read at all
(TC-181's own Expected Results: no field of a refused construction is
readable). Scope: FR-073-AC-3.

## Test Procedure

1. Produce a `stale_dependency`/`byte-digest-mismatch` refusal from the
   replay request (FR-071, TC-186 step 3's construction) whose mismatched
   entry carries several kilobytes of distinctive bytes ("entry X").
2. Produce a malformed-transcript refusal from the witness envelope
   (FR-070, TC-181's construction) from an untrimmed transcript carrying a
   distinctive marker string ("transcript Y"), where the same marker
   string sits inside one well-formed assertion block once the leading and
   trailing untrimmed bytes are removed.
3. Render each refusal cause from steps 1-2 with `{:?}` (`Debug`) and, if
   implemented, `{}` (`Display`), and capture the output.
4. Search each captured output for entry X's raw bytes and transcript Y's
   marker string.
5. Construct a second, validly-admitted replay request whose byte
   provision carries entry X's exact bytes under their own correct,
   matching digest (no mismatch), and look up that entry through the
   request's typed byte-provision accessor by digest. Separately,
   construct a second, validly-admitted witness envelope from transcript
   Y trimmed to its one well-formed assertion block, and call its typed
   accessor (`decode()` or `concrete_values()`) to retrieve the marker
   string's value. Confirm each accessor returns the full, correct,
   unredacted content.

## Expected Results

- No captured output (step 3) contains entry X's raw bytes or transcript
  Y's marker string; each cause's rendering shows at most a digest, locus,
  or other bounded descriptor.
- Step 5's two typed-accessor lookups, each against a separately and
  validly constructed value (never the refused construction's own
  variable), each return the full, correct, unredacted content —
  confirming the redaction in steps 3-4 applies to the refusal cause's
  rendering only, and neither destroys the content nor blocks every typed
  access path to it.
