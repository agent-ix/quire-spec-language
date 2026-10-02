---
id: SR-305
title: "Risk and complexity review of composed compiler admission"
type: SpecReview
analysis: risk-complexity
scope: "Compiler FR-035/036, TC-113–115 and IT-009 under compiler #35 (L2)"
review_set: all
evaluated_revision: "d5047a8"
review_date: "2026-09-10"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-035, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: reviews }
---
## Summary

The L2 specification has a manageable implementation split: extend the existing
lexer/parser with typed composed syntax, then add exact multi-unit dependency
admission. The main volatility is the shared draft and producer contract still
awaiting acceptance. This review identifies implementation risks; it does not
claim implementation, producer availability or executed conformance evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unmitigated specification blocker found in this lens. New package admission and unsettled external contracts carry the risks and named mitigations below. | FR-035/036; TC-113–115; IT-009 |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-035 | Medium | High | More syntax families share precedence and contextual keywords; the shared composed grammar is still a proposal. Logos and the structured/Pratt parser are already in use. | A implements explicit edition dispatch and typed family variants under #35 after affected standard acceptance. TC-113 compares actual trees, original byte spans, malformed forms and frozen historical behavior. Predicate/family evaluation remains in its own tickets. |
| FR-036 | High | High | The first multi-unit composed path combines package-wide identity, dependency failure containment and an external producer/native correspondence contract. Current linkage is single-unit and atomic. | A separates inventory closure, exact selection, typed dependency resolution and downstream capability admission. TC-114/115 use adversarial identity/graph/stage controls; IT-009 requires the actual accepted producer interface. Preserve historical APIs and defer composed interchange to its owning ticket. |

## Top hazards

1. A new keyword or grammar branch could change historical parsing. Keep the
   edition visible at recognition and use the unchanged historical corpus beside
   new typed AST assertions; do not reinterpret old fixture selections.
2. A shared dependency failure or budget stop could become an apparently complete
   package. FR-036 and TC-114 require declared accounting, charged-work refusal,
   dependent dispositions and immutable retry controls. Its typed report must
   retain the distinction between linked, checked and executable stages.
3. A same-shaped model, role or canonical digest could substitute for the selected
   owner. Preserve exact producer/native domains and declaration-owned roles;
   TC-114/115 discriminate them locally and IT-009 checks the real producer.

## Failure-domain alignment

[SR-301](failure-domain.md) covers identity, purity, topology and exhaustion.
[SR-303](dependency.md) and [SR-306](scope-boundary.md) retain the external
acceptance prerequisites and producer ownership. The accounting clarification
from [SR-302](integrity.md) is addressed in FR-036 and TC-114 at `d5047a8`.
The scope adds no distributed coordinator, execution engine, universal memory
ceiling or new cryptographic algorithm. Those would be separate requirements,
not incidental parser/linker implementation decisions.
