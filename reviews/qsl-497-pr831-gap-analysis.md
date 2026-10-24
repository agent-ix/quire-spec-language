---
id: SR-9083
title: "Gap analysis of quire-spec-language PR #831: FR-085-AC-1..3 and FR-303-AC-4"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@05ff783f167720c91bd1af90646106561ce74638; PR #831: FR-085-AC-1 (TC-230), FR-085-AC-2 (TC-231), FR-085-AC-3 (TC-232), FR-303-AC-4 (TC-796 step 4) against qsl-semantics/tests/it/model_relationship.rs, qsl-replay/src/spine/lifecycle/tests.rs and qsl-replay/src/spine/lifecycle.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #831

## Summary

Ticket: QSL-497. Plan completion: not assessed. The oracle is this repo's own spec/ rows for FR-085, FR-303, TC-230, TC-231, TC-232 and TC-796.

- FR-085-AC-1 / TC-230: met. A dangling target end refuses once with `missing_declaration`/`missing-name`, naming the missing type and `Relationship.target`. A dangling source end and two dangling ends are also tested. The control with both ends declared completes.
- FR-085-AC-2 / TC-231: met. The test checks the role, the bounded multiplicity, an absent target role, the target's own multiplicity and the direction.
- FR-085-AC-3 / TC-232: half covered (FND-002).
- FR-303-AC-4 / TC-796 step 4: not met as claimed (FND-001).

## Verdict

Changes requested. FND-001 blocks. FND-002 is recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-303-AC-4 requires the spine `compile` to return the fault naming `n`, `d1` and `d2`, with no package. `fault_of` builds `InternalFault::new("S3", fault.invariant())`, which drops the node and both declarations, so the spine output names none of them. The new test hand-builds a `CompileRefusal::Check` and calls `failure_of` and `refusal_or_fault`. It never runs `compile` and does not assert the naming. Even so, it carries `#[trace("TC-796", "FR-303-AC-4")]` and the PR body claims AC-4. Either drop the AC-4 trace and the claim, or carry the conflict into the spine fault and assert `n`, `d1` and `d2` through a compile seam. | qsl-replay/src/spine/lifecycle.rs:271-277; qsl-replay/src/spine/lifecycle/tests.rs:1735-1780 |
| FND-002 | medium | FR-085-AC-3 has two halves: object-to-object ends, and endpoint-to-endpoint ends. TC-232 step 2 covers the second. The test traced to AC-3 covers only object-to-object. Recorded, not blocking. | qsl-semantics/tests/it/model_relationship.rs:168-186 |
