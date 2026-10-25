---
id: SR-17081
title: "Gap analysis of PR #1488: FR-336-AC-6 coverage after moving SubjectIdentity into qsl-replay (QSL-1010)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8a5a09595cdaabe41727532129b9efd7ff0a74af; PR #1488 (branch qsl-1010-subject-identity); spec/functional/FR-336-bound-a-model-subject-by-its-universes.md (AC-4, AC-6); spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md (step 5); qsl-replay/src/model_check/subject_identity.rs; qsl-analyze/src/model_check/request.rs; qsl-analyze/tests/it/universe_obligation.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1190
    type: reviews
---
## Summary

Ticket: QSL-1010. PR: quire-spec-language#1488. Planless run; no plan was
supplied. Spec files are untouched by the diff, so spec-review was not run.

`quoin matrix --json` (quoin 0.29.1) at the head: FR-336-AC-6
`static_status: tagged`, binders
`the_facade_identity_is_the_request_path_identity_whatever_the_listing_order`
(universe_obligation.rs:241) and `a_package_with_no_emission_is_refused_package`
(:279), `evidence_status: no run evidence` (no run evidence is recorded in
this repo for any row, so that is not specific to this PR).

FR-336-AC-6 clause by clause (spec line 138; TC-1190 step 5):

1. "`qsl_replay::SubjectIdentity::of` over FR-336-AC-4's subject gives the
   identity `qsl-analyze`'s request path writes into the item": asserted.
   The subject is `ReachesTwo` over universe `{a, b, c}` (one of AC-4's
   two), with a second initial state and two proof bounds added so the
   next clause has something to reverse. `author` goes through
   `SubjectContext::new(..).author_clause`, and
   `assert_eq!(item.subject, identity)` compares the stored item with the
   facade value. Since the request path now calls the facade itself, this
   is a wiring check, which is what the clause asks for.
2. "... with its initial states, its universe's keys and its proof bounds
   each listed in reverse order gives an equal identity": asserted
   (`assert_eq!(reversed, identity)`, with all three listings reversed), and
   guarded against an identity that ignores its members
   (`assert_ne!` with one bound changed). Reviewer mutation M2 (keep only the
   first bound) is killed.
3. "... with an equal hash": asserted (`assert_eq!(hash_of(..), hash_of(..))`),
   but only probabilistically for the proof-bound component: see FND-001
   and SR-17080 FND-001 (order-dependent hash survives 25 of 40 runs).
4. "A subject whose checked package has no emission is refused
   `SubjectIdentityRefusal::Package`": asserted, with the inner value pinned
   to `EmitRefusal::NothingToEmit` over a really empty checked package.

Reverse gap: the new code (`SubjectIdentity`, its `Hash`,
`SubjectIdentityRefusal`, the qsl-replay re-export) is owned by FR-336-AC-6
and the FR-336 Outputs (ADR-013 O-09). Nothing unowned is added. No stubs,
no `todo!`, no ignored tests.

## Verdict

CONDITIONAL. FR-336-AC-6 is tagged and three of its four clauses are
asserted soundly; the equal-hash clause is asserted, but its order
independence for the proof bounds is gated only by chance (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-336-AC-6's "equal hash" clause is tagged but its evidence is probabilistic for the proof bounds, the only `HashSet` component (SR-17080 FND-001): with two bounds per subject the two sets iterate alike in about half the runs, so the row can read covered while an order-dependent `Hash` ships. Fix as in SR-17080 FND-001 (more bounds plus repeated reconstruction), then re-run the order-dependent mutation until it fails every run. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:138; qsl-analyze/tests/it/universe_obligation.rs:251-267 |

## Coverage

- FR-336-AC-6: tagged (2 binders); clauses 1, 2 and 4 asserted; clause 3
  asserted with weak (probabilistic) strength for the proof-bound set.
- FR-336-AC-4, AC-5 (context only): their existing tests still pass at the
  head (4 of 4 in `universe_obligation`).
- Semantic review: run, limited to FR-336-AC-6 (requested in the brief).
- Plan completion: not assessed

## Dispositions

Round 1, against follow-up PR #1498 (branch
qsl-1010-subject-identity-review-fixes, head 812890fe3, fix commit
b5171a0b5). PR #1488 had already merged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `SubjectIdentity` now stores `BTreeSet<BoundKey>` (no `HashSet`, derived `Hash`), so the hash is order-independent by construction; the FR-336-AC-6 test checks 8 bounds over all 8 rotations and their reversals (16 orderings), each rebuilt identity equal with an equal hash. The equal-hash clause is now deterministically evidenced. |
