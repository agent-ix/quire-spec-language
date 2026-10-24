---
id: FR-103
title: "Admit a domain package's operations and their frames on the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-103: Admit a domain package's operations and their frames on the spine

## Description

When a `1-draft` unit's `model M` selects a domain package whose object types
or processes declare operations, spine intake (ADR-011 I1) SHALL read each
operation's `frame` into the operation's `OperationEffect`, and the E3
assembler SHALL declare each operation on its object type, with its
parameters, its result and its effect, so that a `pre` or `post` clause can
name it as `M::T::op` (FR-104).

This amends FR-056's Complete-V1 paragraph, which this PR edits to match.
Today intake refuses any operation whose frame declares an entry
(`qsl-semantics/src/model/intake.rs:1263-1289`,
`unsupported_at(.., "operation.frame")`), and the assembler refuses every
`OperationMember` record as `AssemblyCause::UnsupportedModelMember` (the match
arm at `qsl-semantics/src/check/assemble.rs:405-409`), so no unit can use a
model that has an operation. This is the `StateModel` share of ADR-012 §15.3.

Beyond frames, this FR keeps and checks each operation's own `requires` and
`ensures` clauses. It amends this FR's earlier rule that domain-package
operation `pre`/`post` text is shape-checked and not kept (QSL-726).

## Inputs

- One admitted domain package (FR-056, QSpec FR-154), Semantic IR 2.0.0.
- `ModelNormalizationLimitsV1`.

## Outputs

- For each operation member: an `OperationMemberRecord` whose `effect` holds
  the `DeclarationKey`s its frame names, in each member's declared order, and
  whose `requires` and `ensures` hold the operation's kept contract clauses
  (see "A domain package operation's own requires and ensures"). In
  the compiled unit's selected model view, the assembler SHALL set its
  `has_body` value true when a retained unit `OperationBodyForm` (FR-102)
  resolves to that operation's `DeclarationKey`, and false when no such
  declaration exists.
- In the assembled `PackageDeclarations`: each object type or process `M::T`
  with its operations, each operation with its name, its parameters (name and
  value type, in declared order), its result value type or none, and its
  effect.
- Each population record of the package, with its member types and its
  declared extent, available to the checker and to observation admission
  (FR-106) by its declaration key.

## Behavior

- Intake SHALL resolve each `modifies` entry to a field declaration or a
  navigation relationship of the package, and each `creates` and `deletes` entry to an object type or process
  declaration, under QSpec `model-complete.md`'s Frames row, checking the
  entries of `modifies`, then `creates`, then `deletes`, each in document
  order, and reporting the first that fails.
- A `modifies` entry has a scope and a `deletes` entry has a scope (QSpec
  FR-013). FR-154 states how intake reads them, which entries a scope admits,
  and the refusals a scoped entry gives; the scoped members are
  checked in the order FR-154 states, in document order within each member, and a conflict between
  two scopes of one field is reported only after every entry resolves.
- If a frame entry names no declaration of the package, then intake SHALL
  refuse `missing_declaration`/`missing-name` at the entry, retaining the
  operation's IR node identity, the entry, the artifact id and the span.
- Intake SHALL read a `modifies` entry that names a navigation relationship as
  a grant over that relationship's instances: an observation carries them
  (FR-364), and the frame check compares them (FR-106 check 11.5). A
  `creates` or `deletes` entry that names a relationship is the
  `malformed-declaration` refusal below.
- If a frame entry names a declaration of any other meaning, then intake SHALL
  refuse `invalid_model_binding`/`malformed-declaration` at the entry, with
  the same retained data.
- Intake SHALL read an absent frame, or one whose three members are all
  empty, as the empty effect, as it does today.
- The assembler SHALL type each parameter and the result by FR-056's field
  rule: a native `Boolean` or `Integer`, a scalar type as its `Int[lower,
  upper]`, an object type as `Reference<M::T>`, or a record value type as
  `M::R`, with presence and multiplicity applied the same way. A parameter or result type the type
  environment cannot hold SHALL refuse
  `unknown_required_feature`/`unsupported-feature` at the `model`
  declaration, as a field of that type does.
