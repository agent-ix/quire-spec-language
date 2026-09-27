---
id: FR-115
title: "Run an operation frame over an invocation through the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
---
# FR-115: Run an operation frame over an invocation through the spine

## Description

Each operation frame records one `operation-contract` requirement (FR-104,
FR-114). FR-109's `qsl_replay::spine::run_clause` SHALL run that frame on
its own: a `Frame` selection names the operation, the entry admits the
supplied invocation, and the `ProtocolClause` S6a `evaluate` arm checks the
invocation against the operation's frame and reports `frame_violation`/
`unauthorized-change` when it writes, creates or deletes outside it (ADR-012
§12.2 Evaluate row). The check is the one FR-106's step 11 runs
(`enforce_frame` through `admit_invocation`, with the operation's
`OperationEffect`); this requirement adds a way to select the frame as the
thing under test.

## Inputs

- A `ClauseRunRequest` (FR-109) whose selection is `Frame { operation,
  invocation }`: `operation` is the `QualifiedName` `M::T::op`, and
  `invocation` a `DocumentRef` (FR-106) naming a `quire.state.invocation/v1`
  document, with its pre and post snapshots in the provision.

## Outputs

- A `ClauseRunReport` (FR-109) whose provenance holds the operation, the
  identity of its frame node, and the identity and digest of the invocation
  and both snapshots.

## Behavior

- The entry SHALL resolve `operation` as FR-104's Resolution resolves a
  clause's operation, over the compiled package. If it names no operation,
  or an operation that no clause and no attempt names (so the package holds
  no frame node for it, FR-105), then the entry SHALL report stage `select`,
  `missing_declaration`/`missing-name`.
- Admission SHALL run FR-106's checks 1 and 3 to 10 over the invocation and
  its two snapshots, with check 5 comparing the invocation's `context` and
  `operation` to the selected operation. A failure SHALL report stage
  `admit` with FR-106's code, cause and record. An absent document SHALL
  report `incomplete`, `unavailable_observation`, and never a frame
  violation (QSpec FR-013-AC-4).
- The S6a `ProtocolClause` `evaluate` arm SHALL run FR-106's check 11 over
  the admitted pre and post observations with the selected operation's
  `OperationEffect`, in check 11's order, and report the first violation.
- If the check passes, then the entry SHALL report stage `evaluate`,
  `success`, `truth: true`, exit 0.
- If the check finds a change outside the frame, then the entry SHALL report
  stage `evaluate`, `refusal`, `frame_violation`/`unauthorized-change`,
  retaining the invocation, the pre and post selections, the object and the
  field (or the created or deleted object and its type), and the frame
  permission it checked against. If it finds a declared delta that
  disagrees, then the entry SHALL report `population_delta_mismatch`/
  `delta-disagreement` the same way. Both map to O-16 refusal (ADR-012 §15.6)
  and exit 20.
- The frame SHALL come only from the compiled package. The request carries no
  frame and no permission, so no caller can widen it (QSpec FR-013-AC-3).
- A stale identity or version refuses before evaluation, by the rules FR-109
  and FR-106 already state: an expected `package_id` that differs from the
  recompiled one (stage `compile`, `stale_dependency`); invocation or
  snapshot labels or bytes that differ from the selection
  (`stale_dependency`/`revision-mismatch` or `byte-digest-mismatch`); a
  document whose `model` differs from the package's selection
  (`invalid_model_binding`/`wrong-model-selection`).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-115-AC-1 | Over the ConfigVersion unit and package (FR-108 fixtures), `Frame { operation: Config::ConfigVersion::attemptUpdate, invocation: changed-version }`, whose post changes only `child.versionNumber`, reports `evaluate`, `success`, `truth: true`, exit 0, with the frame node's identity and the three document identities in its provenance. | Test (TC-514) |
| FR-115-AC-2 | The forbidden-parent-change invocation (post sets `child.parent` absent) reports `evaluate`, `refusal`, `frame_violation`/`unauthorized-change` naming `child` and `parent` and the frame's `modifies` (`versionNumber`), exit 20. An invocation declaring `created: [child]` reports `population_delta_mismatch`/`delta-disagreement`. A post that adds an object to `config_history` reports `frame_violation`/`unauthorized-change` naming its creation. | Test (TC-514) |
| FR-115-AC-3 | Stale identity and version: an expected `package_id` from another unit reports stage `compile`, `stale_dependency`, naming both; invocation bytes edited under their selected digest report stage `admit`, `stale_dependency`/`byte-digest-mismatch`; an invocation with another `revision` label reports `stale_dependency`/`revision-mismatch`; an invocation whose `model` digest differs reports `invalid_model_binding`/`wrong-model-selection`; an invocation of operation `probe` reports `wrong_snapshot`/`wrong-invocation`. None reaches `evaluate`. | Test (TC-514) |
| FR-115-AC-4 | `Frame { operation: Config::ConfigVersion::missing }` and `Frame` on an operation no clause or attempt names each report stage `select`, `missing_declaration`/`missing-name`. An invocation whose pre snapshot is absent from the provision reports `admit`, `incomplete`, `unavailable_observation`, not `frame_violation`. | Test (TC-514) |
| FR-115-AC-5 | Running one `Frame` request twice gives equal reports, including usage. | Test (TC-514) |

## Dependencies

- FR-106 (invocation admission and the frame check), FR-107 (the
  `ProtocolClause` S6a family kind), FR-109 (the run entry, its report and
  exit codes), FR-104 and FR-114 (the frame's requirement record).
- QSpec FR-013 (frame enforcement), `native-diagnostics.md`
  (`frame_violation`, `population_delta_mismatch`).

## Status

Specified under QSL-296 (QSL-21a). Not yet implemented; QSL-300 (QSL-21e)
implements it.
