---
id: FR-395
title: "Associate the selected call's requirement items through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-395: Associate the selected call's requirement items through the replay facade

## Description

A driver that runs one function of a program and then routes its
requirement items needs to know which of the package's requirement items
belong to the function it ran. `run` (FR-100) returns the package id and
the call's outcome and no checked graph; `items_from_requirements`
(FR-075) writes one item per record of the whole package, with no link to a
function; an item's identity is its package occurrence key. A consumer must
not rebuild that link, because it would re-derive an association QSL's check
stage already decided, and ADR-011 T12 and FR-060 forbid it to name
`qsl_replay::spine`.

This requirement adds the replay-owned entry that runs a call as FR-100
does and returns, with the outcome, the identities of the requirement items
that belong to the selected call. The identities come from the same checked
package the call ran against; nothing is recomputed.

## Terms

- **Selected call**: the function name and the argument list the entry
  received, as FR-100's `Call` carries them. The function is one identifier
  (FR-100 Inputs).
- **Owner of a record**: the `Value` function declaration whose body holds
  the record's claim site (FR-062, "Claim sites"). A `value-validity`
  record has exactly one owner; an `operation-contract` record (a state
  clause's or an operation frame's, FR-104) has none.
- **Item of the call**: the requirement item (FR-075) written from a record
  whose owner is the selected function. A function reached by a call inside
  the selected function's body is not the selected function, so its records
  are not items of the call.
- **Package order**: the records' bytewise occurrence-key order (FR-075),
  over every record of the package. An item's **request index** is its
  position in the package order.

## Inputs

- The arguments of `run` (FR-100 Typed public call): source identity, path,
  bytes, supplied domain packages, dependency input, limits, `Call`, object
  environment and `&Cancel`.

## Outputs

- `run_with_requirements` (design name; the implementing ticket chooses the
  Rust spelling) returns, on success, `RunWithRequirements`: the package's
  `package_id`, the call's `CallOutcome` (both exactly as `run` returns them
  for the same inputs) and `CallRequirements`.
- `CallRequirements` holds the selected call as received (the function name
  and the arguments, in the declared parameter order) and its members, in
  ascending occurrence-key order. A member is a `CallRequirement`: the
  record's `OccurrenceKey` and the item's request index. A member names its
  item by identity; a consumer pairs it with the item `items_from_requirements`
  writes at that request index, whose `occurrence()` equals the member's key.
- The entry's refusal type is `RunRefusal` (FR-100), which this requirement
  extends by no variant.

## Behavior

- **Association rule.** The members are exactly the package's records whose
  owner is the selected function, one member per record. Two claim sites of
  one application node are two records and two members (FR-062). A function
  with no scalar operation application has no record and so no member: the
  result carries an empty member list, which is not a refusal.
- **Arguments do not select items.** The arguments identify the call and are
  echoed; they neither add nor remove a member. Each member's path condition
  is on its item (FR-075); the entry does not evaluate it against the
  arguments.
- **Outcome independence.** The members do not depend on the call's outcome.
  A call whose outcome is `Refused`, `Undefined` or `Incomplete` (the call
  ran) returns its members. A refusal before the call (below) returns no
  `CallRequirements`.
- **One checked graph.** The entry compiles the source once. The call, the
  `package_id` and the members all come from that compile's checked package,
  so the members' occurrence keys are keys of the package whose id is
  returned, and the request indices are positions in that package's order.
- **Typed refusals.** Each is an existing `RunRefusal` arm and FR-272 cause,
  with FR-100's stage, code and category, and no `CallRequirements`:
  - the source, a model, a library or a dependency does not compile, so there
    is no checked graph: `Compile` (FR-100);
  - the function name is empty, is not an identifier, has more than one
    segment, or names no function of the checked package:
    `MissingDeclaration`, `missing_declaration`;
  - the named function's declared result is not Boolean, integer or
    Reference: `UnsupportedResult` (FR-100);
  - an argument fails binding or admission before the call: the refusal
    FR-100 names for it (`UnknownParameter`, `DuplicateArgument`,
    `UnboundParameter`, `WrongValueKind`, `Admission`, `Storage`,
    `DanglingReference`, `Supplied`, `UnsupportedHelper`);
  - the cancel handle stops the run: `Cancelled`.
