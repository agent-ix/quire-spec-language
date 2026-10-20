---
id: SR-13391
title: 'Spec review of PR #1078: FR-185 workloads, derived sums, S4, quantity parameters, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@e4d0be107; git diff refs/remotes/origin/main...e4d0be107 (PR #1078, base main 40f8707b): spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; read against QSpec main 0ad376803 FR-272, FR-322, FR-405, FR-407 and engine main 40f8707b; ticket QSL-920'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-620
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-091
  type: reviews
---

## Summary

Ticket: QSL-920. PR: quire-spec-language#1078, head e4d0be107, scoped to `git diff refs/remotes/origin/main...e4d0be107` (one commit, three files, +58/-2). Spec-only, closing the four spec gaps recorded on QSL-920 from PR #1067: duplicate workload names, the derived-sum bound, the S4 refusal and Quantity<Time> parameters. Integrity review: cross-requirement consistency, relationships and TC coverage of the new ACs. Engine measured on main 40f8707b (PR #1067 merged), in scratch probes appended to qsl-semantics/tests/it/probabilistic_declarations.rs (not committed): (a) a second `workload Steady on Service` parses at S1/S2 and S3 refuses it conflicting_declaration/duplicate-declaration at the name, with no related declarations; with a second domain package selected as `Other`, `Steady on Other` alone checks, but `Steady on Service` plus `Steady on Other` also refuses duplicate-declaration: the checker keeps one workload name per unit (model.rs: 'a claim names a workload by its name alone (FR-186)'). (b) The ceiling is `RATIONAL_BITS_CEILING = 65_536` in qsl-semantics/src/check/probabilistic/exact.rs, inclusive, a fixed constant (no CheckingLimits field; QSL-883 pending); the refusal is reported as StageLimitExceeded/work-budget-exceeded. S3 already bounds a distribution's running sum by it, polling cancel before each addition: `{1: rational(1, 3), 3: rational(1, 2^65534+1)}` (sum denominator 65,536 bits) checks and `{1: rational(1, 3), 3: rational(1, 2^65535+5)}` (65,537 bits) refuses. S3 sums no workload: a workload whose weights sum past the ceiling checks. (c) S4 (qsl-package emit.rs) refuses a checked package holding any random parameter, reward or workload with unsupported_construct/not-yet-implemented, naming counts (ProbabilisticModelNotEmitted{random_parameters, rewards, workloads}), not a declaration. (d) `1 ms` parses and types as a quantity of the unit's dimension, but no domain package can type a parameter as a quantity: the intake's ValueTypeRef is a native type (UUID, Boolean, Integer, Rational, Decimal, Float32, Float64, Text, String, Timestamp) or a package node, and ScalarValueDomain has no quantity form. The test fixture types `d` as `Millis = Int[1, 12]` for that reason (its header cites STD-113). `{1 ms: 90, 3 ms: 7, 12 ms: 3}` on that `d` refuses value-outside-type three times. QSpec checked at main 0ad376803: FR-272's `conflicting_declaration` row has the one cause `duplicate-declaration`, with `subject` the repeated resolved declaration identity of any declaration kind and `related` both declarations, each with its label span, in source-unit order, so the subject/related wording is accurate. FR-322 has no node_tag/semantic_form for a random parameter, workload or reward. FR-272's `not-yet-implemented` names the unbuilt feature, its owning declaration and the selected profile. FR-405 has no duplicate-workload rule; FR-407 refuses a claim's workload of another model `foreign-workload`. Methods: spec-review/base (SR-13389), spec-review/ears (SR-13390), spec-review/integrity (SR-13391); code-review and gap-analysis were not run because no code or test file changed. Ids SR-13386 to SR-13388 were left free for the active PR #1077 reviewer. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 40f8707b and none in FR-185, TC-620 or FR-091; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2); the rollout note names quire 0.36.1/engine 0.50.1, so the binary is newer, not older.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-185-AC-5 | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-duplicate-workload-bullet | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-relationship-FR-302 | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-091-OQ-11 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| TC-620-fixtures | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-5 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-6 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-7 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-8 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| FR-185-AC-7 | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-186-scheduler-bullet | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md | context_only |
| FR-302-duplicate-body-rule | spec/functional/FR-302-link-dispatch-during-unit-compile.md | context_only |
| QSpec-FR-272-conflicting_declaration-row | quire-specification:spec/objects/foundation/FR-272-typed-diagnostic-cause.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-185 now says a workload of the same name on another model is a different workload, and AC-5 and TC-620 step 5 say 'a `Steady` on another model checks'. FR-186 says the opposite: 'A workload name is a package-wide identifier, so `W` resolves to the one workload of that name', and its `foreign-workload` refusal relies on that. With per-model names, `under Steady` would be ambiguous. QSpec FR-272's subject is 'the repeated resolved declaration identity', and per-model uniqueness would make that identity (model, name), not the name. Measured on main 40f8707b with a second package selected as `Other`: `Steady on Service` plus `Steady on Other` refuses duplicate-declaration, because the checker keeps one name per unit. Fix: make workload names unique per package, as FR-186 and the engine do. Refuse a second workload of a name on any model, change AC-5's second clause and TC-620 step 5 to match, and drop the 'different workload' clause. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:149-156, 203 |
| FND-002 | low | The new depends_on FR-302 points at the wrong rule. FR-302 is 'Link dispatch at S3 during unit compile', and its duplicate-declaration rule covers two operation bodies resolving to one DeclarationKey. FR-302 itself cites FR-246 for repeated same-name declarations. The subject/related semantics FR-185 uses come from QSpec FR-272. 'the repeated-name rule is the one FR-302 states for declarations that must be unique' overstates FR-302. Fix: depends_on ix://agent-ix/quire-specification/FR-272 (FR-056 already has this cross-repo edge), and cite FR-272, with FR-246 as the QSL precedent if a QSL citation is wanted. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:12-13, 149-156 |
| FND-003 | low | TC-620's Fixtures line still lists 'the variants FR-185-AC-2 to FR-185-AC-4', although the scope is now AC-1 to AC-8 and steps 5 to 8 need new fixtures: a second model, the coprime-weight distributions and a quantity-typed `d`. Step 6 checks a workload summing 'exactly to' the ceiling, but AC-6 states the exactly-at case only for a distribution. Its expected result omits the workload at-ceiling case. Align the Fixtures line, step 6 and AC-6. | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md:19, 26, 39 |

## Verdict

Not mergeable. FND-001 (high) puts FR-185 in contradiction with FR-186 and the engine on workload-name scope, and must be fixed. What holds: the code/cause pair and the subject/related wording match QSpec FR-272's conflicting_declaration row. TC-620's scope and steps 5 to 8 cover AC-5 to AC-8 one to one. Index completeness passes.
