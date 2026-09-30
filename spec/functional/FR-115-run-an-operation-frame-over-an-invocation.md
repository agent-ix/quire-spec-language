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
invocation against the operation's frame. A write, creation or deletion
outside it is a violation verdict that carries the evaluated frame witness,
the counterexample (ADR-012 §12.2 Evaluate row). The check is the one
FR-106's step 11 runs (`enforce_frame` through `admit_invocation`, with the
operation's `OperationEffect`); this requirement adds a way to select the
frame as the thing under test.

A frame check that ran and found a change outside the frame has evaluated:
its verdict is `violation`, not a refusal. A refusal stays for an input the
check cannot evaluate at all. This is the violation-with-witness shape of
the FR-069 proof result and the FR-070 counterexample, which the function
exemplar already uses. In a clause run the same finding is an admission
refusal (FR-106 check 11), because there the invocation is only input to the
clause under test.

## Inputs

- A `ClauseRunRequest` (FR-109) whose selection is `Frame { operation,
  invocation }`: `operation` is the `OperationName` `M::T::op` (model
  alias, object type and operation, a type the spine owns), and
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
  its two snapshots. Check 4 SHALL compare each document's `model` to the
  package's model selection for the model alias `M` that `operation`
  (`M::T::op`) names, since a `Frame` run has no clause. Check 5 SHALL
  compare the invocation's `context` and `operation` to the selected
  operation. A failure SHALL report stage
  `admit` with FR-106's code, cause and record. An absent document SHALL
  report `incomplete`, `unavailable_observation`, and never a violation
  (QSpec FR-013-AC-4).
- The S6a `ProtocolClause` `evaluate` arm SHALL run FR-106's check 11 over
  the admitted pre and post observations with the selected operation's
  `OperationEffect`, in check 11's order, and stop at the first finding.
- If check 11 finds nothing, then the entry SHALL report stage `evaluate`,
  `success`, `truth: true`, exit 0.
- If check 11's steps 1 to 3 find a change outside the frame, then the
  entry SHALL report stage `evaluate`, `violation`, `truth: false`, exit 10,
  with the evaluated frame witness: the invocation, the pre and post
  selections, the object and the field (or the created or deleted object and
  its type), and the frame permission it was checked against. The witness's
  cause is the catalog's `frame_violation`/`unauthorized-change`, whose
  payload it carries.
- If check 11's step 4 finds a declared delta that disagrees, then the
  entry SHALL report stage `evaluate`, `refusal`, `population_delta_mismatch`/
  `delta-disagreement`, exit 20: the invocation's own inventory is
  inconsistent, so the frame cannot be evaluated over it.
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
| FR-115-AC-2 | The forbidden-parent-change invocation (post sets `child.parent` absent) reports `evaluate`, `violation`, `truth: false`, exit 10, with a frame witness naming `child`, `parent` and the frame's `modifies` (`versionNumber`), cause `frame_violation`/`unauthorized-change`. A post that adds an object to `config_history` reports `violation` with a witness naming its creation. An invocation declaring `created: [child]` reports `evaluate`, `refusal`, `population_delta_mismatch`/`delta-disagreement`, exit 20. | Test (TC-514) |
| FR-115-AC-3 | Stale identity and version: an expected `package_id` from another unit reports stage `compile`, `stale_dependency`, naming both; invocation bytes edited under their selected digest report stage `admit`, `stale_dependency`/`byte-digest-mismatch`; an invocation with another `revision` label reports `stale_dependency`/`revision-mismatch`; an invocation whose `model` digest differs reports `invalid_model_binding`/`wrong-model-selection`; an invocation of operation `probe` reports `wrong_snapshot`/`wrong-invocation`. None reaches `evaluate`. | Test (TC-514) |
| FR-115-AC-4 | `Frame { operation: Config::ConfigVersion::missing }`, `Frame` on an operation no clause or attempt names, `Frame` on `Nope::ConfigVersion::attemptUpdate` (an alias no `model` declaration binds) and `Frame` on `Config::Missing::attemptUpdate` (a type the selected package does not declare) each report stage `select`, `missing_declaration`/`missing-name`. An invocation whose pre snapshot is absent from the provision reports `admit`, `incomplete`, `unavailable_observation`, not a violation. | Test (TC-514) |
| FR-115-AC-5 | Running one `Frame` request twice gives equal reports, including usage. | Test (TC-514) |
| FR-115-AC-6 | The entry resolves `M::T::op` once, to the object type's `DeclarationKey` in the package `M` selects and the operation's identifier, and selects by that resolution. Over a package where `Sub` specializes `ConfigVersion` and declares no operation, `Config::Sub::attemptUpdate` resolves to `Sub`'s key and selects `ConfigVersion`'s frame with `Sub` as context. In a unit selecting two packages that both declare `ConfigVersion`, as `Config` and `Copy`, each alias resolves `ConfigVersion` to its own package's key and selects that package's frame. | Test (TC-514) |

## Dependencies

- FR-106 (invocation admission and the frame check), FR-107 (the
  `ProtocolClause` S6a family kind), FR-109 (the run entry, its report and
  exit codes), FR-104 and FR-114 (the frame's requirement record).
- FR-069 and FR-070 (the violation verdict with its witness).
- QSpec FR-013 (frame enforcement), `native-diagnostics.md`
  (`frame_violation`, `population_delta_mismatch`).

## Status

Specified under QSL-296 (QSL-21a). Implemented under QSL-300 (QSL-21e). The
checked graph records each operation a clause or attempt names with its frame
node (`CheckedGraph::operation_frame`, resolved by `TypeEnvironment::operation`
as FR-104 resolves a clause's operation). `admit_frame_invocation` runs
FR-106's checks 1 and 3 to 10, and the `ProtocolClause` S6a `evaluate` arm
(`CheckedPackageEvaluation::evaluate_frame`) runs check 11 through the same
`decide_frame` a clause run's admission uses. `run_clause`'s `Frame`
selection reports a change outside the frame as
`ClauseDisposition::FrameViolation` with its `FrameWitness`. AC-1 to AC-5 are
tested (TC-514).
