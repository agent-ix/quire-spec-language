---
id: SR-1155
title: "QSL-369 spec review of ADR-022, FR-165 to FR-170, TC-590 to TC-595 and TC-612 to TC-614"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1f141b6c36fb8c35fdbd0f53d21b7365767dd975; diff 404a5a88...1f141b6c; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/usecase/US-020-state-that-something-is-possible-in-a-model.md; spec/functional/FR-165..FR-170; spec/test-cases/TC-590..TC-595, TC-612..TC-614; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: reviews
---

## Summary

Ticket: QSL-369, PR quire-spec-language#569, head 1f141b6c, reviewed as
`git diff 404a5a88...HEAD` (the stale #562 base). One file for the whole
spec-review pass: base checklist, owner-ruling check, technical soundness,
EARS, integrity, scope boundary and the QSpec cross-check against QSpec
FR-390 to FR-394 on `spec/wave-b-q3-possible-hyper`.

`quire validate --scope .` over the 21 added or changed files exits 0 with
no error; `tools/check-index-completeness.sh` passes. TC ids 612 to 614 do
not collide with the hyper PR's TC-610, TC-611, TC-615 and TC-616.

**Ruling compliance (RU-5, QSL-366 undefined rule).** Met. Well-definedness
is global: GV-1 and FR-168/FR-169 settle `proved` from a witness only after
an exploration with no open node and no undefined evaluation. A sampled
witness is a fast path only (GE-2, FR-166 "Phase 0 SHALL NOT settle an
item"). A run that a limit stops, or that ends with open nodes, with a
witness for every initial state settles `inconclusive`,
`WellDefinednessUnchecked` (GV-1, FR-168 `WitnessedUnchecked`, FR-169 V-6
row, TC-614). The cost is recorded in RU-5. Undefined evaluation settles
`refuted` with cause `UndefinedEvaluation{where, cause}` and no new result
kind (GV-7, FR-169 V-4 row, TC-612). EN-2 witnesses settle
`WellDefinednessUnchecked` (GE-3). This matches QSpec FR-391 "Well-definedness
is global" and FR-392 "Settlement".

**Other checks.** No compat path, no fixed cap, ticket ids only in
References, EARS phrasing throughout, every AC has a behavioural TC row in
spec/tests.md, QSpec-owned grammar and verdict table cited not re-specified.
Worked examples checked by hand: §7.1 (9 nodes, 18 edges, witnesses, trap at
`(0, 0)`), §7.2 (trap at `Lost`, `from` and restart variants), §7.3 (path
pair; sequenced variant), FR-168-AC-3's `reset` variant (fff counts 2 via the
`{fff, tff}` SCC, deviation `stepA, reset, stepA, stepB, finish`), and
FR-166-AC-3's trace index range `64..128`.

**Antipatterns found outside the diff (surfaced, not this PR's rows).**
FR-101-AC-10 refuses a sampler `DefinitionRef` whose version is not
`1-draft.1`: a version check on main. QSpec FR-392 bounds walks by the search
horizon only and QSpec FR-394's witness row reads "item `proved`"; both carry
the same defects as FND-001 and FND-004 here and belong to the QSpec lane.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Phase 0 walks end only at a target, a terminal state or the step ceiling `max_depth`, whose published default is `usize::MAX` (FR-126-AC-8), and no other limit charges walk steps. In a cyclic model with no reachable target, such as §7.1's `ReachesThree`, each default walk runs about 2^64 steps, so default-on phase 0 does not terminate and FR-166-AC-2 ("draws 64 walks per instance, finds no witness") cannot pass. Give walks a finite caller-set step budget with a published default, or charge walk steps to `max_transitions`, and name it when reached. | spec/functional/FR-166-sample-witnesses-for-possible-claims.md:87,124; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:143 |
| FND-002 | medium | The explored witness descends "to a node at distance one less", where `d` is the distance to the nearest target **or open** node (E⁺). On a run with open nodes the descent can end at an open node, not a target, so the stated construction does not always yield a witness. Compute a separate distance to target nodes only for the witness. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:83-91; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:149-154,183-185 |
| FND-003 | medium | QSpec FR-392 (and FR-392-AC-8) makes the search horizon a method parameter that "is not a limit" and "not a member of the temporal limits", stated separately in the terminal record. FR-166 puts `max_depth` inside `ModelCheckLimits.limits`, and FR-169 records only "the limits the run used" for V-5 to V-7 and V-6 `WellDefinednessUnchecked`, with no horizon. Align with QSpec: record the horizon as a method parameter beside the limits. | spec/functional/FR-166-sample-witnesses-for-possible-claims.md:53; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:114,117 |
| FND-004 | medium | GX-2 says `P` true at every last node "settles `reproduced-with-evaluated-witness` and the item `proved`". For a `WellDefinednessUnchecked` witness set that contradicts GV-1 and RU-5. FR-169 maps it correctly; the ADR rule should say replay agreement settles the item by GV-1 (V-9 or V-6). | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:244 |
| FND-005 | medium | FR-166 reads the sampler's `DefinitionRef` "from the ecosystem lock", a pinned, version-checked reference (FR-101-AC-10 refuses any version but `1-draft.1`). GX-5 and FR-170 replay a sampled witness from its steps without the sampler, so no consumer of this PR needs the lock-sourced reference. Pass phase 0 the sampler directly and drop the lock dependency. | spec/functional/FR-166-sample-witnesses-for-possible-claims.md:68-69 |
| FND-006 | low | FR-169 records the seed "whenever phase 0 ran"; FR-166 records it "whether the request named it or not" on every run, and QSpec FR-392 records the seed and `witness_samples` on every run. Use one rule, QSpec's. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:118 |
| FND-007 | low | FR-170 Outputs retain "the source identity and digest, the `package_id`, the claim's identities, each initial state's document identity and digest, each replayed post-state digest". No requirement reads them from the result; this is the echo SR-961 FND-009 removed from FR-128. Delete it. | spec/functional/FR-170-replay-state-graph-evidence.md:80-84 |
| FND-008 | low | A node where the target evaluates undefined is neither stated to be a target node nor a non-target node for the trap and E⁺ definitions, and FR-170 trap replay does not say what an undefined target in the closure does. FR-168 also omits QSpec FR-391's tie rule that an undefined node which is itself a trap or a count-2 node settles `Undefined`. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:136-139; spec/functional/FR-170-replay-state-graph-evidence.md:120-126 |
| FND-009 | low | §3 Determinism says a partial run with sampled witnesses settles V-6 "where the same run without it settles V-5 or V-7". Phase 1 can find explored witnesses for every initial state on a partial run, which also settles V-6. Say "may settle". | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:174-177 |
| FND-010 | low | GX-1 says `GraphEvidence` "has three arms" and lists four (`Witness`, `Trap`, `PathPair`, `Undefined`). | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:243 |
| FND-011 | low | GE-1's verdict column lists "GV-1, GV-2, GV-4 to GV-6" and omits GV-3 and GV-7, which EN-1 also produces; QS-4 cites "V-5 to V-8 as GV-5 states" but GV-5 does not cover V-8. | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:142,432 |
| FND-012 | low | "Settlement method" lists V-9 by a witness "sampled, explored or unrolled", and GV-1 lets `Proved{Witness}` carry `Unrolled`. Under RU-5 an EN-2 witness never settles V-9 (GE-3), so `Unrolled` belongs only to `WellDefinednessUnchecked`. | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:211,220-221 |
| FND-013 | low | US-020-EX-1 is titled "The game can be won" but its example is §7.1's ConfigVersion `ReachesTwo`. | spec/usecase/US-020-state-that-something-is-possible-in-a-model.md:47 |
| FND-014 | low | FR-168-AC-4 does not state the initial value of `n`; the open node "`Won` with `n = 1`" at depth 3 holds only when `n` starts at 0. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:165 |
| FND-015 | low | GV-7 and FR-168 set `where` to "that node". QSpec FR-391 requires `where` to name the predicate, the node's state-key digest and the locus of the expression with no value, and FR-394 replays against those. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:133-135 |

## Verdict

Not mergeable as it stands: FND-001 is high (default-on phase 0 does not
terminate on a model with no reachable target). FND-002 to FND-005 are
medium. The owner rulings on well-definedness and undefined evaluation are
applied correctly and consistently with QSpec FR-391 and FR-392; the
findings are engine-soundness and alignment defects, not ruling defects.

## Dispositions

Round 1, reviewed at 666cfdb1ac85a7fc102c7414ab9ad6f307c56abe (`git diff b07f7b10...666cfdb1`; fix round ae35eac9, then 818c5bfb and 666cfdb1).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | ae35eac9 |
| FND-002 | fixed | ae35eac9 |
| FND-003 | fixed | ae35eac9 |
| FND-004 | fixed | ae35eac9 |
| FND-005 | fixed | ae35eac9 |
| FND-006 | fixed | ae35eac9 |
| FND-007 | fixed | ae35eac9 |
| FND-008 | fixed | ae35eac9 |
| FND-009 | fixed | ae35eac9 |
| FND-010 | fixed | ae35eac9 |
| FND-011 | fixed | ae35eac9 |
| FND-012 | fixed | ae35eac9 |
| FND-013 | fixed | ae35eac9 |
| FND-014 | fixed | ae35eac9 |
| FND-015 | fixed | ae35eac9 |

## New findings (disposition pass 1)

Reviewed at 666cfdb1ac85a7fc102c7414ab9ad6f307c56abe, lines of the PR diff only.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | medium | `check_state_graph` rejects with rules FR-338's `CertificateRule` does not have (`PredicateUndefined`, `RankMissing`, `RankBroken`, `OrderBroken`, `CountBroken`), and FR-169 does not add them. It names "the first failing state", a model state, where `CertificateLocus` holds a `ProductStateRef`. It also stops at a reached limit, which settles V-7, but its signature returns `Result<(), CertificateRejection>`, which has no stop arm. It twice cites `check_closure` as FR-127's; it is FR-338's. State the added rules and locus, and give the checker a return type that carries a limit stop, as FR-149 does with `Stopped`. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:97-98,135-160 |
| FND-017 | medium | FR-170's `Undefined` replay reproduces when the first undefined evaluation is at the stem's last state "with an undefined cause equal to the payload's". It never compares the predicate or the locus of `where`, which FR-168 and GV-7 now fill from QSpec FR-391. A payload naming another predicate or expression with the same reason reproduces. This is the defect fixed in FR-128 on #562 (SR-1120 FND-015). Compare `where`'s predicate and locus too. | spec/functional/FR-170-replay-state-graph-evidence.md:162-168 |
| FND-018 | low | ADR-022 cites a bare "FR-181" for the transition identity and canonical order (GX-1, and §3 and QS lines), while its own line 48 says a bare FR id is a QSL requirement. #571 adds a QSL FR-181 (the possible family), so the bare id resolves to the wrong artifact once the stack lands. Write "QSpec FR-181". | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:191,264,455 |
