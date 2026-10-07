---
id: FR-098
title: "Execute a replay request through the layer-6 replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# FR-098: Execute a replay request through the layer-6 replay facade

## Description

QSL SHALL provide the replay executor entry, `qsl_replay::replay`, as the
layer-6 `replay` facade's public API (ADR-013 O-26, C-13, TK-01; ADR-011
§2.1 E9, §6.1). It takes an FR-071 replay request. It recompiles the proved
package's one source unit through the spine (S1 to S4, the same
the FR-278 composition the CLI `compile` uses, FR-027) from the
request's digest-addressed byte provision, against the dependency input the
package reference's `dependencies` entries name (ADR-015 D-4). It requires
the recompiled `package_id` to equal the request's, selects the function
by its `QualifiedName` in the recompiled package's declarations, joins the
replay source's arguments to the function's parameters by parameter node
id, calls the function through S6a (`CheckedPackage::call`), and settles
an FR-072 replay result. The counterexample a request replays refuted its
property, so the proved verdict is `violation`.

The spine compile is public only because `command`, another crate, calls
it. It is not part of the facade CG may call: the FR-060 T12-A rule
refuses any CG reference to `qsl_replay::spine` -- a `use` of it, a path
through it, or a crate alias or glob import that reaches it.

## Inputs

- An FR-071 replay request, in its wire shape.

## Outputs

- An FR-072 replay result on the arm of the request's `ReplaySource`, or a
  structured refusal (`ReplayRefusal`) with no partial result.

## Behavior

- The executor SHALL obtain every source and domain-package input from the
  request's byte provision by digest, and SHALL read no path, environment
  variable or search location. Domain packages are the provision's
  `sha256-jcs` entries, handed to I1 as its package input.
- The executor SHALL refuse a request by the first of ADR-015 D-4's rules,
  each rule applied over all `dependencies` entries, in entry order, before
  the next rule, with QSpec FR-323's codes for the rules FR-323 states; the
  one-source rule and the dependency-input rule are QSL's own and run
  before the recompile.
- The executor SHALL refuse entries not in strictly ascending UTF-8 byte
  order of `identity`, a repeated identity included, as
  `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value`
  at `/package/dependencies`), before it checks any source count or builds
  any dependency input.
- The package reference's `sources` SHALL name exactly one
  `quire.source.bytes/v1` source, the proved package's, and each of its
  `dependencies` entries (QSpec FR-323) SHALL name exactly one such source,
  that dependency's. The executor recompiles each source under its four
  FR-001 labels. A `sources` list or an entry naming a definition document,
  or other than one source, refuses: the recompile reads no definition
  document, and each package compiles from one unit.
- The executor SHALL build the dependency input (FR-099) from the entries:
  each entry's `identity`, and its source's labels, identity
  as path and provided bytes. It SHALL carry a dependency-input refusal,
  such as two entries whose sources share one authority and identity, as
  `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`).
- The executor SHALL carry a refusal of the spine recompile (ADR-015 D-1) as
  `ReplayRefusal::Recompile`. A removed entry refuses as
  `missing_import`/`missing-selection`; when that import is in a
  library, the refusal is wrapped in `CompileRefusal::Dependency` with the
  library's path.
- After the recompile, and before comparing the proved package's
  `package_id`, the executor SHALL refuse an entry whose identity the recompiled package's
  `dependency_selections` does not hold as
  `ReplayRefusal::DependencySelections`, and then an entry whose
  `package_id` differs from the closure's selection of its identity as
  `ReplayRefusal::DependencyIdentityMismatch` (`stale_dependency`/`content-mismatch`), naming
  the identity, the entry's `package_id` (`requested`) and the recompiled
  one. An edited dependency source also changes the proved package's
  `package_id`, so these checks run first and name the stale dependency.
- The recompile SHALL run under the stage limits the request carries. The
  request's `stage_limits` maps each setting name to its bound, every entry
  passes through to its stage, and a setting with no entry runs at its
  published default
  ([FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md),
  [FR-263](FR-263-replay-at-any-depth-under-the-request-limits.md)). An
  `s1.input_bytes` bound above `replay.input_bytes` refuses before the
  recompile.
