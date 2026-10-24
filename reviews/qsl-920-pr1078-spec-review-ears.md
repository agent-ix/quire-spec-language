---
id: SR-13390
title: 'Spec review of PR #1078: FR-185 workloads, derived sums, S4, quantity parameters, EARS'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-920. PR: quire-spec-language#1078, head e4d0be107, scoped to `git diff refs/remotes/origin/main...e4d0be107` (one commit, three files, +58/-2). Spec-only, closing the four spec gaps recorded on QSL-920 from PR #1067: duplicate workload names, the derived-sum bound, the S4 refusal and Quantity<Time> parameters. EARS review of the four new Behavior bullets. Engine measured on main 40f8707b (PR #1067 merged), in scratch probes appended to qsl-semantics/tests/it/probabilistic_declarations.rs (not committed): (a) a second `workload Steady on Service` parses at S1/S2 and S3 refuses it conflicting_declaration/duplicate-declaration at the name, with no related declarations; with a second domain package selected as `Other`, `Steady on Other` alone checks, but `Steady on Service` plus `Steady on Other` also refuses duplicate-declaration: the checker keeps one workload name per unit (model.rs: 'a claim names a workload by its name alone (FR-186)'). (b) The ceiling is `RATIONAL_BITS_CEILING = 65_536` in qsl-semantics/src/check/probabilistic/exact.rs, inclusive, a fixed constant (no CheckingLimits field; QSL-883 pending); the refusal is reported as StageLimitExceeded/work-budget-exceeded. S3 already bounds a distribution's running sum by it, polling cancel before each addition: `{1: rational(1, 3), 3: rational(1, 2^65534+1)}` (sum denominator 65,536 bits) checks and `{1: rational(1, 3), 3: rational(1, 2^65535+5)}` (65,537 bits) refuses. S3 sums no workload: a workload whose weights sum past the ceiling checks. (c) S4 (qsl-package emit.rs) refuses a checked package holding any random parameter, reward or workload with unsupported_construct/not-yet-implemented, naming counts (ProbabilisticModelNotEmitted{random_parameters, rewards, workloads}), not a declaration. (d) `1 ms` parses and types as a quantity of the unit's dimension, but no domain package can type a parameter as a quantity: the intake's ValueTypeRef is a native type (UUID, Boolean, Integer, Rational, Decimal, Float32, Float64, Text, String, Timestamp) or a package node, and ScalarValueDomain has no quantity form. The test fixture types `d` as `Millis = Int[1, 12]` for that reason (its header cites STD-113). `{1 ms: 90, 3 ms: 7, 12 ms: 3}` on that `d` refuses value-outside-type three times. QSpec checked at main 0ad376803: FR-272's `conflicting_declaration` row has the one cause `duplicate-declaration`, with `subject` the repeated resolved declaration identity of any declaration kind and `related` both declarations, each with its label span, in source-unit order, so the subject/related wording is accurate. FR-322 has no node_tag/semantic_form for a random parameter, workload or reward. FR-272's `not-yet-implemented` names the unbuilt feature, its owning declaration and the selected profile. FR-405 has no duplicate-workload rule; FR-407 refuses a claim's workload of another model `foreign-workload`. Methods: spec-review/base (SR-13389), spec-review/ears (SR-13390), spec-review/integrity (SR-13391); code-review and gap-analysis were not run because no code or test file changed. Ids SR-13386 to SR-13388 were left free for the active PR #1077 reviewer. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 40f8707b and none in FR-185, TC-620 or FR-091; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2); the rollout note names quire 0.36.1/engine 0.50.1, so the binary is newer, not older.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-185-derived-sum-bullet | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-duplicate-workload-bullet | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-quantity-parameter-bullet | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-package-emission-bullet | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The duplicate-workload bullet joins a SHALL refusal, a scoping statement ('and a workload of that name on another model is a different workload') and a rationale paragraph ('This is not an invalid_probabilistic_model cause: ...') in one requirement. Keep the If/SHALL statement alone. Move the scope into the statement's condition and the rationale to a note. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:149-156 |
| FND-002 | low | The derived-sum bullet holds four SHALLs: bound, measure, refuse, and poll cancellation. It also adds two non-requirement sentences, on probabilities and on rewards. Split it into one statement per SHALL: an ubiquitous bound and measure, an If ... then refusal, and an ubiquitous poll. Move the probability and reward remarks to notes. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:102-112 |
| FND-003 | low | 'Until QSpec FR-322 defines a node form ..., S4 SHALL refuse' is not an EARS trigger, and 'FR-322 has no node_tag/semantic_form for these declarations today' describes the world at writing time, not the system. Write it state-driven or event-driven, for example 'If a checked package holds a random parameter, a workload or a reward, S4 SHALL refuse ...', and move the FR-322 status to Dependencies or an Open Question. The quantity bullet's lead sentence has no SHALL, and 'this requirement adds none' is meta text. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:174-193 |

