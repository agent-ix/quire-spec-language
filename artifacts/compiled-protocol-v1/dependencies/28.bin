---
id: FR-036
title: "Preserve original source locations through lexical recognition"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When recognizing native source tokens, the frontend SHALL preserve each token's original byte span under the lexical rules of the selected edition.

## Inputs

Original UTF-8 source bytes and exact identity, selected edition, and explicit source/token resource limits.

## Outputs

Recognized token values with original half-open byte spans, or a located typed lexical refusal/resource-incomplete result.

## Behavior

The candidate lexical rules are in the shared grammar. Keywords match complete identifiers and are edition-specific. Decoding JSON string escapes does not replace original spelling or offsets. No normalization, newline conversion or implicit semicolon insertion changes source interpretation. Comments remain in source identity and consume input budget even when they produce no semantic tokens. Invalid UTF-8, initial BOM, raw NUL, bare CR, invalid escapes, leading-zero integers and unclosed lexical forms refuse; no successful partial unit is published.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-036-AC-1 | LF and CRLF inputs with multibyte text produce spans into their own unchanged original bytes, including an error after a comment or string. | Test (TC-036) |
| FR-036-AC-2 | A comment marker inside a string stays text; a keyword prefix inside a longer identifier stays an identifier. | Test (TC-036) |
| FR-036-AC-3 | Invalid encoding, controls/escapes, initial BOM, raw NUL, bare CR and leading-zero integers refuse with the offending original span where bytes are available. | Test (TC-036) |
| FR-036-AC-4 | Adding a keyword in a new edition does not reclassify a historically admitted identifier under its old edition. | Test (TC-036) |
| FR-036-AC-5 | A source or token budget exceeded by one yields resource incompleteness rather than successful partial tokenization. | Test (TC-036) |

## Dependencies

- [Proposed shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Planned matrix](../composed-foundation/tests.md).

This contribution remains proposed until the integrated baseline review. It does
not change historical definition bytes or claim current compiler support.