- **Ambiguous association is a fault.** A `value-validity` record with no
  owner or with two owners, or an owner that names no function of the checked
  package, is a broken check-stage invariant: `Fault`, `runtime_invariant`,
  stage `call`, exit 30. The entry never picks one owner and never returns a
  partial member list.
- **Facade boundary.** The entry and its types are reached through the
  `qsl_replay` root alone. A consumer names neither `qsl_replay::spine` nor
  `qsl_semantics`' checked graph to obtain them (FR-060, T12-A), as for
  `compile_admitted` (FR-394). `run` keeps its signature and result (FR-100)
  and is not changed by this requirement.
- **Relation to settlement.** The association says which items belong to a
  call; it settles nothing. Mapping a provider's result for an item to a
  terminal (the settlement prerequisite QSL-655 owns) takes an item by the
  identity this requirement fixes (the occurrence key and request index) and
  does not depend on this requirement, nor this on it.

## Acceptance Criteria

Fixture **K** is one `1-draft` unit under profile alias `v`, with these
functions (FR-062 RR-2, RR-5; TC-399):

- `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }`;
- `function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }`;
- `function id using v(x: Int[0, 9]): Int[0, 9] pure { x }`;
- `function two using v(): Int[0, 10] pure { inc(1) }`.

K has four records: one for `inc`, three for `sq`, and none for `id` and
`two`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-395-AC-1 | Over K, `run_with_requirements` of `inc` with `x = 3` returns one member; of `sq` with `x = 3` returns three, in ascending occurrence-key order, the two `integer.add` sites at ordinal 0 and 1 of one node and the `integer.mul`; the member sets of `inc` and `sq` are disjoint, and each member's key is a key of K's `graph().requirements()`. | Test (TC-1089) |
| FR-395-AC-2 | Over K, each member's request index is its record's position in the four-record package order, so `items_from_requirements` over K's records yields at that index an item whose `occurrence()` equals the member's key; the members of `inc` and `sq` together are exactly the four items and no item has two calls. | Test (TC-1089) |
| FR-395-AC-3 | Over K, `id` and `two` return an empty member list and outcome `completed`; the `inc` records are not members of `two`, which calls `inc`. | Test (TC-1089) |
| FR-395-AC-4 | Over K, `sq` with `x = 0` and with `x = 9` returns equal member lists; a call whose argument has the wrong kind refuses `WrongValueKind` and returns no members. | Test (TC-1089) |
| FR-395-AC-5 | Over K, `sq` with `x = 3` under accounting `work_units = 0` returns outcome `incomplete` on `work_units` and the same three members as the unlimited call. | Test (TC-1089) |
| FR-395-AC-6 | `package_id` and `CallOutcome` equal those of `run` and of `compile_package` for the same inputs, for each of the calls of AC-1, AC-3 and AC-5. | Test (TC-1089) |
| FR-395-AC-7 | The function `nope`, the empty name, the name `inc.x` and the name `1inc` refuse `MissingDeclaration` with code `missing_declaration`; a source that does not parse refuses `Compile`; neither returns a member. | Test (TC-1089) |
| FR-395-AC-8 | A cancelled handle refuses `Cancelled` with no members. | Test (TC-1089) |
| FR-395-AC-9 | A consumer reaches the entry and its types through the `qsl_replay` root alone, without naming `qsl_replay::spine`; FR-060's T12-A rule reports no call site outside its allowed callers for it. | Test (TC-1089) |
| FR-395-AC-10 | The ambiguity fault is covered at the check stage: every `value-validity` record of FR-062's RR-1 to RR-17 has exactly one owner, equal to the function whose body holds the claim site, and an `operation-contract` record has none (FR-062-AC-14). | Test (TC-1089) |

## Dependencies

- FR-062 (records and their claim sites), FR-075 (items and request
  indices), FR-100 (`run`, `Call`, `RunRefusal`), FR-394 (the facade
  pattern), FR-060 (the T12 boundary), ADR-011.
