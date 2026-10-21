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
  (FR-100 Inputs). The facade stores the arguments as received. The identity
  and equality of a call is its function name and its set of (parameter name,
  value) pairs, so the order in which the arguments arrived (FR-100 lets them
  arrive in any order) does not change it.
- **Owner of a record**: the qualified name of the `Value` function
  declaration whose body holds the record's claim site (FR-062, "Owner"), as
  the checked graph holds that name. A `value-validity` record has exactly
  one owner. Every other record (an `operation-contract` record, FR-104, and a
  `temporal-satisfaction` record) has none.
- **Item of the call**: the requirement item (FR-075) written from a record
  whose owner is the selected function. A function reached by a call inside
  the selected function's body is not the selected function, so its records
  are not items of the call.
- **Package order**: the bytewise occurrence-key order of every record of the
  package (FR-075). An item's **request index** is its position in the package
  order.

## Inputs

- The arguments of `run` (FR-100 Typed public call): source identity, path,
  bytes, supplied domain packages, dependency input, limits, `Call`, object
  environment and `&Cancel`.

## Outputs

- `run_with_requirements` (design name; the implementing ticket chooses the
  Rust spelling) returns, on success, `RunWithRequirements`: the package's
  `package_id`, the call's `CallOutcome` (both as `run` returns them for the
  same inputs) and `CallRequirements`.
- `CallRequirements` holds the `package_id` of the compile that produced the
  members, the selected call as received, and its members in ascending
  occurrence-key order. A member is a `CallRequirement`: the record's
  `OccurrenceKey` and the item's request index.
- The entry's refusal type is `RunRefusal` (FR-100), which this requirement
  extends by no variant.

## Behavior

- **Association rule.** The facade SHALL return as members exactly the
  package's records whose owner is the selected function, one member per
  record. Two claim sites of one application node are two records and two
  members (FR-062). A function with no scalar operation application has no
  record, and the facade SHALL return an empty member list for it, not a
  refusal.
- **Arguments do not select items.** The facade SHALL NOT add or remove a
  member because of an argument's value, and SHALL NOT evaluate a member's
  path condition against the arguments. The path condition stays on the item
  (FR-075).
- **Outcome independence.** The facade SHALL return the members of a call
  whose outcome is `Refused`, `Undefined` or `Incomplete`, so long as the call
  ran. It SHALL return no `CallRequirements` for a refusal before the call
  (below).
- **One checked graph.** The facade SHALL compile the source once, and the
  call, the `package_id` and the members SHALL all come from that compile's
  checked package.
- **Pairing key.** The facade SHALL return, with each member, the request
  index of its item, and with the members the `package_id` of the compile
  that produced them, so that the pairing key of a member and its item is
  (`package_id`, request index). Consumers (informative): a consumer that
  writes items with `items_from_requirements` pairs a member with the item at
  its request index, written from a compile of the same `package_id`, whose
  `occurrence()` equals the member's key; a different `package_id` is the
  consumer's refusal, which the consumer's own specification states.
- **Typed refusals.** The facade SHALL refuse with an existing `RunRefusal`
  arm, FR-100's stage, code and category and QSpec FR-272's cause, and SHALL
  return no `CallRequirements`, in these cases:
  - the source, a model, a library or a dependency does not compile, so there
    is no checked graph: `Compile` (FR-100);
  - the function name is empty, is not an identifier, has more than one
    segment, or names no function of the checked package: `MissingDeclaration`,
    `missing_declaration`;
  - the named function's declared result is not Boolean, integer or Reference:
    `UnsupportedResult` (FR-100);
  - an argument fails binding or admission before the call: the refusal
    FR-100 names for it (`UnknownParameter`, `DuplicateArgument`,
    `UnboundParameter`, `WrongValueKind`, `Admission`, `Storage`,
    `DanglingReference`, `Supplied`, `UnsupportedHelper`);
  - the cancel handle stops the run: `Cancelled`.
- **Broken ownership is a fault.** The facade SHALL refuse with `Fault` (code
  `runtime_invariant`, stage `spine-run`, invariant `requirement-owner-known`,
  exit 30), and return no partial member list, when a `value-validity` record
  has no owner or an owner that names no function of the checked package. The
  owner is a single field, so two owners cannot arise and the facade never
  picks one.
