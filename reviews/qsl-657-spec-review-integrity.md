---
id: SR-1361
title: "Integrity review of QSL-657 capability vocabulary alignment"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4f9c08af9ab3086fc9ce3e216b6f2b3e70c63bba; FR-057, FR-075, FR-080, TC-153, TC-970–979 and both test indexes"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-290
    type: references
---
# SR-1361: Integrity review of QSL-657 capability vocabulary alignment

## Summary

Reviewed requirement consistency, atomicity, observable behavior, and criterion coverage across the exact PR #716 head. FR-075's new descriptor evidence-kind behavior is not propagated into candidate selection or duplicate identity comparison.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Candidate computation remains explicitly keyed only by capability kind, despite FR-290 requiring an exact evidence-kind match for probabilistic items; no changed criterion or test covers candidate non-substitution. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:25-27, 80-98, 245-247 | wrong-requirement |
| FND-002 | high | Descriptor equality and idempotent-repeat criteria omit evidence-kind labels, so registrations that differ in this newly specified descriptor field have no deterministic conflict outcome. This conflicts with FR-290's full-manifest equality rule. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:47-55, 185-198, 243-247 | wrong-requirement |
| FND-003 | medium | TC-973's all-kinds loop omits the evidence-kind prerequisite for the probabilistic descriptor, leaving one test iteration inadmissible under FR-075. | spec/test-cases/TC-973-composed-admission-stage-projection-preserves-stage-ownership.md:18-24; spec/functional/FR-075-compute-candidates-from-registered-backends.md:160-168 | correct-requirement-no-evidence |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-075-AC-7 names identity, advertised pairs and evidence-kind set when defining an equal complete descriptor, but `domains` is also retained on the descriptor by FR-057 and passed in ADR-013 C-28. TC-981 checks only an evidence-kind difference. The domain-only duplicate outcome is therefore unspecified/unverified against FR-290's complete-manifest equality rule. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:43-57, 261, 267; spec/functional/FR-057-admit-shared-capability-kinds.md:262-278; spec/test-cases/TC-981-evidence-kind-difference-conflicts-duplicate-registration.md:18-31; QSpec FR-290 at 3121fa4:252-265 |
| FND-005 | medium | The revised candidate rule has a named registered-backend branch, but the new exact-evidence test and the updated TC-193 exercise only unnamed requests. The named branch's evidence-kind mismatch has no direct verification. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:101-109, 255-266; spec/test-cases/TC-193-candidate-set-matches-advertised-kind.md:13-18, 46-49; spec/test-cases/TC-980-probabilistic-candidates-require-exact-evidence-kind.md:18-24 |

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
| FND-004 | still-open | Descriptor equality criteria and TC-981 omit a domain-only descriptor conflict case. |
| FND-005 | still-open | No test exercises evidence-kind filtering for a named registered backend. |

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
FND-006 is fixed: FR-075-AC-7 explicitly excludes manifest digest from
descriptor equality and TC-982 no longer varies it. FND-007 and FND-008 are
fixed by the matching complete-descriptor rule in ADR-013 and evidence-kind
field in ADR-014. FND-009 remains open because ADR-014 still says FR-290 has
ten kinds.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-007 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-008 | fixed | fixed 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-009 | still-open | ADR-014 §2 continues to state “QSpec FR-290 has ten kinds,” inconsistent with FR-057's twelve-label vocabulary. |

## Dispositions — round 6

Rechecked QSL PR #716 at exact head `82f2f5eb5b2344f4be06b729160b01d6aa8da0e5`. FND-009 is fixed on this branch by commit `103a77ba12fc824bde273f754222a8e2aeedaa46`; the prior round-5 tracker disposition named sibling commit `3601c146b86e813e4ad9f330262e6619ce28a6c0`, which is not an ancestor of this head. No new finding was identified in the later count correction or review-artifact whitespace normalization.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-009 | fixed | fixed 103a77ba12fc824bde273f754222a8e2aeedaa46 |

### Fixed excerpt at 82f2f5eb5b2344f4be06b729160b01d6aa8da0e5

FND-009, ADR-014:

> **QSpec, capabilities.** QSpec FR-290 has twelve kinds and two modes. It has no liveness kind and no quantifier kind: a temporal claim under either profile is `temporal-satisfaction`, and quantification, induction and composition are obligation forms inside a claim that the selected backend's arm settles.
