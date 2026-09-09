---
id: FR-016
title: "Check native types and prove guarded definedness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
---
# FR-016: Check native types and prove guarded definedness

## Description

When a native linked unit is checked against exact source and clause bindings, the native checker shall establish its native types and the definedness of every potentially evaluated subexpression before returning a checked package.

## Inputs

An owned LinkedPackage from link_native, a CheckBindings value containing its
exact FormalSource and complete authored clause/requirement/execution-point
correspondence, and caller-lowered CheckLimits. No runtime sample participates
in type inference or proof. The concrete contract and proof abstraction are
defined in [native-model-checking.md](../../docs/native-model-checking.md).

## Outputs

An immutable CheckedPackage retaining the original parsed/linked unit, native
expression types, exact authored clause bindings, source-to-proof correspondence,
discharged IR proof goals and required runtime observations/population validation.
Failure returns a located Box<Diagnostic> with no checked package. The new check
phase distinguishes ill_typed, undefined_expression, wrong_snapshot and existing
resource_exhausted; existing linking diagnostics retain their meanings.

## Behavior

The native checker shall solve native expression type constraints using exact resolved declaration identities before checking definedness.

If a scalar expression has no unique declared contextual type, then the native checker shall refuse it with ill_typed.

The native checker shall check names, types, observation availability and admitted features in every branch, including unreachable branches.

When a potentially evaluated unwrap or arithmetic operation is checked, the native checker shall discharge its definedness through the existing IR check_expression API under the actual preceding guard conditions.

If the selected proof cannot establish definedness, then the native checker shall return undefined_expression without a logical counterexample.

The native checker shall retain distinct structured identities for values from different observations and lexical bindings.

If a clause root is not Boolean, then the native checker shall refuse the clause with ill_typed.

The native checker shall preserve the original native AST and authored implication locations in its checked result.

Native type constraints cover field receivers, exact nominal/unit equality,
Boolean operators, numeric operators, options, references, sequence elements,
conditionals, lexical bindings and clause roots. Integer/text literals and size
receive their type from these constraints; no model-wide first-fit search or
machine default occurs. size requires an explicit dimensionless integer whose
bounds include every length through the authored sequence maximum. It is not
implicitly converted from IR Length's different integer type.

Definedness follows left-to-right short circuiting and selected conditional
branches. Later guards cannot justify an earlier initializer or operand. At
alternative joins the proof retains only consequences common to all alternatives;
it never unions mutually exclusive presence facts. Syntactically proven Boolean
unreachability can remove definedness goals, but never type/name checking.

Stable proof keys contain exact declaration/receiver/observation or immutable
lexical identity, not source text. A stable-path let alias preserves its captured
key; a compound optional initializer receives its own lexical key. pre(alias)
does not retag the captured value. pre is post-only; result is post-only and is
resolved through the selected operation. The new model's operation parameters
are immutable invocation inputs captured at pre; result is captured at post.
Nested shadowing and duplicate active local names refuse.

The proof is a conservative symbolic abstraction over validated native inputs.
IR owns numeric ranges and guard discharge. Native object/enum/record identity
tests and graph results can be unknown symbolic Booleans for definedness; this
does not lower their execution semantics. Unsupported shapes and input validation
requirements remain explicit. A checked package is not an IR executable package
and contains no healthy/violating assessment of an unsupplied population.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-016-AC-1 | Real rule-model cases distinguish unguarded/right-guard/alternative-join unwraps from implication and conditional guards, preserving exact refusal loci and upstream proof diagnostics. | Test (TC-025, TC-053) |
| FR-016-AC-2 | Actual postcondition cases distinguish pre/post presence and preserve captured aliases; result/pre outside their admitted observations refuse wrong_snapshot. | Test (TC-026, TC-046) |
| FR-016-AC-3 | Guarded Version addition discharges through the actual IR prover; absent/weakened guards, possible zero divisors and signed-minimum/-1 fail definedness, while specified signed division/remainder constants are accepted. | Test (TC-027, TC-047) |
| FR-016-AC-4 | Ambiguous literals and context-free size refuse; an explicit Count context and unambiguous contextual literal constraints are accepted without nominal or unit coercion. | Test (TC-028, TC-048) |
| FR-016-AC-5 | Numeric roots and mismatched conditional branches refuse; Boolean comparisons and the actual Boolean step result postcondition are accepted. | Test (TC-029, TC-048) |
| FR-016-AC-6 | Missing/duplicate/foreign clause/source bindings and mismatched execution anchors refuse before exposing a checked package; successful output retains all original AST/source and authored clause identities. | Test (TC-049) |
| FR-016-AC-7 | Immutable let/quantifier scopes and unreachable-branch checks obey the native rules; repeated compound optional text does not create a shared guard fact. | Test (TC-050, TC-053) |
| FR-016-AC-8 | Caller/hard budgets bound native checking, proof graph construction and actual IR expansion before work; exact and one-over controls return success or resource_exhausted respectively, with no partial package. | Test (TC-051) |
| FR-016-AC-9 | Accepted reference, reachability and operation clauses expose their required population/observation/context validation without fabricating runtime values or an executable backend projection. | Test (TC-052) |

## Dependencies

- [FR-006](FR-006-check-defined-expressions.md) owns the five existing static-judgment criteria.
- [FR-015](FR-015-project-native-model-semantics.md) supplies native model roles.
- [FR-014](FR-014-bind-native-formal-source.md) maps proof loci to exact source.
- [FR-007](FR-007-validate-runtime-inputs.md) is the later input-validation obligation, not a prerequisite implementation for static proofs conditional on valid input.
- [NFR-005](../non-functional/NFR-005-rust-verification-paths.md) requires Rust verification paths.
- [IT-005](../integration/IT-005-qualify-native-model-consumption.md) supplies real, separately qualified rule-model inputs.

## Status

Specified for implementation. TC-025–029 retain their full reference/operation
semantics; a record-only arithmetic demonstration cannot discharge them.
