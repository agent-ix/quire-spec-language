---
id: SR-20261109172912000001
title: "Integrity review of the L5 engine-gap spec batch"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; QSL-1819 (with QSL-1820); branch spec/l5-engine-gaps @ 9514a0e41 vs origin/main db995d817; ADR-029; FR-195; FR-201; FR-280; FR-281; FR-282; FR-286; FR-564; FR-565; FR-567; TC-761; TC-762; TC-763; TC-770; TC-1391; TC-1393; TC-1394"
review_set: subset
---

## Summary

Ticket: QSL-1819 (with QSL-1820). Consistency, atomicity and contradiction check of `git diff origin/main...HEAD` (FR-195, 201, 280, 281, 282, 286, 564, 565, 567; ADR-029 one line; TC-761, 762, 763, 770, 1391, 1393, 1394), measured against ADR-013/028/029, QSpec FR-290 and FR-331 (/home/peter/dev/quire-specification @ dfcc85bc), the code on origin/main (db995d817: qsl-analyze, qsl-replay/src/probability_certificate.rs) and origin/l5/p3-matrix (059b5cff2: qsl-conformance, docs/outcome-parity.md). Tools: quire 0.36.2 (engine 0.50.2), quoin 0.29.1; `quire validate` over the 17 changed files exits 0. No cargo builds were run.

## Verdict

