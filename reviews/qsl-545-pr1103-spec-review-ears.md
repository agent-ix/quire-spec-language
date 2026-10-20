---
id: SR-13581
title: 'Spec review of PR #1103: history-field spec follow-ups from #1093, ears'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@f481e5899; git diff refs/remotes/origin/main...f481e5899 (PR #1103, branch std-545-history-spec): spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md; compared with origin/qsl-545-history 498b438cf (qsl-semantics check/{check,state_model,mod,refinement,refinement/history}.rs, qsl-replay refinement/{history,step,map}.rs, tests/support/refinement_aux.rs), merged FR-135/140/141/142/363, TC-540, QSpec main 44afcf4 FR-375/FR-378, and SR-13460/SR-13461; ticket QSL-545'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-138
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
---

## Summary

Ticket: QSL-545. PR: quire-spec-language#1103, head f481e5899, scoped to `git diff refs/remotes/origin/main...f481e5899` (one commit on main bdf90d48d; three files, +21/-8). Spec-only follow-ups from the #1093 code review (SR-13460, SR-13461). Code read on origin/qsl-545-history, which is still 498b438cf (the coder's fix round is not pushed yet). Verified against the code: (1) the fixture INV is `(1 mod x) + 1` (tests/support/refinement_aux.rs:585): 1 mod 1 = 0, +1 = 1, a member of Int[0, 1]; 1 mod 0 is undefined with `division-by-zero` (asserted at qsl-replay/tests/it/refinement_history.rs:358, same cause FR-141-AC-7 uses for `1 mod i`); integer `/` with no Rational expected type is ill_typed (FR-100-AC-5). TC-554 and FR-138-AC-5 agree; FR-141-AC-6 and FR-142-AC-8 cite the update by name. (2) `advance` evaluates updates over a post observation built from `carried` values, the pre-state history (qsl-replay/src/refinement/history.rs:203-249), so an outside-`pre` read gives the pre value; S3 admits it. (3) `Scope::history_names` is every history row name of the unit's refinements (qsl-semantics/src/check/refinement.rs:618, check/mod.rs:969), and missing-name fires only when the receiver's type has no such field and no navigable relationship end (check/state_model.rs:286-290); otherwise type-mismatch (FR-363-AC-2). (4) the code's `UnsupportedFeature` holds only `ProtocolAbstractSide`; `history-row` is gone. (5) `HistoryValues` is a BTreeMap on (ObjectKey{population, key}, field) and `key` encodes population, key, field, typed value in that order (history.rs:33-80, map.rs:33-40). (6) FR-141 already gives `NoAbstractMatch` for any row whose post candidate set is empty. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-135, FR-138 or TC-554; tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13580), spec-review/ears (SR-13581), spec-review/integrity (SR-13582); code-review and gap-analysis not run (no code or test file changed). This review: EARS conformance of the new and changed statements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-138-S3-missing-name | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-S3-outside-pre | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-L5-key-order | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-L5-candidate-rule | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-135-support-set | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-refusal-span | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-AC-5 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-138-AC-5 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | One bullet carries two SHALLs ('SHALL read the value the pre-state holds' and 'S3 SHALL NOT refuse it') with two subjects and two stages. Fix: split into an S3 statement ('S3 SHALL admit an update that reads a history field outside `pre(...)`') and a Layer 5 statement for the value it reads (see SR-13580 FND-006). | spec/functional/FR-138-check-and-compute-history-fields.md:90 |

## Verdict

One low finding. The other new statements are ubiquitous SHALL statements with a subject; the new missing-name sentences define the names of the existing SHALL, which is acceptable. The ACs are direct assertions. The quire grammar lint reports nothing in FR-135, FR-138 or TC-554.
