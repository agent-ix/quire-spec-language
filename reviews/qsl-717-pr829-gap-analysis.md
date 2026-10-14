---
id: SR-9078
title: "Gap analysis of quire-spec-language PR #829 against QSL-717 (FR-150..153 / TC-195..198 qualification)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@40dab6f61a2bb625206ef812459b3605b08a110b; PR #829 trace table and its new/retagged tests"
review_set: subset
---
# Gap analysis of quire-spec-language PR #829

## Summary

Ticket: QSL-717. Checks the PR body's trace table against the tests it names.
Plan completion: not assessed. Six covered rows were spot-checked, including
all four retags:

- `n01v2_*` → FR-150-AC-7: correct.
- `scalar_original_*` → FR-150-AC-9: correct.
- `r06_*` → FR-151-AC-9: partial.
- `y06_*` → FR-152-AC-8: partial.
- `l12_*` → FR-153-AC-10: correct.
- `r02_r03_r04_*` → FR-151-AC-4: correct.

The rows marked not covered (FR-150-AC-10, FR-151-AC-11..16, FR-152-AC-1/5/7,
FR-153-AC-11, C01-C07, Y07-Y09, L13) match the code: there is no meter
transfer across selections, a flat per-axis cost, no collector, no
navigation tests, and no profile gate. TC-195..198 are explicitly not claimed
complete.

## Verdict

One blocking finding. The trace table claims FR-152-AC-8 is "added" and that
TC-197 Y06 and Y10 are covered. Y10's second clause (a connection end naming
`Probe`) and Y06 as specified (a port owner that is an object type) are not
exercised. When run as specified, both give `dangling_reference`, not
`wrong-export`/actual `none` (see SR-9077 FND-001). The y06 test uses a
different mutation (`has_part_signature = false`, three refusals), not the
vector's owner change (two refusals). The other added tests back their rows
with the stated numbers.

Recorded only, not blocking (medium/low): the r06 retag asserts no member
declaration keys and no conforming-pair provenance for FR-151-AC-9. The C09
"no refund" clause is not asserted. FR-153-AC-8's mandatory `absent` spelling
is not asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Trace table claims FR-152-AC-8 "added" and TC-197 Y06 and Y10 covered. The connection-end half of AC-8 (Y10 clause 2) and the Y06 vector as written (port owner changed to an object type) have no test, and the production code gives `dangling_reference` for both, so the claim is false and the AC fails. | qsl-semantics/tests/it/model_intake.rs:958-1102, qsl-semantics/tests/it/model_systems.rs:937-1000 |
