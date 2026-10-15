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
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: references
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
---
# FR-115: Run an operation frame over an invocation through the spine

## Description

Each operation frame records one `operation-contract` requirement (FR-104,
FR-114). FR-109's `qsl_replay::spine::run_clause` SHALL run that frame on
its own: a `Frame` selection names the operation, the entry admits the
supplied invocation, and the `ProtocolClause` S6a `evaluate` arm checks the
invocation against the operation's frame. A write, creation, deletion or
relationship link change outside it is a violation verdict that carries the evaluated frame witness,
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

- A `ClauseRunReport` (FR-109): the frame run's disposition, the compiled
  package's `package_id`, and usage.

## Frame witness

An evaluated frame violation carries one `FrameWitness` with exactly these
typed members:

| Member | Value |
| --- | --- |
| `code` | `frame_violation` |
| `cause` | `unauthorized-change` |
| `invocation` | FR-106 `DocumentRef` of the checked invocation |
| `pre` | FR-106 `DocumentRef` of its pre snapshot |
| `post` | FR-106 `DocumentRef` of its post snapshot |
| `population` | Declaration identity of the changed population; for a link change, the population of its source end |
| `change` | One change variant below |

`change` is exactly one of `FieldWrite { object, field, modifies }`,
`Created { object, type_name, creates }`,
`Deleted { object, type_name, deletes }`, or
`Retyped { object, pre_type, post_type }`, or
`Link { relationship, difference, source, target, modifies }`. `object` is the changed key;
`field` is the declared field name; each type or permission entry is a
`DeclarationKey` of the compiled package. `modifies`, `creates` and
`deletes` retain their checked permission entries in declared order.
`Retyped` has no authorizing permission.

`Link` names one changed instance of a relationship (FR-364) that FR-106's
check 11.5 refuses. `relationship` is the `DeclarationKey` of the
relationship node, the key a `modifies` entry names (QSpec FR-340).
`difference` is `added` when the instance occurs more often in the post
extent than in the pre extent, and `removed` when it occurs less often.
`source` and `target` are the instance's ends, each an object reference
naming the declaration identity of its population and its object key, as an
instance end of an FR-106 snapshot does. `modifies` retains the frame's
checked `modifies` entries, field and relationship entries alike, in
declared order; `creates` and `deletes` grant no link and are not retained.
A link between two populations has no single changed population, so the
witness's `population` is the one of its `source`.

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
- If check 11's steps 1 to 3, or its step 5, find a change outside the
  frame, then the entry SHALL report stage `evaluate`, `violation`, `truth: false`, exit 10,
  with the evaluated frame witness: the invocation, the pre and post
  selections, the object and the field (or the created or deleted object and
  its type; or, for step 5, the relationship, the changed instance and
  whether it was added or removed), and the frame permission it was checked
  against. The witness's cause is the catalog's `frame_violation`/`unauthorized-change`, whose
  payload it carries.
