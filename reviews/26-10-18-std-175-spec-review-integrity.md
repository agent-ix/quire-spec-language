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
