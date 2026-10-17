---
id: SR-11687
title: "Gap analysis — QSL-798 Text admission, witness String row and String replay (PR 887)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@178ecdca85e971037417e5559f7fe4dce9500234; FR-106-AC-33..38, FR-070-AC-14/15, FR-098-AC-13; TC-1038, TC-905 steps 7-8, TC-1028 steps 6-7"
review_set: subset
---

## Summary

Ticket: QSL-798 (code half, partial). This is a planless gap analysis of the PR diff against the merged criteria. Plan completion: not assessed. No build was run, and the matrix was read from the trace tags in the diff.

## Verdict

**PASS: no blocking findings.**

- FR-070-AC-14: every String and Text decode vector in the AC has a test, including the two-entry `P` then `Q` block and the `replay.input_bytes` length.
- FR-070-AC-15: every refusal vector in the AC has a test, including the JCS-ordered extra-member row `{"extra":"y","kind":"string","text":"x"}`, the member-order row, the `é` escape, the lone surrogate and the raw `%`.
- FR-098-AC-13: E1-E5 and their `!=` counterparts settle as the AC states. True rows settle `inconclusive` with cause `Verdicts`; false rows settle `reproduced-without-witness` with no disagreement. W1-W5 refuse `WrongValueKind` at the stated position from both the `Input` arm and the `Witness` arm.
- PR body: it says plainly that this PR is partial. It lists TC-1038 steps 1, 3 and 7-10 and FR-106-AC-37 as not done, and names the cause: intake cannot reach a Text slot (FR-056-AC-11).
- Trace tags: no tag claims FR-106-AC-37, FR-106-AC-38 or QSL-798 as done.
- Unit tests: each Text admission test's doc comment names its TC-1038 step and calls `admit_native` directly, so none reads as an end-to-end test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Recorded. The new tags on FR-106-AC-33 and FR-106-AC-36 make the computed matrix show those ACs as covered, but the tests cover only the admission slice. Untested: AC-33's document-level parts (same observation and digest under member reorder, the digest of the escaped pair `😀`, counting toward `observation.values`), and AC-36's `name`/`result`/`PostStateRange::Witness` rows. Keep QSL-798 open until FR-056-AC-11 lands; do not close it from the matrix. | qsl-semantics/src/model/observation/document.rs:2918,3077 |
