---
id: SR-411
title: "Gap analysis — Plan-009 protocol-role refusal loci"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-009-native-workflow; Task-026; FR-042-AC-8; TC-121; TM-003; issue #68"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
---

## Summary

The mandatory post-implementation gap pass audited Plan-009's compiler-export boundary and the
issue #68 requirement, test, and implementation slice. All 13 plan tasks are done, FR-042 is 10/10
backed, the regression carries the required `TC-121` and `FR-042-AC-8` tags, and no underspecified
behavior or stub was found in the changed path.

## Verdict

**CONDITIONAL** — the issue #68 slice has no remaining gap, while inherited repository-wide
untracked and unmatched tags keep the whole-corpus rollup at medium/low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Twenty inherited `NFR-007-M-*` tags resolve to no declared matrix row; none is in the issue #68 slice | src/package/encoding/tests.rs; tests/package_construction_cases/limits.rs |
| FND-002 | low | Six inherited unmatched tags remain outside this slice; three are the named ignored IT-004 private-packet lane | src/temporal/formula.rs; tests/composed_temporal_evaluation.rs; tests/composed_temporal_limits.rs; tests/fixture_audit.rs |

## Coverage

- Reconciliation: `quire coverage --scope . --json` with Quire 0.31.0, engine `ca7362d4`.
- Tasks done: 13 / 13 in Plan-009; no stale unchecked delivery item.
- Rows backed by a tagged test: 466 / 477. The only two `unbacked_rows` are also declared
  `no_symbol_rows` (`TC-010` Manual and `FR-017-AC-2` Inspection), so the workflow explicitly
  exempts them from unbacked-row findings; zero status lies were reported. Remaining unbacked total
  capacity is authored planned work, not a completed-row claim.
- Target group: FR-042 is 10 / 10 backed; the exact regression symbol carries `TC-121`,
  `FR-042-AC-5`, and `FR-042-AC-8`.
- Binding census: 736 Rust candidates, 732 tagged, 732 bound; 87 / 87 self-named symbols bound.
- Untraced behaviors / stubs in the issue #68 slice: 0. The private implementation is owned by
  FR-042 and the concrete behavior by AC-8; no placeholder, no-op, `todo!`, public orphan, or gate
  weakening appears.
- Semantic review: skipped because the optional intent-to-test-to-code pass was not requested. The
  mandatory reverse code-to-spec inspection was completed.

The full local no-default-features test run passed, including the 19-test
`native_protocol_emission` binary and the issue #68 regression. Three explicitly named IT-004
private-packet tests were ignored because their external fixture selector was absent; they are
pre-existing and unrelated to this change.