- A `pre` or `post` clause and a protocol attempt name an operation of a
  process as they name one of an object type (FR-104, FR-114): an operation
  declared by a process is admitted, with its frame, into the process's
  declaration, and the frame a clause or attempt takes of it is this
  `OperationEffect`, anchored at the process (FR-105).
- The assembler SHALL declare an operation on the object type or process that
  owns it. The operation is visible on a subtype through FR-081's effective
  view, as a field is, and keeps its declaring type as its owner (FR-105
  anchors it there).
- The S3 assembler SHALL preserve each resolved unit `operation-body`
  expression and its source provenance, and each unit `pre` and `post`
  `StateClauseForm` as a contract clause. It SHALL associate the body and
  clauses with the resolved operation's `DeclarationKey` and keep them
  available to checking and dispatch linking. It SHALL bind the body's
  receiver-less parameter names positionally to the operation's parameters in
  signature order, as required by the QSpec Complete-V1 grammar, and type its
  body against the operation's declared result. The parameter count and
  types and the body type are checked when dispatch linking first reads a
  body (FR-302), before any consumer uses it, and a mismatch refuses
  `ill_typed`/`type-mismatch` at the body; resolving the body to its
  `DeclarationKey` and setting `has_body` do not depend on that check. At most one unit
  `operation-body` SHALL resolve to a given `DeclarationKey`; FR-302 defines
  the duplicate refusal. This multiplicity rule applies to executable bodies,
  not to the distinct unit `pre`/`post` `StateClauseForm`s.
- The assembler SHALL declare each admitted record value type beside object
  types and operations, with its effective fields, invariant clauses and
  supertypes, and SHALL allow object and record fields to use that type by
  value. It SHALL carry a record-typed field through checking and checked
  package emission as QSpec FR-322's self-typed `model`/
  `record_value_type` node, including a record field named by a frame's
  `modifies` entry under QSpec FR-340. The record node SHALL keep its
  `ModelOwner`; it is never an object-reference target.
- The assembler SHALL still refuse systems parts, ports and allocations as
  `UnsupportedModelMember`.
### A domain package operation's own requires and ensures

