---
id: SR-13397
title: 'Spec review of PR #1083: FR-226 load-buffering warning moves to EN-1, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-567. PR: quire-spec-language#1083, head 38b0b3f49, scoped to `git diff refs/remotes/origin/main...38b0b3f49` (two commits, three files, +35/-24). Spec-only. It moves FR-226-AC-1's load-buffering warning from S3 to EN-1's pre-check, which reports it in the item's FR-331 terminal record `warnings`, and has AC-2 cite QSpec's `memory_models` manifest member (QSpec PR #293). Integrity review: consistency across FR-226, FR-224/225, ADR-025 and TC-671, the relationships, and TC coverage. Independent checks. (1) S3 warning channel: in code on main there is none. qsl-foundation/src/diagnostic/stage.rs `Staged<T>` holds only `value` and `work`, with the comment 'T-4 also gives Staged<T> the warnings the stage raised. No stage raises one yet', and neither CheckedPackage (qsl-package/src/checked.rs, src/checking.rs) has a warnings member; no S3 check raises a warning. In the spec, though, ADR-013 T-4 does give S1 to S4 a warning channel ('Staged<T> carries the output and its warnings'), so 'S3 has no warning channel' is true of the code, not of the spec. The PR does not rest on that claim: it rests on MA-1, and MA-1 does make sharing a request-time fact (EN-1 instantiates footprints per transition identity over the subject's universes and argument domains), so the move is sound. (2) FR-331 terminal record `warnings`: present on QSpec main as an accounting-record member listing FR-369 `fairness.strong-would-exclude` and `cross-check-absent{tool}`; QSL FR-133 puts its hint on 'the item's FR-331 terminal record' as a warning diagnostic, and engine ModelCheckRecord carries it as `hints: Vec<StrongFairnessHint>`. So the channel exists, but its member list and the schema's Warning oneOf are closed: `memory.load-buffering-shape` is added to the prose only by QSpec #293 (open), and #293 adds no Warning schema variant. FR-331-AC-30 also exists only in #293 (main ends at AC-29). (3) EN-1's pre-check is the right home: MA-1 decides sharing at instantiation, MA-4 already gives EN-1's pre-check the request-time half of the shape checks with the resolved model in hand, and FR-220 already lists the resolved model as a pre-check input. ADR-025 MX-4, the stage-DAG paragraph and DS-1 now agree with FR-226. (4) grep of the head for load-buffering/load buffering finds no remaining 'S3 warns' text outside FR-226's Use case (FND below); the older reviews/qsl-372-spec-review-integrity.md FND-012 records why the text said 'whatever the model' (S3 could not know the resolved model). (5) TC-671 steps checked by hand against AC-1 to AC-3. Methods: spec-review/base (SR-13395), spec-review/ears (SR-13396), spec-review/integrity (SR-13397). code-review and gap-analysis were not run, because no code or test file changed. Ids SR-13392 to SR-13394 were left free for parallel reviewers. Gates on the head: make check-spec-validation exits 0 with 19 warnings against 17 on main 47047b1df; the 2 new ones are both FR-226 line 104 ([ears:unclassifiable], [ears:missing-subject]). tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2), newer than the rollout note's 0.36.1/0.50.1.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-226-relationships | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-inputs | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-outputs | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-1 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| FR-226-AC-2 | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | examined |
| TC-671-step-1 | spec/test-cases/TC-671-code-link-warning-and-routing.md | examined |
| TC-671-step-2 | spec/test-cases/TC-671-code-link-warning-and-routing.md | examined |
| ADR-025-MX-4 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-stage-dag | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-DS-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| ADR-025-MA-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| FR-224-description | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | context_only |
| QSpec-FR-331-accounting-record | quire-specification:spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |
| QSpec-PR-293-warnings-prose | quire-specification#293 (head c5f1e93c): spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Outputs, the Behavior and AC-2 now depend on QSpec FR-331: the terminal record `warnings`, the `memory_models` descriptor member and FR-331-AC-30. FR-226's relationships still list only QSpec FR-290, FR-353 and FR-439. The Dependencies section's QSpec line also does not name FR-331. Add `depends_on ix://agent-ix/quire-specification/FR-331`, and name FR-331 in Dependencies. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:4-24 |
| FND-002 | low | The Inputs still list only the S3-time inputs (the checked protocol clause, its classifications and its source defaults) plus the manifests. The warning now needs request-time inputs: the request's resolved model (MM-2) and EN-1's instantiated footprints with the shared locations they decide (MA-1). FR-220 already lists the resolved model as a pre-check input. Add both. | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:52-55 |
| FND-003 | low | TC-671 step 1 matches AC-1, apart from three gaps. AC-1 covers 'a branch whose load or store touches a location no other branch accesses', which is two cases: either one can be local. Step 1 has one 'variant whose locations no other branch accesses'. Neither the AC nor the step names the other branch that makes `x` and `y` shared, which MA-1 requires: another branch accessing both, with at least one writing. Neither states the request's resolved model (base FND-001). Add the other branch to the fixture, list the load-local and store-local variants separately, and state the model. | spec/test-cases/TC-671-code-link-warning-and-routing.md:19-22, 32-34 |

## Verdict

Three low findings: traceability and coverage. What holds: TC-671 still verifies FR-226, its title and description name EN-1, steps 2 and 3 match AC-2 and AC-3, and the expected results follow the procedure. ADR-025 MX-4, the stage-DAG paragraph and DS-1 all move the warning to EN-1's pre-check consistently. FR-224, FR-225 and FR-220 have no load-buffering text to update. Index completeness passes. Merge order: this PR must merge after QSpec #293, and not only for AC-2: AC-1 and the Outputs put the warning in FR-331's `warnings`, which lists `memory.load-buffering-shape` only after #293.
