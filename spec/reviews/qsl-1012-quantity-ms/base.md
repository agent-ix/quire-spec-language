---
id: SR-16625
title: "QSL-1012 base review: Quantity<ms> probabilistic values"
type: SpecReview
analysis: base
scope: "QSL-1012 PR #1431 at <sha removed>; FR-120-AC-16 and FR-189-AC-15 implementation, tests, and linked Quantity contracts"
review_set: all
---

## Summary

The change implements the merged Quantity contract through simulation identity and probabilistic measurement. It represents magnitude as an exact rational, carries the declared unit identity, resolves unit typeRefs from the compiling unit, and refuses mismatched support values before sampling. Independent code review found one missing refusal identity/path detail; the second fix added the member URI and typed path, and the reviewer confirmed it resolved the finding.

The scoped matrix reports FR-120-AC-16 and FR-189-AC-15 as `tagged`. Scoped validation of FR-120 and FR-189 exits 0 with only pre-existing module-catalog advisories.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved base-review issue in the selected Quantity encoding path. | FR-120-AC-16; FR-189-AC-15 |

## Verification

- `quire validate --scope . spec/functional/FR-120-*.md spec/functional/FR-189-*.md`: exit 0; changed requirement documents validate.
- `quire matrix --scope /tmp/qsl-1012-scope --format json`, where the temporary scope contains only FR-120, FR-189, TC-1195, TC-624 and the touched test source files; filtered rows show FR-120-AC-16 and FR-189-AC-15 `tagged` with the expected test binders.
- `cargo fmt --all --check`, Clippy for qsl-analyze/qsl-semantics/qsl-eval/qsl-replay, the affected crate suites, and the focused Quantity tests pass on the reviewed implementation.
- No CI workflow or language-boundary gate was narrowed.
