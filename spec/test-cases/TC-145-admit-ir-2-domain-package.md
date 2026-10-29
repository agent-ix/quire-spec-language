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
FR-056-AC-1, FR-056-AC-2, FR-056-AC-4, FR-056-AC-5, FR-056-AC-7,
FR-056-AC-12, FR-056-AC-13, FR-056-AC-14, FR-056-AC-15, FR-056-AC-44,
FR-056-CON-4.

## Test Procedure

1. Lift a small bundle through `agent-ix-extraction-frontend`: one object type
   (`quire.meaning.model.object-type/v1`) with a field member, a field member
   typed by an object type, an operation member and a relationship member with a
   declared name; one value type; one variant type; and one population.
   Pass the lifted bytes and a ModelSelection naming their identity and version to the intake seam.
2. Offer, one at a time: a package input with no bytes under the selected
   identity; bytes whose own identity or version differs from the selection.
3. Offer bytes `agent-ix-semantic-ir` refuses. Then parse documents carrying
   a lone high surrogate escape, a lone low surrogate escape, a reversed pair,
   a lone surrogate in a member name. Parse documents carrying `1e400`,
   `-1e400` and `1e-400`, at the top level and nested, and offer them to
   admission. Parse and offer `{"a":1,"a":2,"n":1e400}`,
   `[1e400`, `{"a":1,"a":2,"n":1e-400}` and `{"x":[1e-400,1e400]}`, and
   `1` and 400 zeros then `.5` and `e-1`. Offer a lone-surrogate
   document, and `not json` with an empty identity and version. Offer a document over `intake.input_bytes`.
   Parse documents carrying, at `/package/count`, `18446744073709551616`,
   `-18446744073709551616`, `1e20`, `9.007199254740993e15`,
   `9007199254740993` and `-9007199254740993`; one carrying `1e20` at
   `/a~1b/0/c~0d`; one carrying such numbers at `/b` and then `/a/0`; and the
   same document carrying `9007199254740992` and `-9007199254740992`. Parse
   documents carrying, at `/package/count`, `9007199254740993.5`,
   `0.1000000000000000000001`, `1e-400`, `-1e-400`, `4.9e-324`, `1e20` and
   `9007199254740993`; one carrying `0.5` at `/a/0` and `1e-400` at `/a/1`;
   and the same document carrying `0.1`, `1.0`, `-0`, `-0.0`, `5e-324`,
   `1e15` and `9007199254740991`. Parse documents carrying
   `1125899906842624.2`, `1500000000000000.2` and `2.9802322387695312e-8`
   (each the even-digit text of a double with two shortest texts), and
   `1125899906842624.3`, `1500000000000000.3` and `2.9802322387695313e-8`;
   and one carrying `1.5e-300`, whose tree holds the double `1.5e-300` reads
   as. Offer
   documents carrying `18446744073709551615`, `18446744073709551616`,
   `123456789012345678901234567890` and `-9007199254740993`; a document
   carrying a multiplicity `upper` of 2^60 + 1; and documents carrying
   `0.1000000000000000000001` and `0.1`.
   Parse documents repeating a member name within one object, and one that
   begins with a byte order mark. Offer a repeated-name document.
4. Change only one artifact's `title` or `displayName` and re-run. Then give two
   artifacts equal titles. Then change the step 1 object-type artifact's id to
   `sys_pump` and name it from a clause as `M::sys_pump`, and separately to
   `sys-pump`, keeping the step 1 object-typed field member typed by that type
   and adding a second object type `Housing` whose field member `pump` is typed
   by that type.
   Then change the identity's final segment to `-sys_pump` and update
   references to it; separately use `sys-pump` with a `kind` that names no
   `constructs` entry.
5. Remove the relationship member's declared name, then separately its source
   span. Then point the object-typed field member at an artifact id absent from
   the package, and separately point the relationship's target end at such an id.
6. Combine one refused node with the valid nodes of step 1.
7. (Retired with FR-056-AC-7: the package selection has no slot to offer.)
8. Lift a business bundle through FCD with model-declaring object artifacts
   `FR-001` and `OP-001`; give the former an underscore-named field and the
   latter a field referring to `FR-001`, plus an owner operation and a
   parameter whose names contain underscores. Admit the resulting package.

## Expected Results

