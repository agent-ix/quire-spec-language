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
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
---
# FR-122: Replay a state-clause counterexample through the replay facade

## Description

A state-clause counterexample is an observation under which a state clause
(an invariant, a precondition or a postcondition, FR-104) evaluates to
`false`. QSL SHALL replay one through the layer-6 replay facade (FR-098):
`qsl_replay::replay_state_clause` takes FR-098's request and a
`WitnessEnvelope<StateClauseCounterexample>`, recompiles the package, checks
the payload's identities against the recompile before any admission, admits
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
  - `clause_node`: the checked node identity (`WireNodeId`) of the clause's
    `state`/`state_clause` node (FR-105);
  - `occurrence`: the clause declaration's `claim` occurrence key (FR-104,
    FR-105), which tells apart two declarations that share one node;
  - `observation`: exactly one of FR-106's two selection inputs:
    - `Invocation { invocation }` for a precondition or postcondition: the
      `DocumentRef` (FR-106) of a `quire.state.invocation/v1` document. It
      names the self object, the parameters and result, and a `pre` and a
      `post` snapshot, each by identity and `sha256-jcs` digest;
    - `Current { snapshot, anchor, self }` for an invariant: the
      `DocumentRef` of a `quire.state.snapshot/v1` document whose
      `observation` is `current`, the initialization or handler anchor
      (`{kind, name}`) it was taken at, and the self object
      (`{population, key}`).
- The envelope's members for a state-clause packet: its `clause_node` is the
  clause's `state`/`state_clause` node, its `occurrence_key` is the clause
  declaration's `claim` occurrence, and its `obligation_identity` is the
  O-09 obligation identity of that clause's `operation-contract` record
  (FR-104 "Requirements"). The executor reads no `selected_function`.
- FR-098's package reference, byte provision and limits. Every document the
  observation names (the invocation and both its snapshots, or the current
  snapshot) is a `sha256-jcs` entry of the byte provision. Each snapshot
  holds every object of each population the clause requires, with every
  declared field, and marks that population `complete` (FR-106 checks 6
  and 7).

## Outputs

- An FR-072 replay result on the envelope's arm, or a typed `ReplayRefusal`
  with no partial result. A result retains the source identity and digest,
  the `package_id`, the payload's `clause`, `clause_node` and `occurrence`,
  the identity and digest of every document admission read, the evaluated
  value when there is one, the evaluation charges and the executor's
  toolchain pin.

## Behavior

- If the envelope's `clause_node` differs from the payload's `clause_node`,
  or its `occurrence_key` from the payload's `occurrence`, then the
  executor SHALL refuse `stale_dependency`/`revision-mismatch` before it
  recompiles, naming the envelope's and the payload's identity (ADR-017
  PF-4).
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
  from the payload's `clause_node`, or its `claim` occurrence key differs
  from the payload's `occurrence`, then the executor SHALL refuse
  `stale_dependency`/`revision-mismatch` before any admission, naming the
  payload's and the recompiled identity. No identity is recovered from a
  display name.
- The executor SHALL admit the observation by FR-106 with the selection
  (`clause`, `observation`), reading every document only from the byte
  provision by its `sha256-jcs` digest. For an `Invocation` observation,
  admission runs FR-106's frame and delta check (check 11) over the pre and
  post snapshots.
- If admission fails, refused or incomplete, then the executor SHALL refuse
  with a `ReplayRefusal` holding FR-106's record, and SHALL evaluate
  nothing.
