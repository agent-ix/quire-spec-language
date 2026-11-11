---
id: SR-2026111015000000000002
title: "base review of QSL-1400 source-declared object types (PR 2674)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2674 origin/main...b029e1e03; spec/functional/FR-569 AC-1..AC-10, FR-105-AC-18, FR-390-AC-70, FR-517-AC-23; spec/test-cases/TC-1407, TC-1408"
review_set: subset
---

## Summary

Ticket: QSL-1400. Checklist review, including a manual criterion-strength pass over every new or amended AC. The Jev-backed criterion-strength skill was not run, because no Jev client is available in this session. IDs, links and the TC trace are well formed, and native `quire validate` over the ten changed files reports no findings. Most ACs carry concrete fixtures and expected values. AC-10's S-versus-D equality is backed by TC-1408's literal expected outcomes, so it is not vacuous. Three ACs have a case that cannot fail as written, or an expected value they leave unstated.

## Verdict

PASS with findings (medium and low). The hash/digest/pin check is clean: FR-569 introduces no digest identity, and its node id is structural.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-569-AC-5's weakening case is vacuous. `Sub` does not redefine `bump`, so per FR-105 a `pre` on `Sub::bump` anchors at `Order`'s `bump` and adds a `requires` there. No redefinition exists for QSpec FR-151's weakening rule to judge, and the AC gives no clause text or refusal code. "Refuses as D refuses" therefore passes when both admit. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:357; spec/functional/FR-105-emit-state-nodes.md:136 |
| FND-002 | medium | Two gaps for subtypes. FR-569-AC-8 states `Sub`'s `supertypes` only, not its `identity_fields`, `members` or `populations`. FR-569's row text ("identity_fields (field names in declared order)") does not say whether an inherited identity is listed. The reader takes identity from these rows, so a `Sub` row with `[]` and one with `["total"]` both pass, although they give different object identity on the bytes route. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:303-314; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:360 |
| FND-003 | medium | FR-569-AC-9's `ix://test/orders/Order` case runs over S, which selects no domain package, so it is refused whatever rule applies. No case tests the lead rule that a bare snapshot name never names a model declaration: a package selecting a model `Order` with a snapshot `"type": "Order"` and no source `Order`. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:361; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:328-331 |
| FND-004 | low | Several smaller gaps. FR-569-AC-1's reserved-word refusal states no code or cause. FR-569-AC-7's "No digest is computed" is an unobservable negative with no test. FR-569-AC-6's "the forall's population reference" is over fixture S, which holds no forall; TC-1408 adds it in a variant it calls "S4", which collides with the stage name S4. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:353; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:358-359; spec/test-cases/TC-1408-declare-object-types-populations-and-operations-in-source.md:68-71 |

## Scope and evidence

Direct review. Native quire 0.40.0 (engine 0.56.0) validates the ten changed files with no spec findings.

- FR-569-AC-1 (examined): FND-004.
- FR-569-AC-2 (examined, clean): concrete resolutions and refusals.
- FR-569-AC-3 (examined, clean): eight concrete collision cases with spans.
- FR-569-AC-4 (examined, clean): concrete codes per case.
- FR-569-AC-5 (examined): FND-001.
- FR-569-AC-6 (examined): FND-004. FR-569-AC-7 (examined): FND-004.
- FR-569-AC-8 (examined): FND-002, and see integrity FND-003.
- FR-569-AC-9 (examined): FND-003.
- FR-569-AC-10 (examined, clean): TC-1408 gives V1 to V6 literal outcomes.
- FR-105-AC-18 (examined): see failure-domain FND-001.
- FR-390-AC-70 (examined, clean). FR-517-AC-23 (examined, clean).
- TC-1408 (examined): Scope lists AC-1 to AC-10 and FR-105-AC-18. TC-1407 (examined).
- No ceremony, tracking or digest identity was added (clean).
