---
id: FR-122
title: "Replay a state-clause counterexample through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: traces_to
---
# FR-122: Replay a state-clause counterexample through the replay facade

## Description

A state-clause counterexample is an observation under which a state clause
(an invariant, a precondition or a postcondition, FR-104) evaluates to
`false`. QSL SHALL replay one through the layer-6 replay facade (FR-098):
`qsl_replay::replay_state_clause` takes FR-098's request and a
`WitnessEnvelope<StateClauseCounterexample>`, recompiles the package, checks
the envelope's clause identities against the recompile before any
admission, admits
the observation by FR-106, evaluates the clause once by FR-107 (the
evaluation `spine::run_clause` runs for a `Clause` selection, FR-109), and
settles an FR-072 replay result (ADR-013 O-25 to O-27).

The state-clause counterexample is the state-clause family's own payload on
the FR-070 envelope: `StateClauseCounterexample`, which implements
`FamilyPayload` (FR-070-AC-5).

## Inputs

- A `WitnessEnvelope<StateClauseCounterexample>` (FR-070) whose payload
  holds:
  - `clause`: the state clause's declared name, as FR-106's
    `ClauseSelection` names it;
  - `observation`: exactly one of FR-106's selection inputs, by the
    clause's kind:
    - `PreCall { snapshot, self, parameters }` for a precondition: the
      `DocumentRef` of a `quire.state.snapshot/v1` document whose
      `observation` is `pre`, the operation's pre state; the self object
      (`{population, key}`); and each declared parameter's value in the
      snapshot value form. A violated precondition means the operation
      never ran, so the observation holds no post state;
    - `Invocation { invocation }` for a postcondition: the `DocumentRef`
      (FR-106) of a `quire.state.invocation/v1` document. It names the self
      object, the parameters and result, and a `pre` and a `post` snapshot,
      each by identity and `sha256-jcs` digest;
    - `Current { snapshot, anchor, self }` for an invariant: the
      `DocumentRef` of a `quire.state.snapshot/v1` document whose
      `observation` is `current`, the initialization or handler anchor
      (`{kind, name}`) it was taken at, and the self object
      (`{population, key}`).
- The envelope's members for a state-clause packet, which carry the
  clause's identity: its `clause_node` is the checked node identity
  (`WireNodeId`) of the clause's `state`/`state_clause` node (FR-105), its
  `occurrence_key` is the clause declaration's `claim` occurrence (FR-104,
  FR-105), which tells apart two declarations that share one node, and its
  `obligation_identity` is the O-09 obligation identity of that clause's
  `operation-contract` record (FR-104 "Requirements"), which CG mints.
- Of the envelope, the executor reads the `package_id`, `clause_node`,
  `occurrence_key` and the `ReplaySource` variant, which selects the
  result's arm. Every value the clause reads (`self`, the operation's
  parameters and its result, and the objects of each observation) comes
  from the payload's admitted observation, so the `ReplaySource`'s transcript or
  `Input` assignments bind nothing. The envelope's `obligation_identity` and
  `selected_function` are CG's members; the executor reads neither and the
  result carries neither.
- FR-098's package reference, byte provision and limits: the request's
  `quire.value.accounting/v1` limits build the evaluation meter, and its
  stage limits bound the recompile as FR-098 states. Every document the
  observation names (the invocation and both its snapshots, or the one
  snapshot of `PreCall` or `Current`) is a `sha256-jcs` entry of the byte
  provision. Each object in a snapshot carries every declared field
  (FR-106 check 6). A population marked `complete` holds every object of
  that population (FR-106 check 7).

## Outputs

- An FR-072 replay result on the envelope's arm, or a typed `ReplayRefusal`
  with no partial result. A result retains the source identity and digest,
  the `package_id`, the payload's `clause`, the envelope's `clause_node` and
  `occurrence_key`,
  the identity and digest of every document admission read, for a
  `PreCall` observation its self object and parameter values, the
  evaluated value when there is one, the evaluation charges and the executor's
  toolchain pin.
- A `Witness`-arm result that reproduces (`false`) carries the FR-351
  record FR-098 gives a Boolean verdict (FR-072, FR-098-AC-2): the
  evaluated Boolean as its deciding element, index 0, an empty value path
  and no trace position. A result that settles `inconclusive`, `Verdicts`
  (`true`) carries the evaluated value and no FR-351 record, since its
  settlement basis is not decisive (FR-072). An `Input`-arm result carries
  the evaluated value and no FR-351 record. A result with no value carries
  neither (FR-072).

