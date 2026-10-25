---
id: SR-17131
title: "Gap analysis of PR #1511: FR-306-AC-3/4, FR-185-AC-1/2 and the untagged FR-304-AC-6, FR-185-AC-9, FR-139-AC-6"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e1fdc52178571c7e371769987290b387a5cc7a78; PR #1511 (branch qsl-tests-546-513-920); spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md (AC-3, AC-4), FR-185-declare-random-parameters-workloads-and-rewards.md (AC-1, AC-2, AC-9), FR-304-check-an-authored-abstraction-relation.md (AC-6), FR-139-type-and-evaluate-population-valued-expressions.md (AC-6); spec/test-cases/TC-805, TC-620; tests/it/abstraction_relation_inert.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-semantics/src/check/abstraction.rs, qsl-eval/tests/it/refinement_populations.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
---
## Summary

Tickets: QSL-513, QSL-920, QSL-546. PR: quire-spec-language#1511. No plan
bundle. Plan completion: not assessed. No spec file changed, so spec-review
was skipped.

`quoin matrix --json` (quoin 0.29.1) at the head: FR-306-AC-3 and
FR-306-AC-4 `tagged` (1 binder each), FR-185-AC-1 `tagged` (1 binder),
FR-185-AC-2 `tagged` (4 binders); FR-304-AC-6, FR-185-AC-9 and FR-139-AC-6
`untagged` (0 binders). All report `no run evidence`.

Criteria against tests and code:

- FR-306-AC-3: asserted in both clauses (records equal; `route` results equal
  over one registry). A production mutation that drops a record when a
  relation is checked fails the test (SR-17130).
- FR-306-AC-4: asserted for all 17 FR-108 cases through `run_clause`, the
  spine run entry; 16 compared whole, `missing-model` by stage and category. A
  production mutation that changes the disposition when a relation is present
  fails the test. Self-check weakness recorded as SR-17130 FND-002.
- FR-185-AC-1: every clause asserted except outcome's support values
  (FND-001).
- FR-185-AC-2: every clause asserted by the union of the four tagged tests
  (SR-17130 Summary lists which test holds which clause).
- FR-304-AC-6 (QSL-546 batch, untagged): no Rust-syntax check of `RustPath`
  segments, tuple-field indices or receivers exists;
  qsl-semantics/src/check/abstraction.rs:16-19 says so. Correctly untagged.
  No open ticket tracks the missing check (FND-002).
- FR-185-AC-9 (untagged clause): no reference-literal form in qsl-cst,
  qsl-forms or qsl-semantics; the integer-into-reference-typed case is tested
  under TC-620 only. Correctly untagged. Its trackers QSL-971 and STD-395 are
  both Done (FND-003).
- FR-139-AC-6 (untagged): S3 refuses the `Int[2, 4]` row
  `ill_typed`/`type-mismatch` naming neither `R::slots` nor the universe size;
  correctly untagged; tracked by QSL-1056 (Backlog).

No coverage inflation beyond FND-001: every new tag is backed by assertions.

## Verdict

FR-306-AC-3/4 are closed with sound tests and FR-185-AC-2 honestly. The
FR-185-AC-1 tag inflates by one clause (FND-001, medium, fix in this PR). The
coder's three "not covered" calls are right; two of them lack a live tracker
(FND-002, FND-003, low, tracker-only, not blocking this PR).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185-AC-1 is now credited to `the_service_model_checks_and_records_its_declarations`, but its clause "`outcome` with `Ok` at `49/50` and `Fail` at `1/50`" is only half asserted: the probabilities are checked, the support values are not, and swapping `Ok` and `Fail` in the fixture passes (SR-17130 FND-001). Fix: assert outcome's values. | qsl-semantics/tests/it/probabilistic_declarations.rs:426-434 |
| FND-002 | low | FR-304-AC-6 is unimplemented (no `invalid_model_binding`/`malformed-declaration` for `type`, `9lives`, `01` or receiver `0`; abstraction.rs:16-19 says the module "checks no Rust syntax (FR-304-AC-6)", citing the AC as if it required no check) and no open ticket tracks it. Fix: file an implementation ticket for the segment, index and receiver checks and correct the module doc. | qsl-semantics/src/check/abstraction.rs:16-19 |
| FND-003 | low | FR-185-AC-9's reference-literal clause stays untestable, but its trackers QSL-971 and STD-395 are both Done and the test doc still points at QSL-971 (probabilistic_declarations.rs:612-617). Fix: reopen QSL-971 or file a live follow-up and point the doc at it. | qsl-semantics/tests/it/probabilistic_declarations.rs:612-617 |

## Dispositions

Round 1, verified at PR #1511 head 2963ff6d6. FR-185-AC-1's outcome clause now has a killing assertion (SR-17130 FND-001). The two tracker findings need ticket actions, not code in this test-only PR, and the head does not change abstraction.rs:16-19 or probabilistic_declarations.rs:614.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Outcome's (rank, probability) pairs are asserted in declared order, rank 1 (`Ok`) at 49/50 and rank 0 (`Fail`) at 1/50; the Ok/Fail swap mutant now fails. |
| FND-002 | deferred | Tracker action outside this test-only PR: the coordinator files an implementation ticket for FR-304-AC-6's segment, index and receiver checks, and that ticket corrects abstraction.rs:16-19. |
| FND-003 | deferred | Tracker action outside this test-only PR: reopen QSL-971 (or file a live follow-up) for FR-185-AC-9's reference-literal clause, then repoint probabilistic_declarations.rs:614. |
