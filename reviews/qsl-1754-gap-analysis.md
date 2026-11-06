---
id: SR-20261105101045
title: "Gap analysis — QSL-1754/1755/1756 emitted structural rows"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-092-AC-26/27, TC-1021 step 15, TC-413 step 9, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/carried_quantity_paths.rs, qsl-package/src/emit/tests/variant_types.rs, qsl-semantics/src/check/lowering.rs, qsl-package/src/emit.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-092", type: reviews}
---
## Summary

The targeted trace chain for the two new FR-092 criteria is present: AC-26 is registered in `variant_types.rs`, and AC-27 is registered in `carried_quantity_paths.rs`; both modules are declared by `emit/tests.rs`. At candidate `1566ee38f17b6b923de448e69978f4a6fe95a492` based on `baa4a67095fd1fe19c77dc0478808bdf78d7b5d5`, source enum rows are supplied by the #2416 base merge, while this diff adds the model-owned enum row producer and the carried quantity path work. The full repository matrix did not complete in this reviewer lane, so this is a changed-scope manual gap check, not a repository-wide matrix verdict.

Plan completion: not assessed

## Verdict

CONDITIONAL. No targeted change-scope trace or implementation gap was found; full repository matrix and reverse-gap checks remain unavailable because `quire matrix --scope .` did not complete during this review.

## Coverage

- Manual changed-scope check: FR-092-AC-26 binds to two real traced tests in `variant_types.rs`; AC-27 binds to three tests in `carried_quantity_paths.rs`. Their declarations are in the test module tree. Assertions check model declaration/member joins, shape order/variants, emitted nodes, the seven-segment carried path, equal scale text repetition, successful strict-reader admission and refusal of a truncated path.
- Attempted computed matrix with `quire matrix --scope . --format tsv`; it remained running and was stopped without producing criterion records. Thus ignored-test status, full-repository untagged criteria, reverse gaps and coverage inflation were not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No targeted change-scope gaps found | FR-092-AC-26, FR-092-AC-27 |
