---
id: SR-1360
title: "Base review of QSL-657 capability vocabulary alignment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4f9c08af9ab3086fc9ce3e216b6f2b3e70c63bba; baseline origin/main@2a90a2df0b29617e7c63aa440d291371efe906b4; QSpec FR-290 origin/main@7a81a2f17323eb5c9752bb4997ac913c8618ffc8; FR-057, FR-075, FR-080, TC-153, TC-970–979, spec/model-linking/tests.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-290
    type: references
---
# SR-1360: Base review of QSL-657 capability vocabulary alignment

## Summary

Reviewed QSL PR #716 at exact head `4f9c08af9ab3086fc9ce3e216b6f2b3e70c63bba`, against freshly fetched QSL `origin/main` `2a90a2df0b29617e7c63aa440d291371efe906b4` and QSpec `origin/main` `7a81a2f17323eb5c9752bb4997ac913c8618ffc8`. The twelve labels are aligned, but candidate selection, duplicate registration semantics, and one test setup remain inconsistent with FR-290 or the draft's own admission rule.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-075 still computes candidates by capability kind alone and carries no requested evidence kind into candidate matching. QSpec FR-290 requires probabilistic candidates to advertise both the capability and the item's exact evidence kind; TC-974/975 only verify admission and retention. A statistical backend can therefore be considered for an exact item. Specify evidence-kind input and candidate filtering, then cover both non-substitution directions. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:25-27, 56-57, 80-98, 245-247; QSpec FR-290 at 7a81a2f:223-244 | wrong-requirement |
| FND-002 | high | FR-075 defines equal descriptors as the same identity and advertised pairs, omitting the newly retained evidence-kind set. Two registrations with the same ID and pairs but different evidence kinds can be accepted as one registration, although QSpec FR-290 treats differing manifest members as a duplicate-backend conflict. Include evidence kinds in descriptor equality and its criteria/tests. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:47-55, 185-198, 243-247; QSpec FR-290 at 7a81a2f:252-265 | wrong-requirement |
| FND-003 | medium | TC-973 asks the test to admit a descriptor for every kind without supplying evidence kinds for `probabilistic-satisfaction`. FR-075 now rejects that descriptor unless it has a nonempty valid evidence-kind set, so the test cannot execute that iteration as written. Add a valid evidence set for that case and assert the resulting candidate behavior. | spec/test-cases/TC-973-composed-admission-stage-projection-preserves-stage-ownership.md:18-24; spec/functional/FR-075-compute-candidates-from-registered-backends.md:160-168 | correct-requirement-no-evidence |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-075-AC-7 calls the identity, advertised pairs and evidence-kind set the equal complete descriptor, but an admitted descriptor also retains `domains` (FR-057-AC-12; ADR-013 C-28). The criterion and TC-981 do not define/test whether a domains-only difference conflicts, although FR-290 compares complete manifest descriptors. State that all descriptor members, including domains, participate and add a domains-only conflict vector. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:43-57, 261, 267; spec/functional/FR-057-admit-shared-capability-kinds.md:262-278, 393; spec/test-cases/TC-981-evidence-kind-difference-conflicts-duplicate-registration.md:18-31; QSpec FR-290 at 3121fa4:252-265 |
| FND-005 | medium | FR-075 defines exact evidence matching for both named and unnamed candidate requests, but TC-193 and TC-980 exercise only unnamed requests. No testcase checks that a registered, named backend advertising `statistical` produces an empty candidate set for an `exact` item (and vice versa). Add the two named-backend mismatch controls. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:101-109, 255-266; spec/test-cases/TC-193-candidate-set-matches-advertised-kind.md:13-18, 46-49; spec/test-cases/TC-980-probabilistic-candidates-require-exact-evidence-kind.md:18-24 |

## Dispositions

Fix-round recheck context: QSL PR #716, exact published head
`e9234569daf861f451b8d761fdc228250d929ec1`, checked against fresh QSL
`origin/main` `2a90a2df0b29617e7c63aa440d291371efe906b4` and QSpec
`origin/main` `3121fa496e61997370c21af37260e79b132624cc`. The original summary
above records the earlier review baseline and head; these dispositions are for
this fix-round SHA. The complete fix-round delta from the reviewed draft is 18
files.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fixed e9234569daf861f451b8d761fdc228250d929ec1 |
| FND-002 | fixed | fixed e9234569daf861f451b8d761fdc228250d929ec1 |
| FND-003 | fixed | fixed e9234569daf861f451b8d761fdc228250d929ec1 |
| FND-004 | still-open | FR-075-AC-7 and TC-981 do not include the retained `domains` field in descriptor equality coverage. |
| FND-005 | still-open | The exact evidence-kind rule's named registered-backend branch has no probabilistic test case. |