- The recompiled `package_id` SHALL equal the request's. No
  `CheckedPackage` is built from wire bytes.
- The executor SHALL resolve the selection by name lookup in the recompiled
  package's declarations (OQ-5), and SHALL refuse a function whose declared
  result is not `Boolean` before any call (predicate replay; FR-357's
  value-parity entry replays a non-`Boolean` function). It SHALL convert each argument's
  `WireNodeId` to a `NodeKey` only by lookup in the recompiled package, and
  order the arguments by the function's declared parameter positions.
  `Input` assignments, and the `WitnessBinding` list the executor builds,
  may arrive in any order. A witness transcript's entries may not: FR-070
  requires them in ascending parameter node id and refuses any other order
  (`DecodeRefusal::EntryOrder`).
- Each argument SHALL arrive as a typed `WitnessValue` and become a value
  of its parameter's declared type. An `Input` assignment carries
  `WitnessValue::Boolean` for a `Boolean` parameter and
  `WitnessValue::Integer` for an integer type; a witness transcript entry
  reads as the parameter's binding type, `0` or `1` (`false`, `true`) for
  `Boolean` and a decimal integer for an integer type. An integer for a
  Boolean parameter, a Boolean for an integer one, or any value for a
  parameter of a kind no `WitnessValue` is, refuses before the call. A value outside a parameter's declared domain
  (`12` for `Int[0, 9]`) refuses at S6a admission. Both refuse with the
  same cause, `WrongValueKind` (`invalid_runtime_input`), naming the
  parameter's position.
- The executor SHALL take a `WitnessValue` tree for a parameter of any value
  type other than `Boolean`, an integer type and `Population<T>`: a leaf of
  enum, text, rational, decimal, float, quantity or reference type, or an
  `Option`, record, tuple, union or collection nested to any depth with
  leaves of any value family. It arrives as an `Input` assignment's value,
  or as a witness entry read as a `Canonical` binding in FR-070's witness
  value text. A `Population<T>` parameter is not a value and refuses
  `WrongValueKind` before the call, as before.
- The executor SHALL convert a `WitnessValue` tree to a kernel value of the
  parameter's declared type. It walks the value against the recompiled
  package's type environment on an explicit heap stack (ADR-030 D-1) and
  builds each node through the kernel's value constructors. It SHALL refuse
  `WrongValueKind`, naming the parameter's position, before the call and
  with no charge, when:
  - a node's shape is not its declared type's kind (for example, a
    sequence for a `Set`, a record for an `Option`, a `float32` for a
    `Float64`, or a decimal for a `Rational`);
  - an enum, record, tuple or union `name` is not the declared type's
    declaration node id, or a quantity's `unit` is not the declared unit;
  - an enum or union `member` is not a member of that declaration;
  - a record's fields are not exactly the declared fields in declaration
    order, or an absent or `null` slot is given for a field that does not
    admit it;
  - an arity differs from the declaration;
  - a reference's `object_type` is not the declared `Reference<T>`'s
    effective object type;
  - a `set` or `ordered-set` holds two elements the kernel's equality
    relates (decimals compare by numeric value, so `1.0` and `1.00` are
    duplicates, though FR-070 gives them distinct value texts);
  - a decoded `set` or `ordered-set` whose elements bear a float. S3 admits
    no declared set type whose element type bears a float (QSpec
    FR-144-AC-6, FR-323-AC-3), so such a value matches no declared type and
    fails conversion as a kind mismatch.
- An enum member converts to the kernel member with its declared rank. A
  text converts to a kernel text of the declared text type. A rational,
  decimal, float or quantity converts exactly, with no rounding. A
  reference converts to the kernel object reference, which a predicate
  replay holds as a value. A replayed function has no object environment,
  so a call that dereferences one completes no value and settles
  `inconclusive` with cause `NoValue`.
