---
id: SR-11685
title: "Spec review — QSL-522 receiver-scoped modifies and scoped deletes in the frame wire (PR 885)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3b405cf3001fbd8ffab872797755fa9b92b7546c; spec/functional/FR-103, FR-105, FR-115, FR-120, FR-154, spec/test-cases/TC-1043"
review_set: subset
---

## Summary

Ticket: QSL-522. One spec-review pass over `git diff origin/main...HEAD`. It covers contradiction, ambiguity, completeness, testability and spec integrity. Every TC-1043 row (W1-W6, I1-I8, X1-X8, D1-D5, F1-F3) was traced through the stated check order. The PR was checked against QSL FR-103, FR-105, FR-115, FR-120, FR-154 and FR-267, and against QSpec FR-013, FR-340, FR-342 and the model-complete Frames row. No build was run. `quire validate` passes, with one warning: FR-154 line 95, "ambiguous term `effective`". That line is new wording in this PR.

## Verdict

**FAIL: three blocking findings.** The parts that hold are listed here. The W vectors use JCS member order, and the `modifies` and `deletes_scoped` orders are right. D1-D5 and F1-F3 trace to their stated results. I8 follows from the rule that a conflict is checked only after every entry resolves. The author's choices (2) to (5) agree with QSpec FR-340/FR-013 and with the merged rule that admits `modifies <relationship>` unscoped. Choice (1), a witness that keeps declaration keys only, fits FR-267's declaration-key wire. The scope is recoverable from the frame node, so it is recorded, not blocking. No tests parse the touched tables. TC-1043 collides with no open PR. The spec text adds no ticket ids and no tracking ceremony.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | X4-X7 (FR-154-AC-10) cannot be reached as written. The step reads "W2 with `deletes_scoped` ..." and so on with the I2 reader. A body edited in place keeps its old node id, so FR-322's node-key check refuses it `invalid_package`/`stale-node-key` before the frame step runs (QSL FR-320-AC-4 states this refusal). The TC must say to re-key the frame node, the anchor that references it, the nodes that depend on that anchor, and the `package_id`. | spec/test-cases/TC-1043-...md:73-81 |
| FND-002 | high | I1-I8, FR-154-AC-8 and FR-154-AC-4 have no concrete intake input. QSL intake reads the model document's `Operation.frame` entries as IR node identity strings (`intake.rs` `frame_entries`). QSpec's model-complete Frames row allows exactly `modifies`, `creates` and `deletes`, and FR-013 and FR-340 define no model-document form for `self.f`, `deletes self` or `deletes p`. The rows use surface text, so a test cannot be written. Some results also depend on the form chosen: I2 `self.d` gives `malformed-declaration` only if the entry names `Gauge.d`'s node. The spec must state the source form, or name the QSpec requirement that fixes it. | spec/functional/FR-154-...md:88-112; TC-1043 step 3 |
| FND-003 | medium | W1-W6 are claimed byte for byte, but they write `"declaration":"<Counter>"` and `"deletes":["<Counter>"]` as quoted strings. A checked-package v2 NodeRef is the object `{"digest":...,"domain":"quire.checked-semantic-node/v1"}` (QSpec positive-clause-operations fixture). Swapping the placeholder in place gives wrong bytes. | spec/test-cases/TC-1043-...md:104-109 |
| FND-004 | medium | Recorded. X7 places two field entries with equal order keys. FR-340 and FR-154 do not say whether a non-strict ascending order is canonical, or whether `conflicting-binding` is checked before the order check. | TC-1043 X7 |
| FND-005 | medium | Recorded, existing before this PR. FR-105's frame row keeps one frame node for two operations whose frames have equal content. QSpec FR-342 refuses a frame shared by two anchors. TC-1043 step 2's variant package (`own` changed to `modifies value`, equal to `all`) builds exactly that shared frame. | spec/functional/FR-105-...md:67 |
| FND-006 | low | Recorded. Minor points: the type of `step` is not stated (D1 writes 1). The operation `t` in step 3 is not in the operations table. Seven clauses share the name `Done`. `ModifiesEntry{field,scope}` cannot hold a relationship entry, which has no scope. FR-154 line 95 has the `effective` validator warning. | TC-1043; FR-154 |
