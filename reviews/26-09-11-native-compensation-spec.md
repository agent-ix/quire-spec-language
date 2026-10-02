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

`/spec-review` base-checklist recheck over the FR-042, TC-121 and
`docs/compiled-protocol-v1.md` changes in the correction commit `b7aafe1`
against the reviewed `c7275f5`. Scope is the rewritten reader paragraphs, the
three new SHALLs, the new attempt-bound paragraph, the rewritten FR-042-AC-6 and
TC-121 step 6; no other requirement is re-analysed. No `AssuranceProfile`
document is installed anywhere in this repository — `^type: AssuranceProfile`
matches nothing under the worktree — so the review set remains the requester's
choice (base plus failure-domain), not a profile selection, and no
`## Assurance Context` section applies. Both previously open mediums are
resolved: the typed-effect lane is now an obligation on the reader with a named
result, and the recovery authorities now carry four explicit SHALLs. What
remains is that one of those new SHALLs quantifies over a set it does not
define.

## Verdict

**CONDITIONAL** — two mediums, one low, no high. The new text is in the right
mood, names actors and results, and stays scoped to static admission; the
residual defects are an undefined term inside a new obligation and an AC that
has grown past the point of identifying what failed.

## Findings

| ID      | Severity | Summary                                                                                                    | Refs                                                                              | Escape Cause        |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- | ------------------- |
| FND-001 | medium   | The new `recovery_bindings` SHALL requires "the declaration's population/closure pairs for recovery and contributing captured origins" without defining contribution, and two implementations define it differently | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:167; docs/compiled-protocol-v1.md:459 | missing-requirement |
| FND-002 | medium   | FR-042-AC-6 grew from eleven refusal conditions to five sentences spanning providers, cardinality, registration, retries, commit, recovery, typed-effect support and five prerequisite classes; a failure no longer identifies what failed | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:204                     | missing-requirement |
| FND-003 | low      | Compensation-anchored await and compensation-associated domain events are emitted and tested but named in no requirement (unchanged from `c7275f5`) | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:203                     | missing-requirement |

## Dispositions of the SR-349 findings recorded at c7275f5

| Prior   | Disposition                                                                                                                                   |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | **Resolved.** Line 156 is now `If an otherwise valid compensation effect carries a non-null value type, then the current reader SHALL return Unsupported::Export`, with the reason given (no authoritative effect-payload selector in the profile) and the identity-mismatch case held at `Invalid::Binding`. TC-121 step 6 names both mutations. The wire contract at `docs/compiled-protocol-v1.md:430-437` matches. Correct register: this is an explicit missing-interface refusal, not a claim that a typed observation may be untyped. |
| FND-002 | **Resolved.** Four new reader SHALLs at lines 161-171 cover the clock's subject/activation anchor/activation prerequisite/temporal contract and authority, the snapshot's type/model/anchor/prerequisite, both progress and closure under the progress contract with their exact three dependencies, and `recovery_bindings` membership. Residual is FND-001 above. |
| FND-003 | **Open and worse.** AC-6 gained conditions rather than being split. Carried above as FND-002.                                                   |
| FND-004 | **Open**, unchanged. Carried above as FND-003.                                                                                                  |

### FND-001 — an obligation over an undefined set

"The reader SHALL require `recovery_bindings` to contain exactly those three
records and the declaration's population/closure pairs for recovery and
contributing captured origins." The three records are defined by the preceding
sentences. *Contributing captured origins* is not defined anywhere in FR-042 or
the wire contract, and the word `exactly` makes the undefined half a refusal
condition rather than a permission.

Two constructions in the repository answer it differently: the emitter selects
typed nodes by source-span containment inside the `recover` expression regions,
the reader walks the validated operand/initializer/origin value graph, and their
`Closure` membership predicates differ on whether the test is the model's export
kind or the shape of `requires`. Either could be the right rule; the requirement
must nominate one, and TC-121 must have a case that would fail if the other were
implemented. Today no AC distinguishes them, so a divergence would surface as a
genuine package being refused in the field. Recorded on the code side as FND-001
in SR-347 and as a failure mode in SR-350.

## Checklist results

- **ID formats.** `FR-042`, `FR-042-AC-1..10`, `TC-121` all conform; no
  duplicates or gaps introduced. The new text cites `Invalid::Binding` and
  `Invalid::Owner` from the existing typed cause catalog and mints no new cause.
- **Validation link integrity.** FR-042-AC-6's Verification column still reads
  `Test (TC-121)`, and TC-121 step 6 is the step that carries the new checks —
  the link is bidirectional and resolves. `quire coverage` shows FR-042-AC-6
  backed, so the AC↔TC↔test triangle is closed for the typed-effect lane and for
  every recovery mutation TC-121 now names.
- **Six coverage rules.** Every AC touched has a TC; TC-121 step 6 names concrete
  mutations and required causes rather than "verify behaviour"; no AC was
  weakened to match the implementation — AC-6 again gained conditions rather than
  losing them, and the typed-effect sentence moved from a permission to a
  refusal.
- **Measurability.** The new reader SHALLs are checkable field by field: subject,
  activation anchor, activation prerequisite, temporal contract and authority for
  the clock; type, model, owning recovery anchor and activation prerequisite for
  the snapshot; the progress contract and the exact clock/effect/snapshot triple
  for progress and closure. The attempt-bound paragraph gives a closed interval,
  `1..=9223372036854775807`, and separates it from consumer work budgets, so it
  is a bound on the authored artifact rather than on runtime behaviour. The one
  unmeasurable term is "contributing captured origins" (FND-001).
- **Mood and actor.** The typed-effect sentence now has both: `the current reader
  SHALL return Unsupported::Export`. The qualifier "current" is load-bearing and
  correct — it scopes the refusal to the present native profile and states the
  condition under which it would change (an explicit producer interface),
  without promising one.
- **Error modes.** Present and specific: FR-042-AC-6 lists cross-wired
  registration, activation, clock and recovery prerequisites, unrelated
  recovery/population records, null model, an ordinary effect with null type and
  the operation-success shortcut, each with a refusal; the wire contract adds
  `Unsupported::Export` for a typed effect and `Invalid::Binding` for a foreign
  operation with a type inserted.
- **Scope discipline retained.** The rewritten delivery paragraph at
  `docs/compiled-protocol-v1.md:590` no longer lists recovery admission among the
  blanket refusals, correctly replacing it with what now emits statically
  (registration/activation, retries, commit/never, authored full/partial recovery
  requirements) and what remains a consumer judgment at runtime. The withdrawn
  wording was stale, not a weakening.

### FND-002 — AC-6 keeps absorbing

AC-6 now covers shared providers, delivery cardinality, registration, bounded
retries through the signed-64 maximum, commit boundaries, full versus partial
recovery, compensation-effect identity under an inserted type, the explicit
unsupported typed-effect result, and five classes of missing or cross-wired
prerequisite. It is still verifiable and no condition is vague, but a single
failing run against AC-6 no longer tells a reader which guarantee broke. Split
the compensation identity chain and the recovery-authority chain into their own
ACs; that would also give the compensation-anchored await and
compensation-associated domain events of FND-003 somewhere to live, since
`AwaitAnchor::Compensation` and `event ... for <compensation>` are emitted, read
back and tested and no requirement mentions either.
