---
id: FR-399
title: "Set a named stage limit through the replay root"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-399: Set a named stage limit through the replay root

## Description

`compile_admitted` (FR-394) and the replay entry points take a `StageLimits`
(FR-263). A consumer of QSL builds one from `(Setting, u64)` pairs, but
`Setting` is a `qsl-foundation` type, and ADR-011 T-12 forbids a consumer such
as the contract generator a direct dependency on `qsl-foundation`. A consumer
with a finite fixture source larger than the default `s1.input_bytes`
(1 MiB) therefore cannot raise the input allowance. This requirement adds a
`StageLimits` constructor that takes FR-255's setting name, so a consumer
sets a named limit through the `qsl_replay` root alone and names no
foundation type.

## Inputs

- A `StageLimits` (the default, or one already built).
- A setting name (FR-255's dotted name, such as `s1.input_bytes`) and a bound.

## Outputs

- `StageLimits::with_named(name, bound)` returns the limits with that entry
  set, or a `StageLimitNameRefusal` naming the rejected name.

## Behavior

- The names accepted are exactly those a request's `stage_limits` carries
  (FR-263): every stage and accounting setting of FR-255's tables except
  `replay.input_bytes` and `format.output_bytes`. `replay.input_bytes` is the
  reader limit a caller passes as `ReplayLimits`, which the root already
  exports. The request decoder and `with_named` share one check, so the two
  cannot disagree.
- An unknown name, or one of the two excluded names, refuses with the typed
  `StageLimitNameRefusal` and returns no limits: `with_named` consumes the
  limits and yields either the whole new value or the refusal.
- A later entry for a name replaces an earlier one. A bound is used as given,
  with no ceiling (FR-255). Setting `s1.input_bytes` above the reader limit
  refuses at `compile_admitted` as `AdmittedResource::AboveReader`, so a
  consumer raises `ReplayLimits::with_input_bytes` with it.
- A limit with no entry runs at its published default: the capability is an
  opt-in and changes no default and no other consumer's caps.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-399-AC-1 | A consumer that names only the `qsl_replay` root compiling a source larger than 1 MiB through `compile_admitted` with default limits is refused as `AdmittedResource::StageLimit`. | Test (TC-1093) |
| FR-399-AC-2 | The same consumer building `StageLimits::default().with_named("s1.input_bytes", n)` with `n` above the source, and `ReplayLimits::default().with_input_bytes(n)`, has the source compiled; with the default `ReplayLimits` it is refused `AdmittedResource::AboveReader`. | Test (TC-1093) |
| FR-399-AC-3 | `with_named` refuses an unknown name, the empty name, `replay.input_bytes` and `format.output_bytes` with `StageLimitNameRefusal` carrying the name, and returns no limits. | Test (TC-1093) |
| FR-399-AC-4 | A default `StageLimits` holds no entry, so the default `s1.input_bytes` is unchanged; a later `with_named` of one name replaces the earlier bound. | Test (TC-1093) |

## Dependencies

- FR-394 (the facade this extends), FR-255 (the setting names), FR-263 (the
  stage-limit entries a request carries), FR-060 and ADR-011 (the T-12
  boundary).
