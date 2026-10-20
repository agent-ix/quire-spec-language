---
id: SR-13396
title: 'Spec review of PR #1083: FR-226 load-buffering warning moves to EN-1, EARS'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-567. PR: quire-spec-language#1083, head 38b0b3f49, scoped to `git diff refs/remotes/origin/main...38b0b3f49` (two commits, three files, +35/-24). Spec-only. It moves FR-226-AC-1's load-buffering warning from S3 to EN-1's pre-check, which reports it in the item's FR-331 terminal record `warnings`, and has AC-2 cite QSpec's `memory_models` manifest member (QSpec PR #293). EARS review of the changed Description and Behavior statements and AC-1, AC-2. Independent checks. (1) S3 warning channel: in code on main there is none. qsl-foundation/src/diagnostic/stage.rs `Staged<T>` holds only `value` and `work`, with the comment 'T-4 also gives Staged<T> the warnings the stage raised. No stage raises one yet', and neither CheckedPackage (qsl-package/src/checked.rs, src/checking.rs) has a warnings member; no S3 check raises a warning. In the spec, though, ADR-013 T-4 does give S1 to S4 a warning channel ('Staged<T> carries the output and its warnings'), so 'S3 has no warning channel' is true of the code, not of the spec. The PR does not rest on that claim: it rests on MA-1, and MA-1 does make sharing a request-time fact (EN-1 instantiates footprints per transition identity over the subject's universes and argument domains), so the move is sound. (2) FR-331 terminal record `warnings`: present on QSpec main as an accounting-record member listing FR-369 `fairness.strong-would-exclude` and `cross-check-absent{tool}`; QSL FR-133 puts its hint on 'the item's FR-331 terminal record' as a warning diagnostic, and engine ModelCheckRecord carries it as `hints: Vec<StrongFairnessHint>`. So the channel exists, but its member list and the schema's Warning oneOf are closed: `memory.load-buffering-shape` is added to the prose only by QSpec #293 (open), and #293 adds no Warning schema variant. FR-331-AC-30 also exists only in #293 (main ends at AC-29). (3) EN-1's pre-check is the right home: MA-1 decides sharing at instantiation, MA-4 already gives EN-1's pre-check the request-time half of the shape checks with the resolved model in hand, and FR-220 already lists the resolved model as a pre-check input. ADR-025 MX-4, the stage-DAG paragraph and DS-1 now agree with FR-226. (4) grep of the head for load-buffering/load buffering finds no remaining 'S3 warns' text outside FR-226's Use case (FND below); the older reviews/qsl-372-spec-review-integrity.md FND-012 records why the text said 'whatever the model' (S3 could not know the resolved model). (5) TC-671 steps checked by hand against AC-1 to AC-3. Methods: spec-review/base (SR-13395), spec-review/ears (SR-13396), spec-review/integrity (SR-13397). code-review and gap-analysis were not run, because no code or test file changed. Ids SR-13392 to SR-13394 were left free for parallel reviewers. Gates on the head: make check-spec-validation exits 0 with 19 warnings against 17 on main 47047b1df; the 2 new ones are both FR-226 line 104 ([ears:unclassifiable], [ears:missing-subject]). tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2), newer than the rollout note's 0.36.1/0.50.1.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-226-description | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-load-buffering-bullet | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-manifest-bullet | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-1 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-2 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The first Behavior bullet holds four SHALLs: the pre-check reports, the warning leaves the verdict unchanged, S3 emits none, and S3 leaves the checked result unaffected. quire's grammar lint flags it at line 104 with [ears:unclassifiable] and [ears:missing-subject]. These are the only 2 new warnings on the head (19 against 17 on main), and FR-226 had none before. Split the bullet into one When ... the pre-check SHALL statement for the report, and one ubiquitous statement for the verdict property (see base FND-004). Express the S3 half as one statement, or drop it (FND-002). | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:101-108 |
| FND-002 | low | 'S3 emits no such warning' in the Description, and the negative 'S3 SHALL emit no load-buffering warning' in the Behavior, mostly record where the warning used to be. A reader new to the spec would not expect S3 to warn. The reason, that sharing is decided only at request time (MA-1), is worth keeping as a note. Consider keeping only the positive requirement (the pre-check reports it) plus that note. If the negative S3 statement stays, keep it to one statement in one place. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:35-36, 106-108 |

## Verdict

Two low findings, both wording. Neither blocks merge on its own, and both can be fixed with the base findings. The manifest bullet is a clean ubiquitous SHALL with a named subject. Each criterion is a direct assertion of the outcome. Merge order: this PR must merge after QSpec #293, and not only for AC-2: AC-1 and the Outputs put the warning in FR-331's `warnings`, which lists `memory.load-buffering-shape` only after #293.

## New findings (disposition pass 1)

Round 1, head f518f3698. The fix commit f518f3698 sits on a merge, d7573afcf, which brings in main 70c7092b9. The diff against main still touches only FR-226, TC-671 and ADR-025, plus this review's round-0 SR files, committed unchanged under reviews/; this file supersedes them. Gates: make check-spec-validation exits 0 with 17 warnings, the same 17 as main 70c7092b9, and none in FR-226 or TC-671. The 2 round-0 warnings at FR-226 line 104 are gone. tools/check-index-completeness.sh passes. QSpec #293 is now at head 48428b7c0 (open). QSpec #293 head 48428b7c0: schema Warning gains the arm {code: memory.load-buffering-shape, load: TransitionIdentity, store: TransitionIdentity}, required, additionalProperties false; FR-331-AC-31 and TC-431 test it; FR-439 gains the emission rule and FR-439-AC-7. Its descriptor description says an absent memory_models means sc only. Merge order unchanged: this PR merges after QSpec #293.

No new findings in this round.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | round 1, f518f3698: - When an item's resolved model is `ra` and a branch of a `parallel` has a `relaxed` or non-atomic load followed in program order by a store to another shared location, with no `acquire` or `seq_cst` load and no `acquire` fence between them, EN-1's pre-check SHALL add one `memory.load-buffering-shape{load, store}` warning, naming the load and the store, to the `warnings` of the item's FR-331 te... |
| FND-002 | fixed | round 1, f518f3698: Note. Which locations are shared is decided only at request time (ADR-025 MA-1), so EN-1's pre-check, not S3, reports the shape; a shared location exists only in a weak `parallel`, and the load-buffering condition of MX-4 applies only under `ra`. |

Round 1 verdict: Round 1: both findings fixed. The bullet is split into three single-SHALL statements, and the negative S3 statement is gone; its reason is kept as a note. The grammar lint reports nothing in FR-226. Clean for EARS.