### Fixed excerpts at e9234569daf861f451b8d761fdc228250d929ec1

FND-001, from FR-075:

> For each requested item, the registry SHALL compute the candidate set as
> follows, matching a registration to the item's capability kind and, when that
> kind is `probabilistic-satisfaction`, the item's evidence kind. A probabilistic
> registration is a candidate only when its admitted evidence-kind set contains
> the item's exact evidence-kind label. `statistical` SHALL NOT substitute for
> `exact`, nor `exact` for `statistical`.

FND-002, from FR-075:

> the registry SHALL compare the complete admitted descriptors, including identity,
> advertised pairs and the evidence-kind set:
>
> - If the two are equal (same identity and every descriptor member, including
>   evidence kinds), the repeat SHALL be treated as one registration.

FND-003, from TC-973:

> 1. Admit a requested pair through the composed linker while varying whether a
>    registry contains a matching backend; for `probabilistic-satisfaction`,
>    give the item the evidence kind `statistical`.
> 2. Admit a backend descriptor advertising the kind, supplying
>    `{statistical}` evidence kinds for `probabilistic-satisfaction`, then
>    inspect the registry and its candidate result for the matching item.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | high | FR-075 makes the manifest digest a member of complete descriptor equality, and TC-982 tests digest-only differences. The installed spec-review checklist classifies requirements and tests that create, retain or check digests as a high-severity antipattern; this digest does not bind a proof to the exact content it proved. Remove the digest equality and test obligation rather than carrying it into this contract. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:206-220, 265; spec/test-cases/TC-982-backend-descriptor-field-difference-matrix.md:24-34 | wrong-requirement |

## Dispositions — round 2

Rechecked PR #716 at exact published head `3789a2b6b21aee071b3687f143ecd48d5afc28ca`, against QSL baseline `2a90a2df0b29617e7c63aa440d291371efe906b4`. TC-982 closes the prior descriptor-field coverage gap and TC-983 closes the named-backend evidence-kind gap. FND-006 is a new checklist finding in this pass and requires a subsequent fix/disposition round.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | fixed 3789a2b6b21aee071b3687f143ecd48d5afc28ca |
| FND-005 | fixed | fixed 3789a2b6b21aee071b3687f143ecd48d5afc28ca |

### Fixed excerpts at 3789a2b6b21aee071b3687f143ecd48d5afc28ca

FND-004, FR-075-AC-7:

> Given a registration naming a `BackendId` already held by the registry with an equal complete descriptor (identity, provider origin, manifest digest, advertised pairs, retained `domains`, and evidence-kind set), the repeat is one registration and is not refused; the registry and its candidate sets are unchanged. A repeat differing only in any descriptor member other than identity is unequal and follows FR-075-AC-4; a different identity is a different registry key.

FND-005, TC-983:

> | `statistical` | `exact` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |
> | `exact` | `statistical` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |

## Dispositions — round 4

Rechecked QSL PR #716 at exact head `4865d61bdbe85914e6d8342276152bcaef686a95`.
FND-006 is fixed: FR-075-AC-7 excludes manifest digest from descriptor equality,
and TC-982 has no digest-only comparison row. FND-007 and FND-008 are fixed in
ADR-013's complete descriptor list and ADR-014's `Requirements` record shape.
FND-009 remains open: ADR-014 still says QSpec FR-290 has ten kinds, while this
change and FR-057 specify twelve.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-007 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-008 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-009 | still-open | ADR-014 still states “QSpec FR-290 has ten kinds” at the reviewed head. |

### Fixed excerpts at 4865d61bdbe85914e6d8342276152bcaef686a95

FND-006, FR-075-AC-7:

> Given a registration naming a `BackendId` already held by the registry with equal identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set, the repeat is one registration and is not refused; the registry and its candidate sets are unchanged. A repeat differing only in any of those compared members is unequal and follows FR-075-AC-4; manifest digest does not affect descriptor equality. A different identity is a different registry key.

FND-007, ADR-013:

> complete descriptor -- same identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set -- are one registration, and a repeat is not refused.

FND-008, ADR-014:

> QSL S3 records `Requirements{kind, evidence_kind, extent, bounds}` per item (§4); `evidence_kind` carries the item's unmodified label when present for `probabilistic-satisfaction` and remains absent when the item omits it, so negotiation can distinguish valid, absent and unknown evidence.
