---
id: SR-473
title: "EARS conformance review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-473: EARS conformance review of ADR-011

## Summary

Reviewed commit `944a1c8` (branch `task/209-stage-dag`):
`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md` and its index
row in `spec/spec.md` (line 388). ADR-011 is a Layer 1 design ADR for #209, not
an FR, NFR or StR, so the EARS scope is empty. The record has no `shall`
statement. Its normative modals are one `must` (§5 "the stage-boundary
constraints that design must meet"), a `Must not` column header in §10, and
six `may` uses (Decision 2, §2.2, §2.3 twice, §5, §6.1). They read as
indicative constraints, which is normal for an ADR, except `may not` in §5
(FND-004). `quire validate --strict` on the ADR reports 1/1 docs grammar-clean,
0 findings. The index row matches the ADR status and ticket. Neither mermaid
diagram has a `;` in a label.

The current-state wording is mostly sound. "today" marks observed state against
ADR-010, not history, and "historical" in §7.1 is the name of the
`quire-contract-ir-historical` crate. The rejected designs are in "Alternatives
Considered", with two small leaks (FND-005, FND-006). The main defects are
internal consistency. Decision 8 states a weaker proof-gate rule than §2.3.
Decision 1 and Consequences allow three module categories, but §6.1 and §6.2
use two more. M-5 labels `evaluate` with the wrong stage. One quotation is
attributed to #205 but its content comes from #216.

Verdict: ACCEPT WITH FINDINGS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Decision 8 says a proof gate "passes only when it discharges at least one proposition over every module it claims". §2.3 needs two conditions per claimed module: a discharged check location in the prover transcript, and a mutation control inside the module that turns the gate red. Consequences also names the mutation control. A reader who stops at the Decision gets the weaker rule. Fix: state both conditions in Decision 8, for example "…passes only when, for every module it claims, it discharges at least one proposition inside that module and a mutation control inside that module turns it red (§2.3)". | ADR-011 Decision 8, §2.3 |
| FND-002 | medium | Decision 1 and the first Consequence say that every module maps to "one stage, a foundation layer, or a seam", and that any other module is a defect #226 reports. §6.1 also has a `tool` layer (`complete::editor`, `complete::edit`, `format`: "tooling over S1"). §6.2 maps `xtask` and `tools/fixture-audit` to "build tooling … not on the stage DAG". It also sends `value::division::negotiate_*` and `value::ieee::negotiate_*` "outside QSL". Read literally, the drift gate would report these modules as defects. Fix: name the tooling layer over S1, build tooling outside the stage DAG, and "moves out of QSL (owning change)" as allowed placements in both Decision 1 and Consequences. Or map each of these rows to a stage, foundation or seam. | ADR-011 Decision 1, §6.1, §6.2, Consequences |
| FND-003 | medium | §7.3 M-5 reads "Split `value::expression` into S3 `check` and S5 `evaluate`". S5 is Contract IR, owned by IR. `evaluate` is layer 5 and stage S6a (§1, §6.1, §6.2). Because layer numbers and stage numbers look alike, a reader can place `evaluate` in the wrong repository. Fix: write "into layer-3 `check` (S3) and layer-5 `evaluate` (S6a)". | ADR-011 §7.3 M-5 |
| FND-004 | low | §5 says lifecycle surfaces "may not bypass §3". "May not" can be read as either a prohibition or a lack of permission, and every other constraint in the record uses "never" or "is forbidden". Fix: write "They call the same stage APIs and never bypass §3". | ADR-011 §5 |
| FND-005 | low | §7.3 ends: "No other crate extraction is approved. `protocol_artifact`, `value` and `model` are the largest modules. They stay modules until they meet §7.2." This names a rejected alternative (size-based splits) outside "Alternatives Considered", which already covers it. Fix: keep only "No other crate extraction is approved; a module becomes a crate only when it meets §7.2" and leave the largest-modules rejection in Alternatives Considered. | ADR-011 §7.3, Alternatives Considered |
| FND-006 | low | "Questions handed to sibling tickets" asks #210 about `capability_report` "now that composed `requests` disposition leaves QSL". "Now that" describes a change over time instead of the decided state. Fix: "given that capability disposition lives outside QSL (FB-12)". | ADR-011 Questions handed to sibling tickets |
| FND-007 | low | SEAM-1 quotes "#205 Layer 2: No deprecated producer path remains reachable", and Alternatives Considered cites "#205's Layer 2 gate". #205's body has no such sentence. The rule is in gate #216 Required evidence ("Old producer/bypass paths are unreachable") and its Failure rule ("Do not pass while two authoritative producer paths coexist"). Fix: cite #216 and quote it exactly, for example "(#216: 'Old producer/bypass paths are unreachable')". Use the same wording in the native-v1 alternative. | ADR-011 §6.2 SEAM-1, Alternatives Considered |
