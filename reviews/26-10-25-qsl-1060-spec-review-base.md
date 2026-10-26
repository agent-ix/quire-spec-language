---
id: SR-1586001
title: "Base spec review of PR #1586 (QSL-1060 protocol-subject component certificate)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #1586 diff origin/main...qsl-1060-liveness-cert-spec; spec/functional/FR-228-derive-memory-fairness-constraints.md, spec/functional/FR-339-check-an-en-1-component-certificate.md, spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md, spec/test-cases/TC-673-memory-fairness.md"
review_set: subset
---

## Summary

Ticket: QSL-1060. A base-checklist review of the PR diff only: FR-228 (Behavior bullets, AC-2, AC-4), FR-339 ("Protocol subjects", AC-7), TC-526 step 6 and TC-673 steps 2 and 4. I traced the AC-7 vectors by hand over the product graph of FR-228-AC-2's store-and-poll protocol, checking them against ADR-025 TSO-4, TSO-7, MS-4, MB-3 and MF-1 to MF-4, FR-212's classes, QSpec FR-430 and FR-436, and FR-126 "Certificates". AC-7 contradicts FR-339's own resolution rule on one point (FND-001). The rest of the vector set is consistent.

## Verdict

**Not mergeable until FND-001 is fixed.** FND-002 to FND-004 are low and can be fixed in the same pass.

Hand trace of FR-228-AC-2 under `tso`, written as reached protocol states after `fork`. S1: `w` has not stored, buffer empty, `r` polls (an `r` self-loop). S2: `w` is done, buffer `[x := 1]`, `r` polls (an `r` self-loop, and `flush(w)` leads to S3). S3: buffer empty, memory `x = 1`, and `r`'s next load reads 1, which ends the automaton. Results:

- S2 is not terminal and has no stutter loop, because `flush(w)` is enabled (MS-4, TSO-4).
- The branch-`w` scheduler class is disabled at S2, because memory steps are in no class (FR-212 "Classes", QSpec FR-430).
- `flush(w)` is the first holding `UnfairWeak` at S2 in the fairness set's JCS order (FR-126). The `each` members sort before the `whole` members, and `flush(r)` is never enabled.
- The branch-`w` scheduler is the first holding `UnfairWeak` at S1.
- The `WitnessFails` vectors that name the `w` scheduler at S2, or `flush(w)` at S1, are correct.
- With bounds `None` (4), no state is bound-limited, so MB-3 is never exercised.
- FR-228-AC-2's filter-level form is also consistent: `flush(w)` is the only failing member over the full set, and the SCC passes without the `memory` members.

On the author's reversal (no "ignore memory constraints" switch, and FR-228-AC-4's TP-1 half dropped), the reversal is sound. ADR-025 MF-1 makes the flush constraint part of the model and never authored. MK-2 refuses an unfair lasso at replay, so a switch would produce counterexamples that cannot be replayed. QSpec FR-436 and FR-430 define no removal of memory constraints. No required behaviour is lost:

- The discriminating content of the old "refuted when ignored" clause is kept at the filter (ADR-018 FA-5).
- The old TP-1 half compared against a counterfactual that no product path can produce.
- FR-221-AC-1 (TC-666) and FR-222-AC-1 (TC-667) assert ADR-025 §10's TP-1 verdicts under `tso` and `ra`, so the safety verdicts stay covered. They verify the verdicts, not the new "reads no fairness constraint" statement (FND-002).

No hash, digest, pin or tracking record is introduced. TC scopes and AC ids are consistent, and no Rust test carries an `FR-228-AC-2`, `FR-228-AC-4` or `FR-339-AC-7` tag yet.

## Reviewed scope

- `FR-339-AC-7`: examined. Protocol subject. FR-228-AC-2's `tso` proof checked under its `ProtocolCertificateRequest` returns a `ComponentCertificate` whose resolved constraints are its fairness set's members in JCS order, with `flush(w)` and `flush(r)` each resolving to one constraint. The post-store component's witness is `UnfairWeak` naming `w`'s `flush` class, and the pre-store component's names the branch-`w` scheduler. The checker accepts it, and the AC lists three `WitnessFails` vectors.
- `FR-339` "Protocol subjects": examined. Resolved constraints follow the fairness set's order. An `each` member gives one constraint per class that a computed edge's step, or a step enabled at a reached state, belongs to. A store cut by a memory bound reads as enabled.
- `FR-228-AC-2`: examined. The claim returns `Holds{Exhaustive}` with an accepted certificate and settles `proved`, `Certified`. The filter rejects the unflushed SCC with `fair weak each flush(w)` as the only failing member, and passes it without the `memory` members.
- `FR-228-AC-4`: examined. A `fair weak` constraint naming `flush` refuses `missing_declaration`/`missing-name`.
- `FR-228` Behavior, the new bullets on the TP-4 certificate, the safety path, and "no removal of memory constraints": examined.
- `TC-526` step 6 and its expected results: examined.
- `TC-673` steps 2 and 4 and their expected results: examined.
- `FR-221-AC-1`, `FR-222-AC-1`, `FR-212-AC-4`, `FR-338` (bound-limited `SuccessorMissing`), `FR-126` "Certificates", ADR-025 TSO-4, TSO-7, MS-4, MB-3, MF-1 to MF-4 and MK-2, QSpec FR-430 and FR-436: context_only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-339-AC-7 says `fair weak each flush(r)` resolves to one constraint, and uses "naming `r`'s `flush` class" as a `WitnessFails` vector. FR-339's own "Protocol subjects" rule resolves an `each` member to one constraint per class that a computed edge's step, or a step enabled at a reached state, belongs to. `r` never stores, so `flush(r)` is never enabled or taken, and it resolves to zero constraints. The AC contradicts the rule, and the rejected vector has no index to name. Either the rule resolves every class of an `each` member, reached or not, or AC-7 says `flush(r)` resolves to none and drops or replaces that vector (TC-526 step 6 then expects two `WitnessFails`). | spec/functional/FR-339-check-an-en-1-component-certificate.md:93-117, :207; spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md:34-38, :54-56 |
| FND-002 | low | The new FR-228 statement "EN-1's safety path (TP-1, TP-2, TP-3) SHALL read no fairness constraint" has no verifying AC. FR-221-AC-1 and FR-222-AC-1, which it cites, check the §10 verdicts, not that property. State it as a consequence of ADR-025 MF-3 rather than a new SHALL, or mark it verified by Inspection. | spec/functional/FR-228-derive-memory-fairness-constraints.md:97-99 |
| FND-003 | low | FR-228-AC-2 does not give `c.r`'s initial value. AC-7's component layout and its `proved` verdict depend on it: an initial 1 settles the claim at position 0 with no accepting product state. State the initial value, for example 0. | spec/functional/FR-228-derive-memory-fairness-constraints.md:115; spec/functional/FR-339-check-an-en-1-component-certificate.md:207 |
| FND-004 | low | FR-339 "Protocol subjects" says a store cut only by a memory bound reads as enabled. In an accepted certificate this rule never applies: FR-338 rejects any bound-limited reached state with `SuccessorMissing` before component work runs. Delete it, or say it applies only to the run's enabled sets (FR-126), not to the checker. | spec/functional/FR-339-check-an-en-1-component-certificate.md:106-110; spec/functional/FR-338-check-an-en-1-closure-certificate.md:932-936 |
