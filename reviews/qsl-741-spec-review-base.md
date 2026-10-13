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
