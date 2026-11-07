---
id: SR-20261105101089
title: "gap-analysis review of QSL-1796 shared equality admission"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1796; qsl-package/src/bare.rs, bare_slot.rs, checked_v2.rs, equality_file.rs, evaluation_view.rs, native_join_failure.rs, reader_protocol_projection.rs, emit/equality_reader.rs, emit/tests/equality_budget.rs; tests/it/bare_package.rs; Cargo.lock; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
---

## Summary

Ticket QSL-1796, PR 2474; reviewed frozen head 5383f67e07bb848619048175b8c77a6db3fbebac. The two TC-first trace pairs bind the new AC to genuine owner decisions and measured low-level work/storage boundaries. However, the source-side public entry mapping through project_package/evaluation_view is not exercised under an equality denial, and bare.world() at equality_budget.rs:144 does not visit an equality slot. Existing bare_equality tests select slots at generous fixture limits but do not assert the new equality-budget boundaries, stage diagnostic bound/setting, or repeated selection/projection reuse. Add real source-view exhaustion value assertions including configured limit/setting/attempted charge, and an actual equality-bearing slot selected/projected repeatedly after exact-budget byte admission. Plan completion: not assessed. Optional semantic audit was not run; this is the mandatory code/test completeness check for the changed PR scope.

## Verdict

CONDITIONAL — A concrete review finding requires a fix in the same PR.

## Assurance Context

No AssuranceProfile document was found under the repository specification; no enforced profile operation or active exception applies. The native test log remains queued on the shared build lock, so no runtime pass is inferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new tests do not exercise source evaluation-view equality exhaustion or repeated slot selection/projection, so they do not verify the public shared-budget and retained-slot obligations of AC-28. | qsl-package/src/emit/tests/equality_budget.rs:17-186 |

## Scope

- Examined FR-531-AC-28: Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget` through IR's admitted equality reader after identity/type projection. Decisions and non-resource witness refusals are retained for later unmetered selection and projection. Successful reads consume native work and storage cumulatively; exact measured bounds admit, and one unit less in either resource refuses with the original typed IR equality denial and recorded first unpaid charge, never an unknown witness or internal fault. Reusing an admitted slot consumes no additional native allowance.
- Context only FR-531-AC-25: An inspection of the public signatures of `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`, `CheckedGraph::selected_exact_identities`, `ReadIdentityRows::selected`, `BarePackage::with_supply` and the evidence constructors finds each takes a `NativeJoinBudget` by mutable reference, finds no `NativeJoinBudget::new` and no `u64::MAX` allowance in a library path below an entry, and finds that `BarePackage` implements neither `Copy` nor `Clone`.
- TC-1331 existing owner artifact and the two new genuine owner-value test bindings were inspected.

## Evidence

Static source and caller inspection; normal dependency lock delta is only quire-contract-model b4a9 to a620, with branch=main unchanged. No workflow diff or gate narrowing. Focused check, Clippy and tests are pending in /tmp/qsl1796-focused.log; this reviewer ran no Cargo build or aggregate gate.

Plan completion: not assessed

Computed matrix: pending engine result at /tmp/qsl1796-matrix.json; no hand-written matrix was created.
