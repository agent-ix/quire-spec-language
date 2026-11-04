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
identities. That is the evidence route, `qsl_eval::temporal::evidence`
(`EvidenceRequest`, `TriggerDelivery`, `SemanticTriggerIdentity`,
`evaluate_temporal_evidence` and `activate`; FR-516, FR-524, FR-526), not
`qsl_replay::spine::run_clause`, whose request has no trigger input. The observation owner supplies the expected opaque identities; QSL
does not authenticate that owner or construct QProtocol's FR-479 binding.

Catches two triggers aliased because their receipts or payloads match, a
duplicate delivery that creates a second instance, a same-identity delivery with
a different payload accepted, a one-byte change absorbed, a display name or
declaration position used as identity, an empty or missing identity accepted,
and an identity with a version-like prefix translated to another one.

Executable controls live in the module `checked_route_trigger` in
`qsl-eval/tests/it`. Identities are opaque byte strings; the test
compares bytes and never parses them.

## Test Procedure

Fixture **R**: a unit with clause `Refund` = `eventually[0,3] holds(settled)`
activated `on each` by an event trigger carrying capture `amount`. Identities:
`IDA` = bytes `74 72 67 2d 41` (`trg-A`), `IDB` = `74 72 67 2d 42` (`trg-B`),
`IDC` = `74 72 67 2d 43` (one byte differs from `IDB` at the last byte).

| # | Delivery | Expected |
| --- | --- | --- |
| 1 | `IDA` and `IDB`, equal receipt `r1`, equal payload and `amount` 5 | two instances with distinct identities; a value captured by one does not satisfy the other (FR-053-AC-7) |
| 2 | `IDA` delivered twice under receipts `r1` then `r2` | one instance; the second receipt is retained on it; captures unchanged |
| 3 | `IDB` replaced by `IDC`, all other fields held fixed | a third instance whose identity bytes equal `IDC` and differ from `IDB`'s; `IDB`'s instance and captures unchanged |
| 4 | substitution attempts: two deliveries with the same identity, one field varied at a time: receipt; capture value; static activation coordinate (the selected `control`); payload | for receipt, capture and coordinate, one instance with the first capture kept; for a different payload, the contradiction refusal (`invalid_runtime_input`/`invalid-value`, `contradicted_identity` equal to the identity bytes, FR-524) as `Err(CallFailure::Admission)` with an empty charge log, no second instance and no replaced capture. With distinct identities and every other field equal, two instances. An identity beginning `v2:` (`v2:trg-A`) is a distinct opaque byte string, a second instance beside `IDA`. No listed field substitutes for identity |
| 5 | the identity constructed from zero bytes; a request with deliveries and no `trigger`; a request whose `trigger` names no delivery (`IDC`); a request with no deliveries; a delivery supplying one capture fact more than the clause declares | the byte constructor refuses empty bytes (`EmptyIdentity`); then `Err(CallFailure::Admission)` with cause `trigger-required`; then `trigger-unknown`; then `Inactive` with no instance and no truth; then the surplus-capture refusal (`surplus_capture_identity` and `surplus_capture_index`, FR-524); each refusal with an empty charge log, no instance, no partial activation, no temporal evaluation. The identity is opaque non-empty bytes with one version, so no `version` or `encoding` member and no absent identity inside a delivery can occur and none is translated (FR-053-AC-9) |
| 6 | with a ledger holding `IDB`: row 5's `trigger-required` request and row 4's contradiction on `IDB`, then `IDA` | the ledger still holds only `IDB`'s instance and payload; `IDA` then activates normally (`Active`) and the ledger holds two instances; the refusals left no state |

## Expected Results

- Instance identity is exactly the trigger identity bytes (with the clause
  subject, FR-044-AC-8 for the origin case).
- Every invalid identity yields a typed outcome without an instance and without
  temporal evaluation.
- Test code carries the tags FR-053-AC-7 to AC-9 and TC-149, in `qsl-eval`.
