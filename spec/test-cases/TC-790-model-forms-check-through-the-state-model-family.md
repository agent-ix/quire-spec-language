---
id: TC-790
title: "Each model form types through StateModel and refuses with a StateModel cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-300
    type: verifies
---
# TC-790: Each model form types through StateModel and refuses with a StateModel cause

## Description

Verify that the five model expression forms type through the `StateModel`
family's `check` hook. Scope: FR-300-AC-1, FR-300-AC-2.

The population spelling and query types come from QSpec FR-339's incorporated
complete-V1 grammar and FR-153-AC-4, FR-153-AC-8 and FR-153-AC-9. Grammar
controls below exercise FR-339-AC-1 and FR-339-AC-4; syntax refusals precede
S3 and are not asserted to be `StateModel` checking causes.

Catches a form left in the `Value` typer: its refusal would carry
`FamilyKind::Value` and the `value` prefix.

## Test Procedure

1. Compile one unit per form, well-typed: a field read, `allInstances<T>(p)`
   over `p: Population<T>[3]`, `lookup<T>(p, r) absent undefined` with
   `r: Reference<T>`, `reaches` inside a clause, and a dispatched query call
   `receiver.member(args)` in a precondition. Supply the actual admitted model
   declarations, population binding and query operation body required by
   QSpec FR-151 and FR-153; do not manufacture a population from a type name.
2. Compile one unit per form, ill-formed: a read of an undeclared field, an
   `allInstances` over an undeclared type, a `lookup` whose key's static
   type does not conform to `T`, a `reaches` over an undeclared field, a
   dispatched call to an undeclared operation.
3. Repeat the population positives with maximum `0` and with no maximum;
   repeat lookup with each of `absent undefined`, `absent empty` and
   `absent refused`, including a conforming subtype reference key.
4. Apply the following source controls to otherwise admitted units. These
   are source fragments, not runnable fixtures or new parser APIs. Import
   model alias `M` explicitly, with `M::Object` an object type,
   `M::Amount` a non-object value type and `M::Objects` an existing population
   declaration. Bind `p` and `r` explicitly where used. Preserve each
   fragment's original bytes and assert the refusal's exact source locus.

| Source control | Expected result / locus |
| --- | --- |
| `p: Population<M::Object>[3]` | Admitted bounded population type; `allInstances<M::Object>(p)` has type `Set<Reference<M::Object>>[0,3]`. |
| `p: Population<M::Object>[0]` | Admitted zero maximum; result type is `Set<Reference<M::Object>>[0,0]`. |
| `p: Population<M::Object>` | Admitted unbounded population type; result type is `Set<Reference<M::Object>>`, with no inferred finite maximum. |
| `p: Population<M::Object>[123456789012345678901234567890]` | Valid unsigned source spelling; syntax imposes no invented machine-integer bound. Any later resource/admission refusal retains its actual stage and locus. |
| `p: Population<M::Object>[]` | Located syntax refusal at the closing `]`, where `uint` is required. |
| `p: Population<M::Object>[-1]` | Located syntax refusal at `-`; a maximum is unsigned. |
| `p: Population<M::Object>[03]` | Located lexical/syntactic refusal on the leading-zero spelling `03`. |
| `p: Population<M::Object>[1.5]` or `[1e3]` | Located numeric-form/syntax refusal on the fractional or exponent spelling; neither supplies `uint`. |
| `p: Population<M::Object>[*]` | Located syntax refusal at `*`; unboundedness is spelled by omitting the brackets. |
| `p: Population<M::Object>[0,3]` | Located syntax refusal at `,`; a population maximum is one bound, not collection bounds. |
| `p: Population<M::Object[3]` | Located syntax refusal at `[`, where `>` is required. |
| `p: Population<M::Object>[3` followed by EOF | Located syntax refusal at EOF, where `]` is required. |
| `p: Population<Boolean>[3]` | Located syntax refusal at `Boolean`; a primitive terminal is not the qualified-name target production. |
| `p: Population<M::Amount>[3]` | Recognized syntax, then located `ill_typed`/`type-mismatch` on the non-object target under FR-153. |
| `p: Population<M::Missing>[3]` with no such declaration | Located unresolved model declaration; never an invented object type or population. |
| `p: population<M::Object>[3]` | Located syntax refusal at `<`; lowercase `population` is an identifier here, not the type constructor. |
| `predicate Population using S (): Boolean { true }` | Located syntax refusal at the reserved native binder `Population`. |
| `predicate population using S (): Boolean { true }` | Admitted native binder; lowercase `population` remains an identifier in this position. |
| `M::Population` or `receiver.Population` with the corresponding model member declared | Contextual member spelling is admitted; resolve the model member by its actual declared kind. |
| `abstraction A using S { population M::Objects to "app::objects"; }` | Existing lowercase abstraction population binding; no uppercase type constructor or runtime population is created by this declaration. |
| `lookup<M::Object>(p, r) absent undefined` or `absent refused` | Static type `Reference<M::Object>`. |
| `lookup<M::Object>(p, r) absent empty` | Static type `Option<Reference<M::Object>>`. |
| `lookup<M::Object>(p, r)` followed by its enclosing delimiter | Located syntax refusal at that delimiter, where mandatory `absent` is required. |
| `lookup<M::Object>(p, r) absent` followed by its enclosing delimiter | Located syntax refusal at that delimiter, where an absence mode is required. |
| `lookup<M::Object>(p, r) absent other` | Located syntax refusal at `other`; no inferred/default absence mode. |
| `allInstances<M::Object>(x)` with an admitted non-population `x` | S3 `ill_typed`/`operator-ineligible`, owned by `StateModel`. |

5. Keep lexical boundary controls with separating whitespace and `//` comments
   between tokens, and qualified member names containing reserved words;
   preserve all original spans. Keep the five ill-formed units from step 2
   independently of the grammar controls, so every model form still exercises
   the `StateModel` refusal path.

## Expected Results

1. Every well-typed unit from step 1 compiles; the checked nodes are `Attribute`, `AllInstances`
   typed `Set<Reference<T>>[0,3]`, `Lookup` typed `Reference<T>`, `Reaches` and a dispatched
   call, each with the static type QSpec FR-151 and FR-153 fix.
2. Every ill-formed unit from step 2 refuses at S3. Each cause has family `FamilyKind::StateModel`,
   a catalog code beginning `state-model`, and the QSpec code and cause the
   expected table in the test states as literals (`missing_declaration`/
   `missing-name` for the absent members and operation, `ill_typed`/
   `type-mismatch` for the `lookup` key).
3. Zero-bound and unbounded populations produce the exact types stated above;
   lookup's three modes produce the FR-153 static types without reading a
   runtime population during checking. Present/absent runtime behavior remains
   owned by FR-153 and its existing cases; successful source checking does not
   establish closure or create members.
4. Every negative source control refuses at its stated syntax or semantic
   boundary, with original source location and no successful checked declaration.
   No syntax failure is relabeled as an S3 family refusal. Valid controls retain
   their selected facet and contextual case rules; no provider installation,
   ambient registry or result type supplies a missing binding or absence mode.