- A step 5 finding SHALL be the `Link` change of the first changed instance in
  step 5's order: relationships in ascending `DeclarationKey` order, then
  instances in ascending source reference key, then ascending target
  reference key (the reference-key order of FR-364's reads). An instance
  whose count differs between the pre and post extent is named once, whatever
  the difference in count. Step 5 runs once after every population's steps 1
  to 4, so a step 1 to 4 finding in any population precedes a link finding,
  and a step 4 refusal of any population is reported instead of it. Links
  that step 5 leaves out (a link with an end at an object the invocation
  `deleted` lists, a relationship not listed complete in both snapshots, a
  relationship the frame's `modifies` names) produce no witness.
- If check 11's step 4 finds a declared delta that disagrees, then the
  entry SHALL report stage `evaluate`, `refusal`, `population_delta_mismatch`/
  `delta-disagreement`, exit 20: the invocation's own inventory is
  inconsistent, so the frame cannot be evaluated over it.
- The frame SHALL come only from the compiled package. The request carries no
  frame and no permission, so no caller can widen it (QSpec FR-013-AC-3).
- A stale identity refuses before evaluation, by the rules FR-109
  and FR-106 already state: an expected `package_id` that differs from the
  recompiled one (stage `compile`, `stale_dependency`); invocation or
  snapshot bytes that differ from the selection
  (`stale_dependency`/`content-mismatch`, QSpec FR-272-AC-14); a
  document whose `model` differs from the package's selection
  (`invalid_model_binding`/`wrong-model-selection`).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-115-AC-1 | Over the ConfigVersion unit and package (FR-108 fixtures), `Frame { operation: Config::ConfigVersion::attemptUpdate, invocation: changed-version }`, whose post changes only `child.versionNumber`, reports `evaluate`, `success`, `truth: true`, exit 0, with the compiled package's `package_id`. | Test |
| FR-115-AC-2 | The forbidden-parent-change invocation (post sets `child.parent` absent) reports `evaluate`, `violation`, `truth: false`, exit 10, with a frame witness naming `child`, `parent` and the frame's `modifies` (`versionNumber`), cause `frame_violation`/`unauthorized-change`. A post that adds an object to `config_history` reports `violation` with a witness naming its creation. An invocation declaring `created: [child]` reports `evaluate`, `refusal`, `population_delta_mismatch`/`delta-disagreement`, exit 20. | Test |
| FR-115-AC-3 | Stale identity: an expected `package_id` from another unit reports stage `compile`, `stale_dependency`, naming both; invocation bytes edited under their selected digest report stage `admit`, `stale_dependency`/`content-mismatch`; an invocation whose `model` digest differs reports `invalid_model_binding`/`wrong-model-selection`; an invocation of operation `probe` reports `wrong_snapshot`/`wrong-invocation`. None reaches `evaluate`. | Test |
| FR-115-AC-4 | `Frame { operation: Config::ConfigVersion::missing }`, `Frame` on an operation no clause or attempt names, `Frame` on `Nope::ConfigVersion::attemptUpdate` (an alias no `model` declaration binds) and `Frame` on `Config::Missing::attemptUpdate` (a type the selected package does not declare) each report stage `select`, `missing_declaration`/`missing-name`. An invocation whose pre snapshot is absent from the provision reports `admit`, `incomplete`, `unavailable_observation`, not a violation. | Test |
| FR-115-AC-5 | Running one `Frame` request twice gives equal reports, including usage. | Test |
| FR-115-AC-6 | The entry resolves `M::T::op` once, to the object type's `DeclarationKey` in the package `M` selects and the operation's identifier, and selects by that resolution. Over a package where `Sub` specializes `ConfigVersion` and declares no operation, `Config::Sub::attemptUpdate` resolves to `Sub`'s key and selects `ConfigVersion`'s frame with `Sub` as context. In a unit selecting two packages that both declare `ConfigVersion`, as `Config` and `Copy`, each alias resolves `ConfigVersion` to its own package's key and selects that package's frame. | Test |
| FR-115-AC-7 | The evaluated field write, creation, deletion and retype frame violations each carry exactly the FrameWitness members and the applicable change variant above, retaining the checked declaration keys and selected document references. | Test |
| FR-115-AC-8 | Over TC-1035's `orders` unit selected as `Orders`, `Frame { operation: Orders::Pump::rehome, invocation: I }`, whose post moves `p1` from `t1` to `t2` in the complete `Pump-housing-Tank`, reports `evaluate`, `success`, `truth: true`, exit 0 when `rehome` modifies `Pump-housing-Tank`, and `evaluate`, `violation`, `truth: false`, exit 10 when its `modifies` is empty. The witness carries cause `frame_violation`/`unauthorized-change`, the three document references, population `Q`, and the `Link` change naming `Pump-housing-Tank`, `removed`, source `p1` and target `t1`, with `modifies` empty. A post adding `p3` with `p3`-`t1`, `created: [p3]` and a frame that creates `Pump` and does not modify the relationship reports the `Link` change naming `p3`-`t1` `added`. A frame that `modifies` only a field of `Pump` retains that entry in the witness's `modifies`. | Test (TC-1037) |
| FR-115-AC-9 | With several changed links the witness names the first by step 5's order: a changed link of `Pump-housing-Tank` before one of `Sys-composition-Pump`; within one relationship the lower source before the higher whatever the difference, then the lower target, so a `removed` instance has no precedence over an `added` one. A step 1 to 3 finding in any population is reported instead of a link finding, and a step 4 disagreement is reported as `population_delta_mismatch`/`delta-disagreement`, exit 20, instead of a link violation. Running one request twice gives equal reports. | Test (TC-1037) |
| FR-115-AC-10 | Step 5 reports no witness for a link an end of which is an object the invocation deletes under a frame that deletes its type, for a relationship listed complete in only one of the two snapshots or incomplete in either, or for a relationship the frame modifies; each reports `evaluate`, `success`, `truth: true`, exit 0. | Test (TC-1037) |

## Dependencies

- FR-106 (invocation admission and the frame check), FR-107 (the
  `ProtocolClause` S6a family kind), FR-109 (the run entry, its report and
  exit codes), FR-104 and FR-114 (the frame's requirement record).
- FR-069 and FR-070 (the violation verdict with its witness).
- QSpec FR-013 (frame enforcement), `native-diagnostics.md`
  (`frame_violation`, `population_delta_mismatch`).

## References

- TC-514 exercises the Frame selection and its typed witness; TC-1037
  exercises the link change.
