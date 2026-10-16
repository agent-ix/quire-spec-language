---
id: FR-104
title: "Check state clauses at S3 and record their requirements"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-010
    type: references
---
# FR-104: Check state clauses at S3 and record their requirements

## Description

When S2 admits a `StateClauseForm` (FR-102), the FR-091 assembler SHALL
resolve its profile alias, its context type, for a `pre` or `post` clause its
operation, and its population domain(s), refusing an unresolved or ambiguous
name at its span (see "Resolution"). The `ProtocolClause` family `check` then
SHALL type the body as a Boolean under its clause kind, with `self`, `result`,
the operation's parameters and `pre(e)` bound as QSpec's state contract states
and every model read qualified by its observation; and SHALL produce one
checked state clause with its identity, or a refusal with a catalog code. Its
`requirements` hook SHALL return one `operation-contract` record per clause
and per frame (ADR-012 §2, §15.7; FR-057).

## Inputs

- A `StateClauseForm`; the assembled `PackageDeclarations` with the
  operations FR-103 admits; `CheckContext` (ADR-012 §2).

## Outputs

- A checked state clause: its kind, its context object type, its operation
  (for `pre` and `post`), its checked Boolean body with each model read's
  observation, its node identity and its `claim` occurrence (ADR-013 O-07,
  O-09), recorded in the `CheckedGraph` under its declared name.
- One `Requirements` record per clause, and one per frame of an operation
  that at least one `pre` or `post` clause, or protocol attempt (FR-114),
  of the unit names.
- Or a refusal with a family cause and catalog code, or
  `StageFailure::Limit`.

## Behavior

### Resolution

The S2 assembler performs every resolution this section describes, exactly
as it does a function's alias and declarations (FR-091): the `ProtocolClause`
family `check` reads the resolved context, operation and population domain(s)
off the assembled declaration rather than re-resolving them. A `reaches`
target's population is the one exception: its type is unknown before the
body is typed, so S3 resolves and refuses it (see "Requirements").

- The assembler SHALL resolve the `using` alias to one of the unit's profile
  selections, exactly as it does for a function (FR-091).
- If `M` names no `model` declaration of the unit, or `T` no object type,
  state machine or process of its package, then the assembler SHALL refuse
  `missing_declaration`/`missing-name` at the name.
- The context `M::T` of a clause is the declaration `T` names, whichever of
  the three it is, and a process is a context as an object type is. The
  `self` of a clause on a process is one run of it (QSpec FR-208): a
  `Reference<M::T>` to the run, whose declared object identity roots the
  clause's receiver path (FR-107), so an invariant on a process holds for
  every run of its population at `current` and a `pre` or `post` clause of a
  process operation reads the run the operation is called on, as one on an
  object type reads its object. An operation `op` a process declares
  (FR-103) is resolved, anchored and framed as an object type's operation is
  (FR-105): its anchor and frame are the ones every clause and attempt that
  names it share, the anchor's `context` is the node of the process that
  declares `op` (`model`/`process`), and the frame is the operation's
  `OperationEffect` as FR-103 and FR-154 admit it. No scoped frame is built
  for a process operation: intake refuses its receiver-scoped members
  (`modifies_receiver`, `deletes_receiver: true`) at the member
  (FR-154-AC-8), so the frame a clause names holds only `every-object` `modifies`
  entries, `creates`, `deletes` and `deletes_parameters` entries, and no
  clause can name a process operation whose frame holds a receiver-scoped
  entry. The anchor context of an operation of a state machine is the node of
  the state machine in the same way, with `self` the machine's context
  (FR-107).
- If a `pre` or `post` clause's `op` names no operation of `M::T`'s effective
  view (FR-103), then the assembler SHALL refuse `missing_declaration`/
  `missing-name` at `op`.
- An operation `op` resolves through `M::T`'s ancestry (FR-103, FR-081's
  effective view, "visible on a subtype through FR-081's effective view, as a
  field is"): a declaration a more derived type of the view makes hides one
  an ancestor makes, and the declaring type it keeps may differ from `T`. If
  `op` names operations of two or more ancestors and neither declaring type
  is more derived than the other, then the assembler SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at `op`, naming those declaring
  types.
- If a state clause's name equals the name of another state clause or of a
  function of the unit, then the checker SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at every declaration of that name,
  as it refuses two functions of one name today
  (`qsl-semantics/src/check/mod.rs:557-572`). Clauses and functions share one
  selection namespace (FR-109).

### Typing

- The checker SHALL check the body against `Boolean`. If the body has another
  type, then the checker SHALL refuse `ill_typed`/`non-boolean-root` at the
  body.
- The checker SHALL type `self` as `Reference<M::T>`, the context object, and
  a field read `self.f` as the `Attribute` node `deref(self).f` produces,
  typed by `f`'s declared type in the effective view. If `self` appears
  outside a state clause, then the checker SHALL refuse
  `missing_declaration`/`missing-name`.
