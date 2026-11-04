---
id: SR-20261104130234
title: "Base specification review — QSL-1642 timed population scope"
type: SpecReview
analysis: base
scope: "FR-230-AC-55, FR-391 timed-predicate paragraph, FR-480-AC-17, TC-1331"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

The refusal contract is observable and TC-1331 names negative and positive controls. The newly authored FR-230 criterion duplicates an existing acceptance-criterion id, invalidating its stable identity and trace link.

## Verdict

**FAIL** — FR-230 has two distinct criteria named FR-230-AC-55.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new timed-population criterion reuses FR-230-AC-55, already assigned to the quantified clock-guard criterion; update the new criterion and its references to a unique next id. | FR-230:879, FR-230:886 |

## Checklist

FR and TC id formats are syntactically valid, but the duplicate AC identity breaks uniqueness. The new expected results and named test controls are concrete. No hash, digest, pin, manifest, or file-tracking instruction was introduced. Whole-tree validation exits zero but did not report this duplicate.

## Dispositions

Round 1, reviewed against final head f9e37ff46e798976cb75b87bf15f5b6b2a6f8a67. The duplicate id is corrected: the existing quantified clock-guard criterion remains FR-230-AC-55; the named-population refusal now has the unique id FR-230-AC-58, uses TC-1334, and the implementation test carries both corrected trace tags. The focused timed-population integration test passed (1 test).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: In a timed model whose domain package declares calls, time invariant HasCalls when { forall(c in Config::calls : true) } { self.x <= 1 ms } and urgent CallsExist when { exists(c in Config::calls : true) } each refuse unsupported_construct/not-yet-implemented at Config::calls, naming the missing timed-predicate population scope, and produce no TimedModel. Time invariant and urgency predicates that read only self still check. The same named-population reads in a unit-level precondition remain admitted under FR-391. |
