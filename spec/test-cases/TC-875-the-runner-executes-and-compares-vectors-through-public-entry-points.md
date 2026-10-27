---
id: TC-875
title: "The conformance runner executes vectors through public entry points and compares typed results"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-336
    type: verifies
---
# TC-875: The conformance runner executes vectors through public entry points and compares typed results

## Description

Verify that the runner runs each vector through its subject's public entry point and compares the result with the expected disposition, including the code and cause of a refusal.

Scope: FR-350-AC-1, FR-350-AC-2, FR-350-AC-8, FR-350-AC-9, FR-350-AC-10.

## Test Procedure

Build a fixture corpus in the test's temporary directory in the QSpec FR-336 vector shape, holding:

1. a positive compile vector whose expected disposition is the checked package identity of a two-function unit; run vectors with one `source`, a `call` file exercising Boolean, Integer, Reference and named Population values and all ten accounting counters, plus a `population` FR-106 snapshot with named object and population fixtures; a `replay` vector with a `replay-request` file containing every FR-071 `ReplayRequestWire` field and byte-provision rows pointing by `bytes_path` to listed sibling `bytes` files; a `format` vector with its expected bytes;
2. a refusal vector whose source adds a `Meter` to a `Second` and which expects `ill_typed`/`unit-mismatch`; the same vector expecting `ill_typed`/`type-mismatch`; a refusal vector expecting `ill_typed`/`type-mismatch` over source QSL admits.

Run the runner over the corpus. Tag its tests
`#[trace("TC-875", "FR-350-AC-1", "FR-350-AC-2", "FR-350-AC-8", "FR-350-AC-9", "FR-350-AC-10", "FR-336-AC-7")]`.

## Expected Results

- Step 1: each positive outcome is `passed`. Every call field maps to the typed FR-100 `Call`, and each named Population resolves only to its actual FR-084/FR-089 admitted binding; native-run/1 remains without a Population argument wire. Replay's same-named request fields map to FR-071 `ReplayRequestWire`; every `bytes_path` is relative to the replay-request directory, matches exactly one listed sibling `bytes` path, and supplies raw bytes for its digest record. The typed request has no filesystem path and reaches FR-098. Each result equals the result of the corresponding existing public entry point, and replay agreement is represented by FR-072.
- Step 2: `passed`; `failed` holding expected `ill_typed`/`type-mismatch` and actual `ill_typed`/`unit-mismatch`; `failed` holding the expected refusal and the actual success.
