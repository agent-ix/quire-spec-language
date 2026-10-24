---
id: SR-1362
title: "Evidence review of QSL-657 capability vocabulary alignment"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language@4f9c08af9ab3086fc9ce3e216b6f2b3e70c63bba; FR-057, FR-075, FR-080, TC-153, TC-970–979, spec/model-linking/tests.md, spec/tests.md; QSpec FR-290 origin/main@7a81a2f17323eb5c9752bb4997ac913c8618ffc8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-290
    type: references
---
# SR-1362: Evidence review of QSL-657 capability vocabulary alignment

## Summary

Reviewed the changed Test verification obligations against their test cases and QSpec FR-290's probabilistic evidence contract. The admission tests establish accepted labels and invalid-label refusals, but the draft does not specify or verify evidence-kind-sensitive candidate computation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-075 and TC-974/975 cover evidence-kind admission and retention only. They do not ensure that a candidate for an `exact` item advertises `exact`, or that `statistical` cannot substitute; QSpec FR-290 requires this candidate rule. Add the missing registry obligation and tests for exact/statistical separation. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:80-98, 245-247; spec/test-cases/TC-974-statistical-evidence-kind-is-admitted-for-probabilistic-backends.md:13-25; spec/test-cases/TC-975-exact-evidence-kind-is-admitted-for-probabilistic-backends.md:13-27; QSpec FR-290 at 7a81a2f:223-244 |
| FND-002 | medium | TC-973's descriptor-admission step does not set a valid evidence kind for the `probabilistic-satisfaction` iteration, so that iteration tests a refusal rather than the stated stage projection. | spec/test-cases/TC-973-composed-admission-stage-projection-preserves-stage-ownership.md:18-24; spec/functional/FR-075-compute-candidates-from-registered-backends.md:160-168 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The amended FR-075 candidate procedure specifies both a named registered-backend case and an unnamed registry scan. TC-193 and TC-980 verify exact evidence matching only with unnamed requests, leaving the named-backend evidence mismatch unverified. Add named `statistical`/`exact` cross-mismatch cases. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:101-109, 255-266; spec/test-cases/TC-193-candidate-set-matches-advertised-kind.md:13-18, 46-49; spec/test-cases/TC-980-probabilistic-candidates-require-exact-evidence-kind.md:18-24 |

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
| FND-003 | still-open | No test exercises evidence-kind filtering for a named registered backend. |

### Fixed excerpts at e9234569daf861f451b8d761fdc228250d929ec1

FND-001, from FR-075:

> For each requested item, the registry SHALL compute the candidate set as
> follows, matching a registration to the item's capability kind and, when that
> kind is `probabilistic-satisfaction`, the item's evidence kind. A probabilistic
> registration is a candidate only when its admitted evidence-kind set contains
> the item's exact evidence-kind label. `statistical` SHALL NOT substitute for
> `exact`, nor `exact` for `statistical`.

FND-002, from TC-973:

> 1. Admit a requested pair through the composed linker while varying whether a
>    registry contains a matching backend; for `probabilistic-satisfaction`,
>    give the item the evidence kind `statistical`.
> 2. Admit a backend descriptor advertising the kind, supplying
>    `{statistical}` evidence kinds for `probabilistic-satisfaction`, then
>    inspect the registry and its candidate result for the matching item.

## Dispositions — round 2

Rechecked PR #716 at exact published head `3789a2b6b21aee071b3687f143ecd48d5afc28ca`, against QSL baseline `2a90a2df0b29617e7c63aa440d291371efe906b4`. TC-983 closes the remaining evidence-analysis gap by covering named-backend match and mismatch controls for both admitted evidence kinds.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | fixed 3789a2b6b21aee071b3687f143ecd48d5afc28ca |

### Fixed excerpt at 3789a2b6b21aee071b3687f143ecd48d5afc28ca

FND-003, TC-983:

> | Requested item evidence kind | Named backend's admitted evidence kind | Control | Expected named candidate outcome |
> | `statistical` | `exact` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |
> | `exact` | `statistical` | Evidence-kind mismatch | Empty candidate set; it is not an unknown-backend marker. |

## Dispositions — round 6

Rechecked the existing evidence-review finding at exact head `82f2f5eb5b2344f4be06b729160b01d6aa8da0e5`. FND-003 remains fixed by commit `3789a2b6b21aee071b3687f143ecd48d5afc28ca`; TC-983 still covers both named-backend evidence-kind mismatch directions. No new evidence-review finding was identified. This round adds no disposition row because the finding's latest outcome is already fixed.