Target design (QSpec TC-235 M06 run 7 and M15 run 6 are the reference rows;
[QSpec TC-235](ix://agent-ix/quire-specification/TC-235) owns them). An
operation declared in a domain package carries its own contract, authored as
the operation's `pre` (its `requires`) and `post` (its `ensures`) clause
lists. That contract is part of the model. Intake keeps it and the checker
evaluates it; it is not discarded and it is not shape-checked only.

- Intake SHALL keep each `pre` entry as a `requires` clause and each `post`
  entry as an `ensures` clause of the operation's `OperationMemberRecord`,
  in each list's declared order. A kept clause holds its authored `language`
  and exact authored `text`, its kind (`requires` or `ensures`), its position
  in the operation's list, and its source identity: the operation's
  `DeclarationKey`, the artifact id and the span of the clause's `origin`.
  Intake SHALL NOT parse the text. A package whose operation declares no `pre`
  or `post` entry keeps an empty list for it, as it does today.
- The spine `compile` driver SHALL parse each kept `quire`-language clause
  text at S1/S2 with the FR-146 expression parser that the unit `pre`/`post`
  clause path uses (FR-102), and SHALL hand the parsed `Expression`s to the S3
  assembler with each clause's kept source identity. The assembler consumes
  parsed expressions and does not parse text, so the semantics layer takes no
  dependency on the S1 parser (ADR-011 §6.1, TC-390). The `ProtocolClause`
  family SHALL check each as a Boolean under the clause kind FR-104 gives that
  position: a `requires` clause as a precondition and an `ensures` clause as a
  postcondition. The operation's receiver (`self`), its parameters by name
  and, for `ensures`, `result` bind as FR-104's state contract states. A
  clause in a language other than `quire` is kept verbatim and is not
  parsed or checked.
- A kept clause names no `using` profile, so none is selected for it. A
  domain-package invariant written `language: quire` (TC-235 M13 and M14) has
  no profile concept either; FR-366 classifies, checks and evaluates it.
  A kept operation clause is therefore checked under the typer's
  profile-independent defaults, with the unit's profile selections playing no
  part. In particular `complete_value` is off, so integer `div`, `rem`, `mod`
  and Integer `/` in a kept clause refuse `profile_prohibited_operator`.
  Checking a kept clause yields a refusal or nothing; it adds no checked state
  clause, claim or requirement record.
- A `requires` clause is a precondition: it reads only the state before the
  invocation. If its text uses `pre(e)`, then the checker SHALL refuse
  `wrong_snapshot`/`forbidden-pre-read`, the cause FR-104 gives a unit `pre`
  clause that reads `pre(e)`. The refusal SHALL name the operation's
  `DeclarationKey`, and SHALL be located at `Operation.pre[k]`, k being the
  clause's position in that list. A kept clause's location is the typed
  origin `ModelOperationClause` (operation, kind, position), which is not a
  unit state clause's origin. The clause's artifact id and span are the kept
  clause's own, found from that operation, kind and position. No checked
  package is produced.
- An `ensures` clause is the operation's postcondition and may use `pre(e)`.
  It is retained on the operation as its postcondition clause, bound to that
  operation's `DeclarationKey`, and it adds no declaration. A checking
  refusal of an `ensures` clause is located at `Operation.post[k]` in the same
  form.
- A text that fails FR-146 parsing or Boolean checking refuses with the
  refusal that parse or check gives, located at `Operation.pre[k]` for a
  `requires` clause and `Operation.post[k]` for an `ensures` clause.
- These kept clauses are distinct from the two unit-level forms. A unit `pre`
  or `post` `StateClauseForm` (FR-102) is a clause the unit authors about an
  operation, and a unit `operation-body` is the candidate body the unit
  authors. The domain package's `requires` and `ensures` are the operation's
  own contract and are never unit clauses and never an operation body. They
  do not supply, replace or stand in for a unit `StateClauseForm` or
  `OperationBodyForm`, and they do not set `has_body` or select a dispatch
  candidate. The unit `pre`/`post` `StateClauseForm`s supply additional
  operation contract clauses. S2 retains their expression trees and source
  provenance under FR-102, and the S3 assembler associates them with the
  resolved operation identity. An operation with no matching unit body has
  `has_body = false` and is not a dispatch candidate.
- The kept `ensures` text is separate from `OperationMemberRecord`'s
  `own_postcondition_clauses`, which are FR-151's refinement guard facts
  (`present(self.f)` or one ordering against an integer literal).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-103-AC-1 | The ConfigVersion domain package (TC-458 fixture: object type `ConfigVersion` with `versionNumber: VersionNumber` bound to `Int[0, 1000]` and optional `parent: ConfigVersion`, population `config_history` over `ConfigVersion` with extent `closed`, and operation `attemptUpdate` with no parameters, result `Boolean` and frame `modifies [ConfigVersion/versionNumber]`) admits. The assembled `Config::ConfigVersion` has fields `versionNumber: Int[0, 1000]` and `parent: Option<Reference<Config::ConfigVersion>>`, and operation `attemptUpdate` with no parameters, result `Boolean` and an effect whose `modifies` is exactly the `versionNumber` field key. FR-056's `value-type/v1` scalar reader is verified end to end through `admit_unit`/`PackageDeclarations::assemble` by a dedicated bound-scalar fixture (`qsl-semantics/tests/it/model_operations.rs::bound_integer_value_type_admits_and_assembles`), and the shared `ConfigVersion` fixture this row's own tests use declares `versionNumber` as that same bound `VersionNumber` scalar. | Test |
| FR-103-AC-2 | The same package with `modifies [ConfigVersion/missing]` refuses `missing_declaration`/`missing-name` at that entry; with `modifies [ConfigVersion]` (an object type) or `creates [ConfigVersion/versionNumber]` (a field) it refuses `invalid_model_binding`/`malformed-declaration` at that entry; with a relationship `Supersedes` declared, `modifies [Supersedes]` admits and the operation's effect modifies `Supersedes`, while `creates [Supersedes]` refuses `invalid_model_binding`/`malformed-declaration` at that entry; with both `modifies [ConfigVersion/missing]` and `creates [ConfigVersion/versionNumber]` it reports only the `modifies` refusal. No declaration is admitted in a refusing case. | Test |
| FR-103-AC-3 | An operation with a parameter `note: Decimal` refuses `unsupported_construct`/`declaration-form` at the parameter, at I1 (intake) -- the same stage and cause a field of that type refuses at, per this FR's own Behavior ("as a field of that type does"); an operation with parameter `delta: VersionNumber` admits it typed `Int[0, 1000]`. FR-056's `value-type/v1` scalar reader is verified against both halves over the shared `ConfigVersion` fixture. The parameter type is `Decimal`, not this row's earlier `Text`: the FCD schema validator's native-scalar vocabulary has no `Text` entry (it spells the concept `String`), so `Text` cannot reach even I1 through the real pipeline, and QSL's own `NativeValueType` has no `String` variant either -- that cross-repo vocabulary gap is QSL-290, not fixed here. | Test |
| FR-103-AC-4 | A package holding `Address { street: Text[0, 64; nfc] }` and object `Account { address: Address }` assembles both declarations beside an object operation. A clause that reads `deref(a).address.street` checks against the record's effective fields, and checked-package emission types `Account.address` by the self-typed `model`/`record_value_type` node with its `ModelOwner`. No `UnsupportedModelMember` names `Address`, and no `Reference<Address>` is emitted. | Test |
| FR-103-AC-5 | Admission is deterministic: admitting the ConfigVersion package twice, and admitting it with its operation and population records reordered in the document, gives equal `PackageDeclarations` and equal effect key lists. | Test |
| FR-103-AC-6 | A frame modifying `Account.address` and a frame entry naming `Address.street` bind distinct declaration-and-field pairs; a record subtype's inherited field resolves through its effective fields. An operation parameter and result typed `Address` retain record value typing through check and emission; a record node supplied as an object-reference target refuses the QSpec FR-322 derived-shape check. | Test |
| FR-103-AC-7 | K2's `OrderLifecycle` with operation `place` given `ensures` `self.placed` admits after the same 21 charges as K2 and adds no declaration. The `OperationMemberRecord` of `place` holds one `ensures` clause with language `quire`, text `self.placed`, position 0, and the source identity of its `origin` (operation key, artifact id, span), and no `requires` clause; `self.placed` is bound as the postcondition of `place`. A domain package whose operation declares no `pre` and `post` keeps empty lists. | Test (TC-993) |
| FR-103-AC-8 | K2 with `OrderLifecycle` given field `cancelled`, and `place` given `requires` `pre(self.placed) = false`, refuses `wrong_snapshot`/`forbidden-pre-read` naming `ix://test/orders/OrderLifecycle/place` at `Operation.pre[0]`, with no checked package. A kept clause is checked under no profile: integer `div` in it refuses `profile_prohibited_operator`. The same package with `requires` `not self.placed` and `ensures` `self.placed and not self.cancelled` checks, and an `ensures` that reads `pre(self.placed)` checks. | Test (TC-993) |
| FR-103-AC-9 | A domain operation's `requires` and `ensures` texts do not become unit clauses or bodies: with domain texts `false` on `A.size` and `B.size` and unit `pre`/`post` clauses `true`, the retained unit clauses are `true`, and `has_body` is set only by the unit `operation-body` declarations. | Test (TC-986) |
| FR-103-AC-10 | Intake reads an operation's inline `redefines` property as the key of the operation it redefines: in a package where `Sub.size` declares `redefines` `ConfigVersion/size`, the admitted record of `Sub.size` names `ConfigVersion/size`, and `ConfigVersion.size`, which declares none, redefines nothing. | Test |

## Dependencies

- FR-056 (intake and assembler), FR-081 (effective view), FR-084
  (population records).
- QSpec FR-154, FR-340 and `model-complete.md`'s Frames row.
- filament-core-data's Semantic IR reader already validates an operation's
  `frame` shape
  (`crates/semantic-ir/src/schema.rs:1656-1663`).