Not mergeable. Verified clean: the ADR-029 line now names SourceProvision and NegotiatedItem, and FR-280's signature matches it. EN-5's manifest (probabilistic-satisfaction, bounded, {exact}, domains population and integer) in FR-195 and FR-280 equals qsl-analyze/src/exact_probabilistic/manifest.rs on main. Every BackendId en-1/en-4/en-5/en-6/en-7 equals the code's BACKEND_ID constants, and no `qsl.statistical` or `RoutedItem` reference is left in spec/. New AC ids (FR-201-AC-8, FR-280-AC-8/9, FR-281-AC-20 to AC-22, FR-286-AC-14) do not collide. Every test tag naming a changed AC (FR-281-AC-1, AC-2, AC-6) still resolves. The rotting counts are gone from FR-564-AC-4/6/8, FR-565-AC-1 and TC-1394. Splitting FR-281-AC-6 into (a) EN-1 LimitReached{max_states} and (b) EN-5 ExactProbMaxStates is sound: the Counter violation needs three states, so max_states 1 can fail it. FR-201-AC-1's values 24/25, 4/5, 1, 0 hold for the unbounded `eventually` variant of the §15.4 Link claim, because attempts is capped at 2 and so the product is acyclic. On FR-282-AC-1 (the author's open question): it must be fixed in this batch (FND-002). The batch's own FR-201 and FR-282 text now says EN-5 is checked in process over a non-emitting package, so the AC cannot pass as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new checked-subject paragraph opens with 'one set of conditions, two ways to obtain its subject', then limits the second way to the shape the current qsl-replay slice handles: unbounded eventually, acyclic, one initial state, at most one `over`, no fairness. It also routes every EN-5 item through that form. Every package with a random parameter is non-emitting (FR-185), which covers every §15 probabilistic model. So every EN-5 result that FR-197 to FR-201 specify through analyze then settles `unsupported`: a cyclic product, a fairness set (FR-200-AC-4's Deliver with fair weak, 24/25), LongRun (FR-201-AC-2), the Lower/ranking and FairTerminates certificates (FR-201-AC-3), quantile (AC-5) and expected reward. FR-281-AC-1's second sentence and FR-201-AC-8 make that limitation normative. This writes the current build limit into the spec as target design. Target: the checked-subject form applies every FR-201 condition. Today's slice limit belongs in the QSL-1820 ticket, not in the FR or an AC. | FR-201:132-148; FR-281:516 |
| FND-002 | high | FR-282-AC-1 cannot pass as written, so it must be fixed in this batch. It sends FR-281-AC-1's EN-5 certificate 'with the byte provision to the EN-5 checker in a fresh process that links only qualified-core crates'. That subject is the Link model, which has random parameters, so S4 refuses to emit it (FR-185). That leaves no ReplayRequestWire for FR-201's wire form. The checked-subject form is defined (FR-201, and FR-282 lines 45-49 of this diff) as running in process over the CheckedPackage that analyze holds. FR-282 line 52 still says every checker 'equally' accepts in a later separate run. Fix, as target design: state the fresh-process path for EN-5. The process re-checks the provision's source bytes (S1-S3, qualified core) into a CheckedPackage, admits the ModelSystem with the binding, and runs the checked-subject form. Then reword lines 45-53 and AC-1 to match. A second gap: FR-282's 'over a package S4 refuses to emit' disagrees with FR-201's 'for every EN-5 item'. | FR-282:45-53, 84 |
| FND-003 | high | This batch fixes EN-5's mode as `bounded` only (FR-195, FR-280). S3 records every probabilistic claim, and A-3 records every temporal clause, as extent Unbounded{domains naming the claim}. QSpec FR-290's mode table and FR-290-AC-6 say 'an unbounded extent on a bounded-only advertisement never settles supported': it settles requires-bound or unsupported/unbounded-extent. FR-281 line 361 now says the same: the disposition 'reaches analyze as RequiresBound', which is also incomplete because it can be unsupported. FR-280 line 82 says a Supported(candidate) item is one that negotiate_* settled supported. Yet FR-281-AC-1 (EN-5 Link), AC-2 (EN-1 Counter P), FR-280-AC-9 and the FR-564 dagger cells all need the item settled Supported(en-5) or Supported(en-1). negotiate_* can never produce that, so the dagger cells hold only with a hand-settled disposition, and the driver's prove never delivers them. The problem already exists for EN-1 and EN-4. This batch extends it to EN-5 and states it outright. Decide the extent or mode rule (for example, a bounded-mode engine accepts an unbounded infinite-trace extent once the subject's proof_bounds are finite), or advertise the mode that matches. | FR-281:361-362; FR-280:82-84, 161-165; FR-195:132-139 |
| FND-004 | high | AC-22 and the new ClaimItem bullet put an EN-1 item's integer-range ProofBound in `extent.bounds` and say those 'become the proof_bounds of the engine's subject'. In QSpec FR-331, a temporal item's extent is unbounded (infinite-trace). Its subject bounds are `subject.proof_bounds`, which are 'independent of extent': a substitution 'moves no entry into extent.bounds' and leaves the classification unchanged. A bounded extent's `bounds` cannot carry them. The new TemporalItem struct has no `subject` (proof_bounds), `abstract_subject`, `fairness`, `stutter_closure` or `smt_encoding`, although these are FR-331 temporal-item members and are part of request identity. Fix: add `subject.proof_bounds` (and the other FR-331 temporal members) to TemporalItem, and rewrite AC-22 and the extent.bounds bullet to bound the Integer field through `subject.proof_bounds`. | FR-281:295-300, 357-362, 537 |
| FND-005 | high | The counterexample set is closed too narrowly. ItemCounterexample.counterexample is 'an FR-364 TemporalCounterexample, or FR-181 assignments', and FR-286 spells only those two. FR-286 also says 'a refuted item with no ItemCounterexample is an internal fault'. QSpec FR-331 `counterexamples` has more members: a probabilistic refutation carries an FR-413 ProbabilisticCounterexample (EN-5's refuted §15.4 Deliver at 0.97, FR-200-AC-3, and FR-202 witnesses, which FR-281's own replay-request list names); a hybrid refutation carries a HybridCounterexample (FR-281's ItemForm::Hybrid); an operation-contract refutation carries a contract-counterexample (FR-485), not plain assignments; and a time-lock counterexample has `final_delay`. Each of these is either unrepresentable or an internal fault. Fix: make Counterexample the FR-331 union and give each member its spelling. | FR-286:171-189; FR-281:396-411 |
| FND-006 | medium | The batch gives the counterexample two homes. Outputs still says each record carries 'its counterexample or certificate where it has one' (AC-7: 'undefined as its counterexample'; AC-14: 'the record carries that counterexample'). The batch adds a separate ItemCounterexample 'for each refuted record whose engine reports one', which implies that a refuted record without a counterexample is legal. FR-286 makes that same case an internal fault. Settle one rule: either the record holds the FR-331 result's counterexample position only and the counterexample lives in ItemCounterexample, or the record holds it. Also say whether every refuted record must have one, and align FR-281 Outputs, AC-7, AC-14 and FR-286. | FR-281:396-427, 519, 528 |
| FND-007 | medium | The ClaimItem struct declares `pub form: ItemForm` with no Option, and ItemForm has only Temporal, Probabilistic and Hybrid. The prose says a value-validity or operation-contract item 'has no form' and carries its check settings in `options`. So the type cannot represent those items. Make `form` an Option, or add a Value/Contract variant. The struct also names ProbabilisticItem's limits `settings` where QSpec FR-331 uses `probabilistic.limits`, though the prose says it follows FR-331's names. | FR-281:278-287, 343-344 |
| FND-008 | medium | The rules conflict for a non-emitting package. The request-level check refuses the whole request (AnalyzeRefusal::Provision) when the provision lacks any source label the package names. Lines 180-185 say an item over a non-emitting package reads no provision and 'a package that does not emit is not refused for that'. It is unclear whether a request over Service or Link with an empty provision is refused. FR-280-AC-5 (Service through provider::run) and TC-761 step 4 pass no SourceProvision, although FR-280 makes it a mandatory input. State whether the provision check applies to every request, and give AC-5 its provision. | FR-281:169-185; FR-280:206 |
| FND-009 | medium | The Counter fixture is the matrix's byte input but is not pinned. It gives 'a bounded integer type with minimum 0' with no maximum, and 'an operation inc that adds one to value'. The existing test/counter package (origin/l5/p3-matrix qsl-conformance/cases/counter-package.json) has Level min 0 max 3, and an `inc` with an empty `post` and only a `modifies` frame, so it sets value to any Level in one step. If that package is reused, 'two inc steps break P' is false: one step reaches 2. The described prefix and replay therefore differ from what the bytes give. Pin the maximum (3), and either add `post value = pre(value) + 1` to the package or describe the havoc `inc`. Say that this is the existing counter package and counter-monitor unit, or a new one. | FR-564:97-107 |
| FND-010 | medium | FR-280-AC-9 cites 'AC-11's EN-1 item over the model with an Integer field'. Read inside FR-280, AC-11 is FR-280-AC-11, which does not exist (FR-280 ends at AC-9). The intended item is FR-281-AC-11's. Qualify the reference as FR-281-AC-11. | FR-280:210 |
| FND-011 | low | The new section says a TemporalCounterexample is 'spelled as the Monitor documents table spells the Violated row's: kind, initial_state, prefix, loop and trace_position'. The Violated row (line 144) spells only prefix, loop and trace_position, with no `kind` or `initial_state`. Spell the analyze members directly, or cite QSpec FR-364. | FR-286:144, 176-182 |
| FND-012 | low | FR-201-AC-8 sits between AC-7 and AC-6 in the table. 'an Unsupported* error' is a wildcard, not a closed set: the code's errors are Cycle (not Unsupported*), UnsupportedInitial, UnsupportedBinding, UnsupportedClaimForm, UnsupportedDraw and ClaimNotBound (qsl-replay/src/probability_certificate.rs on main), and no fairness check exists there. Name the error variants, and move AC-8 after AC-7. This becomes moot if FND-001 removes the shape limit. | FR-201:146-148, 203 |

## Coverage

- ADR-029 (examined)
- FR-195 (examined)
- FR-195-AC-5 (examined)
- FR-201 (examined)
- FR-201-AC-1 (context_only)
- FR-201-AC-8 (examined)
- FR-280 (examined)
- FR-280-AC-2 (examined)
- FR-280-AC-5 (examined)
- FR-280-AC-8 (examined)
- FR-280-AC-9 (examined)
- FR-281 (examined)
- FR-281-AC-1 (examined)
- FR-281-AC-2 (examined)
- FR-281-AC-6 (examined)
- FR-281-AC-11 (examined)
- FR-281-AC-20 (examined)
- FR-281-AC-21 (examined)
- FR-281-AC-22 (examined)
- FR-282 (examined)
- FR-282-AC-1 (examined)
- FR-286 (examined)
- FR-286-AC-14 (examined)
- FR-564 (examined)
- FR-564-AC-4 (examined)
- FR-564-AC-6 (examined)
- FR-564-AC-8 (examined)
- FR-565-AC-1 (examined)
- FR-567-AC-6 (examined)
- TC-761 (examined)
- TC-762 (examined)
- TC-763 (examined)
- TC-770 (examined)
- TC-1391 (examined)
- TC-1393 (examined)
- TC-1394 (examined)
- FR-185 (context_only)
- FR-200-AC-4 (context_only)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | FR-280 says EN-1's manifest lists 'exactly one (kind, mode) pair, (temporal-satisfaction, unbounded)'. ADR-018 §3 (line 308), FR-134 (line 28) and FR-126 Dependencies (line 565) still say EN-1 advertises both (temporal-satisfaction, bounded) and (temporal-satisfaction, unbounded). The two statements conflict. Under QSpec FR-290's table, an unbounded-only candidate already takes a bounded extent through its own arm, so 'unbounded only' is sufficient. Update ADR-018, FR-134 and FR-126 to the single unbounded pair (or have FR-280 list both), and check FR-134's TC-535 vectors. | FR-280:149-156; ADR-018:307-309; FR-134:27-29; FR-126:564-566 |
| FND-014 | low | FR-280 Outputs ('the ItemCounterexample of each refuted record whose engine reports one') and the Behavior bullet ('with the counterexample of each refuted record that has one') still imply that a refuted record may have no counterexample. FR-281 now requires exactly one ItemCounterexample per refuted record, and FR-286 makes a missing one Err(MissingCounterexample). Say 'exactly one ItemCounterexample for each refuted record' in both places. | FR-280:130-133, 178 |
| FND-015 | medium | The new Counter fixture is 'a fixture this requirement defines, with its own bytes', but it reuses the domain package identity test/counter. The matrix's check and monitor cells already ship test/counter 1.0.0 (counter-package.json on origin/l5/p3-matrix), whose inc has no pre or post. One case set would then hold two different packages under the same identity and version, and FR-565-AC-2 checks source identities. Give the fixture its own package identity (for example test/counter-step), or state that the shared counter package gains inc's pre and post and that the existing check and monitor cells still give their kinds over it. | FR-564:100-112; qsl-conformance/cases/counter-package.json (origin/l5/p3-matrix) |
| FND-016 | low | FR-282 ('a package S4 refuses to emit, which is every EN-5 subject') and FR-201 ('and every EN-5 subject has one') state as fact that every EN-5 subject has a random parameter, workload or reward. A probabilistic claim over a model with only nondeterministic choice and no random declaration is an EN-5 subject that S4 emits, and FR-282 keeps the wire form for exactly that case. Drop the 'every EN-5 subject' clauses. The routing rule ('analyze runs the checked-subject form for every EN-5 item') can stand on its own. | FR-282:47-49; FR-201:142-143 |

## Dispositions

Round 1, head b8ff9646e.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | Passable: the fresh process re-checks the provision bytes (S1-S3) into a CheckedPackage, admits the ModelSystem with the binding, and runs the checked-subject form; FR-282 Inputs now take the binding. |
| FND-003 | still-open | Fixed for EN-1 and EN-5, which now advertise unbounded; that matches QSpec FR-368 (infinite-trace classifies unbounded) and FR-290-AC-6. EN-4 is still bounded-only (FR-280:145, FR-280-AC-4, TC-761 step 3), yet FR-195 records every probabilistic claim, the Steady quantile included, as extent Unbounded. The batch's new FR-281 bullet says such an item on a bounded-only candidate reaches analyze as requires-bound or unsupported. FR-280:82 defines Supported(candidate) as negotiation's output. FR-280-AC-5 (Service), FR-281-AC-12 and FR-564's analyze timeout and cancellation cells all use Supported(en-4), which negotiation never gives. Fix in this batch: EN-4 advertises (probabilistic-satisfaction, unbounded) with {statistical}, updating FR-280:145, FR-280-AC-4 and TC-761 step 3. EN-6 is consistent: FR-239 already lists both modes. |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | FR-281 and FR-286 now agree. FR-280 still has a residual, filed as new FND-014. |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | Maximum and inc postcondition pinned. The package identity collides with the existing conformance package, filed as new FND-015. |
| FND-010 | fixed | - |
| FND-011 | fixed | - |
| FND-012 | still-open | The Unsupported* wildcard is gone (AC-8 rewritten, FR-201 applies every condition). FR-201-AC-8 still sits between AC-7 and AC-6 (FR-201:203-204). Move it after AC-7. |
