---
id: FR-022
title: "Compile mapped native clauses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
---
# FR-022: Compile mapped native clauses

## Description

When a caller supplies a verified source map for one native clause, the compiler shall parse, link, check and package its exact body while retaining the original document correspondence and supplied authored identity.

## Inputs

A verified SourceMap, declared language, one existing ClauseBinding, explicit
formal body-source identity, immutable native models, and caller-lowered limits
for the existing parse/link/check/package stages. The body contains a complete
native unit with one clause; no generated header or wrapper is inserted.

## Outputs

An immutable mapped native package, or a typed failure retaining the supplied
clause identity, verified map and original native/package cause.

## Behavior

The compiler shall admit only the declared language `ix:native` and then run
the existing parser rather than treating a language tag as successful parsing.
The compiler shall require exactly one native clause with the supplied binding's
name and preserve the binding's authored requirement, clause and execution point.
The compiler shall use the existing native linker, checker and package constructor.
If a stage fails, then the compiler shall return no partial package and retain
the actual failure stage, code and cause.
The compiler shall map native diagnostic byte spans back through the retained
map without replacing their body coordinates or collapsing discontiguous regions.
The compiler shall preserve package-encoding paths without inventing body spans.
The compiler shall apply the existing stage ceilings with fresh counters per call.

The caller owns original document/digest/region selection and verification before
SourceMap construction. C owns Quire extraction, wire decoding and availability
metadata; this API does not change that metadata or claim complete IT-003 adoption.
Runtime validation/evaluation and optional Boolean lowering consume the resulting
native package through their existing APIs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-022-AC-1 | A mapped complete native clause compiles and retains exact body, original document and authored bindings. | Test |
| FR-022-AC-2 | Wrong language, malformed body, extra clauses and a foreign native clause name refuse without producing a package. | Test |
| FR-022-AC-3 | A native compiler failure retains its code, body location and exact original regions, including Unicode and deleted layout. | Test |
| FR-022-AC-4 | Link/check/package exhaustion retains its actual typed cause; retrying the same mapped request starts fresh. | Test |
| FR-022-AC-5 | A mapped parent/aggregate clause reaches existing runtime evaluation with healthy, violating and refused inputs and unchanged authored identity. | Test |

## Dependencies

- [FR-004](FR-004-verify-source-maps.md): verified exact body/original mapping.
- [FR-019](FR-019-package-checked-native-clauses.md): checked package construction.
- [FR-011](FR-011-integrate-opaque-extraction.md): C-owned extraction adoption.