- The executor SHALL admit a converted argument through S6a
  (`CheckedPackage::call`), the admission every runtime argument takes,
  under FR-106's input rule: a supplied input outside its declared domain is
  refused input, never evidence. A leaf outside its declared range (`12`
  in an `Int[0, 9]` field) or a collection outside its declared cardinality
  (four elements for `Sequence<Int[0, 9]>[0, 3]`) SHALL refuse
  `WrongValueKind` (`invalid_runtime_input`), naming the parameter's
  position, before the call. So do a text outside its declared length or
  profile, a decimal outside its declared range or scales, a rational
  outside its declared domain, and a float or quantity the declared type
  does not admit. Union arguments follow FR-321's admission.
- The conversion SHALL stop at the request's limits, never at a depth. Before
  the call, it SHALL count the occurrences of each converted argument
  (`quire.value.accounting/v1` `occ`) against the request's accounting
  limit `value_occurrences`, and the nodes it converts against
  `work_units`. These are `quire.value.accounting/v1` accounting limits
  (ADR-014 B-2), not stage limits. FR-255 names each by its counter name,
  which is its setting, and a `stage_limits` entry of that name replaces
  the request's accounting-limits value for that counter (FR-255
  Behavior 10). When
  a count exceeds its limit, the replay SHALL settle, with no call, as
  FR-277 settles `execute` reaching an accounting limit: the evaluation
  outcome `Incomplete`, naming the counter, its configured value and the
  count reached. FR-098 settles a call that completes no value that way:
  `inconclusive` with cause `NoValue`. By deliberate choice these checks
  charge nothing to the call's meter. The call then consumes the same
  charges it did in the proving run, so replay parity compares like with
  like. The
  request's or envelope's bytes are bounded by `replay.input_bytes`, as
  [FR-263](FR-263-replay-at-any-depth-under-the-request-limits.md) states.
  The request's measured size counts each `Input` value tree at the byte
  length of its escaped FR-070 value text.