## Verdict

Three low findings, all wording. None blocks merge on its own, and they can be fixed with the base and integrity findings. Each refusal is still stated as an If/SHALL with a code and cause. quire's grammar lint reports no warning in FR-185, so these were found by reading the text.

## New findings (disposition pass 1)

Round 1, head be629264f (fix commit be629264f over merge 04fb7d23, which brings in main 50518689). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-185, TC-620 or FR-091; tools/check-index-completeness.sh passes. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Re-measured on main 40f8707b with the scratch probes: `{1: rational(1, 3), 3: rational(1, 2^65534 + 1)}` checks; `{1: rational(1, 3), 3: rational(1, 2^65535 + 5)}` refuses StageLimitExceeded/work-budget-exceeded with limit 65,536; `Steady on Service` plus `Steady on Other` (a second package selected as `Other`) refuses duplicate-declaration, and `Steady on Other` alone checks; a check of the over-ceiling distribution with a handle cancelled before it starts trips the handle (Requested) and returns stand-in StageLimitExceeded/work-budget-exceeded refusals with limit 0, not the 65,536 ceiling refusal.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The quantity section restates an existing requirement in prose: 'S3 SHALL refuse a support value that is not of the parameter's type `value-outside-type`' repeats the Random parameters bullet ('If a value is not of the parameter's type, S3 SHALL refuse the distribution [`value-outside-type`], naming the parameter') with a weaker statement that names nothing. Drop the SHALL here and keep the section as a note or Open Question about the missing quantity type. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:182-192 |

## Dispositions

Round 1, head be629264f (fix commit be629264f over merge 04fb7d23, which brings in main 50518689). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-185, TC-620 or FR-091; tools/check-index-completeness.sh passes. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Re-measured on main 40f8707b with the scratch probes: `{1: rational(1, 3), 3: rational(1, 2^65534 + 1)}` checks; `{1: rational(1, 3), 3: rational(1, 2^65535 + 5)}` refuses StageLimitExceeded/work-budget-exceeded with limit 65,536; `Steady on Service` plus `Steady on Other` (a second package selected as `Other`) refuses duplicate-declaration, and `Steady on Other` alone checks; a check of the over-ceiling distribution with a handle cancelled before it starts trips the handle (Requested) and returns stand-in StageLimitExceeded/work-budget-exceeded refusals with limit 0, not the 65,536 ceiling refusal.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 1 verdict: FND-001 to FND-003 are fixed: the duplicate-workload and derived-sum statements are split into single-SHALL statements with notes, and S4 is an If/then statement. One new low finding (FND-004), which does not block merge.

Round 2, head efa2a4b4b (fix commit efa2a4b4b over merge cc03e43a, which brings in main 94020c90, a rustfmt-only test change, #1081). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-185, TC-620 or FR-091; tools/check-index-completeness.sh passes. The committed reviews/ SR files equal this review's round-1 files. No `ms` or `Quantity<Time>` support remains in FR-185 or TC-620 outside the quantity note. Re-measured on main 40f8707b: Service on the Millis fixture checks with `d: Int[1, 12]`, support 1, 3, 12 at 9/10, 7/100, 3/100, and `duration` typed Int[1, 12] (Millis). `{1: 9, 1: 1}` refuses repeated-support-value; `true` and `1 ms` refuse value-outside-type; the TC-620 precondition test reads `d < 5` and passes. A check whose handle is tripped before it starts reports tripped = Requested and refusals with limit 0, not 65,536. The AC-6 sums and the AC-5 duplicates behave as in round 1. All 28 probabilistic_declarations tests and probes pass.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | - |

Round 2 verdict: Mergeable as far as EARS is concerned: all four findings are fixed, and the quantity section is a note that defers to the Random parameters rule.
