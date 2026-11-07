---
id: SR-20261105101088
title: "code-review review of QSL-1796 shared equality admission"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1796; qsl-package/src/bare.rs, bare_slot.rs, checked_v2.rs, equality_file.rs, evaluation_view.rs, native_join_failure.rs, reader_protocol_projection.rs, emit/equality_reader.rs, emit/tests/equality_budget.rs; tests/it/bare_package.rs; Cargo.lock; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
---

## Summary

Ticket QSL-1796, PR 2474; reviewed frozen head 5383f67e07bb848619048175b8c77a6db3fbebac. Reviewed both shared-budget call signatures, all callers, native-admission error propagation, the authentic decision cache, lazy witness-stage refusal order, Rust ownership and no_std boundaries, and the IR-only lock update. The new sum preserves the original owner causes; the existing outer CatalogCoded stage error retains the stable catalog. No code defect found. Test completeness is recorded in the separate gap analysis; stale denial contract prose is recorded in integrity.

## Verdict

PASS — No defects found by this method in the changed scope; runtime checks and the final combined-tree gate remain pending.

## Assurance Context

No AssuranceProfile document was found under the repository specification; no enforced profile operation or active exception applies. The native test log remains queued on the shared build lock, so no runtime pass is inferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope

- Examined FR-531-AC-28: Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget` through IR's admitted equality reader after identity/type projection. Decisions and non-resource witness refusals are retained for later unmetered selection and projection. Successful reads consume native work and storage cumulatively; exact measured bounds admit, and one unit less in either resource refuses with the original typed IR equality denial and recorded first unpaid charge, never an unknown witness or internal fault. Reusing an admitted slot consumes no additional native allowance.
- Context only FR-531-AC-25: An inspection of the public signatures of `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`, `CheckedGraph::selected_exact_identities`, `ReadIdentityRows::selected`, `BarePackage::with_supply` and the evidence constructors finds each takes a `NativeJoinBudget` by mutable reference, finds no `NativeJoinBudget::new` and no `u64::MAX` allowance in a library path below an entry, and finds that `BarePackage` implements neither `Copy` nor `Clone`.
- TC-1331 existing owner artifact and the two new genuine owner-value test bindings were inspected.

## Evidence

Static source and caller inspection; normal dependency lock delta is only quire-contract-model b4a9 to a620, with branch=main unchanged. No workflow diff or gate narrowing. Focused check, Clippy and tests are pending in /tmp/qsl1796-focused.log; this reviewer ran no Cargo build or aggregate gate.

