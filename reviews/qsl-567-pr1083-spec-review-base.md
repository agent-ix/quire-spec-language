---
id: SR-13395
title: 'Spec review of PR #1083: FR-226 load-buffering warning moves to EN-1, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@38b0b3f49; git diff refs/remotes/origin/main...38b0b3f49 (PR #1083, base main 47047b1df): spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md, spec/test-cases/TC-671-code-link-warning-and-routing.md, spec/decisions/ADR-025-weak-memory-models-for-parallel.md; read against QSpec main 0ad3768 FR-331 and schema, QSpec PR #293 head c5f1e93c, and engine main 47047b1df; ticket QSL-567'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-226
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-671
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-025
  type: reviews
---

## Summary

Ticket: QSL-567. PR: quire-spec-language#1083, head 38b0b3f49, scoped to `git diff refs/remotes/origin/main...38b0b3f49` (two commits, three files, +35/-24). Spec-only. It moves FR-226-AC-1's load-buffering warning from S3 to EN-1's pre-check, which reports it in the item's FR-331 terminal record `warnings`, and has AC-2 cite QSpec's `memory_models` manifest member (QSpec PR #293). Base review: whether the move is sound, whether the claimed channel exists, AC testability, and behaviour-not-changelog. Independent checks. (1) S3 warning channel: in code on main there is none. qsl-foundation/src/diagnostic/stage.rs `Staged<T>` holds only `value` and `work`, with the comment 'T-4 also gives Staged<T> the warnings the stage raised. No stage raises one yet', and neither CheckedPackage (qsl-package/src/checked.rs, src/checking.rs) has a warnings member; no S3 check raises a warning. In the spec, though, ADR-013 T-4 does give S1 to S4 a warning channel ('Staged<T> carries the output and its warnings'), so 'S3 has no warning channel' is true of the code, not of the spec. The PR does not rest on that claim: it rests on MA-1, and MA-1 does make sharing a request-time fact (EN-1 instantiates footprints per transition identity over the subject's universes and argument domains), so the move is sound. (2) FR-331 terminal record `warnings`: present on QSpec main as an accounting-record member listing FR-369 `fairness.strong-would-exclude` and `cross-check-absent{tool}`; QSL FR-133 puts its hint on 'the item's FR-331 terminal record' as a warning diagnostic, and engine ModelCheckRecord carries it as `hints: Vec<StrongFairnessHint>`. So the channel exists, but its member list and the schema's Warning oneOf are closed: `memory.load-buffering-shape` is added to the prose only by QSpec #293 (open), and #293 adds no Warning schema variant. FR-331-AC-30 also exists only in #293 (main ends at AC-29). (3) EN-1's pre-check is the right home: MA-1 decides sharing at instantiation, MA-4 already gives EN-1's pre-check the request-time half of the shape checks with the resolved model in hand, and FR-220 already lists the resolved model as a pre-check input. ADR-025 MX-4, the stage-DAG paragraph and DS-1 now agree with FR-226. (4) grep of the head for load-buffering/load buffering finds no remaining 'S3 warns' text outside FR-226's Use case (FND below); the older reviews/qsl-372-spec-review-integrity.md FND-012 records why the text said 'whatever the model' (S3 could not know the resolved model). (5) TC-671 steps checked by hand against AC-1 to AC-3. Methods: spec-review/base (SR-13395), spec-review/ears (SR-13396), spec-review/integrity (SR-13397). code-review and gap-analysis were not run, because no code or test file changed. Ids SR-13392 to SR-13394 were left free for parallel reviewers. Gates on the head: make check-spec-validation exits 0 with 19 warnings against 17 on main 47047b1df; the 2 new ones are both FR-226 line 104 ([ears:unclassifiable], [ears:missing-subject]). tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2), newer than the rollout note's 0.36.1/0.50.1.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-226-description | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-use-case | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-outputs | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-load-buffering-bullet | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-manifest-bullet | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-1 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-2 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| ADR-025-MX-4 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-stage-dag | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-DS-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-MA-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| FR-224-description | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | context_only |
| QSpec-FR-331-accounting-record | quire-specification:spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |
| QSpec-PR-293-warnings-prose | quire-specification#293 (head c5f1e93c): spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |
| ADR-013-T-4 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The warning is still owed 'whatever the model a request resolves'. That clause was written because S3 could not know the resolved model (reviews/qsl-372-spec-review-integrity.md FND-012). EN-1's pre-check does know it, so the reason is gone, and the clause now conflicts with three texts. MA-1 defines a shared location only 'in a weak parallel', so for an item resolved to `sc` the trigger ('a store to another shared location') is undefined. FR-224 scopes its sibling pre-check output, the race-freedom item, to items 'whose resolved model is weak'. QSpec #293 lists the warning 'for a weak-memory item'. MX-4 also needs the target condition only under `ra`. The `tso` target is x86-64, which excludes load buffering, and `sc` maps every shared access to SeqCst or a lock. So under `sc` the warning is noise that no precondition stands behind. Fix: report the shape only for an item whose resolved model is weak (or only `ra`, if the owner prefers), and say the same in ADR-025 MX-4. State the resolved model of AC-1's request, and add an `sc` request over the same subject that reports no warning. If the owner wants the warning under every model, say what 'shared' means under `sc`, and align #293's 'for a weak-memory item'. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:33-36, 101-108 |
| FND-002 | medium | The channel FR-226 names cannot carry this warning yet, even after QSpec #293. On QSpec main 0ad3768, FR-331's accounting-record `warnings` lists only `fairness.strong-would-exclude` and `cross-check-absent{tool}`. The schema's AccountingRecord.warnings items are `$ref Warning`, a closed oneOf of those two codes with additionalProperties false. #293 adds `memory.load-buffering-shape{load, store}` to the prose only. It adds no Warning schema variant, so an EN-1 terminal record carrying the warning AC-1 requires fails the FR-331 schema. The schema also sets `warnings` to false on a record settled by disposition. Merge order: this PR depends on #293 for AC-1 and the Outputs as well as for AC-2. #293 (or a follow-up QSpec PR merged first) needs a Warning variant `{code: memory.load-buffering-shape, load, store}`, with the spelling of `load` and `store` stated (attempt site or transition identity). FR-226 should then cite that spelling. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:59-60, 104-105, 125 |
| FND-003 | medium | The Use case still says 'The compiler warns them where a relaxed load followed by a store needs a target that excludes load buffering'. The compiler is S3, and the Behavior now says 'S3 SHALL emit no load-buffering warning'. This is the one stale 'S3 warns' text left on the head. Fix: say the item's terminal record warns them, from EN-1's pre-check, once the request has decided which locations are shared. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:46-48 |
| FND-004 | low | 'the verdict is unchanged' gives no reference to compare against: unchanged from what? The Behavior's 'SHALL leave the item's verdict unchanged' has the same gap. FR-133 states the property its hint has: the diagnostic is outside the counterexample, its replay identity and the obligation identity, and it changes no verdict. State that here as well, so a test can compare the record's value, label, basis, counterexample and obligation identity with and without the shape, for example the same subject with the load made `acquire`. 'S3 checks the package with no warning' can only be checked through ADR-013 T-4's Staged warnings, which the engine does not have yet. Name that output, or reduce the clause to 'S3 checks the package'. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:106, 125 |