- The executor SHALL evaluate the admitted clause once by FR-107 and compare
  verdicts as FR-072 does. The counterexample refuted the clause, so the
  proved verdict is `violation`.
  - If the evaluation completes `false`, then the executor SHALL settle the
    agreement of the envelope's arm: `reproduced-with-evaluated-witness` on
    the `Witness` arm, `reproduced-without-witness` on the `Input` arm.
  - If the evaluation completes `true`, then the executor SHALL settle
    `inconclusive` with cause `Verdicts` (`violation` proved, `success`
    replayed), never repaired (FR-072).
  - If the evaluation completes no value (a `refused`, `incomplete` or
    `undefined` S6a outcome, or a family result), then the executor SHALL
    settle `inconclusive` with cause `NoValue` (FR-072).
  - If the evaluation ends in an outcome FR-100 handles as an internal
    failure (the kernel `Refusal::CheckedInvariant`, or
    `CallFailure::Fault`), then the executor SHALL refuse with the
    `InternalFault` and settle no result.
- Replay SHALL read no path, environment variable, clock or search location,
  and SHALL give the same result for the same request and envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-122-AC-1 | Over FR-108's ConfigVersion unit, an envelope for `VersionUnchanged` whose observation is the changed-version invocation settles `reproduced-with-evaluated-witness`, and the same payload on an `Input`-arm envelope settles `reproduced-without-witness`. An envelope for `ParentOrder` whose observation is violating-parent's current snapshot (anchor `handler validate`, self `child`) settles `reproduced-with-evaluated-witness`. Each result holds the source digest, the `package_id`, the payload's `clause`, `clause_node` and `occurrence`, and the identity and digest of every document admission read: the invocation and both snapshots, or the one current snapshot. | Test (TC-517) |
| FR-122-AC-2 | The `VersionUnchanged` envelope over the unchanged-version invocation, and the `ParentOrder` envelope over healthy-parent's snapshot, each settle `inconclusive`, `Verdicts`, holding `violation` and `success`. The `ParentOrder` violating-parent envelope replayed with the request's `quire.value.accounting/v1` evaluation budget at zero settles `inconclusive`, `NoValue`. | Test (TC-517) |
| FR-122-AC-3 | Stale identity: a `VersionUnchanged` envelope whose envelope and payload `clause_node` both name `ParentOrder`'s node refuses `stale_dependency`/`revision-mismatch` naming both node identities; one whose envelope and payload occurrence are both `VersionUnchanged`'s node at ordinal 1 refuses the same way naming both occurrences; neither admits a document. A source edit that changes the `package_id` refuses by FR-098's stale `package_id` rule. A `clause` naming `Absent`, and one naming the function `sameIdentity`, each refuse `missing_declaration`/`missing-name`. | Test (TC-517) |
| FR-122-AC-4 | Envelope consistency: the changed-version envelope with only its envelope `clause_node` replaced by `ParentOrder`'s node refuses `stale_dependency`/`revision-mismatch` naming the envelope's and the payload's clause node; with only its envelope `occurrence_key` at ordinal 1 it refuses the same way naming both occurrences. Each refuses before the recompile: over a request whose source does not compile, both refuse this way, and the consistent envelope over the same request refuses at the recompile. | Test (TC-517) |
| FR-122-AC-5 | Admission refusals settle no result: the `VersionUnchanged` envelope over the forbidden-parent-change invocation refuses with FR-106's `frame_violation`/`unauthorized-change` record naming `child` and `parent`; the changed-version envelope with its pre snapshot absent from the byte provision refuses with FR-106's `unavailable_observation` record; with its invocation bytes edited under the same digest it refuses `stale_dependency`/`byte-digest-mismatch`; a `VersionUnchanged` envelope whose observation is `Current` over healthy-parent's snapshot refuses `wrong_snapshot`/`wrong-observation`. | Test (TC-517) |
| FR-122-AC-6 | Replaying one envelope twice gives equal results. `StateClauseCounterexample` implements `FamilyPayload`, the envelope carries it as its generic parameter with no string-keyed field, and its `observation` is a sum of the `Invocation` and `Current` inputs, so a payload holds exactly one. | Test (TC-517) |

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
  CG builds the envelope); ADR-017 PF-4 (envelope and payload carry one
  identity and agree).
- The analogue for a frame counterexample is
  [FR-116](FR-116-replay-a-frame-counterexample.md).
