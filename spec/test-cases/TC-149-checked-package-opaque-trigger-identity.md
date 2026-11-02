---
id: TC-149
title: "Preserve opaque trigger identity on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-053, type: verifies }
---
# TC-149: Preserve opaque trigger identity on the checked-package route

## Description

Verify FR-053-AC-7 to FR-053-AC-9 (with FR-044-AC-1, AC-2 and AC-8's identity
rules) where the Layer-4 checked-package route accepts semantic-trigger
identities. The observation owner supplies the expected opaque identities; QSL
does not authenticate that owner or construct QProtocol's FR-479 binding.

Catches two triggers aliased because their timestamps or payloads match, a
duplicate delivery that creates a second instance, a one-byte change absorbed,
a display name or declaration position used as identity, and an invalid or
other-version identity translated to the selected version.

Executable controls live in a new module `checked_route_trigger` beside the spine
clause tests in `qsl-replay`. Identities are opaque byte strings; the test
compares bytes and never parses them.

## Test Procedure

Fixture **R**: a unit with clause `Refund` = `eventually[0,3] holds(settled)`
activated `on each` by an event trigger carrying capture `amount`. Identities:
`IDA` = bytes `74 72 67 2d 41` (`trg-A`), `IDB` = `74 72 67 2d 42` (`trg-B`),
`IDC` = `74 72 67 2d 43` (one byte differs from `IDB` at the last byte).

| # | Delivery | Expected |
| --- | --- | --- |
| 1 | `IDA` and `IDB`, equal timestamp 10, equal receipt `r1`, equal payload and `amount` 5 | two instances with distinct identities; a value captured by one does not satisfy the other (FR-053-AC-7) |
| 2 | `IDA` delivered twice under receipts `r1` then `r2` | one instance; the second receipt is retained on it; captures unchanged |
| 3 | `IDB` replaced by `IDC`, all other fields held fixed | a different instance identity, or a typed refusal; never the instance of `IDB` (FR-053-AC-8) |
| 4 | substitution attempts: two triggers with the same identity but different timestamps, receipts, display text, payloads, captures, static activation coordinates and declaration positions, one field varied at a time | one instance each time (same identity); and with distinct identities and every other field equal, two instances. No listed field substitutes for identity |
| 5 | if the checked-package reader carries a versioned identity encoding (inspect the reader and record the encoded form): missing, empty, malformed (truncated length prefix), stale (superseded version), mismatched (identity for another clause) and other-version identities | each refused or reported incomplete with a typed non-truth record; no instance, no partial activation, no evaluation begins; the other-version value is not translated (FR-053-AC-9). If the reader has no encoded form, the step records `not applicable: no encoded identity on the route` |
| 6 | row 5's refused deliveries followed by `IDA` | `IDA` activates normally; the refusals left no state |

## Expected Results

- Instance identity is exactly the trigger identity bytes (with the clause
  subject, FR-044-AC-8 for the origin case).
- Every invalid identity yields a typed outcome without an instance and without
  temporal evaluation.
- Test code carries the tags FR-053-AC-7 to AC-9 and TC-149.
