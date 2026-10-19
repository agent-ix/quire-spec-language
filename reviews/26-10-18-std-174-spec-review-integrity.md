---
id: SR-12891
title: "Integrity review of PR #985: NoInstance across the sibling cause enumerations"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@3e1272e03; ADR-018 V-6 and InconclusiveCause list; FR-125 Instancing; FR-167:113; FR-168 Outputs; FR-169 V-6; FR-181 Outputs; FR-182 V-6; FR-239 Outcomes; ADR-022 GV-5; ADR-023 HS-3, HV-4; ADR-026 TV-1"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
---

## Summary

Ticket: STD-174. PR quire-spec-language#985. Consistency check of the new
`NoInstance` cause against every other enumeration of inconclusive causes
that omits it. The question for each one is whether its item can have zero
instances. If it can, the omission is a real gap. FR-125 Instancing (lines
97-118) instances temporal clauses and state-graph claims, and reports
`NoInstance` per item before any engine runs. Engine-level and per-instance
outcome lists therefore cannot produce it. Only the settlement maps need it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-022 GV-5 has the defect this PR fixes in ADR-018. State-graph claims are instanced by FR-125 (Instancing covers "state-graph claim" explicitly), and FR-169's V-6 row already maps `NoInstance (FR-125)`. GV-5 lists the pre-run `NoInitialState` but not `NoInstance`, so the ADR and its FR disagree. Fix: add the same "Amended by FR-125" note to GV-5, in this PR or a follow-up. | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:206; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:128 |
| FND-002 | medium | A zero-instance hyper clause is unspecified. ADR-023 HS-3 gives a `behaviours (v: V::Vault)` clause "one instance per key of that population's universe", so an empty universe gives zero instances. However, FR-125 Instancing covers only temporal clauses and state-graph claims, and neither FR-182's V-6 row nor ADR-023 HV-4 lists `NoInstance`. HV-4 also enumerates which ADR-018 V-6 causes apply, so the new V-6 entry does not carry over. An implementer cannot tell whether such an item settles `inconclusive`, refuses, or holds vacuously. This needs a follow-up: extend FR-125 Instancing (or FR-173/FR-175) to hyper object parameters, and add `NoInstance` to FR-182 V-6 and HV-4. It is out of scope for this 2-line PR. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:114,277; spec/functional/FR-182-settle-a-hyper-verdict.md:84; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:99 |

## Verdict

Mergeable. Both findings are about files outside the diff. They are
real gaps, so they should become follow-ups rather than block this PR.
Nothing in the diff contradicts another artifact.

The author's scope call, checked per enumeration:

- FR-167:113 (`NoInitialState` ends every item): not a gap. This is a
  subject-wide end of exploration. A zero-instance item never reaches the
  exploration.
- FR-168 Outputs `NoInitialState` variant: not a gap. `StateGraphOutcome` is
  per item instance, and a zero-instance item has no instance. FR-169 maps
  `NoInstance` straight from FR-125.
- FR-181 Outputs: not a gap. These are engine outcomes for one HP-5 instance,
  settled by FR-169's map, which has `NoInstance`. The hyper instancing gap is
  FND-002.
- FR-182 V-6: real gap (FND-002).
- FR-239 Outcomes: not a gap. These are per-instance zone-engine outcomes.
  ADR-026 TV-1 settles timed items by ADR-018 V-1 to V-8 unchanged, so they
  pick up this PR's V-6 note.
- ADR-022 GV-5: real gap (FND-001).
- ADR-023 HV-4: real gap (FND-002).
