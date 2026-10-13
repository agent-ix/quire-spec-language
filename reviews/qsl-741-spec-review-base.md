---
id: SR-8708
title: "Spec review (base) of quire-spec-language PR #805 (QSL-741): observation.values refusal code"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3f013f617751966548c21596b1a3c53652a3400d; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md, spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md"
review_set: subset
---
# Spec review (base) of quire-spec-language PR #805

## Summary

Ticket: QSL-741. The PR changes four spec texts:
- FR-261 Behavior 4 and FR-261-AC-3
- TC-733 step 3
- FR-255 Behavior 1
- FR-106's values-per-document behaviour

They now name `resource_exhausted`/`insufficient-next-charge` for `observation.values`, as FR-106-AC-16 already did. Context read:
- QSpec `native-diagnostics.md` and its `resource_exhausted` and `stage_limit_exceeded` rows
- QSpec `value-accounting.md`
- QSpec FR-272
- FR-122
- ADR-012 §15.6

Examined:
- FR-261 Behavior 4 (examined)
- FR-261-AC-3 (examined)
- TC-733 step 3 (examined)
- FR-255 Behavior 1 and the setting table row `observation.values` (examined)
- FR-106 values-per-document behaviour and FR-106-AC-16 (examined)
- FR-122 executor admission bullet (context_only)
- ADR-012 §15.6 outcome table (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-261 Behavior 4 sentence ("`observation.values` is an accounting counter") contradicts three other texts. FR-255 says an execution budget under `quire.value.accounting/v1` "has no row in this table" and is not set through `--limit`, but `observation.values` has a row (kind "node count") and is set through `--limit`. FR-122 says "no `quire.value.accounting/v1` counter names an admission limit". QSpec `value-accounting.md` defines no observation counter. The catalog requires an `insufficient-next-charge` refusal to retain an "accounting contract", and no spec says which contract `observation.values` belongs to, so the code names `quire.value.accounting/v1` (SR-8706 FND-002). Fix: give FR-106-AC-16 as the reason for the code, not "accounting counter", and name the contract the refusal retains (for example `ObservationLimits`). Alternatively, amend FR-255's paragraph and FR-122 so they agree. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:56-58; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:80,113-117; spec/functional/FR-122-replay-a-state-clause-counterexample.md:146 |
| FND-002 | medium | FR-106 and FR-255 now list different field sets for this record. FR-255 Behavior 1 says the `observation.values` outcome "carries the same four values", that is `kind`, `bound`, `actual` and `setting`. FR-106's new parenthetical lists the stage, accounting contract, effective limit, prior usage, work item, setting and count reached, with no `kind`. The code follows FR-106 and fails FR-255-AC-1 (SR-8706 FND-001). FR-255's table also keeps kind "node count" for a row whose outcome is now `resource_exhausted`. Fix: add `kind` to FR-106's list, so one field set is stated in both places. | spec/functional/FR-106-admit-snapshots-and-invocations.md:400-406; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:127-135 |
| FND-003 | medium | ADR-012 §15.6's outcome table still says "admission (FR-106) \| a document over a reader limit \| refusal \| `stage_limit_exceeded`". A document over `observation.values` now refuses `resource_exhausted`, so the row is stale. Its category column also says refusal, while `Code::category` gives incomplete for both codes. Fix: split the row, or name both codes and the incomplete category. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1347 |
| FND-004 | low | No spec names the record's fields for the five retained items. FR-106 gives prose only ("the stage, the accounting contract, the effective limit, the prior usage and that value as the work item"). So `accounting_contract`, `prior` and `work_item` exist only in code, and the tests assert names no requirement defines. QSpec's accounting record already names these items `limit`, `consumed` and `charge_point`. Fix: name the record fields in FR-106, and prefer the existing names (SR-8706 FND-003). | spec/functional/FR-106-admit-snapshots-and-invocations.md:400-406 |
| FND-005 | low | FR-261 Behavior 4 states "`observation.objects` is a stage limit" with no criterion that separates it from `observation.values`. Both are per-document counts charged as the tree is walked. The planner's catalog argument (QSpec defines `stage_limit_exceeded` for S1-S4, I2, family `check`, replay and route, and not observation admission) applies to objects equally. Leaving objects alone is what the ticket asks. But the sentence asserts a classification the spec does not ground. Fix: drop the clause, or state what makes objects a stage limit. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:56-58 |

## Verdict

FR-261-AC-3 and TC-733 step 3 now agree with FR-106-AC-16 on the code, bound 50,000, prior 50,000 and count 50,001. That was QSL-741 AC 3. The AC is atomic and testable.

FR-106's parenthetical makes the catalog's five retained items explicit.

The defects are about consistency:
- the "accounting counter" rationale conflicts with FR-255 and FR-122 (FND-001)
- the two field lists differ (FND-002)
- one ADR row is stale (FND-003)

Only `base` was run. EARS, integrity and failure-domain were not run, because the hunks are clause-level rewordings of existing requirements and add no requirement, object or relationship. **Fix FND-001 and FND-002 before merge.** They decide the code's FND-001 and FND-002 in SR-8706.

## New findings (disposition pass 1)

Reviewed at `08dcb08376af112b261ef7196049eeef6cd5d1e2`. The PR now states `stage_limit_exceeded`/`node-count-exceeded` in FR-106 Behavior and AC-16, FR-261 Behavior 4 and AC-3, and TC-733 step 3. FR-255 and ADR-012 are unchanged from the base.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-261 Behavior 4's new sentence is wrong in two ways. It says `observation.objects` and `observation.values` each "has a row in FR-255's setting table, so a count past its bound refuses `stage_limit_exceeded`/`node-count-exceeded` naming the setting, as FR-106 states". (1) Having a row does not decide the code. FR-255 Behavior 1 keeps `resource_exhausted` for the format, intake and library rows. What decides it is that observation admission is a stage whose requirement gives its limits no other code. (2) FR-106 states the code for `observation.values` only. It says nothing about `observation.objects`. Fix: "Each is a stage limit under FR-255 Behavior 1, so a count past its bound refuses `stage_limit_exceeded`/`node-count-exceeded` naming the setting", and drop "as FR-106 states", or add the objects case to FR-106. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:53-58; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:121-131 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | rejected | mooted by redirect: the "`observation.values` is an accounting counter" sentence is gone. FR-261 Behavior 4 now gives the FR-255 row as the reason (see FND-006 for its wording), and no accounting contract is named anywhere. |
| FND-002 | fixed | 08dcb08376af112b261ef7196049eeef6cd5d1e2: FR-106 no longer lists a competing field set. It names the cause `node-count-exceeded` (the kind), the setting, the bound and the count reached, with a reference to FR-255, which matches FR-255 Behavior 1's four fields and the table's "node count" kind. |
| FND-003 | still-open | The code half is mooted: the ADR-012 §15.6 row's `stage_limit_exceeded` is right again. The category half of the finding stands, and the PR body's "ADR-012 section 15.6 is already right" misstates it. The row "admission (FR-106) \| a document over a reader limit \| refusal \| `stage_limit_exceeded`" gives O-16 category refusal. But `Code::is_incomplete` includes `StageLimitExceeded` (qsl-foundation/src/diagnostic.rs:300-309). ADR-013 O-16 puts bound exhaustion under incomplete, and the compile row of the same table says "refusal or limit \| refusal, incomplete". Fix: the row's category cell becomes `incomplete`. |
| FND-004 | rejected | mooted by redirect: the record has no fields beyond FR-255's `kind`, `bound`, `actual` and `setting`, which FR-255 Behavior 1 names. |
| FND-005 | fixed | 08dcb08376af112b261ef7196049eeef6cd5d1e2: the ungrounded "`observation.objects` is a stage limit" clause is replaced. Its replacement has its own wording defect, recorded as FND-006. |
| FND-003 | fixed | 543558986b1dfcbe7b18d3ad0cc6b744298edfa3: the ADR-012 §15.6 row now reads "admission (FR-106) \| a document over a reader limit \| incomplete \| `stage_limit_exceeded`". |
| FND-006 | fixed | 543558986b1dfcbe7b18d3ad0cc6b744298edfa3: FR-261 Behavior 4 now reads "Each is a stage limit under FR-255 Behavior 1: a count past its bound refuses `stage_limit_exceeded`/`node-count-exceeded` naming the setting." |
| FND-007 | fixed | 5ff49edaf955fbb44b7508f2527b35d7ae4a0264: FR-255 Behavior 1 is now keyed by carrying type and cites ADR-014 §1 and ADR-013 T-4. Observation admission's re-run walks are listed under `resource_exhausted`, and the code now emits it (`caller_budget_refusal`). The replay front end still disagrees; recorded as SR-8706 FND-008. |
| FND-008 | fixed | 5ff49edaf955fbb44b7508f2527b35d7ae4a0264: `explore.*` has its own row: "no catalog code: exploration returns `Outcome::Bounded` (FR-101)". AC-1 excepts it. |
| FND-009 | fixed | 5ff49edaf955fbb44b7508f2527b35d7ae4a0264: `dependency.*` and `identity.input_bytes` are in the stage-limits row. |
| FND-010 | fixed | 5ff49edaf955fbb44b7508f2527b35d7ae4a0264: FR-255-AC-1 now reads "returns the code Behavior 1's table gives for the limit's carrying type ... and `kind` for `stage_limit_exceeded`". |
| FND-011 | fixed | 5ff49edaf955fbb44b7508f2527b35d7ae4a0264: "L57" is gone. The row names the causes: `ancestor-steps` or `family-steps` for a walk ceiling, `insufficient-next-charge` for a denied charge. |
| FND-012 | fixed | 8cdf7abd4900f9ea06b8a2a04df31bc35e12db02: FR-255 Behavior 1 now claims `kind`, `bound`, `actual` and `setting` only for `stage_limit_exceeded`, and the field set for observation admission's re-run walk. The compile and S6a gap is listed as follow-up (b) (QSL-745). AC-1 is narrowed to match. |
| FND-013 | deferred | QSL-745 (c): FR-255 Behavior 1 now says "the QSpec catalog does not list `ancestor-steps` or `family-steps` as causes of `resource_exhausted` (FR-150-AC-11 describes the shape as an incomplete record)". The PR body lists the same, and QSL-745 (c) asks for a planner ruling on the form before a QSpec PR. |
| FND-014 | fixed | 8cdf7abd4900f9ea06b8a2a04df31bc35e12db02: ADR-012 §15.6 gains "admission (FR-106) \| a re-run `model.*` walk past its `ModelNormalizationLimitsV1` ceiling \| incomplete \| `resource_exhausted`", and FR-106 gains the matching bullet. |

## New findings (disposition pass 2)

Reviewed at `543558986b1dfcbe7b18d3ad0cc6b744298edfa3`. The planner's ruling A is now in FR-255 Behavior 1 as a stage-to-code table. NFR-012, FR-082 and QSpec FR-150 are unchanged. QSpec #243 was read at `299a3672`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | FR-255 Behavior 1 now contradicts ADR-014 §1 and ADR-013 T-4. FR-255 says "The stage that refuses decides the code" and "Observation admission refuses `stage_limit_exceeded` even for the `model.*` walks it re-runs on a domain package: the stage that refuses is admission". ADR-014 §1's classification rule says: "A limit is classified by the type that carries it, not by the stage that reads it. A ceiling of an accounting-contract limits type is B-2 wherever it is read, including `ModelNormalizationLimitsV1`". B-2 yields `resource_exhausted`. ADR-013 T-4 says the same: "is the caller's meter wherever it is read (ADR-014 §1)". Observation admission's re-walk charges `ModelNormalizationLimitsV1.ancestor_steps` (observation.rs:1333, document.rs:1454-1458). So the ADRs require `resource_exhausted`, while FR-255 and the code (`limit_refusal`, document.rs:1466) give `stage_limit_exceeded`. Fix: a planner decision. Either amend ADR-014 §1 (and its B-2 row) and ADR-013 T-4 to the stage rule, with this carve-out, or reverse the carve-out. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:127-140; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:126-131; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:981 |
| FND-008 | medium | The table puts exploration (`explore.*`) under `stage_limit_exceeded`, and the PR's audit says it "agrees". Exploration never produces that code. Reaching `max_states` or `max_transitions` returns `Outcome::Bounded { stats, frontier, limit: Limit::States/Transitions }` (explore.rs:271-283, 358-382), category incomplete (explore.rs:204), with no catalog code and no `LimitExceeded`. That is FR-101's own contract (FR-101 Inputs: "names it and its value in `Outcome::Bounded`"). Fix: move exploration to its own row ("no catalog code: `Outcome::Bounded` naming the `Limit`, FR-101"), and correct the audit row. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:136; qsl-eval/src/simulation/explore.rs:195-207,271-283; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md:63-74 |
| FND-009 | medium | The table, offered as the single source, leaves out two stages that have setting rows. These are `dependency.*` (S4 source resolution) and `identity.input_bytes` (identity encoding). Both refuse `LimitExceeded`, so `stage_limit_exceeded`: see qsl-replay/src/spine/lifecycle.rs:1033-1085 and qsl-semantics/src/value/semantic_node.rs:232, mapped at qsl-replay/src/spine.rs:200 and 250-266. The catch-all "every other stage" names its stages and settings explicitly, and neither appears. Fix: add S4 source resolution and identity encoding, with `dependency.*` and `identity.input_bytes`, to the `stage_limit_exceeded` row. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:136; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:85-88 |
| FND-010 | medium | "No new AC; FR-255-AC-1 covers it" does not hold as AC-1 is worded. AC-1 binds each row to "the row owner's specified limit outcome", not to Behavior 1's table. For `model.ancestor_steps` the row owners (NFR-012, FR-082-AC-3) specify `resource_exhausted`/Incomplete. Yet the AC-1-tagged test `a_deep_hierarchy_is_judged_by_the_callers_ancestor_steps_not_a_fixed_ceiling` (state_clauses.rs:4265-4311) asserts `stage_limit_exceeded`/`edge-count-exceeded` at observation admission. So the table's code column has no criterion, and the one test of the carve-out is bound to an AC whose text says otherwise. Fix: reword AC-1 to "returns the code FR-255 Behavior 1's table gives for the stage that refuses", keeping the existing tests. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:185; qsl-semantics/tests/it/state_clauses.rs:4265-4314 |
| FND-011 | low | The model/admission row says "`resource_exhausted`, with the cause NFR-012 L57, FR-082-AC-3 and QSpec FR-150 name". "L57" is a line number, which drifts. The three sources also do not name one cause. QSpec FR-150 and TC-195 N01 give `insufficient-next-charge`. FR-082-AC-3 names `ancestor_steps` with no cause tag. The code's ancestor walk refuses with the cause `ancestor-steps` (model/index.rs:840-842, refusal.rs:1257). Fix: drop "L57", and say "with the cause its owning requirement names". | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:131 |

## New findings (disposition pass 3)

Reviewed at `5ff49edaf955fbb44b7508f2527b35d7ae4a0264`. QSpec #243 is unchanged at `299a3672`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | FR-255 Behavior 1 now says "Each outcome with a catalog code keeps it and carries the setting name as its `setting` field with `bound` and `actual`". AC-1 requires those fields for every row. That now covers `model.*` and `admission.*` wherever they are read, but outside observation admission their records carry no such fields. `ModelRefusalCause::catalog_fields` returns `None` for `AncestorSteps` and `FamilySteps` (refusal.rs:1149-1176). The S6a family refusal carries only `code` and `location` (qsl-replay/src/spine/call.rs:136-141, 856). So the records do not match. Admission's `resource_exhausted`/`ancestor-steps` record has `setting`, `bound` and `actual`; the same code and cause from population admission or a family call has none. Fix: add the fields at those producers, or limit the sentence and AC-1 to the stage-limit and format/intake/library rows. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:130-139,182; qsl-semantics/src/model/refusal.rs:1149-1176; qsl-replay/src/spine/call.rs:136-141 |
| FND-013 | medium | The table names `ancestor-steps` and `family-steps` as `resource_exhausted` causes, citing FR-082-AC-3 and FR-150. Neither QSpec source supports that. QSpec's catalog lists only `insufficient-next-charge`, `allocation-failed` and `reservation-size-overflow` under `resource_exhausted`, and no QSpec file on `main` contains `ancestor-steps`. QSpec FR-150-AC-11 gives an ancestor-walk ceiling as `incomplete(ancestor_steps,n-1,n-1,1,model.ancestor-edge)`, which is the `insufficient-next-charge` shape. FR-082-AC-3 names no cause tag. The compile path already used this cause; this PR now emits it from observation admission too, and states it in FR-255. Fix (planner): add the two causes to the QSpec catalog (in #243 or a sibling PR), or map walk ceilings to `insufficient-next-charge` with the incomplete record. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:131; qsl-semantics/src/model/refusal.rs:1257; QSpec native-diagnostics `resource_exhausted` row; QSpec FR-150-AC-11 |
| FND-014 | low | ADR-012 §15.6 has no admission row for the new caller-budget refusal. Admission can now return `resource_exhausted` (re-run model walks), but its only limit row is "a document over a reader limit \| incomplete \| `stage_limit_exceeded`". FR-106 does not mention the case either. Fix: add "a re-run model walk over the caller's `ModelNormalizationLimits` \| incomplete \| `resource_exhausted`". | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1347 |
