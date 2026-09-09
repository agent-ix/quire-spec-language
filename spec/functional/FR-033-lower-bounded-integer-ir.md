---
id: FR-033
title: "Export bounded integer expressions through the existing IR"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: references
---
## Description

When integer-ir/v1 is selected, the compiler shall lower checked Boolean and bounded integer expressions through the existing strict executable IR binder.

## Inputs

A NativePackage, an explicit lowering target and the existing caller-lowered
node/depth/byte limits. Existing lower calls and the standalone lower command
default to boolean-oracle/v1. The optional command suffix
`--target integer-ir/v1` selects this IR-only target; unknown names refuse
before file intake. The request remains native-compile/1.
The existing Rust standalone fixture generator emits integer-healthy and
integer-violating source/run requests with a bounded amount declaration.

## Outputs

The existing NativeProjection retains its selected target, native package,
actual IR executable-projection/v1 bytes, strict BoundPackage and direct-read
correspondence. Integer bounds, signed domain and reject-overflow policy come
from the checked nominal scalar's actual representation. Nominal scalar names
and units remain in the originating native model identity; the IR projection
does not replace that authority. This target does not assert codegen acceptance.

## Behavior

The compiler shall reuse the existing complete-package lowering and strict binder.
The integer target shall translate Boolean/integer literals, direct linked state/input reads, parentheses, Boolean operators, numeric negation, integer arithmetic and scalar comparisons using the existing IR constructors.
The compiler shall retain operand order, authored clauses, source coordinates and declaration observation correspondence.
The compiler shall serialize integer declarations and literals in the existing flattened IR wire shape.
The compiler shall retain the actual IR definedness judgment and any upstream refusal.
If any clause contains fields, objects/references, collections, calls, local bindings or another unadmitted form, then the compiler shall refuse the complete projection without partial output.
The compiler shall preserve boolean-oracle/v1 behavior and its existing numeric refusals.
The standalone command shall emit exact selected projection bytes on success and the selected target in lowering-failure diagnostics.
If work limits are exhausted, then the compiler shall return incomplete without an artifact.
The compiler shall use fresh limits for each request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-033-AC-1 | The actual strict IR reader reconstructs integer literals, direct declarations, arithmetic/comparisons and source-bound guarded definedness with exact signed bounds and operator order. | Test |
| FR-033-AC-2 | State/pre/post and captured-input correspondence retain original model/declaration identities; changed scalar bounds alter the relevant native and bound identities. | Test |
| FR-033-AC-3 | The Boolean default still refuses numeric clauses; unsupported later clauses and node/depth/byte exhaustion return no partial projection, and fresh retries succeed. | Test |
| FR-033-AC-4 | Actual command target selection exports exactly the library's bytes, identifies integer lowering failures and rejects unknown targets before dependent file access; runnable integer fixtures produce true/false natively and identical projection bytes, while existing codegen explicitly refuses numeric projection. | Test |

## Dependencies

- [FR-009](FR-009-lower-qualified-projections.md): existing complete projection/binder.
- [FR-029](FR-029-export-executable-projection.md): existing standalone lower command.