## Behavior

- The executor SHALL recompile and check the package by FR-098's rules, in
  FR-098's order, before it resolves the payload's `clause`.
- If the recompiled `package_id` differs from the request's or the
  envelope's, then the executor SHALL refuse by FR-098's stale `package_id`
  rule.
- The executor SHALL resolve `clause` in the recompiled package's one name
  table of state clauses and functions (FR-109). If it names no state
  clause, then the executor SHALL refuse `missing_declaration`/
  `missing-name`.
- If the resolved declaration's `state`/`state_clause` node identity differs
  from the envelope's `clause_node`, then the executor SHALL refuse
  `stale_dependency`/`revision-mismatch` before any admission, naming the
  envelope's and the recompiled node identity.
- If the node identities agree and the declaration's `claim` occurrence key
  differs from the envelope's `occurrence_key`, then the executor SHALL
  refuse `stale_dependency`/`revision-mismatch` before any admission,
  naming the envelope's and the recompiled occurrence. The node goes first
  because the occurrence key holds the node identity. No identity is
  recovered from a display name.
- If the observation's form does not match the resolved clause's kind
  (`PreCall` for a precondition, `Invocation` for a postcondition,
  `Current` for an invariant), then the executor SHALL refuse
  `wrong_snapshot`/`wrong-observation` before any admission, naming the
  clause kind and the observation form.
- The executor SHALL admit the observation by FR-106 with the selection
  (`clause`, `observation`), reading every document only from the byte
  provision by its `sha256-jcs` digest, under FR-106's default
  `ObservationLimits` (no `quire.value.accounting/v1` counter names an
  admission limit). For an `Invocation` observation, admission runs
  FR-106's frame and delta check (check 11) over the pre and post
  snapshots; a `PreCall` observation has no post snapshot and admission
  runs no frame check.
- If admission fails, refused or incomplete, then the executor SHALL refuse
  with a `ReplayRefusal` holding FR-106's record, and SHALL evaluate
  nothing. An observation whose required population is not marked
  `complete` is FR-106's `Incomplete` (`incomplete_population`/
  `incomplete-scope`, check 7), so it refuses this way with that record and
  settles no result.
- The executor SHALL evaluate the admitted clause once by FR-107, with a
  fresh evaluation meter built from the request's
  `quire.value.accounting/v1` limits, and compare verdicts as FR-072 does. The counterexample refuted the clause, so the
  proved verdict is `violation`.
  - If the evaluation completes `false`, then the executor SHALL settle the
    agreement of the envelope's arm: `reproduced-with-evaluated-witness` on
    the `Witness` arm, `reproduced-without-witness` on the `Input` arm. The
    agreement is between the proved verdict and the clause's evaluation
    over the admitted documents.
  - If the evaluation completes `true`, then the executor SHALL settle
    `inconclusive` with cause `Verdicts` (`violation` proved, `success`
    replayed), never repaired (FR-072).
  - If the evaluation completes no value (a `refused`, `incomplete` or
    `undefined` S6a outcome, or a family result), then the executor SHALL
    settle `inconclusive` with cause `NoValue` (FR-072). An exhausted
    evaluation meter is an `incomplete` outcome and settles this way.
  - If the evaluation ends in an outcome FR-100 handles as an internal
    failure (the kernel `Refusal::CheckedInvariant`, or
    `CallFailure::Fault`), then the executor SHALL refuse with the
    `InternalFault` and settle no result.
  - If FR-107 returns `CallFailure::Input` (`UnknownClause` or
    `ObservationsMismatch`), which name lookup and admission for the same
    clause rule out, then the executor SHALL refuse with an
    `InternalFault` and settle no result.
- Replay SHALL read no path, environment variable, clock or search location,
  and SHALL give the same result for the same request and envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-122-AC-1 | Over FR-108's ConfigVersion unit, an envelope for `VersionUnchanged` whose observation is the changed-version invocation settles `reproduced-with-evaluated-witness`, and the same payload on an `Input`-arm envelope settles `reproduced-without-witness`. An envelope for `ParentOrder` whose observation is violating-parent's current snapshot (anchor `handler validate`, self `child`) settles `reproduced-with-evaluated-witness`. Each result holds the source digest, the `package_id`, the payload's `clause`, the envelope's `clause_node` and `occurrence_key`, and the identity and digest of every document admission read: the invocation and both snapshots, or the one current snapshot. Each `Witness`-arm result's FR-351 record holds `false` as its deciding element, index 0, an empty value path and no trace position; the `Input`-arm result holds `false` and no FR-351 record. | Test (TC-517) |
