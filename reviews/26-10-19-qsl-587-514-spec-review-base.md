---
id: SR-13061
title: "Base spec review of PR #1020: FR-251 AC-4 to AC-8 coverage (TC-1081) and FR-307-AC-6 (TC-1082)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@66ef5a3e3; git diff origin/main...HEAD: FR-251 AC table, FR-307 Behavior/AC-6/Dependencies, ADR-017 AR-4, TC-706, TC-1081, TC-1082"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1081
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1082
    type: reviews
---

## Summary

Tickets: QSL-587, QSL-514. PR quire-spec-language#1020. Review set: subset
(base, ears-conformance, integrity, scope-boundary). Base checklist over the
diff: ID formats, TC id uniqueness (TC-1081 and TC-1082 are unused on
origin/main at b41d0a960 and in the open PRs' file lists), index rows (none
exist: spec/tests.md was removed in #860, the matrix is computed), link and
relationship integrity (`quire validate` exit 0 on all six changed files), and
whether each TC step matches its AC. No hash, digest or pin use. TC-1081 steps
1 to 5 map one-to-one onto FR-251-AC-4 to AC-8 and cover every listed
substitution, control and comparison case. TC-1082 steps 1 and 2 match
FR-307-AC-6's two sentences.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-307's new rule "the bound items are exactly the items whose result carries bindings" (and AC-6's closing "the items carrying bindings are exactly the bound items") is false for a bound item that references no element: a clause that reads no model member, has no extent domain and is not a frame, precondition or postcondition node is not refused, and "returns every binding it references", which is none. A driver that selects "items whose result carries bindings", as the bullet invites, drops that item from the QSpec FR-331 request. Fix: partition by result kind (bound result versus refusal), or state that a bound item with no referenced element is bound with an empty binding set, and add that case to AC-6. | spec/functional/FR-307-export-the-bindings-each-item-references.md:79-82,93 |
| FND-002 | low | TC-1081 step 1 expects "No source parsing, clause argument, atom callback or RT temporal translation occurs", but the procedure gives no way to observe it (no instrumented parser or callback, no build without those components), so that expectation cannot fail as written. Name the observation, for example building the monitor in a context where the parser, callback and RT translation entry points are absent or counted. | spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md:21,31 |

## Verdict

Not mergeable until FND-001 is fixed or rejected with a reason (for example,
a rule elsewhere that every item references at least one element). FND-002 is
a low TC wording gap. TC-706's narrowed scope, the FR-251 Verification cells
and the ADR-017 AR-4 pointer are correct.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-307's Downstream dependency still says the driver "puts only the items that carry bindings in the QSpec FR-331 request". After the FND-001 fix, an item that references no element is bound with an empty binding set, so it carries no bindings, and this sentence still describes the driver dropping it. ADR-017 AR-4 says "only the bound items". Say "only the bound items" here as well. | spec/functional/FR-307-export-the-bindings-each-item-references.md:101-105 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 095366db6: the Per-item results bullet now says the export SHALL return each item as a bound result (an empty binding set when the item references no element) or a refusal. AC-6 adds "A requested item that references no model element returns a bound result with an empty binding set, not a refusal", and TC-1082 step 2 tests it. The residual wording in Dependencies is FND-003. |
| FND-002 | fixed | 095366db6: TC-1081 step 1 builds with poisoned source bytes, no clause argument, and atom-callback and RT-translation hooks that fail the test if invoked. The expected result asserts that none is read or invoked. |
