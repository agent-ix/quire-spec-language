---
id: SR-7812
title: "Code review — QSL-707 checked v2 union fixture identities"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@bbfc7c9b13d2253194aa97574e36513072fdeb0d; Cargo.lock, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests.rs"
review_set: subset
---

## Summary

Reviewed QSL-707 PR #738 at frozen head `bbfc7c9b13d2253194aa97574e36513072fdeb0d` against `d8ba421360419d942fe0108cd35a6cac3dd56c8f`. The IR-739 lock pin, test-side union application-key derivation, fixture rekeying, and first-occurrence limit oracle are consistent with the exercised contracts; no code or Rust findings were identified.

## Verdict

**PASS** — changed test helpers derive keys from the checked-v2 wire preimage, keep projection/package identities synchronized, and assert the occurrence refusal at the first charged source-map entry.

## Coverage

- Rust review folded into this code-review artifact. Rust build, lint, and test gates were not run because the dispatch explicitly prohibited builds and tests.
- Manually examined bindings: TC-421 → FR-095-AC-3; TC-253 → FR-087-AC-3; TC-254 → FR-087-AC-4; TC-429 → FR-096-AC-10; TC-720 → FR-255-AC-1; TC-721 → FR-255-AC-4; TC-739 → FR-264-AC-4.
- QSL-707 is the validated ticket key. The change contains no production source or specification edits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

No real findings existed in this artifact to disposition (the Findings row above is the required clean-review placeholder). Rechecked at `e2341608b249e3bbdb138dedab9ce570219957dd`: the aggregate union producer/I2 admission test and frame fixture correction introduce no code or Rust review finding. No builds or tests were run.
