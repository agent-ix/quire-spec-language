---
id: SR-11690
title: "Spec review of quire-spec-language PR #889: frame nodes per anchor, scope order tiebreak, receiver forms on process operations"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1ff815056e31c52e96ecd24bae1e1732ca140a46; spec/functional/FR-105-emit-state-nodes.md, spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md, spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/test-cases/TC-1043-receiver-scoped-frame-entries-and-scoped-deletes-in-the-frame-node-and-decision.md"
review_set: subset
---
# Spec review of quire-spec-language PR #889

## Summary

Ticket: QSL-807 (follow-up). One pass of spec-review with the contradiction,
ambiguity, completeness, testability and spec-integrity sub-analyses over the
PR diff (`origin/main...HEAD`). Spec-only PR, so no code review. Only major
issues block; mediums and lows are recorded in the Verdict.

Oracles: QSL FR-092 (structural-node preimage), FR-103, FR-104 (frame
records, AC-5), FR-105, FR-114, FR-115-AC-11, FR-154, FR-305, TC-1043 on
main. QSpec on main and on the companion branch: model-complete.md Frames
row, FR-013, FR-208, FR-322 (occurrences), FR-340, FR-342,
interface_004. filament-core-data `schema/semantic/v1/semantic-ir.schema.json`
on main (`frame`: closed, required `modifies`/`creates`/`deletes` only).

Examined:
- FR-105 Behavior (anchor/frame emission, entry order), FR-105-AC-7 (examined)
- FR-114 Description (examined)
- FR-154 Inputs, Behavior: Intake receiver rule, Emission order, FR-154-AC-8, FR-154-AC-10 (examined)
- TC-1043 fixture (Flow), reachability notes, step 2 B3, step 3 I13 to I17, step 4 X8/X9, Results (examined)
- FR-104 frame-record rule and FR-104-AC-5, FR-092 preimage, FR-305 Behavior (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1043's reachability notes mark the new vectors wrongly. Every package in the test is emitted from the one fixture, which declares `bump`, `merge`, `own`, `drop` and `dispose` with scoped members; the filament-core-data semantic-IR validator runs on the whole model document before intake and its `frame` schema is closed over `modifies`/`creates`/`deletes` today, so no package built from the fixture validates. Yet the note says "`noop`, `all`, B1 and B2 ... are reachable today" (B1 is the fixture without `all`, so it still holds `bump` and `merge`), and the new B3 (FR-105-AC-7) and the new X8/X9 (edits of `bump`'s W1, FR-154-AC-10) are not marked as depending on the schema change at all. Fix: mark B1, B2, B3, step 1 and X1 to X9 as depending on the schema change, or give the reachable-today vectors their own fixture without the scoped operations. | spec/test-cases/TC-1043-receiver-scoped-frame-entries-and-scoped-deletes-in-the-frame-node-and-decision.md:40,88-92,105-108 |

## Verdict

FAIL, one blocking finding (FND-001).

Traced and correct:
- (a) FR-105, FR-114 and FR-104-AC-5 now agree: one anchor per (declaring
  type, operation); the frame node is keyed by FR-092's structural preimage
  (tag, form, `semantic_type`, body; no owner, as a frame is not
  model-owned), so equal frames of one type are one node. B3: `own` and
  `all` both lift to W5 with `semantic_type` Counter, so one frame node; the
  two anchors differ by `operation`, so two anchor nodes both depending on
  it; `DoneOwn`/`DoneAll` reference different anchors, so two clause nodes.
  FR-105's occurrence row (ascending declaring key, operation name) gives
  `all` before `own`, matching the expected result and FR-104's one record
  per operation. QSpec FR-342 as amended refuses only a duplicate (context,
  operation) pair and admits the shared frame; FR-322's one-or-more
  occurrences holds (two). The QSpec and QSL texts say the same.
- (b) QSpec FR-340's order key (digest, name, scope every-object first) is
  mirrored in FR-105 and FR-154. The "no admitted frame has equal
  (declaration, name) keys" claim holds: equal scope is a schema duplicate,
  differing scope is conflicting-binding, and relationship entries name a
  relationship node, so never tie with a field entry. X8 is misordered
  (receiver before every-object), X9 is ordered; both refuse
  conflicting-binding because every entry resolves and the conflict
  outranks the order defect.
- (c) FR-154's process receiver rule matches the QSpec Frames row and FR-013
  (shapes, then receiver rule with `modifies_receiver` first, then entries);
  I13 to I17 match QSpec TC-944 R15 to R18 and R20. They are intake rows on
  `go`, which no clause names, so they need no anchor. FR-103 admits
  process operations.
- No test parses the touched files (only a comment names FR-105). No
  tracking/hash ceremony. No new ticket ids in spec text. FR-105-AC-7, I13
  to I17 and X8/X9 collide with no open PR.

Recorded (medium, not blocking):
- FR-154's receiver rule names only process operations; QSpec FR-013 and the
  Frames row also refuse on state-machine operations. QSL does not say what
  intake does with a state-machine operation's frame.
- ADR-012's ProtocolClause table still says frame nodes are "one per
  operation (FR-105)"; with shared frame nodes this now reads wrong.
- TC-1043 gives `go` no `frame` in the base fixture (read as the empty
  effect under FR-103, but unstated).
