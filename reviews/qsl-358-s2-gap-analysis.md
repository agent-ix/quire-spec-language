---
id: SR-956
title: "QSL-358 slice 2 gap analysis of PR 566: declaration move against the slice goal and FR-068-AC-6 / TC-390 / TC-175"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bcc946cd8154ceefeb1ee0f9cb476d359b726015; QSL-358 slice 2 items (move value::declaration, value::containment, the enum runtime half; cut 1 operations table; cut 2 registry-local limit; repoint callers; FR-068-AC-6 and TC-390 lists; ADR-011); PR body; FR-068-AC-6, TC-175, TC-390, FR-082-AC-6 (TC-220)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-390
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-175
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 2). PR: quire-spec-language#566 at bcc946cd.

Slice goal (phase 1 report, ticket comment): move declaration (registry,
construction, checked equality) with cuts 1 and 2, and repoint about 53 files, so
RT can delete `composite.rs` 760-1568, the `equality.rs` residue and
`containment.rs`. Each item checked:

- Registry, construction and checked equality are in `quire_semantic_value::declaration`.
  Done.
- `containment` is in `quire_semantic_value::containment`. Done.
- The enum runtime half (planned for slice 3) is moved here, because the checked
  equality's `Enum` schedule needs `compare_enum` and `EnumMemberIndex`. The PR
  body says so. Done, and it is in scope.
- Cut 1, `OperationTable`. Done, see SR-955.
- Cut 2, `EnvironmentFailure` with `stage_failure`. Done, see SR-955.
- Callers repointed, with no shim. Done: no `qsl_semantics::value::declaration`
  or `value::containment` path is left in Rust code.
- The FR-068-AC-6 and TC-390 lists. The code lists (`xtask` `LAYER_PERMITTED_MODULES`
  and the `BELOW_CORE` test constant) are updated. The spec text is not. See FND-001.

FR-082-AC-6 (TC-220): `check_and_evaluation_share_one_default_ancestor_ceiling`
and `divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation` still
assert the stage limit, now through `stage_failure`. The binding is correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR body says it updated "FR-068-AC-6's permitted list and TC-390's below-core roots". Only the code moved. FR-068-AC-6 still lists `value::containment` and `value::declaration` (the declared-type registry) as permitted `semantic_value` modules. It does not list `value::operation` or `value::environment_stage`, which `check` now imports (`assemble.rs`, `check.rs`). Its SV entry still reads "`stop`, `quantity` and `unit`'s runtime half". The AC says a module not on the list fails, so the spec read literally refuses imports the code allows. TC-175's description repeats the same stale list. TC-390 step 2's file list does not name `value/operation.rs` or `value/environment_stage.rs`, which the test's `BELOW_CORE` now scans. Update all three to match `LAYER_PERMITTED_MODULES` and `BELOW_CORE`. Slice 1's open FND-007 also rewrites TC-390 step 2, so do both together. | spec/functional/FR-068-split-expression-checking-into-check-stage.md:330-342; spec/test-cases/TC-175-move-stays-inside-m5-scope.md:36-41; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:44-47 |
| FND-002 | low | `OperationTable::resolve`'s `types.object_type(*declaring).is_some()` guard is the one semantic line the cut adds. The old walk got the same filter for free from iterating `object_types`. No test reaches it: every table in a passing test comes from a successful assembly, where every key is admitted. If the guard is deleted, every test still passes. Add one unit test: declare an operation under a key the environment does not admit, and expect `resolve` to return `Missing` for that key. Without the guard it returns `Declared`, because `conforms` is reflexive. | qsl-semantics/src/value/operation.rs:117-130 |

## Verdict

The slice delivers its goal. RT's three named residue blocks now have a home in
the no_std leaf, behaviour is unchanged, and the acceptance tests still bind.
FND-001 is a spec-to-code drift that the PR body claims is fixed, so fix it here.
FND-002 is a small missing test.