- Each ADR-013 O-26 refusal SHALL be a typed `ReplayRefusal` variant with no
  partial result: request decode refusals (FR-071), a limit above the
  reader limit, a source reference that is not one source unit, a
  `dependencies` entry whose `package_id` differs from the recompiled
  closure's selection of its identity (`DependencyIdentityMismatch`), a
  `dependencies` list that is not strictly ascending or holds an entry the
  closure does not (`DependencySelections`), a dependency-input refusal
  (`DependencyInput`), a recompile
  refusal or stage limit (`stage_limit_exceeded`), a stale `package_id` (`stale_dependency`/`content-mismatch`, carrying the request's and the recompiled `package_id`), a
  selection naming no function node, an argument naming no parameter, a
  parameter bound twice or not at all, a witness that does not decode
  (naming the request's `obligation_identity`), an S6a
  admission refusal (wrong type, or a value outside the declared
  domain), and a selected function whose declared result is not
  `Boolean`. Each has a catalog code (`ReplayRefusal::code`).
- A replay whose verdict differs from `violation` SHALL settle
  `inconclusive` with both verdicts as its typed cause (`Verdicts`), and one
  whose S6a outcome completes no value (`refused`, `incomplete`,
  `undefined` or a family result) SHALL settle `inconclusive` with cause
  `NoValue`, never repaired (FR-072).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | A request whose byte provision carries its one source and, under their `sha256-jcs` digests, the domain packages the source selects recompiles from those bytes alone, keeps its `package_id`, and replays. A package reference naming a definition document or two sources refuses before any recompile. | Test (TC-444) |
| FR-098-AC-2 | The selection resolves by `QualifiedName` in the recompiled package; `Input` assignments and `Witness` entries join the function's parameters by parameter node id, in declared parameter order (`Input` assignments in any order; transcript entries in FR-070's ascending node-id order), and a Boolean parameter takes a witness entry `0` or `1` and an `Input` assignment's `WitnessValue::Boolean` (an `Input` `WitnessValue::Integer` for it refuses `WrongValueKind`); the call runs through S6a, and an agreeing replay settles `reproduced-without-witness` (`Input`) or `reproduced-with-evaluated-witness` (`Witness`) with no QSpec FR-351 record, since a function call's result has no decisive occurrence (ADR-031 SW-7), carrying the call's charges. | Test (TC-444) |
| FR-098-AC-3 | A meaning-affecting source edit refuses `stale_dependency`/`content-mismatch` by `package_id`, naming both identities. A presentation-only edit, which keeps the `package_id`, refuses by source digest. | Test (TC-444) |
| FR-098-AC-4 | Each refusal in Behavior -- a missing input, a byte/digest mismatch, a stale `package_id`, a selection naming no function node, an arity mismatch, a type mismatch and a value outside the declared domain (each `WrongValueKind`), a limit above the reader limit, a recompile stage limit at S1 or S3, and a selection whose declared result is not `Boolean` (refused before any call, even with no accounting budget) -- refuses with its typed variant and no partial result. | Test (TC-444) |
| FR-098-AC-5 | A replay that disagrees with the refuted property settles `inconclusive` with cause `Verdicts`, and one that completes no value with cause `NoValue`, each holding both verdicts; neither is repaired. | Test (TC-444) |
| FR-098-AC-6 | A request for a proved package importing `test/units`, whose `dependencies` entry names `test/units`, its `package_id` and its one source, replays from the byte provision alone. With that entry's source bytes replaced by an edit that changes `test/units`'s `package_id` (digests updated to match), the replay refuses `ReplayRefusal::DependencyIdentityMismatch` (`stale_dependency`), naming `test/units`, the entry's `package_id` and the recompiled one, with no verdict; with only the entry's `package_id` changed, it refuses `ReplayRefusal::DependencyIdentityMismatch` naming the same identity. | Test (TC-444) |
| FR-098-AC-7 | A request whose `dependencies` entries are swapped, or repeat one identity, refuses `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value` at `/package/dependencies`) before any recompile; one carrying an extra entry no import reaches refuses `DependencySelections` after the recompile; two entries whose sources share one authority and identity refuse `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`); one lacking an entry refuses `ReplayRefusal::Recompile` carrying `missing_import`/`missing-selection` at the import; an entry naming two sources, or a definition document, refuses as a source reference that is not one source unit. None yields a verdict. | Test (TC-444) |
| FR-098-AC-8 | Over `record Inner { a: Int[0, 9]; b?: Boolean; }`, `union Shape { Circle(Int[0, 9]), Empty }` and `record Outer { i: Inner; o: Option<Int[0, 9]>; s: Sequence<Int[0, 9]>[0, 3]; u: Shape; }`, a predicate `p(x: Outer): Boolean` replays its counterexample from an `Input` assignment and from a `Witness` entry, and each settles as AC-2 states, with the replayed call evaluating the nested values. Each of these refuses `WrongValueKind` naming position 0, before the call and with no charge: a sequence given for `o`; an `Inner` whose `name` is `Outer`'s node id; a record missing `a`, and one with an undeclared field; an absent `a`; union member `Square`; `Circle` with no component; a leaf `12` for `a`; and four elements for `s`. Over `g(v): Boolean`, whose `v` is a `Set` of a `Decimal` type with scales 0 to 2, a set holding `1.0` and `1.00` (distinct value texts, numerically equal) refuses `WrongValueKind` the same way. | Test (TC-906) |
| FR-098-AC-9 | For the AC-8 counterexample, a request whose accounting limit `value_occurrences` is one below the argument's occurrence count makes no call and settles `inconclusive` with cause `NoValue`, carrying the outcome `Incomplete` that names `value_occurrences`, its configured value and the count reached. A request whose `work_units` is below the argument's node count settles the same way, naming `work_units`. Each replays with the limit raised to fit, and its charges equal those of a call with no pre-call check. | Test (TC-906) |
| FR-098-AC-10 | One predicate per leaf family, each taking one parameter of that family (an enum `Color`, `Text[0, 8]`, `Rational`, a `Decimal` with scales `[0, 2]`, `Float64`, a quantity in a declared unit `m`, and a `Reference<T>`), replays its counterexample from a `Witness` entry in FR-070's witness value text and settles as AC-2 states; the reference predicate tests the reference for equality with itself and does not dereference it. Each of these refuses `WrongValueKind` naming position 0, before the call: an enum value naming another enum's declaration; a nine-scalar text; a decimal with scale 3; a `float32` for the `Float64`; a quantity in another unit; and a reference whose `object_type` is another type's. A predicate that dereferences its reference settles `inconclusive` with cause `NoValue`. | Test (TC-906) |
| FR-098-AC-11 | A proved package whose one source declares `function wide using v(x: Int[0, 18446744073709551615]): Boolean pure { x <= 18446744073709551614 }` recompiles from its byte provision to the request's `package_id`, and an `Input` assignment of `x` = 18446744073709551615, carried on the replay wire as the decimal string `"18446744073709551615"`, settles `reproduced-without-witness` with the call's result `false`. The same holds for `x: Int[0, 9223372036854775808]` with body `x <= 9223372036854775807` and `x` = `"9223372036854775808"`, for `x: Int[0, 18446744073709551616]` with body `x <= 18446744073709551615` and `x` = `"18446744073709551616"`, and for `x: Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]` with body `x > -170141183460469231731687303715884105728` and `x` = `"-170141183460469231731687303715884105728"`. As the QSL-source stand-in for QSL-642's CG wide-range model (the u64 range CG admits under IR-624 AC35), a unit declaring `record Meter { reading: Int[0, 18446744073709551615]; }` and `function meter_over using v(m: Meter): Boolean pure { m.reading <= 18446744073709551614 }` recompiles from its byte provision to the request's `package_id`. | Test (TC-913) |

