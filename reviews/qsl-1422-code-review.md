---
id: SR-1988008
title: "Code review — QSL-1422 nested owner_model refusal pointer"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/owner_model.rs"
review_set: subset
---

## Summary

Reviewed the single test expectation change for the external `owner_model` package key case against FR-383-AC-26 and TC-1063. The changed pointer matches the reader's nested `node` field error and preserves the existing malformed-wire refusal.

## Verdict

**PASS** — the exact focused test passes, the diff changes only the expected pointer, and the repository's format and diff checks pass.

## Evidence

- `cargo test --locked -p qsl-package each_defect_of_the_edge_is_refused_where_the_reader_reaches_it --lib` — 1 passed, 477 filtered out.
- `cargo fmt --all -- --check` — passed.
- `git diff --check` — passed.
- The branch diff changes only `qsl-package/src/emit/tests/owner_model.rs:286`, from the parent `owner_model` pointer to `owner_model/node` for the external package-key mutation.

## Scope

- `FR-383-AC-26`: owner-model edges are exact model declaration nodes and each reader refusal is tested against the expected locus.
- `TC-1063`, row 11: the external owner-model edge mutation is refused as `malformed_wire`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
