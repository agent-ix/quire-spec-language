---
id: SR-13078
title: "Failure-domain review of PR #1023: model-subject lists and precedence"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: FR-147 internal list, FR-135 support set"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---

## Summary

Ticket: QSL-887. Walked the failure cases of the internal list: shape and
spelling defects, unresolved names, duplicates, library values, empty versus
absent, the model-subject list, and the interaction with the not-yet-built
refusals. Resolve-first precedence, the attributable-to-no-refinement case,
the empty list never refusing and the two-refusal case under
`abstract Spec::Twice` match QSpec FR-375 and TC-310 mutation 17.

Open item checked (not a finding against this PR): FR-147 limits a right
side's abstract node to attempt, event, send, receive or finish, while the
list can name timeout and fence nodes. QSpec FR-432 does not restrict the node
kind of `node <A>::<protocol>::<node>(…)`. The line is unchanged by this PR,
and "One class, one decision" correctly confines the conflict to classes a
right side can name, so the two rules agree whichever way QSpec settles it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 Precedence and the unbuilt feature | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 Defects table | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-7 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147-AC-11 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 right-side node kinds | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The model-subject reading is ambiguous. Precedence says every defect is reported whatever the build supports "except the model-subject row", then says a model-subject list's entries "resolve against no protocol, so only the not-yet-built refusal is reported for a well-formed non-empty list". Under the Defects table, `Spec::Twice::finish` on `CasRefinesCounter` is an entry whose protocol is not the selected protocol, i.e. `missing-name`, which by precedence would be reported and suppress the not-yet-built refusal, contradicting AC-7. The text does not say which defects are checked on a model-subject list: spelling only, or also alias mismatch (`Other::Twice::finish`), unknown memory kind, repeated spelling or two objects. State it, e.g. "For a model-subject refinement, entries are checked for shape, spelling and repetition only; alias, protocol and name resolution are not applied", and add one model-subject vector each for a wrong alias and a repeated spelling to AC-11. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:183-197 |
| FND-002 | low | The Defects row "a non-empty list for a refinement whose abstract subject is a model (once the feature is built)" refusing `malformed-declaration` at the object's pointer has no criterion: AC-7 and AC-11 cover only a build that omits the feature. QSpec FR-432-AC-9 covers it. Add a vector for a build that has the feature, or state that it is QSpec's vector. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:170 |

## Verdict

Mergeable after FND-001. FND-002 is a coverage gap for a build that does not
exist yet.

QSpec note for the planner: the same ambiguity is in QSpec. FR-432's third
Defects row makes an entry whose protocol is not the selected protocol
`missing-name`, and a model subject selects none, yet FR-375 and FR-432 say a
model-subject non-empty list is refused only as not yet built, and TC-310
mutation 17 expects exactly that for `Spec::Twice::finish` under
`abstract Spec`. QSpec should say which list checks run on a model-subject
refinement. This is a QSpec defect to hand to the planner; QSL's reading
(skip name resolution) matches the QSpec vectors.

## Dispositions

Round 1, against head 40892cef1 (fix commit edced4cd7).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | edced4cd7: a "model-subject refinement" rule says S3 SHALL check shape, spelling, repetition and that `refinement` names a refinement, and SHALL NOT resolve alias, protocol, node, template, channel, role or memory kind; AC-11 and TC-552 step 11 add `Other::Twice::finish` on `CasRefinesCounter` giving no `missing-name`. (The fix leaves AC-11's first vector inconsistent; recorded as SR-13075 FND-005.) |
| FND-002 | fixed | edced4cd7: FR-147-AC-12 and TC-552 step 12 cover the built-feature model-subject refusal, `malformed-declaration` at (the unit's value, `/0`). |
