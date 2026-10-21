---
id: SR-13649
title: 'Spec review of PR #1138: QSpec consumer alignment, base checklist'
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@fd5cd6d5e; git diff origin/main...fd5cd6d5e (PR #1138, branch qsl-spec-std-consumers, base main 5c4a96692): FR-057, FR-125, FR-134, FR-146, FR-153, FR-161, FR-218, FR-314, FR-337, TC-535, TC-893, TC-1056, TC-1079; read against origin/main code (qsl-route/src/lib.rs, qsl-route/tests/it/fairness_kinds.rs, qsl-semantics/src/check/protocol_requirement.rs, protocol_relationship.rs, refusal.rs, qsl-cst/src/grammar.rs, qsl-replay/src/certificate.rs), QSL FR-136, FR-315, FR-338, and quire-specification main fd7a694 (FR-468, FR-375) and open PRs #313 (9f0d0c8) and #314 (7722857); tickets STD-197, STD-193, STD-194, STD-186, STD-217, QSL-877"
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-057
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-125
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-153
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-161
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-314
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-337
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-893
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1056
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1079
  type: reviews
---

## Summary

Ticket: STD-197 (also STD-193, STD-194, STD-186, STD-217, QSL-877). PR quire-spec-language#1138, head fd5cd6d5e. Base checklist: AC to TC traceability, ticket ownership, cross-repository references and AC id collisions with open QSL PRs.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-057 registration causes | spec/functional/FR-057-admit-shared-capability-kinds.md | examined |
| FR-134 admission rule | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134-AC-3 | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134-AC-4 | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134-AC-5 | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134-AC-6 | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| TC-535 vectors | spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md | examined |
| FR-146 safety half | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-218 not emitted | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-14 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-16 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| TC-1056 row 17 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| TC-1056 row 23 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| FR-314-AC-3 | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| TC-893 step 4 | spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md | examined |
| FR-337-AC-5 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| TC-1079 step 3 | spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md | examined |
| FR-153-AC-3 | spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md | examined |
| FR-161-AC-2 | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |
| FR-125 one instance | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-146's new safety-half rule has no AC or TC row. The new bullet 'The refinement's safety half combines its records' settlements as QSpec FR-468 states: Certified only when ... otherwise Uncertified, or Inconclusive with cause hypotheses-not-established' is backed by no acceptance criterion: FR-146-AC-1 to AC-7 and TC-551 are unchanged and none settles the half from a set of records (all parts verified, one hypothesis unproved, one part uncertified). QSpec FR-468-AC-3 and AC-6 give the vectors. Add an FR-146 AC (and TC-551 step) for the half, or drop the bullet and leave the rule to QSpec FR-468. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:167-172; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:178-185 |
| FND-002 | medium | FR-218-AC-16's new not-emitted clause has no TC-1056 row and names no clause kind. FR-218-AC-16 now asserts 'a requirement of a clause S4 does not emit yet refuses unsupported_construct/not-yet-implemented at that clause with no package, and the requirement's own refusals are decided regardless'. TC-1056 changes only row 23; no row compiles a requirement of a not-emitted clause, and no row combines such a clause with a requirement refusal (for example requires temporal Nothing beside it). The AC also names no concrete clause kind, so a tester cannot build the vector: name one (for example a clause with an each activation or a capture, which S4 does not emit) and add a TC-1056 row for both halves. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:419; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:265-269; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md:72-75 |
| FND-003 | medium | No open ticket owns the code that must follow FR-134-AC-3 to AC-5. After this PR, origin/main code contradicts the ACs its tests are tagged with: qsl-route admit_fairness_kinds still returns UnknownFairnessKind for a non-list member and for an unknown label, DuplicateFairnessKind and AbsentFairnessKinds (as_str 'unknown-fairness-kind', 'duplicate-fairness-kind', 'absent-fairness-kinds'), and qsl-route/tests/it/fairness_kinds.rs asserts those codes under #[trace("TC-535", "FR-134-AC-3")], AC-4 and AC-5, so the matrix shows AC-3 to AC-5 covered by tests that assert the superseded behaviour. The PR body names QSL-877 / PR #1011 as the code consumer, but QSL-877 is Done and #1011 is merged. The lead should open a code ticket (single cause, defect retention, not-a-list for non-list members) before or with the merge. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:122-124; qsl-route/src/lib.rs:262-292; qsl-route/src/lib.rs:528-554; qsl-route/tests/it/fairness_kinds.rs:227-290 |
| FND-004 | medium | The PR cites QSpec ACs that exist only in open QSpec PRs #313 and #314. FR-337-AC-5, FR-337 Inputs, FR-125, FR-153-AC-3, FR-161-AC-2 and TC-1079 cite QSpec FR-370-AC-13, and FR-134 cites FR-368-AC-8 to AC-10 and the FR-272 invalid-fairness-kinds row. None exists on quire-specification main (fd7a694): they are added by #314 and #313, both open. The PR also deletes FR-134's Remaining work entry, so once merged nothing in QSL records that the cause and the over rule are still unratified. Merge this PR only after #313 and #314 land, or keep a one-line Remaining work note naming them until they do. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:133; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:68-73; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:161-165; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:84-87; spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md:34-37 |
| FND-005 | medium | FR-314-AC-3 and TC-893 step 4 collide with open PR #672. Open QSL PR #672 ('Specify certificate proof-step loci and wire correspondence') also adds an FR-314-AC-3 (command numbers for ProofStep loci) and a TC-893 step 4, and edits FR-338. Whichever merges second carries a duplicate AC id and a duplicate TC step. Renumber this PR's AC (for example FR-314-AC-4 and TC-893 step 5) or coordinate with #672's owner and record the choice. | spec/functional/FR-314-check-an-smt-proof-certificate.md:150; spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md:33-36; quire-spec-language#672 spec/functional/FR-314-check-an-smt-proof-certificate.md (adds FR-314-AC-3) |

## Verdict

Not mergeable as it stands. The STD-197 FR-218 alignment (export key only; requirement bound by clause position) matches protocol_relationship.rs and protocol_requirement.rs, the over-parameter refusal matches the QSL grammar (TemporalClause writes over before clock, so S1 meets `clock`), and the fairness single cause matches QSpec #313. make check-spec-validation and tools/check-index-completeness.sh exit 0 with 17 warnings on main and on the PR, none in a touched file. The findings are traceability gaps: an unbacked new rule (FND-001), an AC with no TC row (FND-002), a code consumer with no ticket (FND-003), citations of unmerged QSpec ACs (FND-004) and an AC id collision with #672 (FND-005).

## New findings (disposition pass 1)

Head 5e73cbeb1 (PR #1138 round 1).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-218-AC-16 and TC-1056 row 23a need clause emission the code refuses, and no ticket is named. FR-218-AC-16 and the new TC-1056 row 23a say a requirement of a clause with an `on each` activation and a capture checks and binds by position. The code on origin/main refuses such a clause: temporal_clause.rs `is_emitted` admits only `on origin` with no capture, and `not_emitted` refuses every other clause `unsupported_construct`/`not-yet-implemented`. The spec is right to drop that refusal, because FR-337 states the emission of every checked clause. But row 23a cannot pass until the code emits those clauses, and neither the PR nor the FRs name the ticket that owns that work, as FR-134 does with QSL-925. Name or open that ticket (FR-337 emission of `on each` and captured clauses), for example in a FR-218 or FR-337 Remaining work line. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:419; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md:73; qsl-semantics/src/check/temporal_clause.rs:1131-1147 |
| FND-007 | medium | Merging #672 and then #1138 conflicts in three files, and the two disagree on who owns the locus wire spelling. Trial merge (origin/main 5c4a96692, then #672 head, then #1138 head 5e73cbeb1). #672 alone conflicts with main in TC-893: main dropped the Status section. #1138 on top then conflicts in FR-314 (the locus paragraph, the AC-3 vs AC-4 rows, Dependencies), in FR-338 (the paragraph after check_closure) and in TC-893 (Scope, step 4 vs 5, Expected). All are resolvable by taking both sides, except one contradiction. #672 says 'QSpec FR-331 owns the wire spelling of every CertificateRule and of every CertificateLocus variant'. #1138's FR-338 defines `unrolling`, `base`, `step` and `step/<row>`. QSpec FR-331 on main fixes only `certificate-rejected{rule, at}`, with `at` a Nonempty string (schema.json:675, vector `at: unrolling`), so it spells no locus. A QSL-defined spelling is therefore admissible today, and #672's sentence is the inaccurate one. Whichever PR lands second should keep FR-338's spelling paragraph and reword #672's sentence (FR-331 carries rule and at as non-empty strings; FR-338 spells them), or raise a QSpec ticket to catalog the spellings. Also set the TC-893 Scope to 'FR-314-AC-1 to FR-314-AC-4' and keep #672's step 4 beside #1138's step 5. Until #672 lands, TC-893's procedure jumps from step 3 to 5. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:83-86; spec/functional/FR-314-check-an-smt-proof-certificate.md:95-100; spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md:18-75; quire-specification proposals/backend-provider-v1/schema.json:675 |

## Dispositions

Round 1, head 5e73cbeb1, checked against origin/main 5c4a96692 code, QSL FR-315/FR-338, QSpec main fd7a694 and QSpec #313 648d2d0 / #314 13bd4aa.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 5e73cbeb1: / FR-146-AC-8 / The safety half of `CasRefinesCounter` settles from its records and hypothesis items by QSpec FR-468: all eight records proved with verified certificates and every hypothesis item certified, `proved`, `Certified`; the same with one hypothesis item `Uncertified`, `Uncertified`; with the invariant `self.value < 0` whose hypothesis items are not proved, `inconclusive`, `hypotheses-not-established`, although every step record is proved; with a backend counterexample for one `Step` record, `Inconclusive(InductionNotClosed{depth: 1})`; with a counterexample for the `Initial` record,  |
| FND-002 | fixed | Fixed in 5e73cbeb1: / FR-218-AC-16 / `requires temporal Due;` over a temporal clause `Due` binds the requirement to `Due`'s position among the unit's temporal clauses, from which S4 mints its node key, the same for a clause with an `on each` activation and a capture; `requires temporal Nothing;` and a clause offered only through an import alias each refuse `missing_declaration`/`missing-name` at the name; `requires temporal Accept;` naming a predicate refuses `ill_typed`/`operator-ineligible` at the name, naming the kind; `Due` required twice is recorded once. Each refusal produces no package. / Test (TC-1056) / / TC-1056 / 23a / row 23 with `Due` declared with an `on each (x: ...)` activati |
| FND-003 | fixed | Fixed in 5e73cbeb1: - QSpec ratifies the cause `invalid-fairness-kinds` and FR-368-AC-8 to AC-10 in quire-specification#313; the refusals here follow that text. The code that emits the three earlier causes (`qsl-route/src/lib.rs`, `qsl-route/tests/it/fairness_kinds.rs`) is Linear QSL-925. (QSL-925 exists, Backlog) |
| FND-004 | fixed | Fixed in 5e73cbeb1: - QSpec ratifies the cause `invalid-fairness-kinds` and FR-368-AC-8 to AC-10 in quire-specification#313; the refusals here follow that text. The code that emits the three earlier causes (`qsl-route/src/lib.rs`, `qsl-route/tests/it/fairness_kinds.rs`) is Linear QSL-925. / FR-337: - QSpec FR-370-AC-13 (a temporal clause always names an `over` parameter) lands with quire-specification#314; the `over` statements in this requirement and in TC-1079 follow it. |
| FND-005 | fixed | Fixed in 5e73cbeb1: / FR-314-AC-4 / Refinement certificates, over `CasRefinesCounter` (seven step rows): the certificate with a base part and seven step parts, each refuting its FR-315 refinement query with checked rules, is `Verified`; with six step parts it is rejected `ShapeMismatch` at `Step { row: Some(6) }`, with / TC-893 Scope: FR-314-AC-1, FR-314-AC-2 and FR-314-AC-4. |