- Step 1 yields exactly one declaration per IR node ascending by (package
  identity, IR node identity); each type's IR node identity equals its artifact
  id; exports are one `object`; from its members, `field` records, one `reference`,
  one `operation` and one `relationship` carrying the declared name and span;
  one `scalar` naming the value type and its native value type; one `enum` with
  its `variant` records; and one `population`.
- Step 2 refuses with `missing_import`/`missing-selection` and
  `invalid_model_binding`/`wrong-model-selection` respectively, with no
  declaration.
- Step 3 admits no declaration and retains every reader diagnostic with its IR
  node, artifact id and span. The parse refuses each surrogate case. It refuses `1e400` and `-1e400`
  `noncanonical_wire`/`inexact-integer` and `1e-400`
  `noncanonical_wire`/`inexact-number`, each with its `document_pointer`, and
  admission does the same.
  `{"a":1,"a":2,"n":1e400}` and `[1e400` refuse `inexact-integer` at `/n` and
  `/0`; `{"a":1,"a":2,"n":1e-400}` is a repeated-name document, malformed at
  the parse; `{"x":[1e-400,1e400]}` names `/x/1`,
  ahead of the earlier inexact number. 1 then 400 zeros then `.5` refuses
  `inexact-number`, and 1 then 400 zeros then `e-1` `inexact-integer`.
  The lone-surrogate document and `not json` refuse `invalid_model_binding`/`malformed-declaration`. Each
  limit case refuses `resource_exhausted`/`intake-limit-exceeded` naming its
  limit. The parse refuses each whole number beyond ±2^53
  `noncanonical_wire`/`inexact-integer` with `document_pointer` naming it
  (`/package/count`, `/a~1b/0/c~0d`, and `/b`, the first in document order),
  and admits ±2^53. It refuses `9007199254740993.5`,
  `0.1000000000000000000001`, `1e-400`, `-1e-400` and `4.9e-324`
  `noncanonical_wire`/`inexact-number` at `/package/count`, `1e20` and
  `9007199254740993` (which fits both causes) `inexact-integer`, and names
  `/a/1` in the two-number document; it admits `0.1`, `1.0`, `-0`, `-0.0`,
  `5e-324`, `1e15` and `9007199254740991`. No large-integer document admits: admission refuses
  `noncanonical_wire`/`inexact-integer` before check 3, as it does the
  2^60 + 1 bound. The `0.1000000000000000000001` document refuses
  `noncanonical_wire`/`inexact-number` and the `0.1` document admits. The
  parse refuses each repeated-name
  document at the repeated name's byte offset and the byte-order-mark
  document at byte 0, and the repeated-name document refuses
  `invalid_model_binding`/`malformed-declaration`.
- Step 4: the title and displayName runs leave every key, export, ordering and
  linker binding byte-identical; equal titles stay two distinct declarations.
  The id runs change the type's key and its members' keys (FR-154-AC-6): the
  `sys_pump` run admits the type keyed `ix://<package identity>/sys_pump` and
  the clause resolves to it;
  the `sys-pump` run admits the type keyed
  `ix://<package identity>/sys-pump`, reports no
  `missing_declaration`/`missing-name` for either field member that references
  it, and keeps those references bound to the admitted type; the `-sys_pump`
  run refuses that node with `invalid_model_binding`/`malformed-declaration`,
  naming node, artifact and span, and admits no declaration of the package;
  the `sys-pump` run with the unknown kind refuses only that kind as
  `invalid_model_binding`/`malformed-declaration` (FR-154-AC-8).
- Step 5 refuses the unnamed and unspanned relationship members with
  `invalid_model_binding`/`malformed-declaration`, and both dangling targets with
  `missing_declaration`/`missing-name`, each naming node, artifact and span.
- Step 6 reports every refusal in FR-154's declaration refusal order (node
  order, and within one node FR-154's table order) and admits no declaration of
  the package.
- Step 7 is retired.
- Step 8 runs the actual FCD lift and QSL intake path. It admits declarations
  keyed by `ix://<package identity>/FR-001` and
  `ix://<package identity>/OP-001`; the underscore-named field, operation and
  parameter identities are preserved verbatim, and the field reference from
  `OP-001` resolves to `FR-001` (FR-056-AC-44).
- Assertions compare typed refusal codes and loci, never message text.
