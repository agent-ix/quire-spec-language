---
id: SR-163
title: "Code and Rust review of public rule-model source"
type: SpecReview
analysis: code-review
scope: "src/model_source.rs; src/located_json.rs; shared object adapter; fixture callers; tests/model_source.rs"
review_set: subset
---
## Summary

Author PR-readiness re-review of fbdf687 using the actual code-review, rust-review
and portable rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**CONDITIONAL** — independent review findings 1–6 and 8 are addressed;
the pinned IR resource-classification follow-up remains below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved 1/8: the exact-source borrow contract is documented, owned matching RawValue is refused, and public reads require an explicit ceiling. | src/located_json.rs; tests/located_json.rs |
| FND-002 | low | Resolved 2/3: duplicate/unknown scalars and each resource category carry typed context; AC-3/4 and independent adverse controls own the behavior. | src/model_source.rs; tests/model_source.rs; FR-025 |
| FND-003 | low | Resolved 4/5/6: wire/decode/lower modules are separate, one budget helper charges before group decoding, and drafts/errors retain effective limits. | src/model_source/; src/model_source.rs |
| FND-004 | medium | The pinned IR has no resource-classification helper. The existing two-code mapping remains coupled to that revision; request a producer-owned predicate before expanding it. | src/model_source.rs; quire-contract-ir 690bde7; issue #27 |

The full suite at fbdf687 passed after the correction. The focused model/occurrence
suite passed 11 tests, including all seven entry categories and unchanged frozen
model bytes. Source-only public API changes add no I/O, unsafe code, dependency
or workflow change. The earlier baseline checks below remain historical evidence.
Strict all-targets/all-features Clippy, formatting, cached minimal build and
warnings-denied rustdoc also passed on the corrected implementation.

## Checks

Reviewed source-before-decode limits, checked entry arithmetic, bounded type
lowering, original raw occurrence mapping, closed record/unit decoding and
typed failure retention. No production panic, unsafe block, unchecked numeric
cast, I/O or concurrent state was introduced. Serde owns JSON grammar; existing
IR constructors and native admission own validity. The fixture reexport is a
deliberate compatibility shim to the real library implementation.

The original five tests execute public APIs, with actual trace attributes and a
preexisting fixed model artifact oracle. The complete suite passed 283 tests
and three compile-fail doctests; four existing assurance tests remain ignored.
Formatting, strict all-targets/all-features Clippy, warnings-denied rustdoc,
cached minimal build and both fixture audits passed. No hosted CI ran;
workflow_dispatch remains the only trigger. No dependency or license change.
This is author review, not independent assurance.
