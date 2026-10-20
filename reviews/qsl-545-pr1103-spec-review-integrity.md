---
id: SR-13582
title: 'Spec review of PR #1103: history-field spec follow-ups from #1093, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@f481e5899; git diff refs/remotes/origin/main...f481e5899 (PR #1103, branch std-545-history-spec): spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md; compared with origin/qsl-545-history 498b438cf (qsl-semantics check/{check,state_model,mod,refinement,refinement/history}.rs, qsl-replay refinement/{history,step,map}.rs, tests/support/refinement_aux.rs), merged FR-135/140/141/142/363, TC-540, QSpec main 44afcf4 FR-375/FR-378, and SR-13460/SR-13461; ticket QSL-545'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-138
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-540
  type: reviews
---

## Summary

Ticket: QSL-545. PR: quire-spec-language#1103, head f481e5899, scoped to `git diff refs/remotes/origin/main...f481e5899` (one commit on main bdf90d48d; three files, +21/-8). Spec-only follow-ups from the #1093 code review (SR-13460, SR-13461). Code read on origin/qsl-545-history, which is still 498b438cf (the coder's fix round is not pushed yet). Verified against the code: (1) the fixture INV is `(1 mod x) + 1` (tests/support/refinement_aux.rs:585): 1 mod 1 = 0, +1 = 1, a member of Int[0, 1]; 1 mod 0 is undefined with `division-by-zero` (asserted at qsl-replay/tests/it/refinement_history.rs:358, same cause FR-141-AC-7 uses for `1 mod i`); integer `/` with no Rational expected type is ill_typed (FR-100-AC-5). TC-554 and FR-138-AC-5 agree; FR-141-AC-6 and FR-142-AC-8 cite the update by name. (2) `advance` evaluates updates over a post observation built from `carried` values, the pre-state history (qsl-replay/src/refinement/history.rs:203-249), so an outside-`pre` read gives the pre value; S3 admits it. (3) `Scope::history_names` is every history row name of the unit's refinements (qsl-semantics/src/check/refinement.rs:618, check/mod.rs:969), and missing-name fires only when the receiver's type has no such field and no navigable relationship end (check/state_model.rs:286-290); otherwise type-mismatch (FR-363-AC-2). (4) the code's `UnsupportedFeature` holds only `ProtocolAbstractSide`; `history-row` is gone. (5) `HistoryValues` is a BTreeMap on (ObjectKey{population, key}, field) and `key` encodes population, key, field, typed value in that order (history.rs:33-80, map.rs:33-40). (6) FR-141 already gives `NoAbstractMatch` for any row whose post candidate set is empty. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-135, FR-138 or TC-554; tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13580), spec-review/ears (SR-13581), spec-review/integrity (SR-13582); code-review and gap-analysis not run (no code or test file changed). This review: consistency of the changed text with the artifacts that cite it, and AC coverage of the new rules.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TC-540-step-5 | spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-138-references-support-set | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-2 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-5 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| TC-554-fixture | spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md | examined |
| FR-138-L5-key-order | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-135-AC-5 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-refusal-span | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-540, which verifies FR-135-AC-5, still checks `RegisterHistory` with one and two `history` rows and expects each to refuse `not-yet-implemented` naming `history-row`. FR-135-AC-5 no longer has that half, and FR-138-AC-1 says `RegisterHistory` checks, so the TC contradicts both. Fix: drop `RegisterHistory` from step 5 and its expected result; keep `CasTwice`, `CasRefinesCounter`, and add the internal-list case FR-135-AC-5 names. | spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md:34 |
| FND-002 | medium | FR-138's References still say a build whose support set omits `history-row` refuses each row `not-yet-implemented` (FR-135 'Refinement features the build does not check'), which FR-135 no longer lists. Fix: remove the bullet, or reword it to match FR-135 once SR-13580 FND-004 is settled. | spec/functional/FR-138-check-and-compute-history-fields.md:158 |
| FND-003 | medium | The rules the PR adds have no AC: FR-138-AC-2's missing-name case is an invariant of `Impl::Register` reading `self.last` (same type), so neither an unrelated type's undeclared field sharing a history name (missing-name) nor an unknown non-history field (type-mismatch) is pinned; nothing exercises an outside-`pre` read; and the key order has none (SR-13461 FND-006: dropping the history part of the key survives). Fix: extend FR-138-AC-2 with the two field cases, add an AC for an update reading `self.last` outside `pre` (for example `on write: self.last` gives the pre value), and an AC for the key order over two objects and two fields. | spec/functional/FR-138-check-and-compute-history-fields.md:143 |
| FND-004 | low | The bullet records a disagreement with its parent ('QSpec FR-378 words this order as "history-row order"; this requirement's order is the one the key uses') inside the requirement, with no reference to where the QSpec change is tracked. Until QSpec changes, FR-138 overrides QSpec FR-378 on a key a certificate (FR-149) compares. Fix: keep the order in the SHALL, and move the note to References with the tracking ticket. | spec/functional/FR-138-check-and-compute-history-fields.md:116 |
| FND-005 | low | 'with no other diagnostic for the row' was written for the history half; a protocol abstract side is a declaration header, not a row. Fix: 'with no other diagnostic about the abstract side'. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:221 |

## Verdict

Not mergeable: TC-540 still expects the refusal FR-135-AC-5 dropped (FND-001). FR-138-AC-5 and TC-554 agree on the respelled update, FR-141-AC-6 and FR-142-AC-8 cite it by name, and the index is complete.
