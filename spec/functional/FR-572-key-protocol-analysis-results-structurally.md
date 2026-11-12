---
id: FR-572
title: "Key protocol analysis results structurally and deterministically"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-175
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
---
# FR-572: Key protocol analysis results structurally and deterministically

## Description

Every projection and realizability result SHALL carry the structural key of
QSpec FR-174-AC-6 or FR-175-AC-6 as typed members, SHALL never stand in a
digest for a member, and SHALL be a function of its key. It implements
ADR-036 PA-13 to PA-15 and QSpec FR-174-AC-6 and FR-175-AC-6.

## Inputs

The members of the key (Outputs).

## Outputs

`ProtocolAnalysisKey`, stored on the result as a structured value. For a
projection its members are: the FR-288 qualified definition key, the role,
the profile, the channel, choice and visibility premises as selected
(FR-054) and the bounds. For a realizability result they are: the definition
key, the strategy profile, the role knowledge and visibility, the channel and
environment premises, the bounds and the producing analyzer identity and
version. `Bounds` has two key members: `max_depth` (the horizon in events
along a path or play) and `max_history` (local history length,
realizability only) (PA-14).

The role-knowledge and environment premises are optional-valued key members
(PA-8): `None` ("not stated") and `Some` of an empty set ("declared empty")
are both **present** values of the member and are different keys. A key
struct with the member missing altogether is the absent member that is
refused.

## Behavior

- Constructing a key shall fail on the first absent member with
  `invalid_runtime_input`/`missing-member` naming it; no member shall be
  defaulted. A reader shall rebuild the key from a stored result and refuse
  an artifact with an absent member the same way (PA-13).
- Display name, path, span, producing run and timestamp shall not be fields
  of the key and shall not change it (PA-13). The assessed-subject root
  (FR-574) is a request input, not a key member, and shall not change it.
  The root copied into a counterexample's separating witness record is
  request-scoped settlement data, not part of the derived artifact the key
  identifies.
- `max_depth` and `max_history` shall have no default, and a search that
  completes its method up to the horizon without a counterexample is
  `inconclusive` (FR-573). `max_states`,
  `max_transitions` and the evaluation meter are run limits with published
  defaults outside the key (PA-14; ADR-018 V-7).
- One key shall yield one result. Collections shall be ordered by checked
  node id or an explicit canonical order, and no hash order, randomness,
  clock or scheduler shall reach the result. The analyzer identity is
  `qsl-analyze::protocol_analysis` with the crate version (PA-15).
- A projection shall be a function of its five members only; its analyzer
  identity is not a member, so a change that alters any projection output is
  a requirement change (PA-15).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-572-AC-1 | Changing the definition key, the role, the profile, a channel, choice or visibility premise, or a bound yields a different projection key, and changing only display name, path, span or producing run yields an equal key (implements FR-174-AC-6). | Test (TC-1412) |
| FR-572-AC-2 | Changing the definition key, the strategy profile, the role knowledge or visibility, a channel or environment premise, a bound or the analyzer identity or version yields a different realizability key, and changing only display name, path, span or producing run yields an equal key (implements FR-175-AC-6). | Test (TC-1412) |
| FR-572-AC-3 | The result carries every key member as a typed value, and the key holds no digest: no member is a digest of the artifact or of another member. A realizability key with the role-knowledge or environment premise `None` is present and unequal to one with that premise `Some` of the empty set; neither is refused. | Test (TC-1412) |
| FR-572-AC-4 | For each key member in turn, building a key without it, and reading a stored result with it removed, are each refused `invalid_runtime_input`/`missing-member` naming that member, and no member takes a default. An optional-valued premise set to `None` is not an absent member. | Test (TC-1412) |
| FR-572-AC-5 | Two runs of one analyzer identity and version on one realizability key and one assessed-subject root return the same result, and two runs on one projection key and one assessed-subject root return the same result, byte for byte on the result's serialized form (implements FR-175-AC-6; PA-15). | Test (TC-1412) |
| FR-572-AC-6 | A run stopped by `max_states`, `max_transitions` or the evaluation meter leaves the key unchanged between runs with different values of those limits, and the stopped run carries no artifact (PA-14). A request with the assessed-subject root changed alone yields an equal key. | Test (TC-1412) |

## Dependencies

- ADR-036 PA-13 to PA-15; ADR-018 V-7.
- [FR-570](FR-570-project-a-global-protocol-onto-one-role.md), [FR-571](FR-571-decide-whether-local-strategies-realize-a-global-protocol.md).
- QSpec FR-174, FR-175, FR-288, FR-054.

## References

- ADR-036; QSL-1889, QSL-1893.
