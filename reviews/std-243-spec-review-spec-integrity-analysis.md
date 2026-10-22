---
id: SR-13970
title: 'Spec review of PRs #1187 and #328: spawn/retire instance member, pop resolution, send/receive owner, predicate reads (STD-243), spec-integrity analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@949dcb17f; git diff origin/main...949dcb17f (PR #1187, branch std-protocol-243, merge base 48b47ab99): spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md, spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/test-cases/TC-653, TC-654, TC-661, TC-663; agent-ix/quire-specification@2ad2f256 (PR #328, branch std-protocol-243-qspec, merge base 014f0ce05): spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md, spec/test-cases/TC-382-replicated-role-lifetimes.md; ticket STD-243'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-208
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-209
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-216
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Ticket: STD-243. PRs: quire-spec-language#1187 (head 949dcb17f, two commits; the brief named 8a47c9f, the branch has since moved and this review is of 949dcb17f) and quire-specification#328 (head 2ad2f256). Both are based on main, not stacked. Method: spec-review with the spec-integrity sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

Gates. QSL at head: `make check-spec-validation` exit 0 with no warning on a touched file; `tools/check-index-completeness.sh` exit 0. The diff is under `spec/` only, touching nothing under `docs/` and no test-parsed vector table. QSpec at head: `quire validate --scope . 'spec/**/*.md'` reports 134 failed documents, the same as origin/main (134), none in a touched file; `cargo test --test checked_package_v2 --test backend_provider_v1` passes (14 and 117 tests).

Faithfulness to the ticket. All five gaps are closed as the ticket describes. Gap 1: TS-2, FR-208-AC-4, QSpec FR-426 table row and FR-426-AC-15. The code already builds `Members::Role { role, object, instance }` with `thread: None` (qsl-eval/src/simulation/protocol/identity.rs:366-373,788-795). Gap 2: RR-1, FR-218 S3 bullet and AC-28, QSpec FR-428 and AC-11. The grammar takes `from ident()` (qsl-cst/src/grammar.rs:1567-1568), so the lead default "bare declared name" can be built. Gap 3: RR-2, FR-208 and FR-218 statements, AC-5 and AC-29, QSpec FR-428-AC-12. Gap 4: FT-1 and FR-216 with AC-6. The code reads `slot_reads` for the activation guard and the `until(p)` predicate into the enabling footprint (qsl-eval/src/simulation/protocol/footprint.rs:956-1020). The spec's "every object whose type declares `f`" is narrower than the ticket's "any object" and is still sound; AC-6 asserts that the footprint holds those locations, which a superset satisfies. Gap 5: the sentence leaves FR-209-AC-4 and TC-654 step 4, and the FR-209 Dependencies name QSpec FR-428-AC-3, which already reads "Two requests that differ only in `max_live_instances` carry the same obligation identity" (Test TC-381). ADR-013 O-09 exists ("Clause and obligation identity").

The lead defaults agree across both halves: `pop` is a bare name in the package declaring `T`, with the same refusal codes in RR-1, FR-218 and QSpec FR-428; the predicate-read rule covers model-state field reads only, with a record-binder read reading no model location; the identity sentence moves to QSpec FR-428-AC-3. The QSL and QSpec halves also agree on the refusal codes, the `send`/`receive` ownership, `NoRoleInstance` and `no-role-instance`, and the `instance` member. FND-006 covers the one place where they differ, the `thread` member.

Ids: no collisions. FR-208-AC-4/5, FR-216-AC-6, FR-218-AC-28/29, FR-426-AC-15, FR-428-AC-11/12 and TC-382 RR-10..12 are new and used once. No other open PR in either repo adds these ids.

Other open PRs. The brief says #1189 (STD-233) also edits TC-661 Scope. At #1189's head e2239cc it does not: its files are ADR-025, ADR-027, FR-205, FR-206, FR-216, FR-221, TC-650, TC-651 and TC-666. It edits FR-216-AC-4 and the ADR-027 PS-8 and ST-2 rows, which are separate hunks from this PR's. `git merge-tree` of 949dcb17f with e2239cc is clean, so there is no TC-661 one-line conflict to resolve at present. In QSpec, #329 (STD-233 QSpec half, head d2b6b37) adds FR-426-AC-16 at the same place where #328 adds FR-426-AC-15, after AC-14. `git merge-tree` reports a content conflict in FR-426. The ids are distinct, so whichever PR merges second keeps both rows.

