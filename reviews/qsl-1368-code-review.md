---
id: SR-1957018
title: "Code review — QSL-1368 FR-334 AC-4 vector conformance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; tests/it/unbounded_declarations.rs"
review_set: subset
---

## Summary

Reviewed the frozen PR head `d602c5d1d1b75ccccc28d521b55c94763c0df2b2` with code-review and Rust-review lanes. The test reads the current QSpec vectors in place, covers all 8 collection and 3 population cases, expands structural key references, and asserts one emitted QSL node per expected key; no findings.

## Verdict

**PASS** — no review findings. Gap-analysis is N/A because the diff adds only an integration conformance test, with no production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

- Diff scope: `tests/it/unbounded_declarations.rs` only; no CI workflows or gate changes.
- QSpec source checked at the current `agent-ix/quire-specification` default branch: version `quire.collection-population-type-vectors/v1`, 8 collection cases (`CT-01`–`CT-08`) and 3 population cases (`PT-01`–`PT-03`). The changed test reads `proposals/checked-package-v2/collection-population-type-vectors.json` under `QSPEC_DIR`; it copies no vector data.
- `expand_vector_key` recursively resolves `node_key_ref` segments. `assert_emitted_vector_key` compares structural NodeKeys and requires exactly one emitted node match for each case.
- The test checks nonzero case arrays, unbounded versus bounded collection keys, unbounded versus zero-bounded population keys, and distinct population maxima.
- Trace tags include `FR-334-AC-4`, `TC-844`, and `QSpec-FR-144-AC-9`, `QSpec-FR-144-AC-13`, `QSpec-FR-153-AC-9`, correctly qualifying cross-repository requirements.
- No `AssuranceProfile` exists under `spec/`; none applies. The repository's AGENTS trace-tag policy and Rust review guidance were checked.
- Tests were not rerun by the reviewer to avoid a duplicate build. The lead reports the focused vector test passed (1/1) and the full `unbounded_declarations` module passed (9/9) against the QSpec checkout.
