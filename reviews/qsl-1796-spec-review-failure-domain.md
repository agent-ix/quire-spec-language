---
id: SR-20261105101093
title: "failure-domain review of QSL-1796 shared equality admission"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; QSL-1796; qsl-package/src/bare.rs, bare_slot.rs, checked_v2.rs, equality_file.rs, evaluation_view.rs, native_join_failure.rs, reader_protocol_projection.rs, emit/equality_reader.rs, emit/tests/equality_budget.rs; tests/it/bare_package.rs; Cargo.lock; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
---

## Summary

Ticket QSL-1796, PR 2474; reviewed frozen head 5383f67e07bb848619048175b8c77a6db3fbebac. Checked finite work/storage exhaustion, first-unpaid preservation, original owner error provenance, missing/foreign/unknown witness staging, byte-only selected identities and nonrecursive reuse. The specification explicitly refuses resource denial rather than substituting an empty/unknown decision. No additional failure-domain omission found.

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


## Static disposition round 1

Reviewed fix commit 08f2b8385 and custody head a75573bf2. No new production behavior was introduced; stale denial docs were corrected and genuine source/slot tests were added. No new finding in this method. Runtime execution of the revised focused tests is pending in /tmp/qsl1796-focused-fix1.log; no runtime pass or aggregate gate is claimed.

## Runtime adoption round 2

**PASS at source head a3b2b327bebe17f2e7699be41b7fecfc44fb3b6b.** Both original medium findings remain fixed. Narrow adoption of 4149c3a25 moves only the six-line EqualityDecisionFailure declaration before the test module, without a lint allowance or semantic change. The later changes affect tests/it/bare_equality.rs only: they locate the equality inside the authored retry conjunction, bind its authentic owner row through admitted protocol origin/path/application identity, preserve the full CheckedEquality decision and original location, and evaluate both true and false outcomes during repeated exact-budget slot reuse. No new findings.

Actual receipts: /tmp/qsl1796-focused-fix2.log has strict scoped Clippy success and all three equality_budget tests passing, including the public source-view original-cause and exact stage-limit oracle. That run then recorded 14 bare_equality passes and one failure caused by the fixture's conjunction root being incorrectly assumed to be an equality. /tmp/qsl1796-focused-fix3.log records a test-only compile failure from using a nonexistent node getter on a QSV expression Location. The corrected authentic protocol-row join passes in /tmp/qsl1796-focused-fix4.log: all 15 bare_equality tests and all 26 bare_package tests pass, with zero failures/ignores, followed by successful all-target strict Clippy. The three package tests' production source is unchanged across the later test-only commits. Earlier fix1 compile passed before strict Clippy rejected the enum's position after the test module; no tests ran in that failed command sequence. These failed receipts remain preserved.

No aggregate or verification-only gate was run by this reviewer. Final combined-tree acceptance is the lead's one gate on the final head. The computed matrix remains unavailable as disclosed; exact trace bindings and real value assertions were inspected manually, without claiming full-repository matrix coverage.
