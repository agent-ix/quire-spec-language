---
id: SR-349
title: "Base spec review of the compensation emission and payload-free effect requirements"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

`/spec-review` base checklist over the FR-042 and TC-121 changes in this
increment (two new normative paragraphs, a rewritten FR-042-AC-6, and TC-121
step 6's new sentences) plus the `docs/compiled-protocol-v1.md` wire-contract
paragraphs they depend on. No `AssuranceProfile` document is installed anywhere
in this repository — `grep "^type: AssuranceProfile"` over the worktree returns
nothing, and none applied in the parent scope — so the review set is the
requester's choice (base plus failure-domain), not a profile selection. IDs,
links and the six coverage rules hold; the substantive defect is that the
narrow contract's typed-effect half is stated as a fact rather than as an
obligation on anyone.

## Verdict

**CONDITIONAL** — two mediums. The new requirement text is measurable, scoped to
static admission and correctly refuses to promise runtime ordering, but its
typed-effect sentence carries no SHALL and no verification, and the recovery
authorities it describes have no reader obligation stated.

## Findings

| ID      | Severity | Summary                                                                                                    | Refs                                                                              | Escape Cause        |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- | ------------------- |
| FND-001 | medium   | The typed compensation-effect view is described declaratively ("retains its own value type and authoritative model export") with no SHALL, no actor and no verification | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:158; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:94 | wrong-requirement   |
| FND-002 | medium   | The reader SHALL at line 155 enumerates the registration/activation/attempt chain but omits the clock, snapshot and progress/closure prerequisites the same increment emits | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:155                     | missing-requirement |
| FND-003 | low      | FR-042-AC-6 now carries eleven distinct refusal conditions in one row; the compensation half warrants its own AC | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:187                     | missing-requirement |
| FND-004 | low      | Compensation-anchored await and compensation-associated domain events are emitted and tested but named in no requirement | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:186                     | missing-requirement |

## Checklist results

- **ID formats.** `FR-042`, `FR-042-AC-1..10`, `TC-121` all conform; no
  duplicates or gaps introduced. The new text cites `Invalid::Binding` and
  `Invalid::Owner` from the existing typed cause catalog and mints no new cause.
- **Validation link integrity.** FR-042-AC-6's Verification column still reads
  `Test (TC-121)`, and TC-121 step 6 is the step that carries the new checks —
  the link is bidirectional and resolves. `quire coverage` shows FR-042-AC-6
  backed, so the AC↔TC↔test triangle is closed for everything except the
  typed-view sentence (FND-001).
- **Six coverage rules.** Every AC touched has a TC; the TC step names concrete
  mutations and required causes rather than "verify behaviour"; no AC was
  weakened to match the implementation — AC-6 gained conditions rather than
  losing them.
- **Measurability.** The new paragraphs are checkable: "null value type, the
  exact compensation operation export, and its owning obligation, retry anchor
  and attempt-binding prerequisite" names four comparable fields. The
  surrounding prose that would normally read as vague ("does not authorize an
  untyped observation", "cannot replace its recovery relation") is scope
  language about what the artifact does *not* claim, which is the right register
  for this requirement and is not a vagueness finding.
- **Error modes.** Present and specific: FR-042-AC-6 lists cross-wired
  registration/activation links, null model, an ordinary effect with null type
  and the operation-success shortcut, each with a refusal.

### FND-001 — a fact where an obligation belongs

Line 155 is a proper `The reader SHALL refuse ...`. Line 158 then says "an
explicitly selected typed compensation-effect view retains its own value type
and authoritative model export" — descriptive mood, no actor, no SHALL, and no
statement that the model must be *that compensation's operation*. TC-121 step 6
mirrors it ("an explicitly selected typed effect view retains its
payload/model fields") with no mutation to require. The result is the gap
recorded as FND-001 in SR-347: the reader gates on `value_type.is_none()` and
the typed lane has neither a requirement to enforce nor a test to catch it.
Rewrite as an obligation on the reader with the same four identity fields as the
null-type lane, and give TC-121 a mutation (add a type, substitute the model,
require `Invalid::Binding`).

### FND-002 — the reader obligation stops at the attempt

The same increment emits, per obligation, a clock requiring the activation, a
snapshot requiring the activation, and progress/closure each requiring
(clock, effect, snapshot) — and line 143's older paragraph already promises
"the exact full/partial recovery predicates with their declared
population/relationship and closure authorities". No SHALL says what the reader
must refuse there, and the reader accordingly checks only binding kinds. Either
extend line 155's enumeration to the recovery authorities or state explicitly
that recovery-authority attribution is a consumer-side obligation; the current
text reads as though it were already covered.

### FND-003/FND-004 — structural notes

AC-6 now covers shared providers, delivery cardinality, registration, retries,
commit boundaries, full/partial recovery *and* the entire payload-free effect
contract. It is still verifiable, but a failure against it no longer identifies
what failed. A separate AC for the compensation identity chain would also give
the await/event behaviour in FND-004 somewhere to live: `AwaitAnchor::Compensation`
and `event ... for <compensation>` are emitted, read back and tested by the
seventh test, and no requirement mentions either.