## Dependencies

- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md): the
  witness transcript, its witness value text and `decode`.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): inputs out of their
  declared domain are refused input;
  [FR-321](FR-321-admit-supplied-union-values.md): union argument admission;
  [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md):
  `LimitExceeded`.
- [FR-071](FR-071-implement-typed-replay-request.md): the request and its
  decode-time refusals.
- [FR-072](FR-072-implement-typed-replay-result.md): the result and its
  settlement.
- [FR-027](FR-027-export-compiled-native-package.md): the spine compile.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input and the S4 source resolution; ADR-015 D-4.
- ADR-013 O-25, O-26, C-11, C-13, OQ-5; ADR-011 §2.1 E9, §4, §6.1.
- QSpec FR-323 (`byte_provision`, `replay`).

## Status

Implemented. TC-444 passes locally for AC-1 to AC-5. TC-444
also covers a predicate whose body calls another declared
function (the QSL-22 Layer 3 exemplar's shape), confirming the S4 emitter
writes the checked `call` node codegen's FR-021 oracle generator reads.

AC-6 and AC-7 (ADR-015 D-4) are implemented;
TC-444 step 7 passes locally.

AC-11 (QSL-642, integer bounds up to i128) is implemented; TC-913 passes locally.
AC-8 to AC-10 (composite and leaf-family arguments, QSL-640) are implemented
(`qsl_replay::replay` takes `ReplayLimits` for a raised `replay.input_bytes`): TC-906 passes
locally for records, options, sequences, sets, enums, text, rationals,
decimals, floats and references, the occurrence and node limits, and each
refusal. A replay runs its call over an object environment with unresolved
references (`ObjectEnvironment::with_unresolved_references`): a reference
argument is admitted by its identity, and a read through one completes no
value (`unknown_required_feature`/`unsupported-feature`, construct
`dereference`), so it settles `inconclusive` with cause `NoValue`. Pending:

- the union cases of AC-8 and AC-9: S6a admits no union argument until QSL-503;
- a quantity-typed parameter in source (AC-10): complete-V1 source can name no
  unit as a type (STD-113), so the quantity conversion is tested directly.