## Verdict

Not mergeable yet. Nothing is high. FND-001 to FND-003 (medium) should be fixed, and FND-002 also needs a QSpec change: #293, or a QSpec PR that lands before this one, must add the Warning schema variant. What holds: moving the warning to EN-1's pre-check is right. MA-1 makes sharing a request-time fact. MA-4 already gives the pre-check the request-time shape checks, and FR-220 gives it the resolved model. ADR-025 MX-4, the stage-DAG paragraph and DS-1 agree with FR-226. AC-2's `memory_models` member matches #293's schema (a nonempty, duplicate-free subset of sc, tso and ra, on temporal-satisfaction descriptors only). Merge order: this PR must merge after QSpec #293, and not only for AC-2: AC-1 and the Outputs put the warning in FR-331's `warnings`, which lists `memory.load-buffering-shape` only after #293.

## New findings (disposition pass 1)

Round 1, head f518f3698. The fix commit f518f3698 sits on a merge, d7573afcf, which brings in main 70c7092b9. The diff against main still touches only FR-226, TC-671 and ADR-025, plus this review's round-0 SR files, committed unchanged under reviews/; this file supersedes them. Gates: make check-spec-validation exits 0 with 17 warnings, the same 17 as main 70c7092b9, and none in FR-226 or TC-671. The 2 round-0 warnings at FR-226 line 104 are gone. tools/check-index-completeness.sh passes. QSpec #293 is now at head 48428b7c0 (open). QSpec #293 head 48428b7c0: schema Warning gains the arm {code: memory.load-buffering-shape, load: TransitionIdentity, store: TransitionIdentity}, required, additionalProperties false; FR-331-AC-31 and TC-431 test it; FR-439 gains the emission rule and FR-439-AC-7. Its descriptor description says an absent memory_models means sc only. Merge order unchanged: this PR merges after QSpec #293.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | AC-1's new oracle compares against the wrong thing. It says the record's 'value, label, basis, counterexample and obligation identity equal those of the same request with the load made `acquire`', and TC-671's expected result says the same. That variant is a different program. Its source differs, so its checked package identity differs, and ADR-025 MM-5 puts the subject beside the model in the obligation identity, so the obligation identities cannot be equal. Under `ra` an `acquire` load also joins the message's view into `cur_b` (RA-2), so the explored behaviours differ too. For some claims the value or counterexample differs as well, and AC-1 names no claim. The round-0 base FND-004 suggested this comparison as an example; that suggestion was wrong. Fix: compare against the same request, as QSpec #293's FR-439-AC-7 does ('equal to the settlement without it'). For example: removing `warnings` from the record leaves a record equal to the one EN-1 writes for the same request with the shape check off. Or assert directly that the obligation identity and the counterexample's replay identity are computed without the warnings. Keep the `acquire` variant only as the no-warning case. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:138 |

