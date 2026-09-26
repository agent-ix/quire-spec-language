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
`qsl_replay::spine::compile` the CLI `compile` uses, FR-027) from the
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
  each entry's `identity` and `version`, and its source's labels, identity
  as path and provided bytes. It SHALL carry a dependency-input refusal,
  such as two entries whose sources share one authority and identity, as
  `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`).
- The executor SHALL carry a refusal of the spine recompile (ADR-015 D-1) as
  `ReplayRefusal::Recompile`. A dependency whose source recompiles to a
  `package_id` other than its import's recorded digest refuses there as
  `DependencyIdentityMismatch` at that import, a removed entry as
  `missing_import`/`missing-selection`, and an entry with a changed version
  as `stale_dependency`/`revision-mismatch`; when that import is in a
  library, the refusal is wrapped in `CompileRefusal::Dependency` with the
  library's path.
- When the recompiled `package_id` equals the request's, the executor SHALL
  refuse an entry whose identity the recompiled package's
  `dependency_selections` does not hold as
  `ReplayRefusal::DependencySelections`, and then an entry whose
  `package_id` differs from the closure's selection of its identity as
  `ReplayRefusal::DependencyIdentityMismatch` (`stale_dependency`), naming
  the identity, the entry's `package_id` (`requested`) and the recompiled
  one.
- The recompile SHALL run under the stage limits the request carries. The
  request's S1 `text_input_bytes` bounds S1's source bytes, and its S3
  `work_units` bounds the checker's work budget. No
  `quire.value.accounting/v1` counter names an S2 or I1 limit, so those
  stages run under their published defaults. An S1 limit above the reader
  limit (the FR-071 reader bound, 1 MiB) refuses before the recompile.
- The recompiled `package_id` SHALL equal the request's. No
  `CheckedPackage` is built from wire bytes.
- The executor SHALL resolve the selection by name lookup in the recompiled
  package's declarations (OQ-5), and SHALL refuse a function whose declared
  result is not `Boolean` before any call. It SHALL convert each argument's
  `WireNodeId` to a `NodeKey` only by lookup in the recompiled package, and
  order the arguments by the function's declared parameter positions,
  whatever order they arrive in.
- Each canonical integer assignment SHALL become a value of its
  parameter's declared type: an integer for an integer type, and `0` or
  `1` (`false`, `true`) for `Boolean`. Any other value for a Boolean
  parameter, or any value for a parameter of a kind no integer is,
  refuses before the call. A value outside a parameter's declared domain
  (`12` for `Int[0, 9]`) refuses at S6a admission. Both refuse with the
  same cause, `WrongValueKind` (`invalid_runtime_input`), naming the
  parameter's position.
- Each ADR-013 O-26 refusal SHALL be a typed `ReplayRefusal` variant with no
  partial result: request decode refusals (FR-071), a limit above the
  reader limit, a source reference that is not one source unit, a
  `dependencies` entry whose `package_id` differs from the recompiled
  closure's selection of its identity (`DependencyIdentityMismatch`), a
  `dependencies` list that is not strictly ascending or holds an entry the
  closure does not (`DependencySelections`), a dependency-input refusal
  (`DependencyInput`), a recompile
  refusal or stage limit (`stage_limit_exceeded`), a stale `package_id`, a
  selection naming no function node, an argument naming no parameter, a
  parameter bound twice or not at all, a witness that does not decode, an
  S6a admission refusal (wrong type, or a value outside the declared
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
| FR-098-AC-2 | The selection resolves by `QualifiedName` in the recompiled package; `Input` assignments and `Witness` bindings join the function's parameters by parameter node id, in declared parameter order whatever order they arrive in, and a Boolean parameter takes `0` and `1`; the call runs through S6a, and an agreeing replay settles `reproduced-without-witness` (`Input`) or `reproduced-with-evaluated-witness` with its FR-351 record (`Witness`), carrying the call's charges and the executor's toolchain pin. | Test (TC-444) |
| FR-098-AC-3 | A meaning-affecting source edit refuses by `package_id`, naming both identities. A presentation-only edit, which keeps the `package_id`, refuses by source digest. | Test (TC-444) |
| FR-098-AC-4 | Each refusal in Behavior -- unknown version, a missing input, a byte/digest mismatch, a stale `package_id`, a selection naming no function node, an arity mismatch, a type mismatch and a value outside the declared domain (each `WrongValueKind`), a limit above the reader limit, a recompile stage limit at S1 or S3, and a selection whose declared result is not `Boolean` (refused before any call, even with no accounting budget) -- refuses with its typed variant and no partial result. | Test (TC-444) |
| FR-098-AC-5 | A replay that disagrees with the refuted property settles `inconclusive` with cause `Verdicts`, and one that completes no value with cause `NoValue`, each holding both verdicts; neither is repaired. | Test (TC-444) |
| FR-098-AC-6 | A request for a proved package importing `test/units`, whose `dependencies` entry names `test/units`, its `package_id` and its one source, replays from the byte provision alone. With that entry's source bytes replaced by an edit that changes `test/units`'s `package_id` (digests updated to match), the replay refuses `ReplayRefusal::Recompile` carrying `DependencyIdentityMismatch` (`stale_dependency`), naming `test/units`, the recorded `package_id` and the recompiled one, with no verdict; with only the entry's `package_id` changed, it refuses `ReplayRefusal::DependencyIdentityMismatch` naming the same identity. | Test (TC-444) |
| FR-098-AC-7 | A request whose `dependencies` entries are swapped, or repeat one identity, refuses `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value` at `/package/dependencies`) before any recompile; one carrying an extra entry no import reaches refuses `DependencySelections` after the recompile; two entries whose sources share one authority and identity refuse `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`); one lacking an entry refuses `ReplayRefusal::Recompile` carrying `missing_import`/`missing-selection` at the import; an entry naming two sources, or a definition document, refuses as a source reference that is not one source unit. None yields a verdict. | Test (TC-444) |

## Dependencies

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

Implemented under QSL-5. TC-444 passes locally for AC-1 to AC-5. QSL-257
added TC-444 coverage of a predicate whose body calls another declared
function (the QSL-22 Layer 3 exemplar's shape), confirming the S4 emitter
writes the checked `call` node codegen's FR-021 oracle generator reads.

AC-6 and AC-7 (ADR-015 D-4) are implemented under QSL-255 part (b);
TC-444 step 7 passes locally.
