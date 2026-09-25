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
§2.1 E9, §6.1). It takes an FR-071 replay request. It recompiles the one
source unit the request's package reference names through the spine (S1
to S4, the same `qsl_replay::spine::compile` the CLI `compile` uses,
FR-027) from the request's digest-addressed byte provision. It requires
the recompiled `package_id` to equal the request's, selects the function
by its `QualifiedName` in the recompiled package's declarations, joins the
replay source's arguments to the function's parameters by parameter node
id, calls the function through S6a (`CheckedPackage::call`), and settles
an FR-072 replay result. The counterexample a request replays refuted its
property, so the proved verdict is `violation`.

The spine compile is public only because `command`, another crate, calls
it. It is not part of the facade CG may call: the FR-060 T12-A rule
refuses a CG call into `qsl_replay::spine::`.

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
- The package reference SHALL name exactly one `quire.source.bytes/v1`
  source. The executor recompiles that source under its four FR-001 labels.
  A reference naming a definition document, or more than one source,
  refuses: the recompile reads no definition document, and it compiles one
  unit.
- The recompile SHALL run under the stage limits the request carries. The
  request's S1 `text_input_bytes` bounds S1's source bytes, and its S3
  `work_units` bounds the checker's work budget. No
  `quire.value.accounting/v1` counter names an S2 or I1 limit, so those
  stages run under their published defaults. An S1 limit above the reader
  limit (the FR-071 reader bound, 1 MiB) refuses before the recompile.
- The recompiled `package_id` SHALL equal the request's. No
  `CheckedPackage` is built from wire bytes.
- The executor SHALL resolve the selection by name lookup in the recompiled
  package's declarations (OQ-5). It SHALL convert each argument's
  `WireNodeId` to a `NodeKey` only by lookup in the recompiled package, and
  order the arguments by the function's declared parameter positions.
- Each ADR-013 O-26 refusal SHALL be a typed `ReplayRefusal` variant with no
  partial result: request decode refusals (FR-071), a limit above the
  reader limit, a source reference that is not one source unit, a recompile
  refusal or stage limit (`stage_limit_exceeded`), a stale `package_id`, a
  selection naming no function node, an argument naming no parameter, a
  parameter bound twice or not at all, a witness that does not decode, an
  S6a admission refusal (wrong type, or a value outside the declared
  domain), and a selected function that completes a non-Boolean value.
- A replay whose verdict differs from `violation`, including one whose S6a
  outcome completes no value (`refused`, `incomplete`, `undefined` or a
  family result), SHALL settle `inconclusive` with both verdicts as its
  typed cause, never repaired.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | A request whose byte provision carries its one source and, under their `sha256-jcs` digests, the domain packages the source selects recompiles from those bytes alone, keeps its `package_id`, and replays. A package reference naming a definition document or two sources refuses before any recompile. | Test (TC-444) |
| FR-098-AC-2 | The selection resolves by `QualifiedName` in the recompiled package; `Input` assignments and `Witness` bindings join the function's parameters by parameter node id; the call runs through S6a, and an agreeing replay settles `reproduced-without-witness` (`Input`) or `reproduced-with-evaluated-witness` with its FR-351 record (`Witness`), carrying the call's charges and the executor's toolchain pin. | Test (TC-444) |
| FR-098-AC-3 | A meaning-affecting source edit refuses by `package_id`, naming both identities. A presentation-only edit, which keeps the `package_id`, refuses by source digest. | Test (TC-444) |
| FR-098-AC-4 | Each refusal in Behavior -- unknown version, a missing input, a byte/digest mismatch, a stale `package_id`, a selection naming no function node, an arity mismatch, a type mismatch, a value outside the declared domain, a limit above the reader limit, a recompile stage limit, and a non-predicate selection -- refuses with its typed variant and no partial result. | Test (TC-444) |
| FR-098-AC-5 | A replay that disagrees with the refuted property, or completes no value, settles `inconclusive` with a typed cause holding both verdicts, and is never repaired. | Test (TC-444) |
| FR-098-AC-6 | A dependency whose source, recompiled from the byte provision, yields a `package_id` other than the one the proved package records refuses with `DependencyIdentityMismatch` (`stale_dependency`), ADR-011 §4 dependency binding. | Test (planned) |

## Dependencies

- [FR-071](FR-071-implement-typed-replay-request.md): the request and its
  decode-time refusals.
- [FR-072](FR-072-implement-typed-replay-result.md): the result and its
  settlement.
- [FR-027](FR-027-export-compiled-native-package.md): the spine compile.
- ADR-013 O-25, O-26, C-11, C-13, OQ-5; ADR-011 §2.1 E9, §4, §6.1.
- QSpec FR-323 (`byte_provision`, `replay`).

## Status

Implemented under QSL-5. TC-444 passes locally for AC-1 to AC-5.

AC-6 is not delivered. No package can have a dependency today: E3 refuses a
unit that declares an `import`, the emitter writes
`dependency_selections: []`, and `CheckedPackage::dependencies` is always
empty (FR-087, AC-13 notes; ADR-011 §2). The owner is the QSpec checked-package
v2 schema defect that types a `dependency_selections` entry as a
definition selection, then QSL-6 (ADR-011 M-4), which fills the E4
dependency closure. Remaining work: QSL-6.
