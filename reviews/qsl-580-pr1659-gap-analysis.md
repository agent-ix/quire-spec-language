---
id: SR-1659002
title: "gap-analysis of PR #1659 (QSL-580k) against FR-230 quantified reads and resets, FR-230-AC-21"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1659 git diff origin/qsl-580j-record-collections...369c03677, against FR-230 Selection, Quantified reads, Resets by selection and FR-230-AC-21, and STD-493 comment 9dd8a2ac items 24 and 25"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---
# SR-1659002: gap-analysis of PR #1659 (QSL-580k) against FR-230 quantified reads and resets, FR-230-AC-21

## Summary

Plan completion: not assessed. Tested AC-21 clauses:
- the `forall` reset with `e.n > 1`;
- the `forall` guard, one entry per size 0 to 3;
- the `exists` guard, one part per slot with "holds the slot" and `n = 2`.

Each is tested at S3 and in eval, tagged TC-685.

**Deferral item 24 (`p[i]`, `p[k]`): real, but the stated cause is half right.**
- **Right:** QSL FR-091 lists indexing `e[i]` as `UnrepresentedConstruct` at S2, and FR-091-AC-7 tests `xs[0]` refused. No S2 form exists: `ExprNode::Indexed` is the hyperproperty `read @ trace`, not indexing. So `self.ts[1].x` cannot reach S3, and FR-091 and FR-230 conflict.
- **Not accurate:** "no QSpec operation selects a collection element". QSpec FR-415 "Reset by selection" and "Reading", and FR-415-AC-12, define `p[i].x` and `p[k].x` as place selectors, with the pre-state index and the presence obligation. Only a value-level element-at is missing from `quire.op.collection.*`.
- **Who must act:** the ruling is QSL's own. FR-091 needs an indexing form for place selection. QSpec does not block it.

**Deferral item 25 (quantified time-invariant body): real.** The shared grammar gives `'{', clock-bound, { 'and', clock-bound }, '}'` (QSpec proposals shared-grammar.md:957), and QSL FR-393 `TimeInvariantForm.bounds: Vec<ClockBoundForm>` has no room for a quantifier. FR-230 and QSpec FR-415 both ask for a `forall` body, so the conflict is in the specs themselves.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230 "Quantified reads" says that "for an unbounded one the record is the family of those entries indexed by size". The PR refuses an unbounded collection as `TimedNotChecked` (probe: `exists` over an unbounded `Set<Timer>`). As FR-230 "Selection" says, a `Set` or `Bag` is read through `forall`/`exists` only, so an unbounded set of timers cannot be read at all. This is an undeclared deferral: it is not on STD-493 and not ticketed. | FR-230 Quantified reads; quantified.rs 126-131 |
| FND-002 | low | Correct the item-24 note. QSpec FR-415 defines the place selectors; the blocker is QSL FR-091's missing indexing form, a QSL-side decision. | STD-493 9dd8a2ac item 24; FR-091; QSpec FR-415 |
| FND-003 | low | FR-230 admits the quantifiers "in a clock-reading declaration", and for a collection below a record field (`p` a clock collection place expression). Both are listed only under "Not covered here", with no ticket or STD-493 entry. | FR-230 Quantified reads |
| FND-004 | low | FR-230 and QSpec FR-415 do not say in which state `P` of `forall(e in p: P(e) implies e.x = c)` is read. The PR reads the pre-state, matching the selector rule. This needs a spec sentence before 580l lets the same step rewrite the elements. | FR-230 Resets by selection |
| FND-005 | low | AC-21's `count<Int>(e in self.ts: e.x >= 1 ms)`, `self.a = self.b`, `contains` refusals and the `size(self.ts) > 1`/`e.n` data reads are not traced in this diff. Confirm that 580j's tests carry them, or tag them here. | FR-230-AC-21 |

## Verdict

Changes requested for FND-001. Either implement the size-indexed family for unbounded collections, or file it as a gap on STD-493 with a ticket, and state it in AC-21's partial coverage. The other findings are notes and ticket hygiene. Items 24 and 25 are real gaps; item 24's wording should be corrected.

## Coverage

- I re-read FR-230's Selection, Quantified reads, Resets by selection and AC-21, FR-091's S2 refusal list and OQ-5, and FR-393's `TimeInvariantForm`.
- QSpec at bd73a062: FR-415 "Reset by selection", "Reading" and AC-12, the shared grammar's `time-invariant`, and FR-145's operation table.
- The PR body and the STD-493 comment were treated as untrusted claims and re-measured.

## Dispositions

Round 1, fix commit a97e471b5.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The size-indexed family for an unbounded collection is moved to slice 580m by the coordinator's ruling. Record it there. |
| FND-002 | fixed | STD-493 item 24 is corrected in comment 0909829b, which I read back: QSpec FR-415 defines the selectors, and the gap is QSL FR-091's. |
| FND-003 | deferred | Quantifiers in clock-reading declarations, and below a record field, are moved to 580m by the coordinator's ruling. |
| FND-004 | fixed | The state `P` reads is filed as STD-493 item 27 (0909829b): the pre-state for now, pending a ruling. SR-1662001 FND-002 shows the related post-read conflict for collection writes. |
| FND-005 | fixed | Verified: 580j's `a_record_collection_reads_its_data_and_refuses_value_comparison` (TC-685, documented as part of FR-230-AC-21) covers `size(self.ts)`, data reads of `e.n` through a traversal, and the `count`, `contains`, `=`, `!=`, whole-element and `convert` refusals. |