- The checker SHALL admit `result` only in a `post` clause of an operation
  that declares a result, typed as that result. If `result` appears anywhere
  else, then the checker SHALL refuse `wrong_snapshot`/`wrong-anchor` at
  `result`, naming the clause kind and the operation.
- The checker SHALL bring each parameter of the operation into scope by name
  in a `pre` or `post` body, typed as FR-103 declares it, and no parameter
  into an invariant's scope.
- Equality of two `Reference<M::T>` values SHALL check as identity equality,
  as it does for function parameters today.

### Observations of reads

- The checker SHALL give every model read (a field read, `deref`, `reaches`,
  `allInstances`, `lookup`) one observation: `current` in an invariant, `pre`
  in a precondition and `post` in a postcondition, except inside `pre(e)`,
  where reads of `self`, and reads through references obtained inside `e`,
  are `pre`.
- A reference value keeps the observation it was read in, and a read through
  it uses that observation (QSpec: "Dereference follows the reference's own
  universe/type/observation"). A `let` binder keeps its initializer's
  observation.
- A read through a reference-typed operation parameter SHALL take the
  clause's own observation, as a read through `self` does: `pre` in a
  precondition and `post` in a postcondition, and `pre` inside `pre(e)`. So in
  a postcondition `deref(target).versionNumber` reads `post` and
  `pre(deref(target).versionNumber)` reads `pre`; in a precondition
  `deref(target).versionNumber` reads `pre`. The clause's observation is the
  qualification the binding supplies (QSpec `state-contract.md`, "The
  model/binding must supply any reference qualification before evaluation").
- The checker SHALL admit `pre(e)` only in a `post` clause (the existing
  `ForbiddenPreRead` rule) and only when `e` itself contains an eligible
  state read: `self`, a field read (through `self` or a parameter
  reference), `allInstances`, `lookup` or `pre`.
- If `pre(e)`'s operand reads only parameters, literals, `result` or `let`
  binders bound outside the `pre`, or reads a field only through a `let`
  binder bound outside the `pre`, then the checker SHALL refuse
  `wrong_snapshot`/`forbidden-pre-read` at the `pre`. This covers QSpec's
  `pre(delta)`, `pre(result)`, `let v = self.version in pre(v)` and
  `let s = self in pre(s.version)` rows, and `pre(target)` over a
  reference-typed parameter `target` in a postcondition: QSpec
  `state-contract.md` states "A pre selector cannot retag a post-qualified
  parameter reference" and "a bare or parameter-only historical selection
  refuses". `let s = pre(self) in s.version` is admitted and reads `pre`.
- If `reaches` appears outside a state clause, then the checker SHALL refuse
  `ill_typed`/`operator-ineligible` at the `reaches`: only a clause has an
  observation to traverse (ADR-012 §15.2).
- The checker SHALL admit `reaches(a, b, edge)` only when `edge` names a field
  of an object type `M::T` whose type is `Reference<M::T>`,
  `Option<Reference<M::T>>` or a sequence of `Reference<M::T>`. For a package
  selecting `quire.model.complete/v1`, the checker SHALL apply QSpec
  [FR-043](ix://agent-ix/quire-specification/FR-043)'s conformance to the
  static edge owner `T` to each operand and retain the checked operand
  references without erasing the type identity that FR-043 uses for each
  object's most-specific identity at evaluation; QSpec
  [FR-010](ix://agent-ix/quire-specification/FR-010) delegates this traversal
  profile to FR-043. For any other selection, `a` and `b` SHALL both be
  `Reference<M::T>` of that exact object type, as before. An ineligible
  operand or edge shape, including a set, bag or ordered set of references,
  SHALL refuse `ill_typed`/`operator-ineligible` at the `reaches`. When `edge` names a
  relationship end instead of a field, [FR-365](FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md)
  checks, lowers and evaluates the form.

### Definedness facts

- The checker SHALL run the static definedness check (`CheckMode::Linked`)
  over the body, with presence and interval facts extended to model read
  paths: a stable path is a root (`self`, a parameter or a `let` binder) and
  its field steps, including the steps `deref(value(p)).f` takes, and each
  fact is keyed by (observation, path).
- A fact that `present(p)` or an ordering establishes at one observation
  SHALL discharge only an obligation at the same (observation, path). So
  `present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber`
  checks, and a fact about `pre(self.parent)` never discharges
  `value(self.parent)`.
- If a `value(p)` has no presence fact at its own (observation, path), then
  the checker SHALL refuse `undefined_expression`/`unproved-presence` at the
  `value`. QSpec's `present(pre(self.parent)) implies value(self.parent)`
  refuses this way.

### Requirements

- The `requirements` hook SHALL return one `Requirements` with capability kind
  `operation-contract` for each clause, keyed by its `claim` occurrence
  (ADR-012 §13.5).
- The hook SHALL return one `operation-contract` record for each frame of an
  operation that a `pre` or `post` clause or a protocol attempt (FR-114) of
  the unit names, keyed by one
  `generated` occurrence of the frame node (FR-105): the checker SHALL mint
  one occurrence per distinct operation the unit's clauses and attempts
  name, in
  ascending (declaring type `DeclarationKey`, operation name as UTF-8 bytes)
  order over those operations, before lowering any clause; every clause
  naming the same operation shares its occurrence. Two operations are two
  occurrences even when their frame nodes coincide (equal
  `modifies`/`creates`/`deletes` and declaring type, FR-105): the occurrence
  is per operation identity, not per frame-node identity, so two operations
  never merge into one record and neither raises a fault. The ordinal SHALL
  NOT depend on source order: reversing the unit's clauses SHALL NOT change
  which operation a given key names. An operation no clause and no attempt
  names gets no record.
- Each record's extent SHALL follow ADR-014 §4 as FR-097 classifies it, over
  the clause's `self`, `result` and parameter types and the populations the
  clause ranges over: the context's population (every object of which the
  clause holds for) and each population a `reaches` walks. A domain-package
  population declaration never has a maximum (QSpec FR-153's own record has
  none), so each one is always an unbounded `Population(None)` domain,
  boundable by `Cardinality`.
- The checker SHALL key a population domain as
  `DomainKey::Population{member_type, ordinal}` with `member_type` the
  `model`/`object_type` or `model`/`process` node of the population's *canonical*
  member type: the least, in ascending `DeclarationKey` order, of its
  declared member types -- never a node of the clause's context type `T`
  itself when `T` is a proper subtype of a member (SR-736 FND-010), and
  never the particular member that happens to cover `T` when the population
  declares more than one member type (SR-736 FND-011): a population has
  exactly one `DomainKey`, whatever member type a clause's context conforms
  to. A clause over `T` and one over a subtype of `T`, or over any other
  member type the same population declares, all key that population's
  domain identically. `ordinal` names the population: its ordinal among the package's population declarations in ascending
  `DeclarationKey` order (its own `Ord`: `package`, then `node`, each as
  UTF-8 bytes, `qsl-semantics/src/model/key.rs:80-84`). The ordinal is stable
  within one domain-package digest only, and domain keys SHALL NOT be
  compared across digests. A population record has no node of its own
  (FR-094), so its domain is named by the object type plus the population, as
  a model member is (ADR-012 §15.4). A `reaches` over the context population
  adds no second domain.
- The context population SHALL be the one population with no maximum that
  covers `T`: `T` itself, or a proper supertype of `T`, is one of its member
  types, by conformance (FR-084's `allInstances<T>` reads this way, not by
  exact identity). If no population covers `T`, then the clause's extent
  names no population domain for it. If `T` conforms to a member type of two
  or more populations with no maximum, then the extent cannot name exactly
  one population, and the checker SHALL refuse `ambiguous_declaration`/
  `ambiguous-name` at the clause's `on`, naming those populations in
  ascending `DeclarationKey` order.
- When `T` is a member type of several populations, the population a run's
  context object belongs to SHALL be the one the selection's `self` names
  (FR-106: `Current`'s `self`, or the invocation's `self`).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-104-AC-1 | Over the ConfigVersion package (FR-103-AC-1), `ParentOrder` (`present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber`), `NoCycle` (`not reaches(self, self, parent)`) and `post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }` each check, with kind `Invariant`, `Invariant` and `Postcondition`; `self` is `Reference<Config::ConfigVersion>`, `self.versionNumber` is `Int[0, 1000]` and `self.parent` is `Option<Reference<Config::ConfigVersion>>`. `ParentOrder`'s reads are all `current`; in `VersionUnchanged` the left read is `post` and the right `pre`. FR-056's `value-type/v1` scalar reader is verified against this exact bound over the shared `ConfigVersion` fixture, which declares `self.versionNumber` as the bound `VersionNumber` scalar. | Test (TC-459) |
| FR-104-AC-2 | `post R using v on Config::ConfigVersion::attemptUpdate { result }` checks with `result: Boolean`. `result` in an invariant, and in a `pre` clause of `attemptUpdate`, each refuse `wrong_snapshot`/`wrong-anchor` at `result`. | Test (TC-459) |
| FR-104-AC-3 | `on Config::Missing` and `on Config::ConfigVersion::missing` refuse `missing_declaration`/`missing-name` at the missing name; an invariant body `self.versionNumber` refuses `ill_typed`/`non-boolean-root`; a second clause named `ParentOrder`, and a function named `ParentOrder` beside the clause, each refuse `ambiguous_declaration`/`ambiguous-name` at both declarations. | Test (TC-460) |
| FR-104-AC-4 | `pre(self.versionNumber)` in an invariant refuses `wrong_snapshot`/`forbidden-pre-read`; `pre(result)` in a postcondition refuses `wrong_snapshot`/`forbidden-pre-read`; `reaches(self, self, versionNumber)` refuses `ill_typed`/`operator-ineligible`; `reaches(x, y, parent)` in a function body refuses `ill_typed`/`operator-ineligible`; an unguarded `deref(value(self.parent)).versionNumber < 5` refuses `undefined_expression`/`unproved-presence` at the `value`. | Test (TC-460) |
| FR-104-AC-5 | The ConfigVersion unit with `ParentOrder`, `NoCycle` and `VersionUnchanged` yields exactly four `operation-contract` records: one per clause, keyed by its `claim` occurrence, and one for `attemptUpdate`'s frame, keyed by the frame's own occurrence. Each has extent `Unbounded` with one domain, the `config_history` population, of kind population and boundable by `Cardinality`, keyed by the `ConfigVersion` object type node and `config_history`'s declaration ordinal. The same unit without `VersionUnchanged` yields exactly two, with no frame record. Over the package with a second, unbounded population `archive` of `ConfigVersion`, each clause refuses `ambiguous_declaration`/`ambiguous-name` at its `on`, naming `archive` and `config_history`. Two operations of equal frame content (both an empty `modifies`/`creates`/`deletes`) each still keep their own frame record, whether their own records differ or coincide. | Test (TC-461) |
| FR-104-AC-6 | Checking the same unit twice, and checking it with its clauses in another order, gives each clause the same node identity and the same requirement record keys. Two clauses with equal kind, anchor and body and different names share a node identity and differ in their `claim` occurrence (ordinals in source order). | Test (TC-461) |
| FR-104-AC-7 | In postconditions of `attemptUpdate` over a package whose frame modifies `parent`: `present(pre(self.parent)) implies deref(value(self.parent)).versionNumber > 0` refuses `undefined_expression`/`unproved-presence` at `value(self.parent)`; `pre(present(self.parent) implies deref(value(self.parent)).versionNumber > 0)` checks; `let v = self.versionNumber in pre(v) = 1` and `let s = self in pre(s.versionNumber) = 1` each refuse `wrong_snapshot`/`forbidden-pre-read` at the `pre`; `let s = pre(self) in s.versionNumber = 1` checks with its read `pre`. | Test (TC-459) |
| FR-104-AC-8 | Over a package that adds operation `probe(target: ConfigVersion)` on `ConfigVersion` with no result and an empty frame: the postcondition `deref(target).versionNumber = 1` checks with its read `post`; the precondition `deref(target).versionNumber = 1` checks with its read `pre`; the postcondition `pre(deref(target).versionNumber) = 1` checks with its read `pre`; the postcondition `pre(target) = target` refuses `wrong_snapshot`/`forbidden-pre-read` at the `pre`. | Test (TC-459) |
| FR-104-AC-9 | Under `quire.model.complete/v1`, `reaches(self, target, parent)` on a `ConfigVersion` subtype with its inherited `parent: Option<Reference<ConfigVersion>>` field checks with `self` retained as the subtype reference and `target` as the base reference, without erasing the operand identity FR-043 evaluates; a target type not conforming to the edge owner refuses `ill_typed`/`operator-ineligible`. The subtype/base operands under any other admitted model selection refuse `ill_typed`/`operator-ineligible` by the exact-type rule. | Test |
| FR-104-AC-10 | Over the ConfigVersion package (TC-458's) with a process `Flow` (identity field `x`, field `n: Integer`) that declares operation `go` whose frame is `modifies [Flow/n]` and a closed population of `Flow`, `pre P using v on Config::Flow::go { self.n < 3 }`, `post Q using v on Config::Flow::go { self.n = pre(self.n) + 1 }` and `invariant I using v on Config::Flow at current { self.n >= 0 }` each check, `self` being `Reference<Config::Flow>` and `self.n` an `Attribute` of type `Integer`; `P`'s read is `pre`, `Q`'s reads are `post` and `pre`, `I`'s read is `current`. `pre M using v on Config::Flow::missing { true }` refuses `missing_declaration`/`missing-name` at `missing`. | Test (TC-459, TC-460) |

## Dependencies

- FR-102 (forms), FR-103 (operations), FR-062 and FR-097 (requirement
  records and extent), FR-088 (clause identity), FR-057 (the
  `operation-contract` kind).
- QSpec `state-contract.md` ("Operation anchors, aliases and captures",
  "Finite graph extension"), FR-043, FR-010 and FR-153.