## New findings (disposition pass 2)

Round 2, head dd66e262a (one commit over f518f3698). The diff against main 70c7092b9 still touches only FR-226, TC-671 and ADR-025, plus this review's round-1 SR files, committed unchanged under reviews/; this file supersedes them. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in FR-226 or TC-671; tools/check-index-completeness.sh passes. QSpec #293 is now at head 11741c6f3 (open). Its FR-439-AC-9 uses the same oracle ('equal to the settlement without it') and the same absent-member rule ('a candidate whose descriptor has no memory_models receives SB resolved to sc and not SB resolved to tso or ra'), so the two specs agree. Merge order unchanged: this PR merges after QSpec #293.

No new findings in this round.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | round 1, f518f3698: code claim (ADR-025 §8). For an item resolved to `ra`, EN-1's pre-check SHALL warn of each load-buffering shape of its subject (ADR-025 MX-4). EN-1's provider manifest SHALL advertise the memory models it decides, and QSL's `route` candidate step (FR-075) SHALL give an item over a subject with a weak `parallel` only candidates that advertise its resolved model (ADR-025 MX-7). / - EN-1 SHALL add... |
| FND-002 | fixed | round 1, f518f3698: QSpec #293 head 48428b7c0: schema Warning gains the arm {code: memory.load-buffering-shape, load: TransitionIdentity, store: TransitionIdentity}, required, additionalProperties false; FR-331-AC-31 and TC-431 test it; FR-439 gains the emission rule and FR-439-AC-7. FR-226 Dependencies: - QSpec owns the provider manifest's memory-model member and the warning's wire form (QSpec FR-331) and the rou... |
| FND-003 | fixed | round 1, f518f3698: freedom for plain fields, and a target that compiles RC11 soundly. The item's terminal record warns them where a relaxed load followed by a store needs a target that excludes load buffering, once the request has decided which locations are shared. |
| FND-004 | fixed | round 1, f518f3698: - The warning SHALL be outside the counterexample, its replay identity and the obligation identity, and SHALL change no verdict, as FR-133 states of its hint. |
| FND-005 | fixed | round 2, dd66e262a: \| FR-226-AC-1 \| For a request resolved to `ra` over a package with branch `t` (`r1 := x (relaxed); y := 1 (relaxed)`) and branch `s` (`y := 0 (relaxed); x := 1 (relaxed)`), which makes `x` and `y` shared, EN-1's pre-check puts one `memory.load-buffering-shape{load, store}` warning naming both `t` attempts in the item's terminal record `warnings`, and the record's value, label, basis, counterexa... |

Round 1 verdict: Round 1: FND-001 to FND-004 fixed. One new medium, FND-005: AC-1's equality oracle compares against a different package, whose obligation identity cannot match. Not mergeable until FND-005 is fixed. The fix is a one-line change in AC-1 and in TC-671's expected result. Then merge after QSpec #293.

Round 2 verdict: Round 2: FND-005 fixed. AC-1 and TC-671 now compare the record with the same request settled without the warning, as QSpec FR-439-AC-9 does, and the `acquire` variant appears only as a no-warning case. Every base finding is fixed. Mergeable once QSpec #293 has merged.
