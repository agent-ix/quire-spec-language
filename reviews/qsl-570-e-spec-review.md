---
id: SR-12319
title: "Spec review of the FR-057 evidence-kind admission edit and the stale BackendDescriptor::admit name (QSL-570 part E, PR #1204)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@65a31e407; spec/functional/FR-057-admit-shared-capability-kinds.md (the added paragraph, AC-12, AC-14); spec/functional/FR-075-*.md (evidence kinds section, AC-11); FR-280-*.md (AC-4); FR-288-*.md; ADR-013 C-28; TC-447, TC-973, TC-974, TC-976 (context); QSpec FR-290 Evidence kinds (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---
# SR-12319: Spec review of the FR-057 evidence-kind admission edit and the stale BackendDescriptor::admit name (QSL-570 part E, PR #1204)

## Summary

Ticket: QSL-570. The PR's only spec change is one paragraph added to FR-057. It says:

> The registry SHALL judge a backend's evidence-kind labels (FR-075-AC-11) last, after every advertised pair, `domains` and `fairness_kinds`, and only for a backend that advertises `probabilistic-satisfaction`: evidence-kind labels supplied by any other backend are ignored and the admitted descriptor holds none. A label repeated within the set is one member.

Read with it:

- FR-057's sibling paragraphs on `fairness_kinds` and `domains`, and FR-057-AC-12 and AC-14.
- FR-075's "Probabilistic evidence kinds are checked at descriptor admission" and FR-075-AC-11.
- QSpec FR-290 "Evidence kinds", which says a probabilistic backend advertises "a nonempty set of evidence kinds, each exactly `statistical` or `exact`".

The lead asked a separate question: is the spec's `BackendDescriptor::admit` right, or the code? `grep -rn 'fn admit' qsl-route/src` finds nothing. QSL-593 (b237f3192, #729) folded admission into `descriptor_from_manifest` (the label path) and `BackendDescriptor::new` (the typed path). The code is right and the spec name is stale. The ruling is to change the spec wording to the real names, with no alias and no rename. A full grep finds the name in more places than FR-057, FR-075, FR-280 and ADR-013 C-28 (FND-001).

## Verdict

**No major finding.** The added paragraph matches the code, QSpec FR-290 and the style of its sibling paragraphs.

FND-001 is required in this PR. It is the lead's ruling, and the PR's "Remaining" note about it is not a disposition. FND-002 makes the new SHALL text testable. FND-003 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec names `BackendDescriptor::admit`, and no such function exists. QSL-593 folded it into `descriptor_from_manifest` (labels) and `BackendDescriptor::new` (typed). Replace the name everywhere with the real names, with no alias. Occurrences at 65a31e407: FR-075 "Probabilistic evidence kinds are checked at descriptor admission" (2) and FR-075-AC-11; FR-280-AC-4; FR-057-AC-14; ADR-013 C-28 ("to `BackendDescriptor::admit`"); FR-288 ("the origin passed to `new` or `admit`") and FR-288-AC-5; TC-447 line 36; TC-973 line 32; TC-974 line 19; TC-976 line 13. These are more places than the PR body names. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:211,214,300; spec/functional/FR-280-*.md:88; spec/functional/FR-057-*.md:407; spec/decisions/ADR-013-*.md:1083; spec/functional/FR-288-*.md:64,84; spec/test-cases/TC-447-*.md:36; TC-973-*.md:32; TC-974-*.md:19; TC-976-*.md:13 |
| FND-002 | medium | The added paragraph makes three SHALL obligations, and no AC states any of them: evidence is judged last; another backend's labels are ignored; a repeated label is one member. FR-057-AC-12 does state the analogous `domains` ordering. Each code path is untested (SR-12318 FND-004). The paragraph also says "the registry" judges, where FR-075-AC-11 places the check in descriptor conversion, before registry insertion. Add the three cases to FR-075-AC-11, or a new FR-057 AC that states the order and names the conversion entry points. FR-075 owns evidence admission, so FR-075 is the better home. | spec/functional/FR-057-admit-shared-capability-kinds.md:286-291 |
| FND-003 | low | "A label repeated within the set is one member" reverses the sibling rules without saying why: a repeated `domains` kind refuses `invalid-domains` (FR-057), and a repeated fairness kind refuses `duplicate-label` (FR-134/FR-272). QSpec FR-290 says only "a set". Either cite FR-290's set reading as the reason, or refuse a repeat as the siblings do. | spec/functional/FR-057-admit-shared-capability-kinds.md:290-291 |

## Dispositions

Round 1, fixed in f4af7d95e.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f4af7d95e: stale name replaced in FR-075, FR-280, FR-288, FR-057-AC-14, ADR-013 C-28, TC-447, TC-973, TC-974, TC-976; no alias |
| FND-002 | fixed | f4af7d95e: rules moved into FR-075 (section and AC-11) with the conversion as subject; FR-057 paragraph removed |
| FND-003 | fixed | f4af7d95e: FR-075 states the reason: FR-290 states the evidence kinds as a set |
