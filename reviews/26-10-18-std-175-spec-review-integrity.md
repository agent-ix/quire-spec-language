---
id: SR-12891
title: "Integrity review of STD-175 zero-instance hyperproperty items (QSL PR #987)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@944699b543e42ba482bc3cd405281b27dcf10ba9; PR #987 diff against origin/main 49bcf9a: FR-125, FR-172, FR-182, TC-607; cross-checked against ADR-023, FR-126, FR-127, FR-176 to FR-179, and QSpec PR #281"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
---
# SR-12891: Integrity review of STD-175 (QSL PR #987)

## Summary

Ticket: STD-175. This pass checks that the change is consistent within QSL
and with the paired QSpec PR #281.

Both repos give the same outcome: inconclusive, unsettled, `no-instance` /
`NoInstance`, before any state is explored, and the ACs and vectors both
say requires-bound comes first. QSpec states that order as a SHALL; QSL does
not (FND-004). FR-182's V-6 row now lists `NoInstance`, which FR-126 and FR-127
already carry, and FR-176 to FR-179 inherit it through FR-126's
`ModelCheckOutcome`. FR-172 states the per-key instance and the
product over several aliases (any empty factor gives none) in the same words
as QSpec FR-395.

## Verdict

Three medium findings: the product case is stated but not tested, ADR-023
still excludes `NoInstance` from hyper items, and FR-125 Instancing does not
bring hyper clauses under the `RequiresBound`-first rule. There is one low
finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-172 now states that a clause over several aliases has zero instances when any one parameter's universe is empty. That is the product rule from the ticket. Neither FR-182-AC-9 nor TC-607 step 7 exercises it: both use a single alias, `v: V::Vault`. An engine that takes the first parameter's universe, or the union of the universes, passes every vector and still runs SavesPower (`d: S::Device, e: N::Device`) with `N`'s universe empty. Add the two-alias case to AC-9 and step 7. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:93-96; spec/functional/FR-182-settle-a-hyper-verdict.md:131; spec/test-cases/TC-607-hyper-verdicts-settle.md:27 |
| FND-002 | medium | ADR-023 is not amended. HV-4 lists exactly which ADR-018 V-6 causes apply to hyper items, `UndecidedSuccessor`, `NoInitialState`, `ReplayParity` and `ReplayRefused` ("All are V-6"), and leaves out `NoInstance`. HS-3 gives one instance per universe key and says nothing of the empty product. HM-7 treats `NoInitialState` as the only empty-domain case. STD-174 (open PR #985) adds an "Amended by FR-125: it also gains `NoInstance`" note to ADR-018 V-6 for the same cause, but ADR-023 HV-4 names its causes itself and does not inherit that note. A reader of ADR-023 therefore finds `NoInstance` excluded from hyper items, which contradicts FR-182's new V-6 row. Add the same "Amended by FR-125" note to HV-4 and HS-3 (and HM-7) in #987. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:114; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:164; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:277 |
| FND-003 | low | FR-172 and FR-182 now depend on FR-125 Instancing for the zero-instance rule, but neither has an FR-125 `relationships` edge, so the dependency does not appear in the link graph. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:95-96; spec/functional/FR-182-settle-a-hyper-verdict.md:84 |
| FND-004 | medium | FR-125 Instancing's governing statement still covers only "a checked temporal clause or state-graph claim" (line 98). Its `RequiresBound` bullet covers "a population whose member type conforms to `T`" (lines 109-111). The PR adds the hyper clause only to the zero-instance sub-bullet, so the rule that `RequiresBound` is classified before `NoInstance` reaches hyper clauses by analogy, not by a SHALL. FR-172 also sends the reader to FR-125 for `NoInstance` alone. One implementer classifies `RequiresBound` first for a hyper parameter; another settles `NoInstance` because FR-172 names nothing else. Extend the line-98 statement (and the `RequiresBound` bullet) to hyper object parameters, each instanced per FR-172 over its resolved population's universe. Or have FR-172 cite the `RequiresBound`-first rule itself. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:98-99; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:109-115; spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:93-96 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The fix for FND-004 adds "hyper clause" to FR-125's governing Instancing SHALL, but that sentence instances "one instance per object of the subject's universes whose type conforms to the `over` parameter's type `T`". Its next bullet adds the objects of every population that holds conforming objects, subtypes included. A hyper clause has object parameters, not an `over` parameter, and FR-172 and the new bullet at :118-120 give it one instance per key of the single resolved population. The SHALL now gives hyper clauses two different instance sets. One implementer adds the objects of a subtype population to `v: V::T`; another uses only the resolved population's keys. The requires-bound bullet has the same problem: it fires on "a population whose member type conforms to `T`", which need not be the resolved one. Keep line 98 as it was for claims, and give hyper clauses their own SHALL: per-key instances over the product of the resolved populations' universes, with `RequiresBound` first for each resolved population. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:98-108; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:111-115; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:118-120 |
| FND-006 | low | The claim zero-instance bullet (FR-125:116-117) no longer carries its SHALL. The "QSL SHALL then report `NoInstance`" sentence now sits under the hyper-clause bullet, so a reader of the claim bullet finds no consequence there, even though the hyper sentence says "a claim or hyper clause". Put the SHALL in its own bullet after both cases. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:116-123 |

## Dispositions

Round 1, reviewed at 470eeeb35 (merge of origin/main 7915615f1). Hand check
of TC-607 step 7: with `S::Device` empty and `N::Device` of 3 keys, an engine
that reads only the first parameter finds no instance and passes. With the
reverse, the same engine finds instances, explores, and fails. A union
reading fails both runs.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 2, reviewed at 6b569e4ada6b8615a75ed82fa1807d481fbe278a. FR-125
Instancing now has separate rules:

- Temporal clauses and state-graph claims are instanced by conformance over
  `over` (lines 99-117), with their own RequiresBound and NoInstance SHALLs.
- Hyper clauses are instanced per key of the one population each parameter
  resolves to, over the product (lines 118-123). They have their own
  RequiresBound-first SHALL and an any-empty-factor NoInstance SHALL
  (lines 124-126).

The instance-count rules no longer overlap. FR-172 S3 (per key, product,
NoInstance by FR-125), FR-182 V-6, FR-182-AC-9 and TC-607 step 7 all agree
with the new bullets.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