| FR-122-AC-2 | The `VersionUnchanged` envelope over the unchanged-version invocation, and the `ParentOrder` envelope over healthy-parent's snapshot, each settle `inconclusive`, `Verdicts`, holding `violation` and `success`, the evaluated `true` and no FR-351 record. The `ParentOrder` violating-parent envelope replayed with the request's `quire.value.accounting/v1` evaluation budget at zero settles `inconclusive`, `NoValue`. | Test (TC-517) |
| FR-122-AC-3 | Stale identity: the changed-version `VersionUnchanged` envelope whose `clause_node` names `ParentOrder`'s node refuses `stale_dependency`/`revision-mismatch` naming the envelope's and the recompiled node identity, and names the nodes the same way when its `occurrence_key` is also at ordinal 1; with only its `occurrence_key` at ordinal 1 it refuses the same way naming both occurrences; none admits a document (its invocation absent from the byte provision changes none of these refusals). A source edit that changes the `package_id` refuses by FR-098's stale `package_id` rule. A `clause` naming `Absent`, and one naming the function `sameIdentity`, each refuse `missing_declaration`/`missing-name`. | Test (TC-517) |
| FR-122-AC-4 | Precondition: over TC-466 step 3's `probe` unit (precondition `ReachesTarget`, `reaches(self, target, parent)`, over the chain `a -> b -> c`), an envelope whose observation is `PreCall` over the chain's pre snapshot with `self` `a` and `target` `a` settles `reproduced-with-evaluated-witness`, holding that one snapshot's identity and digest and no post snapshot; with `target` `c` it settles `inconclusive`, `Verdicts`. The result holds `self` `a` and `target` `a` as the values it reproduced. Form mismatches refuse `wrong_snapshot`/`wrong-observation` with no admission (the observation's documents absent from the byte provision change none of these refusals): `ReachesTarget` with an `Invocation` observation over the `probe` invocation, `VersionUnchanged` with a `PreCall` observation, and `VersionUnchanged` with a `Current` observation over healthy-parent's snapshot. | Test (TC-517) |
| FR-122-AC-5 | Admission refusals settle no result: the `VersionUnchanged` envelope over the forbidden-parent-change invocation refuses with FR-106's `frame_violation`/`unauthorized-change` record naming `child` and `parent`; the changed-version envelope with its pre snapshot absent from the byte provision refuses with FR-106's `unavailable_observation` record; with its invocation bytes edited under the same digest it refuses `stale_dependency`/`byte-digest-mismatch`; the `ParentOrder` envelope over incomplete-population's snapshot (`complete: false`) refuses with FR-106's `Incomplete` `incomplete_population`/`incomplete-scope` record. | Test (TC-517) |
| FR-122-AC-6 | Replaying one envelope twice gives equal results. `StateClauseCounterexample` implements `FamilyPayload`, the envelope carries it as its generic parameter with no string-keyed field, and its `observation` is a sum of the `PreCall`, `Invocation` and `Current` inputs, so a payload holds exactly one. | Test (TC-517) |

## Dependencies

- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md)
  (envelope and `FamilyPayload`),
  [FR-072](FR-072-implement-typed-replay-result.md) (replay result),
  [FR-098](FR-098-execute-a-replay-request.md) (replay facade, recompile and
  refusal order), [FR-104](FR-104-check-state-clauses.md) and
  [FR-105](FR-105-emit-state-nodes.md) (clause node, `claim` occurrence and
  `operation-contract` record),
  [FR-106](FR-106-admit-snapshots-and-invocations.md) (documents and
  admission), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (evaluation),
  [FR-109](FR-109-run-a-state-clause-through-the-spine.md) (the clause run
  whose name lookup and evaluation replay shares),
  [FR-108](FR-108-run-the-configversion-spine-corpus.md) (the ConfigVersion
  corpus the criteria use).
- ADR-013 O-25 to O-27 and OQ-H (QSL owns the payload and the replay entry;
  CG builds the envelope); ADR-017 PF-4 and G-2 (one carrier per identity:
  the envelope carries the clause's, and the payload does not repeat it).
- FR-107-AC-3's `probe` unit
  (TC-466 step 3) is the precondition fixture.
- The analogue for a frame counterexample is
  [FR-116](FR-116-replay-a-frame-counterexample.md).
