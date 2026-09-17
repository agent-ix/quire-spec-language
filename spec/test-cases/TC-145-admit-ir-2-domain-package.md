---
id: TC-145
title: "Admit a Semantic IR 2.0.0 domain package as model declarations"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-145: Admit a Semantic IR 2.0.0 domain package as model declarations

## Description

Verify FR-154 admission, reader refusal, artifact-id identity, relationship
exports and all-or-nothing declaration refusal through the intake seam. Scope:
FR-056-AC-1, FR-056-AC-2, FR-056-AC-4, FR-056-AC-5, FR-056-AC-7 (digest-slot
refusal), FR-056-CON-4.

## Test Procedure

1. Lift a small bundle through `agent-ix-extraction-frontend`: one object type
   (`quire.meaning.model.object-type/v1`) with a field member, a field member
   typed by an object type, an operation member and a relationship member with a
   declared name; one value type; one variant type; and one population.
   Pass the lifted bytes and a ModelSelection naming their identity, version and
   independently computed `sha256-jcs` digest to the intake seam.
2. Offer, one at a time: a selection whose digest domain is not `sha256-jcs`; a
   package input with no bytes under the selected digest; bytes whose recomputed
   digest differs; bytes whose own identity or version differs from the
   selection. Then offer a combination of the first and third faults.
3. Offer bytes `agent-ix-semantic-ir` refuses.
4. Change only one artifact's `title` or `displayName` and re-run. Then give two
   artifacts equal titles. Then change the step 1 object-type artifact's id to
   `sys_pump` and name it from a clause as `M::sys_pump`, and separately to
   `sys-pump`, keeping the step 1 object-typed field member typed by that type
   and adding a second object type `Housing` whose field member `pump` is typed
   by that type.
   Then change the id to `sys-pump` and also point its `kind` at a name with no
   `constructs` entry.
5. Remove the relationship member's declared name, then separately its source
   span. Then point the object-typed field member at an artifact id absent from
   the package, and separately point the relationship's target end at such an id.
6. Combine one refused node with the valid nodes of step 1.
7. Offer the admitted package's `sha256-jcs` digest in a raw-byte artifact digest
   slot and in a compiled-artifact digest slot of a native package selection.

## Expected Results

- Step 1 yields exactly one declaration per IR node ascending by (package
  identity, IR node identity); each type's IR node identity equals its artifact
  id; exports are one `object`; from its members, `field` records, one `reference`,
  one `operation` and one `relationship` carrying the declared name and span;
  one `scalar` naming the value type and its native value type; one `enum` with
  its `variant` records; and one `population`.
- Step 2 refuses with `stale_dependency`/`digest-domain-mismatch`,
  `missing_import`/`missing-selection`, `stale_dependency`/`byte-digest-mismatch`
  and `invalid_model_binding`/`wrong-model-selection` respectively, with no
  declaration; the combined case reports `digest-domain-mismatch`, the first
  check in FR-154 order.
- Step 3 admits no declaration and retains every reader diagnostic with its IR
  node, artifact id and span.
- Step 4: the title and displayName runs leave every key, export, ordering and
  linker binding byte-identical; equal titles stay two distinct declarations.
  The id runs change the type's key and its members' keys (FR-154-AC-6): the
  `sys_pump` run admits the type keyed `ix://<package identity>/sys_pump` and
  the clause resolves to it;
  the `sys-pump` run refuses that node exactly once with
  `invalid_model_binding`/`malformed-declaration`, naming node, artifact and
  span, reports no `missing_declaration`/`missing-name` for the step 1
  object-typed field member or for `Housing`'s `pump` field member, both of
  which reference it, and admits no declaration of the package;
  the `sys-pump` run with the unknown kind refuses that node twice with
  `invalid_model_binding`/`malformed-declaration`, the object id refusal first
  and the unknown-kind refusal second (FR-154-AC-8).
- Step 5 refuses the unnamed and unspanned relationship members with
  `invalid_model_binding`/`malformed-declaration`, and both dangling targets with
  `missing_declaration`/`missing-name`, each naming node, artifact and span.
- Step 6 reports every refusal in FR-154's declaration refusal order (node
  order, and within one node FR-154's table order) and admits no declaration of
  the package.
- Step 7 refuses both substitutions; the digest is accepted only in its
  `sha256-jcs` slot.
- Assertions compare typed refusal codes and loci, never message text.
