---
id: SR-12887
title: "Base spec review of STD-162 state-graph instancing move (PR #981)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #981 (git diff origin/main...HEAD at 12778aab4); spec/functional/FR-165-check-state-graph-claims-at-s3.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md, spec/test-cases/TC-590-s3-checks-state-graph-claims.md, spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-590
    type: reviews
---

## Summary

Ticket: STD-162. Base checklist review of the five edited artifacts. The
intended code behaviour is PR #968 (QSL-535, branch
qsl-535-e11fe-state-graph-claims, open). Examined: FR-165 Parsing (S1), the
Forms bullet on `over`, the three Predicates bullets, FR-165-AC-1, AC-4 and
AC-5; FR-125 Instancing and FR-125-AC-7; TC-520 step 6 and its expected
result; TC-590 step 1 and step 4 and their expected results. Context only:
ADR-018 §6 (the ConfigVersion unit; `attemptUpdate` has no parameters),
ADR-022 §7.1 to §7.3, FR-166 Inputs, FR-168 Inputs, and the PR #968 S1
grammar and S3 checker (`qsl-cst/src/grammar.rs`,
`qsl-semantics/src/check/state_graph_claim.rs`).

Oracle question from the brief (would the TCs fail if instancing stayed at
S3?). TC-590 step 1 would not: "each with its `over` binding recorded" is also
true of an S3 that records the binding and builds instances. Only FR-165's
Outputs struct, which has no instance member, pins the S3 side. TC-520 step 6
would fail only if the one-object case reuses the checked claim from the
two-object case, and FR-125-AC-7 does not say that (FND-002).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-165-AC-5 names "the operation parameter `newVersion`", but no TC-590 fixture has such a parameter. ADR-018 §6's `attemptUpdate`, which ADR-022 §7.1 reuses, has no parameters, and §7.2 and §7.3 declare none called `newVersion`. The test then only checks that an unknown name gives `missing-name`. It cannot fail for an S3 that looks up the model's operation parameters and refuses a real parameter read as `expression-form`, which is the case the AC exists to exclude. Fix: give TC-590 a fixture operation with a parameter (for example a variant `attemptUpdate(newVersion: Int[0, 1000])`), and name it in the AC and the TC fixtures. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:157; spec/test-cases/TC-590-s3-checks-state-graph-claims.md:23-24,35-38 | wrong-requirement |
| FND-002 | medium | In FR-125-AC-7, "the same claim over a subject whose universe holds one object, with `over`" has no clear referent. The nearest claim, `Anything`, has no `over`. The AC also does not require one checked claim reused across both subjects. That reuse is the only part of the AC that shows instancing happens at request time. A test that checks again per subject passes even if S3 instantiated from the universes. Fix: "`ReachesTwo`, checked once, yields two instances (`a`, `b`) over `{a, b}` and one (`a`) over `{a}`, from the same `CheckedStateGraphClaim`". Say the same in TC-520 step 6. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:162; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md:31-33,49-51 | wrong-requirement |
| FND-003 | low | FR-166 Inputs ("each `Possible` item (FR-165) with its instances") and FR-168 Inputs ("Each state-graph item (FR-165) and its instances") still say FR-165 produces the instances. Cite FR-125 for the instances. | spec/functional/FR-166-sample-witnesses-for-possible-claims.md:76-77; spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:46 | wrong-requirement |
| FND-004 | low | FR-165-AC-4 writes two of its three vectors as `not eventually ...` and `(eventually ...)`, and TC-590 step 4 gives none of them, so the operand and the span of `eventually` are left to the test author. Spell out all three predicates. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:156; spec/test-cases/TC-590-s3-checks-state-graph-claims.md:35-38 | wrong-requirement |

## Verdict

No high finding. Mergeable once FND-001 and FND-002 are fixed: both are
wording or fixture fixes in FR-165-AC-5, FR-125-AC-7 and their TC steps. The
two lows can go in the same round.

Checked and sound: the move is internally coherent. FR-165's Outputs already
had `over: Option<OverBinding>` and no instance member, and the old "check to
one instance per object" bullet contradicted that struct. The new Forms
bullet removes the contradiction. FR-125 Inputs now name the checked
state-graph claim, and the FR-165 dependency was added. The deadlock-freedom
item (FR-165-AC-3, FR-124) is per subject and does not depend on instancing.
FR-165-AC-4's no-`over` vector and the three temporal-operator vectors match
the new Parsing and Predicates bullets. Against PR #968: the parameter read
(`missing-name`), the `result` read (`ExprNode::Result`, an anchor, refused
as `expression-form`), the temporal operator (parsed at S1 by
`StateGraphPredicate`/`StateGraphFormula`, refused at S3) and no S3
instancing all agree with this PR. The one difference is `over`: #968
requires `over` in all three forms (`x("over"), bound_parameter()`) and
stores `over: OverParameter`, not an `Option`. #968 must make `over`
optional to satisfy FR-165-AC-4. FR-165 merges cleanly with #968's own
FR-165 edit (git merge-tree).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 2fbe63ee5: the TC-590 fixture adds `setVersion(newVersion: Int)`, and FR-165-AC-5 refuses `c.versionNumber = newVersion` with `missing-name` while `pre(...)` stays `expression-form`, so an S3 that looks up operation parameters now fails the test. |
| FND-002 | fixed | 2fbe63ee5: FR-125-AC-7 now says "`ReachesTwo`, checked once, yields `a`, `b` over `{a, b}` and `a` over a subject whose universe is `{a}`, without being re-checked", and TC-520 step 6 reuses the same checked claims. |
| FND-003 | fixed | 2fbe63ee5: FR-166:77 and FR-168:46 now cite FR-125 for the instances. |
| FND-004 | fixed | 2fbe63ee5: FR-165-AC-4 spells out all three predicates in full. |