- **Facade boundary.** The facade SHALL expose the entry and its result
  types (`run_with_requirements`, `RunWithRequirements`, `CallRequirements`
  and `CallRequirement`) at the `qsl_replay` root, where a consumer reaches
  them. The root SHALL also re-export every type of `run`'s signature and
  result that it does not export today, so that a consumer calls the entry
  and reads its result without naming `qsl_replay::spine`: `Call`,
  `CallArgument`, `CallOutcome`, `CallValue`, `CallRefusal` and
  `CallIncomplete` (the payloads of `CallOutcome::Refused` and
  `::Incomplete`), `RunRefusal`, `SpineLimits`, `PackageId` (from
  `qsl_semantics::library`), `ObjectEnvironment` (from `qsl_semantics`' model
  object environment), `SuppliedLimits` (from `qsl_semantics`' value
  supplied module) and `Cancel` (with `CancelCause`, from `quire_exact`).
  `DependencyInput` and `SourceIdentity` are already on the root and are not
  listed. `qsl_replay::spine` stays internal, a consumer names neither it nor
  `qsl_semantics`' checked graph (FR-060, T12-A), as for `compile_admitted`
  (FR-394), and `run` keeps its signature and result (FR-100).
- **Relation to settlement.** The association settles nothing. Mapping a
  provider's result for an item to a terminal (QSL-655) takes an item by the
  identity this requirement fixes and does not depend on this requirement,
  nor this on it.

## Acceptance Criteria

Fixture **K** is one `1-draft` unit under profile alias `v`, with these
functions (FR-062 RR-2, RR-5; TC-399):

- `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }`;
- `function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }`;
- `function id using v(x: Int[0, 9]): Int[0, 9] pure { x }`;
- `function two using v(): Int[0, 10] pure { inc(1) }`.

K has four records, in package order: the `integer.mul` of `sq` (request
index 0); the `x + 1` node's ordinal 0, which is `inc`'s `integer.add` with
result bound `Int[0, 10]` (index 1); and the same node's ordinals 1 and 2,
which are `sq`'s two `integer.add` sites (indices 2 and 3). The `id` and `two`
functions have none.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-395-AC-1 | Over K, `inc` with `x = 3` returns one member, request index 1; `sq` with `x = 3` returns three members in ascending key order, request indices 0, 2 and 3, the `integer.mul` first and then the two `integer.add` sites at ordinals 1 and 2 of the shared `x + 1` node; the two member sets are disjoint and each key is a key of K's records. | Test (TC-1089) |
| FR-395-AC-2 | Over K, each member's request index is the position of its key in the four-record package order, and the items `items_from_requirements` writes over K's records, at indices 0 to 3, have the members' keys; each of the four items is a member of exactly one of the calls of `inc` and `sq`. | Test (TC-1089) |
| FR-395-AC-3 | Over K, `id` and `two` return an empty member list and outcome `completed`; the `inc` record is not a member of `two`, which calls `inc`. | Test (TC-1089) |
| FR-395-AC-4 | Over K, `sq` with `x = 0` and with `x = 9` return equal member lists; `sq` with a Boolean `x` refuses `WrongValueKind` at position 0 with no `CallRequirements`. | Test (TC-1089) |
| FR-395-AC-5 | Over K, `sq` with `x = 3` under accounting `work_units = 0` returns outcome `incomplete` on `work_units` and the same three members as the unlimited call. | Test (TC-1089) |
| FR-395-AC-6 | For the calls of AC-1, AC-3 and AC-5 over K, the returned `package_id` equals the `package_id` of `run` and of `compile_package` over the same source, models, libraries and limits, and the returned outcome equals `run`'s outcome. | Test (TC-1089) |
| FR-395-AC-7 | The function `nope`, the empty name, `inc.x` and `1inc` refuse `MissingDeclaration` with code `missing_declaration`; a source that does not parse refuses `Compile`; none returns a member. | Test (TC-1089) |
| FR-395-AC-8 | A cancelled handle refuses `Cancelled` with no members. | Test (TC-1089) |
| FR-395-AC-9 | A consumer reaches the entry, its result types and FR-100's call types through the `qsl_replay` root alone, without naming `qsl_replay::spine`; FR-060's T12-A rule reports no call site outside its allowed callers for them. | Test (TC-1089) |
| FR-395-AC-10 | The association step, run directly on a checked package whose record map has a `value-validity` record with no owner, refuses `Fault` with stage `spine-run` and invariant `requirement-owner-known`; the same for a record whose owner names no function of the package; neither returns a partial member list. | Test (TC-1089) |
| FR-395-AC-11 | Two `Call`s of `add` of FR-062 RR-3 (`function add using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }`) whose arguments arrive as `x`, `y` and as `y`, `x` are equal calls and return equal members. | Test (TC-1089) |

## Dependencies

- FR-062 (records, claim sites and owner), FR-075 (items and request
  indices), FR-100 (`run`, `Call`, `RunRefusal`), FR-394 (the facade
  pattern), FR-060 (the T12 boundary), ADR-011.