Wording is behaviour-only, with rationale clauses and no changelog, status or review language. Neither PR body paraphrases review findings.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-027 RR-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:196 | examined |
| ADR-027 RR-2 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:197 | examined |
| ADR-027 TS-2 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:268 | examined |
| ADR-027 FT-1 (spawn/retire and predicate-read rule) | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 | examined |
| ADR-027 FT-3 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:296 | context_only |
| FR-208 statements (spawn/retire identity, send/receive owner) | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:65-72 | examined |
| FR-208-AC-4 | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:85 | examined |
| FR-208-AC-5 | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:86 | examined |
| FR-209 bound statement | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:89-91 | examined |
| FR-209-AC-4 (edited) | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:121 | examined |
| FR-209 Dependencies bullet | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:131-133 | examined |
| FR-216 spawn/retire and predicate-read statement | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:100-108 | examined |
| FR-216 activate enabling footprint | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:121-127 | examined |
| FR-216-AC-6 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:143 | examined |
| FR-218 Outputs owner of send/receive | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:89 | examined |
| FR-218 scope region and pop resolution statements | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:359-368 | examined |
| FR-218-AC-28 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:439 | examined |
| FR-218-AC-29 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:440 | examined |
| FR-211 NoRoleInstance cause ordering | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:122-128 | examined |
| FR-218 model-state definition | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:110-121 | context_only |
| ADR-013 O-09 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:290 | context_only |
| QSpec FR-426 identity table (instance row) and AC-15 | quire-specification spec/functional/protocol/FR-426-apply-protocol-steps.md:183-200,224 | examined |
| QSpec FR-428 replicated roles prose and AC-11, AC-12 | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:59-80,150-151 | examined |
| QSpec FR-428-AC-3 | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:142 | context_only |
| QSpec TC-382 RR-10 to RR-12 | quire-specification spec/test-cases/TC-382-replicated-role-lifetimes.md:44-46 | examined |
| Spawn/retire identity code | qsl-eval/src/simulation/protocol/identity.rs:366-373,788-800 | context_only |
| Guard and until(p) footprint code | qsl-eval/src/simulation/protocol/footprint.rs:956-1020 | context_only |
| Replicated role grammar | qsl-cst/src/grammar.rs:1564-1571 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-218-AC-29's region clause ("a lifetime `scope` region over `Work` nodes by `w` and a `send` from `w` includes the `send`") does not place the `send` outside the smallest control that holds the `Work` nodes. A test that puts the `send` beside a `Work` node passes even when S3 ignores send ownership, because that region already holds it. Place the `send` where counting it changes the region, for example `Work` inside `sequence S` and the `send` after `S`, so the region is `S`'s parent and not `S`. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:440 |
| FND-002 | low | ADR-027 FT-1 names "the binders and captures `p` names" as locations a predicate reads, but FR-216's statement names only "the binders `p` names". The two QSL texts disagree on whether protocol captures are in the footprint. Add captures to FR-216, or remove them from FT-1. | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:102-103; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294 |
| FND-003 | low | RR-2 and FR-208 now make a `send` or `receive` owned by a replicated role a per-instance step that names the acting instance. FT-1 ("a step `by` a replicated role reads `role(r)`"), FT-3 and FR-216 ("A step `by r` SHALL read `role(r)`") still say only "a step `by r`", and FR-218 extends "by" to owned send/receive for the scope region alone. The text does not say whether those steps read `role(r)`. If they do not, a `spawn` looks independent of the `send` it enables. The code keys on the actor (footprint.rs:325), so it includes them. State that a step taken by a role instance, `send` and `receive` included, reads `role(r)`. | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:108; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294,296 |
| FND-004 | low | FR-211 still orders causes for "a thread at an `attempt`, `event`, `send` or `receive` node ... owned by a replicated role (`by r`)". After this PR, a `send` or `receive` is owned through its channel's `from`/`to` role and not through a `by r` clause, so the parenthetical misdescribes those two kinds. Drop "(`by r`)" or say "owned by a replicated role (`by r`, or its channel's role for `send` and `receive`)". | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:125-128 |
| FND-005 | low | QSpec FR-426-AC-15 requires a `spawn` or `retire` identity to hold "no `thread`", which matches QSpec's table (`thread` only for thread-owned steps) and the code (`thread: None`). QSL TS-2, edited in this PR, still gives every step the template `{"type":"protocol-transition","step":<kind>,"thread":<path>,"node":<wire node id>,…}` and does not exempt `spawn` and `retire`, and FR-208-AC-4 does not mention it. On this point the QSL half reads differently from the QSpec half. Say in TS-2 that `spawn`, `retire` and the channel and memory steps carry no `thread` or `node`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:268 |
| FND-006 | low | QSpec TC-382 RR-10 declares `max 1 lifetime until(p)` and takes "`retire` of `w1` in each" protocol instance, but never says what `p` is. The vector cannot be built as written. Use RR-08's `until(w.done)` with a step that sets `done`, or state `p`. | quire-specification spec/test-cases/TC-382-replicated-role-lifetimes.md:44 |

## Verdict

Mergeable once FND-001 is fixed. FND-001 is medium: as written, FR-218-AC-29's region clause cannot fail. FND-002 to FND-006 are low. Both halves close the ticket's five gaps as the ticket describes, agree with the code already merged for spawn/retire identity and guard reads, and keep the lead defaults consistent. Every new id is unique. The QSL and QSpec halves agree, apart from FND-005. Gates are green. No TC-661 conflict with #1189 exists at its current head. QSpec #328 and #329 conflict textually in FR-426's AC table, which takes a one-line rebase. SR-13972 holds the coverage note on FR-209's remaining obligation-identity clause.
